use crate::config::AppConfig;
use crate::jobs::Job;
use crate::slack::client::SlackClient;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<AppConfig>,
    pub slack_client: SlackClient,
    pub job_sender: mpsc::UnboundedSender<Job>,
}

impl AppState {
    pub fn new(pool: PgPool, config: AppConfig, job_sender: mpsc::UnboundedSender<Job>) -> Self {
        let slack_client = SlackClient::new(config.slack_bot_token.clone());
        Self {
            pool,
            config: Arc::new(config),
            slack_client,
            job_sender,
        }
    }
}
