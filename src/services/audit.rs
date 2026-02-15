use crate::db::models::IncidentId;
use crate::db::queries::audit;
use crate::error::IncidentResult;
use serde_json::Value;
use sqlx_postgres::PgPool;

pub struct AuditService {
    pool: PgPool,
}

impl AuditService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn log_action(
        &self,
        incident_id: Option<IncidentId>,
        action: String,
        actor_id: String,
        old_state: Option<Value>,
        new_state: Option<Value>,
        details: Option<Value>,
    ) -> IncidentResult<()> {
        audit::log_action(
            &self.pool,
            incident_id,
            action,
            actor_id,
            old_state,
            new_state,
            details,
        )
        .await
    }
}
