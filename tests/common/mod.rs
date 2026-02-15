use sqlx_postgres::PgPool;
use sqlx_postgres::PgPoolOptions;
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, OwnedMutexGuard};

static TEST_MUTEX: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

fn global_test_mutex() -> Arc<Mutex<()>> {
    TEST_MUTEX.get_or_init(|| Arc::new(Mutex::new(()))).clone()
}

pub struct TestContext {
    pub pool: PgPool,
    _guard: OwnedMutexGuard<()>,
}

impl TestContext {
    pub async fn new() -> Self {
        // Serialize integration tests sharing a single database to avoid cross-test cleanup races.
        let guard = global_test_mutex().lock_owned().await;

        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://incident_bot:password@localhost:5432/incident_bot_test".to_string()
        });

        // Create pool
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        incident_bot::db::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        Self {
            pool,
            _guard: guard,
        }
    }

    pub async fn cleanup(&self) {
        // Clean up test data
        sqlx::query::query("DELETE FROM incident_notifications")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query::query("DELETE FROM incident_timeline")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query::query("DELETE FROM audit_log")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query::query("DELETE FROM incidents")
            .execute(&self.pool)
            .await
            .ok();
    }
}
