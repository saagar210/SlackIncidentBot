use crate::app_state::AppState;
use crate::db::models::Severity;
use crate::error::{IncidentError, IncidentResult};
use crate::services::incident::IncidentService;
use crate::services::notification::NotificationService;
use crate::slack::blocks;
use crate::slack::events::SlashCommandPayload;
use tracing::{error, info};

pub async fn handle_severity(state: AppState, payload: SlashCommandPayload) -> IncidentResult<()> {
    // Parse command: "/incident severity P1" or "/incident severity P2 reason text"
    let parts: Vec<&str> = payload.text.splitn(3, ' ').collect();

    let severity_str = if parts.len() > 1 {
        parts[1].trim()
    } else {
        return state
            .slack_client
            .post_to_response_url(
                &payload.response_url,
                blocks::error_blocks("Usage: /incident severity [P1|P2|P3|P4] [optional reason]"),
            )
            .await;
    };

    let new_severity: Severity = match severity_str.parse() {
        Ok(s) => s,
        Err(_) => {
            return state
                .slack_client
                .post_to_response_url(
                    &payload.response_url,
                    blocks::error_blocks("Invalid severity. Use P1, P2, P3, or P4"),
                )
                .await;
        }
    };

    let reason = if parts.len() > 2 {
        Some(parts[2].trim().to_string())
    } else {
        None
    };

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
    if let Err(IncidentError::PermissionDenied { .. }) = incident_service
        .validate_commander(&incident, &payload.user_id)
        .await
    {
        return state
            .slack_client
            .post_to_response_url(
                &payload.response_url,
                blocks::permission_denied_blocks("change incident severity"),
            )
            .await;
    }

    // Check if already at this severity
    if incident.severity == new_severity {
        return state
            .slack_client
            .post_to_response_url(
                &payload.response_url,
                vec![serde_json::json!({
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!("Incident is already {}", new_severity.label())
                    }
                })],
            )
            .await;
    }

    // Change severity
    let (updated_incident, old_severity) = incident_service
        .change_severity(
            incident.id,
            new_severity,
            payload.user_id.clone(),
            reason.clone(),
        )
        .await?;

    // Post to channel
    let severity_blocks = blocks::severity_change_blocks(
        old_severity,
        new_severity,
        &payload.user_id,
        reason.as_deref(),
    );

    if let Some(_channel_id) = &updated_incident.slack_channel_id {
        let notification_service = NotificationService::new(
            state.pool.clone(),
            state.slack_client.clone(),
            state.config.clone(),
        );

        if let Err(e) = notification_service
            .notify_severity_change(&updated_incident, old_severity, severity_blocks)
            .await
        {
            error!("Failed to post severity change: {}", e);
        }
    }

    // Enqueue Statuspage sync if component mapping exists
    if let Ok(Some(component_id)) = crate::db::queries::statuspage::get_component_id(
        &state.pool,
        &updated_incident.affected_service,
    )
    .await
    {
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
        "Severity changed for incident {} from {:?} to {:?} by {}",
        incident.id, old_severity, new_severity, payload.user_id
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
                    "text": format!("✅ Severity changed to {}", new_severity.label())
                }
            })],
        )
        .await
}
