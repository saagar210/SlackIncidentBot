use crate::db::models::{Incident, IncidentId, IncidentStatus, Severity, SlackChannelId};
use crate::error::IncidentResult;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_incident(
    pool: &PgPool,
    title: String,
    severity: Severity,
    affected_service: String,
    commander_id: String,
) -> IncidentResult<Incident> {
    let incident = sqlx::query_as::<_, Incident>(
        r#"
        INSERT INTO incidents (title, severity, affected_service, commander_id, status, declared_at)
        VALUES ($1, $2, $3, $4, 'declared', NOW())
        RETURNING *
        "#,
    )
    .bind(title)
    .bind(severity)
    .bind(affected_service)
    .bind(commander_id)
    .fetch_one(pool)
    .await?;

    Ok(incident)
}

pub async fn get_incident_by_id(pool: &PgPool, id: IncidentId) -> IncidentResult<Incident> {
    let incident = sqlx::query_as::<_, Incident>(
        r#"
        SELECT * FROM incidents WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(crate::error::IncidentError::NotFound)?;

    Ok(incident)
}

pub async fn get_incident_by_channel(
    pool: &PgPool,
    channel_id: &str,
) -> IncidentResult<Incident> {
    let incident = sqlx::query_as::<_, Incident>(
        r#"
        SELECT * FROM incidents WHERE slack_channel_id = $1 AND status != 'resolved'
        "#,
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await?
    .ok_or(crate::error::IncidentError::NotFound)?;

    Ok(incident)
}

pub async fn update_channel_id(
    pool: &PgPool,
    incident_id: IncidentId,
    channel_id: SlackChannelId,
) -> IncidentResult<()> {
    sqlx::query(
        r#"
        UPDATE incidents SET slack_channel_id = $1, updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(channel_id)
    .bind(incident_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_status(
    pool: &PgPool,
    incident_id: IncidentId,
    status: IncidentStatus,
) -> IncidentResult<()> {
    sqlx::query(
        r#"
        UPDATE incidents SET status = $1, updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(status)
    .bind(incident_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_severity(
    pool: &PgPool,
    incident_id: IncidentId,
    severity: Severity,
) -> IncidentResult<()> {
    sqlx::query(
        r#"
        UPDATE incidents SET severity = $1, updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(severity)
    .bind(incident_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn resolve_incident(
    pool: &PgPool,
    incident_id: IncidentId,
) -> IncidentResult<Incident> {
    let incident = sqlx::query_as::<_, Incident>(
        r#"
        UPDATE incidents
        SET status = 'resolved',
            resolved_at = NOW(),
            duration_minutes = ROUND(EXTRACT(EPOCH FROM (NOW() - declared_at)) / 60),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(incident_id)
    .fetch_one(pool)
    .await?;

    Ok(incident)
}

pub async fn list_channels_by_prefix(pool: &PgPool, prefix: &str) -> IncidentResult<Vec<String>> {
    let channels = sqlx::query_scalar::<_, String>(
        r#"
        SELECT slack_channel_id FROM incidents
        WHERE slack_channel_id LIKE $1 || '%' AND slack_channel_id IS NOT NULL
        "#,
    )
    .bind(prefix)
    .fetch_all(pool)
    .await?;

    Ok(channels)
}
