use crate::app_state::AppState;
use crate::error::{IncidentError, IncidentResult};
use crate::services::incident::IncidentService;
use crate::services::timeline::TimelineService;
use crate::slack::blocks;
use crate::slack::events::SlashCommandPayload;
use tracing::info;

pub async fn handle_timeline(state: AppState, payload: SlashCommandPayload) -> IncidentResult<()> {
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

    // Get timeline
    let timeline_service = TimelineService::new(state.pool.clone());
    let events = timeline_service.get_timeline(incident.id).await?;

    // Format and post timeline
    let timeline_blocks = blocks::timeline_blocks(&events);

    // Post to incident channel (visible to everyone)
    if let Some(channel_id) = &incident.slack_channel_id {
        state
            .slack_client
            .post_message(channel_id, timeline_blocks)
            .await?;
    }

    info!("Timeline displayed for incident {}", incident.id);

    // Acknowledge via response_url
    state
        .slack_client
        .post_to_response_url(
            &payload.response_url,
            vec![serde_json::json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "✅ Timeline posted to channel"
                }
            })],
        )
        .await
}
