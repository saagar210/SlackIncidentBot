use crate::app_state::AppState;
use crate::error::{IncidentError, IncidentResult};
use crate::services::incident::IncidentService;
use crate::services::postmortem::PostmortemService;
use crate::slack::blocks;
use crate::slack::events::SlashCommandPayload;
use serde_json::json;
use tracing::info;

pub async fn handle_postmortem(state: AppState, payload: SlashCommandPayload) -> IncidentResult<()> {
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

    // Check if incident is resolved
    if !incident.status.is_terminal() {
        return state
            .slack_client
            .post_to_response_url(
                &payload.response_url,
                blocks::error_blocks(
                    "Incident must be resolved before generating postmortem. Use `/incident resolved` first.",
                ),
            )
            .await;
    }

    // Generate postmortem
    let postmortem_service = PostmortemService::new(state.pool.clone());
    let postmortem_md = postmortem_service.generate(&incident).await?;

    // Post postmortem as code block
    let postmortem_blocks = vec![
        json!({
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": "📋 Incident Postmortem Draft",
            }
        }),
        json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("```\n{}\n```", postmortem_md)
            }
        }),
        json!({
            "type": "context",
            "elements": [{
                "type": "mrkdwn",
                "text": "_Edit this template and add action items, root cause analysis, and lessons learned._"
            }]
        }),
    ];

    // Post to incident channel
    if let Some(channel_id) = &incident.slack_channel_id {
        state
            .slack_client
            .post_message(channel_id, postmortem_blocks)
            .await?;
    }

    info!("Postmortem generated for incident {}", incident.id);

    // Acknowledge via response_url
    state
        .slack_client
        .post_to_response_url(
            &payload.response_url,
            vec![json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "✅ Postmortem draft posted to channel"
                }
            })],
        )
        .await
}
