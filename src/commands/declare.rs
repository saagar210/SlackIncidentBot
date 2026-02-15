use crate::app_state::AppState;
use crate::error::IncidentResult;
use crate::services::notification::NotificationService;
use crate::slack::blocks;
use crate::slack::events::SlashCommandPayload;
use crate::slack::modals;
use crate::utils::channel;
use chrono::Utc;
use tracing::{error, info};

pub async fn handle_declare(state: AppState, payload: SlashCommandPayload) -> IncidentResult<()> {
    // Fetch active templates
    let templates = crate::db::queries::templates::list_active_templates(&state.pool).await?;

    // Open modal with templates
    let modal = modals::declare_incident_modal(&state.config.services, &templates);
    state
        .slack_client
        .open_modal(&payload.trigger_id, modal)
        .await?;

    Ok(())
}

pub async fn handle_modal_submission(
    state: AppState,
    view: crate::slack::events::ViewPayload,
    user_id: String,
) -> IncidentResult<()> {
    // Parse modal values
    let values = &view.state.values;

    let title = values
        .get("title_block")
        .and_then(|v| v.get("title_input"))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::error::IncidentError::ValidationError {
            field: "title".to_string(),
            reason: "Required".to_string(),
        })?
        .to_string();

    let severity_str = values
        .get("severity_block")
        .and_then(|v| v.get("severity_select"))
        .and_then(|v| v.get("selected_option"))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::error::IncidentError::ValidationError {
            field: "severity".to_string(),
            reason: "Required".to_string(),
        })?;

    let severity: crate::db::models::Severity =
        severity_str
            .parse()
            .map_err(|e| crate::error::IncidentError::ValidationError {
                field: "severity".to_string(),
                reason: e,
            })?;

    let service = values
        .get("service_block")
        .and_then(|v| v.get("service_select"))
        .and_then(|v| v.get("selected_option"))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::error::IncidentError::ValidationError {
            field: "service".to_string(),
            reason: "Required".to_string(),
        })?
        .to_string();

    let commander_id = values
        .get("commander_block")
        .and_then(|v| v.get("commander_select"))
        .and_then(|v| v.get("selected_user"))
        .and_then(|v| v.as_str())
        .unwrap_or(&user_id)
        .to_string();

    if commander_id == user_id {
        info!(
            "Commander not explicitly selected, defaulting to modal submitter: {}",
            user_id
        );
    }

    info!("Declaring incident: {}", title);

    // Generate incident ID upfront (needed for channel name)
    let incident_id = uuid::Uuid::new_v4();

    // Create Slack channel FIRST (fail fast if Slack is down)
    let date = Utc::now().date_naive();
    let (channel_id, channel_name) =
        channel::create_incident_channel(&state.slack_client, &service, date, incident_id).await?;

    // Create incident in DB with channel ID
    // If this fails, we'll clean up the channel (compensation pattern)
    let incident = match sqlx::query_as::query_as::<_, crate::db::models::Incident>(
        r#"
        INSERT INTO incidents (id, title, severity, affected_service, commander_id, status, declared_at, slack_channel_id)
        VALUES ($1, $2, $3, $4, $5, 'declared', NOW(), $6)
        RETURNING *
        "#,
    )
    .bind(incident_id)
    .bind(&title)
    .bind(severity.as_db_str())
    .bind(&service)
    .bind(&commander_id)
    .bind(&channel_id)
    .fetch_one(&state.pool)
    .await
    {
        Ok(inc) => inc,
        Err(e) => {
            error!("Failed to create incident in DB, cleaning up channel: {}", e);
            // Compensation: Archive the channel we just created
            if let Err(archive_err) = state.slack_client.archive_channel(&channel_id).await {
                error!("Failed to archive channel during cleanup: {}", archive_err);
            }
            return Err(e.into());
        }
    };

    // Log to timeline
    let timeline_service = crate::services::timeline::TimelineService::new(state.pool.clone());
    timeline_service
        .log_event(
            incident.id,
            crate::db::models::TimelineEventType::Declared,
            format!("Incident declared: {}", title),
            commander_id.clone(),
        )
        .await?;

    // Log to audit
    let audit_service = crate::services::audit::AuditService::new(state.pool.clone());
    audit_service
        .log_action(
            Some(incident.id),
            "incident_declared".to_string(),
            commander_id.clone(),
            None,
            None,
            Some(serde_json::json!({
                "title": title,
                "severity": severity,
                "service": service,
            })),
        )
        .await?;

    // Invite users to channel
    let mut invitees = vec![commander_id.clone()];

    // Add service owners if configured
    if let Some(owners) = state.config.service_owners.get(&service) {
        invitees.extend(owners.clone());
    }

    // Remove duplicates
    invitees.sort();
    invitees.dedup();

    if let Err(e) = state.slack_client.invite_users(&channel_id, invitees).await {
        error!("Failed to invite users to channel: {}", e);
        // Non-fatal: continue with incident creation
    }

    // Post and pin incident details
    let detail_blocks = blocks::incident_declared_blocks(&incident);
    match state
        .slack_client
        .post_message(&channel_id, detail_blocks)
        .await
    {
        Ok(ts) => {
            // Pin the message
            if let Err(e) = state.slack_client.pin_message(&channel_id, &ts).await {
                error!("Failed to pin incident details: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to post incident details: {}", e);
        }
    }

    // Send notifications based on severity
    let notification_service = NotificationService::new(
        state.pool.clone(),
        state.slack_client.clone(),
        state.config.clone(),
    );

    let notification_blocks = blocks::incident_declared_blocks(&incident);
    if let Err(e) = notification_service
        .notify_incident_declared(&incident, notification_blocks)
        .await
    {
        error!("Failed to send notifications: {}", e);
        // Non-fatal: incident is created, just notifications failed
    }

    // Enqueue Statuspage sync if component mapping exists
    if let Ok(Some(component_id)) =
        crate::db::queries::statuspage::get_component_id(&state.pool, &service).await
    {
        let job = crate::jobs::Job::StatuspageSync {
            incident_id: incident.id,
            component_id,
            status: incident.status,
            severity: incident.severity,
        };

        if let Err(e) = state.job_sender.send(job) {
            error!("Failed to enqueue Statuspage sync job: {}", e);
            // Non-fatal: best-effort sync
        }
    }

    info!(
        "Incident {} declared successfully in #{}",
        incident.id, channel_name
    );

    Ok(())
}
