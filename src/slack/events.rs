use crate::app_state::AppState;
use crate::error::IncidentResult;
use crate::slack::blocks;
use crate::slack::verification::verify_slack_signature;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info};

#[derive(Debug, Deserialize)]
pub struct SlashCommandPayload {
    pub command: String,
    pub text: String,
    pub user_id: String,
    pub channel_id: String,
    pub response_url: String,
    pub trigger_id: String,
}

#[derive(Debug, Deserialize)]
struct InteractionPayload {
    #[serde(rename = "type")]
    pub interaction_type: String,
    pub user: User,
    pub view: Option<ViewPayload>,
    pub trigger_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct User {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct ViewPayload {
    pub callback_id: String,
    pub state: ViewState,
}

#[derive(Debug, Deserialize)]
pub struct ViewState {
    pub values: serde_json::Map<String, Value>,
}

pub async fn handle_slash_command(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    // Verify Slack signature
    let signature = headers
        .get("X-Slack-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let timestamp = headers
        .get("X-Slack-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Err(e) = verify_slack_signature(&state.config.slack_signing_secret, timestamp, &body, signature) {
        error!("Signature verification failed: {}", e);
        return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
    }

    // Parse payload
    let payload: SlashCommandPayload = match serde_urlencoded::from_str(&body) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse slash command: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid request").into_response();
        }
    };

    debug!("Received slash command: {} {}", payload.command, payload.text);

    // Spawn async task to process command
    let state_clone = state.clone();
    let user_id = payload.user_id.clone();
    let command = payload.command.clone();
    let channel_id = payload.channel_id.clone();
    let response_url = payload.response_url.clone();
    tokio::spawn(async move {
        if let Err(e) = process_slash_command(state_clone.clone(), payload).await {
            error!(
                "Error processing command - user_id: {}, command: {}, channel_id: {}, error: {}",
                user_id, command, channel_id, e
            );
            // Attempt to notify user via response_url
            let error_blocks = crate::slack::blocks::error_blocks(&format!(
                "Command failed: {}",
                e
            ));
            if let Err(post_err) = state_clone.slack_client.post_to_response_url(&response_url, error_blocks).await {
                error!("Failed to post error to response_url: {}", post_err);
            }
        }
    });

    // Return 200 OK immediately (Slack's recommended ack-then-process pattern)
    // Slack requires response within 3 seconds. Processing happens asynchronously.
    // Errors are reported to user via response_url in spawned task.
    StatusCode::OK.into_response()
}

async fn process_slash_command(state: AppState, payload: SlashCommandPayload) -> IncidentResult<()> {
    let parts: Vec<&str> = payload.text.split_whitespace().collect();
    let subcommand = parts.first().copied().unwrap_or("");

    match subcommand {
        "declare" => {
            crate::commands::declare::handle_declare(state, payload).await?;
        }
        "status" => {
            crate::commands::status::handle_status(state, payload).await?;
        }
        "severity" => {
            crate::commands::severity::handle_severity(state, payload).await?;
        }
        "resolved" => {
            crate::commands::resolved::handle_resolved(state, payload).await?;
        }
        "timeline" => {
            crate::commands::timeline::handle_timeline(state, payload).await?;
        }
        "postmortem" => {
            crate::commands::postmortem::handle_postmortem(state, payload).await?;
        }
        _ => {
            let blocks = blocks::error_blocks(&format!(
                "Unknown subcommand: {}. Available: declare, status, severity, resolved, timeline, postmortem",
                subcommand
            ));
            state.slack_client.post_to_response_url(&payload.response_url, blocks).await?;
        }
    }

    Ok(())
}

pub async fn handle_interaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    // Verify Slack signature
    let signature = headers
        .get("X-Slack-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let timestamp = headers
        .get("X-Slack-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Err(e) = verify_slack_signature(&state.config.slack_signing_secret, timestamp, &body, signature) {
        error!("Signature verification failed: {}", e);
        return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
    }

    // Parse URL-encoded payload
    let form_data: std::collections::HashMap<String, String> = match serde_urlencoded::from_str(&body) {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse interaction: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid request").into_response();
        }
    };

    let payload_json = match form_data.get("payload") {
        Some(p) => p.clone(),
        None => {
            error!("Missing 'payload' field in interaction request");
            return (StatusCode::BAD_REQUEST, "Missing payload field").into_response();
        }
    };
    let payload: InteractionPayload = match serde_json::from_str(&payload_json) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse interaction payload JSON: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid request").into_response();
        }
    };

    debug!("Received interaction: {}", payload.interaction_type);

    // Spawn async task to process interaction
    let state_clone = state.clone();
    let user_id = payload.user.id.clone();
    let interaction_type = payload.interaction_type.clone();
    tokio::spawn(async move {
        if let Err(e) = process_interaction(state_clone, payload).await {
            error!(
                "Error processing interaction - user_id: {}, type: {}, error: {}",
                user_id, interaction_type, e
            );
        }
    });

    // Return 200 OK immediately
    StatusCode::OK.into_response()
}

async fn process_interaction(state: AppState, payload: InteractionPayload) -> IncidentResult<()> {
    match payload.interaction_type.as_str() {
        "view_submission" => {
            if let Some(view) = payload.view {
                if view.callback_id == "declare_incident_modal" {
                    crate::commands::declare::handle_modal_submission(state, view, payload.user.id).await?;
                }
            }
        }
        _ => {
            info!("Unhandled interaction type: {}", payload.interaction_type);
        }
    }

    Ok(())
}
