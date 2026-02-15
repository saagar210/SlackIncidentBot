use crate::db::models::IncidentTemplate;
use crate::error::IncidentResult;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn list_active_templates(pool: &PgPool) -> IncidentResult<Vec<IncidentTemplate>> {
    let templates = sqlx::query_as::<_, IncidentTemplate>(
        r#"
        SELECT * FROM incident_templates
        WHERE is_active = true
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(templates)
}

pub async fn get_template_by_name(
    pool: &PgPool,
    name: &str,
) -> IncidentResult<Option<IncidentTemplate>> {
    let template = sqlx::query_as::<_, IncidentTemplate>(
        r#"
        SELECT * FROM incident_templates
        WHERE name = $1 AND is_active = true
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    Ok(template)
}

pub async fn get_template_by_id(
    pool: &PgPool,
    id: Uuid,
) -> IncidentResult<Option<IncidentTemplate>> {
    let template = sqlx::query_as::<_, IncidentTemplate>(
        r#"
        SELECT * FROM incident_templates
        WHERE id = $1 AND is_active = true
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(template)
}
