use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    // Required
    pub slack_bot_token: String,
    pub slack_signing_secret: String,
    pub database_url: String,

    // Optional (Phase 2+)
    #[serde(default)]
    pub statuspage_api_key: Option<String>,
    #[serde(default)]
    pub statuspage_page_id: Option<String>,

    // Server
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,

    // Notification routing
    #[serde(default)]
    pub p1_dm_recipients: Vec<String>,
    #[serde(default)]
    pub p2_channels: Vec<String>,
    #[serde(default)]
    pub p1_channels: Vec<String>,

    // Service owners mapping
    #[serde(default)]
    pub service_owners: HashMap<String, Vec<String>>,

    // Available services
    #[serde(default)]
    pub services: Vec<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenvy::dotenv().ok();

        let mut builder = config::Config::builder();

        // Load from environment variables
        builder = builder
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .try_parsing(true)
                    .list_separator(","),
            )
            // Parse SERVICE_OWNERS as JSON
            .set_override_option(
                "service_owners",
                std::env::var("SERVICE_OWNERS")
                    .ok()
                    .and_then(|s| serde_json::from_str::<HashMap<String, Vec<String>>>(&s).ok()),
            )?;

        let config = builder.build()?;
        config.try_deserialize()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.slack_bot_token.is_empty() {
            return Err("SLACK_BOT_TOKEN is required".to_string());
        }
        if self.slack_signing_secret.is_empty() {
            return Err("SLACK_SIGNING_SECRET is required".to_string());
        }
        if self.database_url.is_empty() {
            return Err("DATABASE_URL is required".to_string());
        }

        // Warn if notification channels not configured (medium severity issue)
        if self.p1_channels.is_empty() && self.p1_dm_recipients.is_empty() {
            tracing::warn!("No P1 notification channels configured - P1 incidents will not broadcast");
        }
        if self.p2_channels.is_empty() {
            tracing::warn!("No P2 notification channels configured - P2 incidents will not broadcast");
        }

        Ok(())
    }
}
