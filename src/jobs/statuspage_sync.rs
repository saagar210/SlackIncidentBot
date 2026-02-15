use crate::adapters::statuspage::StatuspageClient;
use crate::db::models::{IncidentId, IncidentStatus, Severity};
use crate::error::IncidentResult;
use tracing::{error, info};

pub async fn execute(
    statuspage_client: &StatuspageClient,
    incident_id: IncidentId,
    component_id: String,
    status: IncidentStatus,
    severity: Severity,
) -> IncidentResult<()> {
    info!(
        "Syncing incident {} to Statuspage component {} (status: {:?}, severity: {:?})",
        incident_id, component_id, status, severity
    );

    match statuspage_client
        .update_component_status(&component_id, status, severity)
        .await
    {
        Ok(_) => {
            info!("Successfully synced incident {} to Statuspage", incident_id);
            Ok(())
        }
        Err(e) => {
            error!(
                "Failed to sync incident {} to Statuspage: {}",
                incident_id, e
            );
            // Don't propagate error - log and continue
            // Statuspage sync is best-effort
            Ok(())
        }
    }
}
