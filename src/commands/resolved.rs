use crate::app_state::AppState;
use crate::error::{IncidentError, IncidentResult};
use crate::services::incident::IncidentService;
use crate::services::notification::NotificationService;
use crate::slack::blocks;
use crate::slack::events::SlashCommandPayload;
use tracing::{error, info};

pub async fn handle_resolved(state: AppState, payload: SlashCommandPayload) -> IncidentResult<()> {
    // Get incident from channel
    let incident_service = IncidentService::new(state.pool.clone());
    let incident = match incident_service
        .get_latest_by_channel(&payload.channel_id)
        .await
    {
        Ok(inc) => inc,
        Err(IncidentError::NotFound) => {
            return state
                .slack_client
                .post_to_response_url(
                    &payload.response_url,
                    blocks::error_blocks("No incident found in this channel"),
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
                blocks::permission_denied_blocks("resolve the incident"),
            )
            .await;
    }

    // Check if already resolved
    if incident.status.is_terminal() {
        return state
            .slack_client
            .post_to_response_url(
                &payload.response_url,
                vec![serde_json::json!({
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": "✅ Incident is already resolved"
                    }
                })],
            )
            .await;
    }

    // Resolve incident
    let resolved_incident = incident_service
        .resolve_incident(incident.id, payload.user_id.clone())
        .await?;

    // Post resolution to channel
    let resolution_blocks = blocks::resolution_blocks(&resolved_incident, &payload.user_id);

    if let Some(_channel_id) = &resolved_incident.slack_channel_id {
        let notification_service = NotificationService::new(
            state.pool.clone(),
            state.slack_client.clone(),
            state.config.clone(),
        );

        if let Err(e) = notification_service
            .notify_resolution(&resolved_incident, resolution_blocks)
            .await
        {
            error!("Failed to post resolution: {}", e);
        }
    }

    // Enqueue Statuspage sync if component mapping exists
    if let Ok(Some(component_id)) = crate::db::queries::statuspage::get_component_id(
        &state.pool,
        &resolved_incident.affected_service,
    )
    .await
    {
        let job = crate::jobs::Job::StatuspageSync {
            incident_id: resolved_incident.id,
            component_id,
            status: resolved_incident.status,
            severity: resolved_incident.severity,
        };

        if let Err(e) = state.job_sender.send(job) {
            error!("Failed to enqueue Statuspage sync job: {}", e);
        }
    }

    info!(
        "Incident {} resolved by {} (duration: {:?} min)",
        incident.id, payload.user_id, resolved_incident.duration_minutes
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
                    "text": "✅ Incident marked as resolved"
                }
            })],
        )
        .await
}
