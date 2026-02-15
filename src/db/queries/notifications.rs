use crate::db::models::{IncidentId, NotificationRecord, NotificationStatus, NotificationType};
use crate::error::IncidentResult;
use sqlx::PgPool;

pub async fn log_notification(
    pool: &PgPool,
    incident_id: IncidentId,
    notification_type: NotificationType,
    recipient: String,
    status: NotificationStatus,
    error_message: Option<String>,
) -> IncidentResult<NotificationRecord> {
    let record = sqlx::query_as::<_, NotificationRecord>(
        r#"
        INSERT INTO incident_notifications (incident_id, notification_type, recipient, status, error_message)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(incident_id)
    .bind(notification_type)
    .bind(recipient)
    .bind(status)
    .bind(error_message)
    .fetch_one(pool)
    .await?;

    Ok(record)
}
