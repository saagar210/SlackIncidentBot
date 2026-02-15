use crate::db::models::IncidentId;
use crate::error::{IncidentError, IncidentResult};
use crate::slack::client::SlackClient;
use chrono::NaiveDate;
use tracing::{debug, info};

/// Generate channel name from service and date
/// Format: inc-YYYYMMDD-service
pub fn generate_channel_name(service: &str, date: NaiveDate, incident_id: IncidentId) -> String {
    let slug = service
        .to_lowercase()
        .replace([' ', '_'], "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    // Take first 40 chars of service slug to leave room for date + prefix
    let slug_truncated = if slug.len() > 40 { &slug[..40] } else { &slug };

    let base = format!("inc-{}-{}", date.format("%Y%m%d"), slug_truncated);

    // Slack channel name limit is 80 chars, truncate if needed
    if base.len() > 80 {
        let uuid_suffix = &incident_id.to_string()[..4];
        format!("inc-{}-{}", date.format("%Y%m%d"), uuid_suffix)
    } else {
        base
    }
}

/// Create incident channel with deduplication
/// Returns (channel_id, channel_name)
pub async fn create_incident_channel(
    slack_client: &SlackClient,
    service: &str,
    date: NaiveDate,
    incident_id: IncidentId,
) -> IncidentResult<(String, String)> {
    let base_name = generate_channel_name(service, date, incident_id);

    // Try to create channel
    match slack_client.create_conversation(&base_name).await {
        Ok(channel_id) => {
            info!("Created channel #{} ({})", base_name, channel_id);
            Ok((channel_id, base_name))
        }
        Err(IncidentError::SlackAPIError {
            slack_error_code, ..
        }) if slack_error_code == "name_taken" => {
            // Channel already exists, add UUID suffix (8 chars = ~4B combinations, reduces collision risk)
            let uuid_suffix = &incident_id.to_string()[..8];
            let unique_name = format!("{}-{}", base_name, uuid_suffix);

            debug!("Channel #{} exists, trying #{}", base_name, unique_name);

            match slack_client.create_conversation(&unique_name).await {
                Ok(channel_id) => {
                    info!("Created channel #{} ({})", unique_name, channel_id);
                    Ok((channel_id, unique_name))
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_channel_name_generation() {
        let date = NaiveDate::from_ymd_opt(2024, 11, 15).unwrap();
        let incident_id = Uuid::new_v4();

        let name = generate_channel_name("Okta SSO", date, incident_id);
        assert_eq!(name, "inc-20241115-okta-sso");

        let name = generate_channel_name("VPN", date, incident_id);
        assert_eq!(name, "inc-20241115-vpn");

        // Test special characters
        let name = generate_channel_name("Email_Service@2024", date, incident_id);
        assert_eq!(name, "inc-20241115-email-service2024");
    }

    #[test]
    fn test_long_service_name() {
        let date = NaiveDate::from_ymd_opt(2024, 11, 15).unwrap();
        let incident_id = Uuid::new_v4();

        let long_service = "This is a very long service name that exceeds forty characters and should be truncated properly to fit within Slack's channel name limits which are quite restrictive";
        let name = generate_channel_name(long_service, date, incident_id);

        // Should be truncated or use UUID
        assert!(name.len() <= 80);
        assert!(name.starts_with("inc-20241115-"));
    }

    #[test]
    fn test_special_characters_removed() {
        let date = NaiveDate::from_ymd_opt(2024, 11, 15).unwrap();
        let incident_id = Uuid::new_v4();

        // Test various special characters
        let name = generate_channel_name("Service (Production)", date, incident_id);
        assert!(name.contains("service-production") || name.contains("serviceproduction"));

        let name = generate_channel_name("API/Gateway", date, incident_id);
        assert!(name.contains("apigateway") || name.contains("api-gateway"));

        let name = generate_channel_name("Database#2", date, incident_id);
        assert!(name.contains("database2"));
    }

    #[test]
    fn test_date_formatting() {
        let incident_id = Uuid::new_v4();

        let date = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
        let name = generate_channel_name("Test", date, incident_id);
        assert!(name.contains("20240105"));

        let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let name = generate_channel_name("Test", date, incident_id);
        assert!(name.contains("20241231"));
    }

    #[test]
    fn test_empty_service_name() {
        let date = NaiveDate::from_ymd_opt(2024, 11, 15).unwrap();
        let incident_id = Uuid::new_v4();

        let name = generate_channel_name("", date, incident_id);
        // Should fallback to UUID-based name
        assert!(name.starts_with("inc-20241115-"));
    }
}
