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
    pub p1_users: Vec<String>,
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
        let service_owners = parse_service_owners_env()?;
        let p1_channels = resolve_channel_list(
            std::env::var("P1_CHANNELS").ok(),
            std::env::var("NOTIFICATION_CHANNEL_GENERAL").ok(),
        );
        let p2_channels = resolve_channel_list(
            std::env::var("P2_CHANNELS").ok(),
            std::env::var("NOTIFICATION_CHANNEL_ENGINEERING").ok(),
        );

        // Load from environment variables
        builder = builder
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .try_parsing(true)
                    .list_separator(","),
            )
            .set_override_option("service_owners", service_owners)?
            .set_override_option("p1_channels", p1_channels)?
            .set_override_option("p2_channels", p2_channels)?;

        let config = builder.build()?;
        config.try_deserialize()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.slack_bot_token.is_empty() {
            return Err("SLACK_BOT_TOKEN is required".to_string());
        }
        if !self.slack_bot_token.starts_with("xoxb-") {
            return Err("SLACK_BOT_TOKEN must start with xoxb-".to_string());
        }
        if self.slack_signing_secret.is_empty() {
            return Err("SLACK_SIGNING_SECRET is required".to_string());
        }
        if self.database_url.is_empty() {
            return Err("DATABASE_URL is required".to_string());
        }
        if self.services.is_empty() {
            return Err("SERVICES cannot be empty".to_string());
        }

        // Configuration must be complete for Statuspage integration.
        if self.statuspage_api_key.is_some() ^ self.statuspage_page_id.is_some() {
            tracing::warn!(
                "STATUSPAGE_API_KEY and STATUSPAGE_PAGE_ID should be set together; integration will be disabled"
            );
        }

        // Warn if notification channels not configured (medium severity issue)
        if self.p1_channels.is_empty() && self.p1_users.is_empty() {
            tracing::warn!(
                "No P1 notification channels configured - P1 incidents will not broadcast"
            );
        }
        if self.p2_channels.is_empty() {
            tracing::warn!(
                "No P2 notification channels configured - P2 incidents will not broadcast"
            );
        }

        Ok(())
    }
}

fn parse_service_owners_env() -> Result<Option<HashMap<String, Vec<String>>>, config::ConfigError> {
    match std::env::var("SERVICE_OWNERS") {
        Ok(raw) => {
            let parsed =
                serde_json::from_str::<HashMap<String, Vec<String>>>(&raw).map_err(|e| {
                    config::ConfigError::Message(format!("Invalid JSON in SERVICE_OWNERS: {e}"))
                })?;
            Ok(Some(parsed))
        }
        Err(_) => Ok(None),
    }
}

fn resolve_channel_list(primary: Option<String>, legacy: Option<String>) -> Option<Vec<String>> {
    if let Some(raw) = primary {
        let parsed = parse_csv_list(&raw);
        if !parsed.is_empty() {
            return Some(parsed);
        }
    }

    if let Some(raw) = legacy {
        let parsed = parse_csv_list(&raw);
        if !parsed.is_empty() {
            return Some(parsed);
        }
    }

    None
}

fn parse_csv_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_channel_list_prefers_primary() {
        let resolved = resolve_channel_list(
            Some("C_PRIMARY,C_SECONDARY".to_string()),
            Some("C_LEGACY".to_string()),
        )
        .expect("Expected channels");
        assert_eq!(resolved, vec!["C_PRIMARY", "C_SECONDARY"]);
    }

    #[test]
    fn test_resolve_channel_list_uses_legacy_when_primary_missing() {
        let resolved =
            resolve_channel_list(None, Some("C_LEGACY".to_string())).expect("Expected channels");
        assert_eq!(resolved, vec!["C_LEGACY"]);
    }

    #[test]
    fn test_parse_csv_list_trims_and_ignores_empty_values() {
        let parsed = parse_csv_list(" C1, ,C2,, C3 ");
        assert_eq!(parsed, vec!["C1", "C2", "C3"]);
    }

    #[test]
    fn test_validate_requires_non_empty_services_and_token_prefix() {
        let config = AppConfig {
            slack_bot_token: "xoxp-not-bot-token".to_string(),
            slack_signing_secret: "secret".to_string(),
            database_url: "postgres://localhost/postgres".to_string(),
            statuspage_api_key: None,
            statuspage_page_id: None,
            host: "0.0.0.0".to_string(),
            port: 3000,
            p1_users: vec![],
            p2_channels: vec![],
            p1_channels: vec![],
            service_owners: HashMap::new(),
            services: vec![],
        };

        let err = config.validate().expect_err("Expected validation error");
        assert_eq!(err, "SLACK_BOT_TOKEN must start with xoxb-");
    }

    #[test]
    fn test_validate_requires_services() {
        let config = AppConfig {
            slack_bot_token: "xoxb-valid-token".to_string(),
            slack_signing_secret: "secret".to_string(),
            database_url: "postgres://localhost/postgres".to_string(),
            statuspage_api_key: None,
            statuspage_page_id: None,
            host: "0.0.0.0".to_string(),
            port: 3000,
            p1_users: vec![],
            p2_channels: vec![],
            p1_channels: vec![],
            service_owners: HashMap::new(),
            services: vec![],
        };

        let err = config.validate().expect_err("Expected validation error");
        assert_eq!(err, "SERVICES cannot be empty");
    }
}
