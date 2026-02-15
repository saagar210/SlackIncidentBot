use crate::db::models::{IncidentStatus, Severity};
use crate::error::{IncidentError, IncidentResult};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct StatuspageClient {
    http_client: Client,
    api_key: String,
    page_id: String,
}

#[derive(Debug, Serialize)]
struct ComponentUpdateRequest {
    component: ComponentUpdate,
}

#[derive(Debug, Serialize)]
struct ComponentUpdate {
    status: String,
}

impl StatuspageClient {
    pub fn new(api_key: String, page_id: String) -> Self {
        // Set 30-second timeout to prevent hanging requests to Statuspage API
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            api_key,
            page_id,
        }
    }

    /// Update component status on Statuspage
    pub async fn update_component_status(
        &self,
        component_id: &str,
        status: IncidentStatus,
        severity: Severity,
    ) -> IncidentResult<()> {
        let statuspage_status = Self::map_status(status, severity);

        debug!(
            "Updating Statuspage component {} to status: {}",
            component_id, statuspage_status
        );

        let url = format!(
            "https://api.statuspage.io/v1/pages/{}/components/{}",
            self.page_id, component_id
        );

        let request = ComponentUpdateRequest {
            component: ComponentUpdate {
                status: statuspage_status.to_string(),
            },
        };

        let response = self
            .http_client
            .patch(&url)
            .header("Authorization", format!("OAuth {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status_code = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Statuspage API error ({}): {}", status_code, error_text);
            return Err(IncidentError::ExternalAPIError {
                service: "Statuspage".to_string(),
                message: format!("HTTP {}: {}", status_code, error_text),
            });
        }

        info!(
            "Successfully updated Statuspage component {} to {}",
            component_id, statuspage_status
        );

        Ok(())
    }

    /// Map incident status + severity to Statuspage component status
    /// https://developer.statuspage.io/#operation/patchPagesPageIdComponentsComponentId
    fn map_status(status: IncidentStatus, severity: Severity) -> &'static str {
        match status {
            IncidentStatus::Declared | IncidentStatus::Investigating => {
                // Map severity to impact level
                match severity {
                    Severity::P1 => "major_outage",         // Critical impact
                    Severity::P2 => "partial_outage",       // High impact
                    Severity::P3 => "degraded_performance", // Medium impact
                    Severity::P4 => "degraded_performance", // Low impact
                }
            }
            IncidentStatus::Identified | IncidentStatus::Monitoring => {
                // Issue identified/being monitored
                match severity {
                    Severity::P1 => "partial_outage", // Still significant
                    Severity::P2 => "degraded_performance",
                    Severity::P3 | Severity::P4 => "degraded_performance",
                }
            }
            IncidentStatus::Resolved => "operational", // Back to normal
        }
    }

    /// Test connectivity to Statuspage API
    pub async fn test_connection(&self) -> IncidentResult<()> {
        let url = format!("https://api.statuspage.io/v1/pages/{}", self.page_id);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("OAuth {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(IncidentError::ExternalAPIError {
                service: "Statuspage".to_string(),
                message: format!("Connection test failed: HTTP {}", response.status()),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_mapping_p1() {
        use IncidentStatus::*;

        // P1 Critical
        assert_eq!(
            StatuspageClient::map_status(Declared, Severity::P1),
            "major_outage"
        );
        assert_eq!(
            StatuspageClient::map_status(Investigating, Severity::P1),
            "major_outage"
        );
        assert_eq!(
            StatuspageClient::map_status(Identified, Severity::P1),
            "partial_outage"
        );
        assert_eq!(
            StatuspageClient::map_status(Monitoring, Severity::P1),
            "partial_outage"
        );
        assert_eq!(
            StatuspageClient::map_status(Resolved, Severity::P1),
            "operational"
        );
    }

    #[test]
    fn test_status_mapping_p2() {
        use IncidentStatus::*;

        // P2 High
        assert_eq!(
            StatuspageClient::map_status(Declared, Severity::P2),
            "partial_outage"
        );
        assert_eq!(
            StatuspageClient::map_status(Investigating, Severity::P2),
            "partial_outage"
        );
        assert_eq!(
            StatuspageClient::map_status(Identified, Severity::P2),
            "degraded_performance"
        );
        assert_eq!(
            StatuspageClient::map_status(Resolved, Severity::P2),
            "operational"
        );
    }

    #[test]
    fn test_status_mapping_p3_p4() {
        use IncidentStatus::*;

        // P3/P4 Low priority
        assert_eq!(
            StatuspageClient::map_status(Declared, Severity::P3),
            "degraded_performance"
        );
        assert_eq!(
            StatuspageClient::map_status(Investigating, Severity::P4),
            "degraded_performance"
        );
        assert_eq!(
            StatuspageClient::map_status(Resolved, Severity::P3),
            "operational"
        );
    }
}
