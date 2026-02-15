use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::from_row::FromRow;
use sqlx::row::Row;
use sqlx_postgres::PgRow;
use std::io::{Error as IoError, ErrorKind};
use uuid::Uuid;

// ── Newtypes for type safety ──
pub type IncidentId = Uuid;
pub type SlackUserId = String; // e.g., "U024BE7LH"
pub type SlackChannelId = String; // e.g., "C024BE91L"

// ── Severity ──
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    P1,
    P2,
    P3,
    P4,
}

impl Severity {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Severity::P1 => "P1",
            Severity::P2 => "P2",
            Severity::P3 => "P3",
            Severity::P4 => "P4",
        }
    }

    pub fn from_db_str(s: &str) -> Result<Self, String> {
        s.parse()
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncidentStatus {
    Declared,
    Investigating,
    Identified,
    Monitoring,
    Resolved,
}

impl IncidentStatus {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            IncidentStatus::Declared => "declared",
            IncidentStatus::Investigating => "investigating",
            IncidentStatus::Identified => "identified",
            IncidentStatus::Monitoring => "monitoring",
            IncidentStatus::Resolved => "resolved",
        }
    }

    pub fn from_db_str(s: &str) -> Result<Self, String> {
        s.parse()
    }

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

impl std::str::FromStr for IncidentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "declared" => Ok(IncidentStatus::Declared),
            "investigating" => Ok(IncidentStatus::Investigating),
            "identified" => Ok(IncidentStatus::Identified),
            "monitoring" => Ok(IncidentStatus::Monitoring),
            "resolved" => Ok(IncidentStatus::Resolved),
            _ => Err(format!("Invalid incident status: {}", s)),
        }
    }
}

// ── Incident (DB row) ──
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineEventType {
    Declared,
    StatusUpdate,
    SeverityChange,
    Resolved,
}

impl TimelineEventType {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            TimelineEventType::Declared => "declared",
            TimelineEventType::StatusUpdate => "status_update",
            TimelineEventType::SeverityChange => "severity_change",
            TimelineEventType::Resolved => "resolved",
        }
    }

    pub fn from_db_str(s: &str) -> Result<Self, String> {
        s.parse()
    }
}

impl std::str::FromStr for TimelineEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "declared" => Ok(TimelineEventType::Declared),
            "status_update" => Ok(TimelineEventType::StatusUpdate),
            "severity_change" => Ok(TimelineEventType::SeverityChange),
            "resolved" => Ok(TimelineEventType::Resolved),
            _ => Err(format!("Invalid timeline event type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub id: Uuid,
    pub incident_id: IncidentId,
    pub event_type: TimelineEventType,
    pub message: String,
    pub posted_by: SlackUserId,
    pub timestamp: DateTime<Utc>,
}

// ── Notification Record ──
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    SlackChannel,
    SlackDm,
}

impl NotificationType {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            NotificationType::SlackChannel => "slack_channel",
            NotificationType::SlackDm => "slack_dm",
        }
    }

    pub fn from_db_str(s: &str) -> Result<Self, String> {
        s.parse()
    }
}

impl std::str::FromStr for NotificationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "slack_channel" => Ok(NotificationType::SlackChannel),
            "slack_dm" => Ok(NotificationType::SlackDm),
            _ => Err(format!("Invalid notification type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationStatus {
    Sent,
    Failed,
    Pending,
    Throttled,
}

impl NotificationStatus {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            NotificationStatus::Sent => "sent",
            NotificationStatus::Failed => "failed",
            NotificationStatus::Pending => "pending",
            NotificationStatus::Throttled => "throttled",
        }
    }

    pub fn from_db_str(s: &str) -> Result<Self, String> {
        s.parse()
    }
}

impl std::str::FromStr for NotificationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "sent" => Ok(NotificationStatus::Sent),
            "failed" => Ok(NotificationStatus::Failed),
            "pending" => Ok(NotificationStatus::Pending),
            "throttled" => Ok(NotificationStatus::Throttled),
            _ => Err(format!("Invalid notification status: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NotificationRecord {
    pub id: Uuid,
    pub incident_id: IncidentId,
    pub notification_type: NotificationType,
    pub recipient: String,
    pub sent_at: DateTime<Utc>,
    pub status: NotificationStatus,
    pub error_message: Option<String>,
}

// ── Incident Template ──
#[derive(Debug, Clone, Serialize)]
pub struct IncidentTemplate {
    pub id: Uuid,
    pub name: String,
    pub title: String,
    pub severity: Severity,
    pub affected_service: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn decode_parse_error(field: &str, value: &str, err: String) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(IoError::new(
        ErrorKind::InvalidData,
        format!("invalid {} '{}': {}", field, value, err),
    )))
}

impl<'r> FromRow<'r, PgRow> for Incident {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let severity_raw: String = row.try_get("severity")?;
        let status_raw: String = row.try_get("status")?;

        let severity = Severity::from_db_str(&severity_raw)
            .map_err(|e| decode_parse_error("severity", &severity_raw, e))?;
        let status = IncidentStatus::from_db_str(&status_raw)
            .map_err(|e| decode_parse_error("status", &status_raw, e))?;

        Ok(Self {
            id: row.try_get("id")?,
            slack_channel_id: row.try_get("slack_channel_id")?,
            title: row.try_get("title")?,
            severity,
            status,
            affected_service: row.try_get("affected_service")?,
            commander_id: row.try_get("commander_id")?,
            declared_at: row.try_get("declared_at")?,
            resolved_at: row.try_get("resolved_at")?,
            duration_minutes: row.try_get("duration_minutes")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for TimelineEvent {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let event_type_raw: String = row.try_get("event_type")?;
        let event_type = TimelineEventType::from_db_str(&event_type_raw)
            .map_err(|e| decode_parse_error("event_type", &event_type_raw, e))?;

        Ok(Self {
            id: row.try_get("id")?,
            incident_id: row.try_get("incident_id")?,
            event_type,
            message: row.try_get("message")?,
            posted_by: row.try_get("posted_by")?,
            timestamp: row.try_get("timestamp")?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for NotificationRecord {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let notification_type_raw: String = row.try_get("notification_type")?;
        let status_raw: String = row.try_get("status")?;
        let notification_type = NotificationType::from_db_str(&notification_type_raw)
            .map_err(|e| decode_parse_error("notification_type", &notification_type_raw, e))?;
        let status = NotificationStatus::from_db_str(&status_raw)
            .map_err(|e| decode_parse_error("status", &status_raw, e))?;

        Ok(Self {
            id: row.try_get("id")?,
            incident_id: row.try_get("incident_id")?,
            notification_type,
            recipient: row.try_get("recipient")?,
            sent_at: row.try_get("sent_at")?,
            status,
            error_message: row.try_get("error_message")?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for IncidentTemplate {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let severity_raw: String = row.try_get("severity")?;
        let severity = Severity::from_db_str(&severity_raw)
            .map_err(|e| decode_parse_error("severity", &severity_raw, e))?;

        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            title: row.try_get("title")?,
            severity,
            affected_service: row.try_get("affected_service")?,
            description: row.try_get("description")?,
            is_active: row.try_get("is_active")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
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
