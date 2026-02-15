use crate::db::models::{IncidentId, TimelineEvent, TimelineEventType};
use crate::error::IncidentResult;
use sqlx::PgPool;

pub async fn log_event(
    pool: &PgPool,
    incident_id: IncidentId,
    event_type: TimelineEventType,
    message: String,
    posted_by: String,
) -> IncidentResult<TimelineEvent> {
    let event = sqlx::query_as::<_, TimelineEvent>(
        r#"
        INSERT INTO incident_timeline (incident_id, event_type, message, posted_by)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(incident_id)
    .bind(event_type)
    .bind(message)
    .bind(posted_by)
    .fetch_one(pool)
    .await?;

    Ok(event)
}

pub async fn get_timeline(pool: &PgPool, incident_id: IncidentId) -> IncidentResult<Vec<TimelineEvent>> {
    let events = sqlx::query_as::<_, TimelineEvent>(
        r#"
        SELECT * FROM incident_timeline
        WHERE incident_id = $1
        ORDER BY timestamp ASC
        "#,
    )
    .bind(incident_id)
    .fetch_all(pool)
    .await?;

    Ok(events)
}
