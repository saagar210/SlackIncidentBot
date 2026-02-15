use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── Newtypes for type safety ──
pub type IncidentId = Uuid;
pub type SlackUserId = String; // e.g., "U024BE7LH"
pub type SlackChannelId = String; // e.g., "C024BE91L"

// ── Severity ──
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "UPPERCASE")]
pub enum Severity {
    P1,
    P2,
    P3,
    P4,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::P1 => "P1 (Critical)",
            Severity::P2 => "P2 (High)",
            Severity::P3 => "P3 (Medium)",
            Severity::P4 => "P4 (Low)",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Severity::P1 => "🔴",
            Severity::P2 => "🟡",
            Severity::P3 | Severity::P4 => "🟢",
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "P1" => Ok(Severity::P1),
            "P2" => Ok(Severity::P2),
            "P3" => Ok(Severity::P3),
            "P4" => Ok(Severity::P4),
            _ => Err(format!("Invalid severity: {}", s)),
        }
    }
}

// ── Incident Status (State Machine) ──
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum IncidentStatus {
    Declared,
    Investigating,
    Identified,
    Monitoring,
    Resolved,
}

impl IncidentStatus {
    /// Returns valid transitions FROM this state.
    pub fn valid_transitions(&self) -> &[IncidentStatus] {
        use IncidentStatus::*;
        match self {
            Declared => &[Investigating, Identified, Monitoring, Resolved],
            Investigating => &[Identified, Monitoring, Resolved],
            Identified => &[Monitoring, Resolved],
            Monitoring => &[Resolved],
            Resolved => &[], // Terminal
        }
    }

    pub fn can_transition_to(&self, target: &IncidentStatus) -> bool {
        self.valid_transitions().contains(target)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, IncidentStatus::Resolved)
    }
}

// ── Incident (DB row) ──
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Incident {
    pub id: IncidentId,
    pub slack_channel_id: Option<SlackChannelId>,
    pub title: String,
    pub severity: Severity,
    pub status: IncidentStatus,
    pub affected_service: String,
    pub commander_id: SlackUserId,
    pub declared_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub duration_minutes: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Timeline Event ──
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum TimelineEventType {
    Declared,
    StatusUpdate,
    SeverityChange,
    Resolved,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TimelineEvent {
    pub id: Uuid,
    pub incident_id: IncidentId,
    pub event_type: TimelineEventType,
    pub message: String,
    pub posted_by: SlackUserId,
    pub timestamp: DateTime<Utc>,
}

// ── Notification Record ──
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum NotificationType {
    SlackChannel,
    SlackDm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum NotificationStatus {
    Sent,
    Failed,
    Pending,
}

#[derive(Debug, Clone, FromRow)]
pub struct NotificationRecord {
    pub id: Uuid,
    pub incident_id: IncidentId,
    pub notification_type: NotificationType,
    pub recipient: String,
    pub sent_at: DateTime<Utc>,
    pub status: NotificationStatus,
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_parsing() {
        assert_eq!("P1".parse::<Severity>().unwrap(), Severity::P1);
        assert_eq!("p2".parse::<Severity>().unwrap(), Severity::P2);
        assert!("P5".parse::<Severity>().is_err());
    }

    #[test]
    fn test_state_machine_transitions() {
        use IncidentStatus::*;

        assert!(Declared.can_transition_to(&Investigating));
        assert!(Declared.can_transition_to(&Resolved));
        assert!(!Resolved.can_transition_to(&Investigating));
        assert!(Resolved.is_terminal());
        assert!(!Declared.is_terminal());
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::P1.label(), "P1 (Critical)");
        assert_eq!(Severity::P1.emoji(), "🔴");
        assert_eq!(Severity::P3.emoji(), "🟢");
    }
}
