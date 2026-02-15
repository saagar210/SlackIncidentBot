pub mod models;
pub mod queries;

use crate::error::IncidentResult;
use sqlx_postgres::PgPool;
use sqlx_postgres::PgPoolOptions;
use tracing::info;

pub async fn create_pool(database_url: &str) -> IncidentResult<PgPool> {
    // Set connection pool size to 20 to handle concurrent Slack webhooks/commands.
    // Each request may hold a connection for DB queries during incident operations.
    // 20 connections supports ~15-20 concurrent operations with headroom for background jobs.
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await?;

    info!("Database connection pool created");
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> IncidentResult<()> {
    use std::path::Path;

    info!("Running database migrations");
    let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let migrator = sqlx::migrate::Migrator::new(migrations_dir.as_path())
        .await
        .map_err(|e| crate::error::IncidentError::InternalError(e.to_string()))?;

    migrator
        .run(pool)
        .await
        .map_err(|e| crate::error::IncidentError::DatabaseError(e.into()))?;
    info!("Database migrations complete");
    Ok(())
}

pub async fn health_check(pool: &PgPool) -> bool {
    sqlx::query::query("SELECT 1").fetch_one(pool).await.is_ok()
}
