pub mod statuspage_sync;
pub mod worker;

use crate::db::models::{IncidentId, IncidentStatus, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Job {
    StatuspageSync {
        incident_id: IncidentId,
        component_id: String,
        status: IncidentStatus,
        severity: Severity,
    },
}
