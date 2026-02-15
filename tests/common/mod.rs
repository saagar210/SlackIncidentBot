use incident_bot::{AppConfig, AppState};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashMap;

pub struct TestContext {
    pub pool: PgPool,
    pub state: AppState,
}

impl TestContext {
    pub async fn new() -> Self {
        // Create test config
        let config = AppConfig {
            slack_bot_token: "xoxb-test-token".to_string(),
            slack_signing_secret: "test-secret".to_string(),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://incident_bot:password@localhost:5432/incident_bot_test".to_string()),
            statuspage_api_key: None,
            statuspage_page_id: None,
            host: "0.0.0.0".to_string(),
            port: 3000,
            p1_users: vec!["U024TEST1".to_string()],
            p2_channels: vec!["C024TEST1".to_string()],
            p1_channels: vec!["C024TEST2".to_string()],
            service_owners: HashMap::new(),
            services: vec!["Test Service".to_string()],
        };

        // Create pool
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.database_url)
            .await
            .expect("Failed to connect to test database");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        // Create job channel for background tasks
        let (job_sender, _job_receiver) = tokio::sync::mpsc::unbounded_channel();

        // Create state
        let state = AppState::new(pool.clone(), config, job_sender);

        Self { pool, state }
    }

    pub async fn cleanup(&self) {
        // Clean up test data
        sqlx::query("DELETE FROM incident_notifications")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("DELETE FROM incident_timeline")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("DELETE FROM audit_log")
            .execute(&self.pool)
            .await
            .ok();
        sqlx::query("DELETE FROM incidents")
            .execute(&self.pool)
            .await
            .ok();
    }
}
