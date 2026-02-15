use crate::error::IncidentResult;
use sqlx::PgPool;

pub async fn get_component_id(pool: &PgPool, service_name: &str) -> IncidentResult<Option<String>> {
    let component_id = sqlx::query_scalar::<_, String>(
        r#"
        SELECT component_id FROM statuspage_mappings
        WHERE service_name = $1
        "#,
    )
    .bind(service_name)
    .fetch_optional(pool)
    .await?;

    Ok(component_id)
}
