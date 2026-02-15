use crate::app_state::AppState;
use crate::error::{IncidentError, IncidentResult};
use crate::services::incident::IncidentService;
use crate::services::notification::NotificationService;
use crate::slack::blocks;
use crate::slack::events::SlashCommandPayload;
use tracing::{error, info};

pub async fn handle_status(state: AppState, payload: SlashCommandPayload) -> IncidentResult<()> {
    // Extract message from command text (everything after "status")
    let parts: Vec<&str> = payload.text.splitn(2, ' ').collect();
    let message = if parts.len() > 1 {
        parts[1].trim()
    } else {
        return state
            .slack_client
            .post_to_response_url(
                &payload.response_url,
                blocks::error_blocks("Usage: /incident status [message]"),
            )
            .await;
    };

    if message.is_empty() {
        return state
            .slack_client
            .post_to_response_url(
                &payload.response_url,
                blocks::error_blocks("Status message cannot be empty"),
            )
            .await;
    }

    // Get incident from channel
    let incident_service = IncidentService::new(state.pool.clone());
    let incident = match incident_service.get_by_channel(&payload.channel_id).await {
        Ok(inc) => inc,
        Err(IncidentError::NotFound) => {
            return state
                .slack_client
                .post_to_response_url(
                    &payload.response_url,
                    blocks::error_blocks("No active incident in this channel"),
                )
                .await;
        }
        Err(e) => return Err(e),
    };

    // Validate commander
    if let Err(IncidentError::PermissionDenied { .. }) =
        incident_service.validate_commander(&incident, &payload.user_id).await
    {
        return state
            .slack_client
            .post_to_response_url(
                &payload.response_url,
                blocks::permission_denied_blocks("post status updates"),
            )
            .await;
    }

    // Post status update
    let updated_incident = incident_service
        .post_status_update(incident.id, message.to_string(), payload.user_id.clone())
        .await?;

    // Post to channel
    let status_blocks = blocks::status_update_blocks(
        updated_incident.severity,
        message,
        &payload.user_id,
    );

    if let Some(_channel_id) = &updated_incident.slack_channel_id {
        let notification_service = NotificationService::new(
            state.pool.clone(),
            state.slack_client.clone(),
            state.config.clone(),
        );

        if let Err(e) = notification_service
            .notify_status_update(&updated_incident, status_blocks)
            .await
        {
            error!("Failed to post status update: {}", e);
        }
    }

    // Enqueue Statuspage sync if component mapping exists
    if let Ok(Some(component_id)) = crate::db::queries::statuspage::get_component_id(&state.pool, &updated_incident.affected_service).await {
        let job = crate::jobs::Job::StatuspageSync {
            incident_id: updated_incident.id,
            component_id,
            status: updated_incident.status,
            severity: updated_incident.severity,
        };

        if let Err(e) = state.job_sender.send(job) {
            error!("Failed to enqueue Statuspage sync job: {}", e);
        }
    }

    info!(
        "Status update posted for incident {} by {}",
        incident.id, payload.user_id
    );

    // Acknowledge via response_url
    state
        .slack_client
        .post_to_response_url(
            &payload.response_url,
            vec![serde_json::json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "✅ Status update posted"
                }
            })],
        )
        .await
}
