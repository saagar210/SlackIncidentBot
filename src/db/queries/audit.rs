use crate::db::models::IncidentId;
use crate::error::IncidentResult;
use serde_json::Value;
use sqlx_postgres::PgPool;

pub async fn log_action(
    pool: &PgPool,
    incident_id: Option<IncidentId>,
    action: String,
    actor_id: String,
    old_state: Option<Value>,
    new_state: Option<Value>,
    details: Option<Value>,
) -> IncidentResult<()> {
    sqlx::query::query(
        r#"
        INSERT INTO audit_log (incident_id, action, actor_id, old_state, new_state, details)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(incident_id)
    .bind(action)
    .bind(actor_id)
    .bind(old_state)
    .bind(new_state)
    .bind(details)
    .execute(pool)
    .await?;

    Ok(())
}
