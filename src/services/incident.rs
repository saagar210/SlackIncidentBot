use crate::db::models::{Incident, IncidentId, IncidentStatus, Severity, TimelineEventType};
use crate::db::queries::incidents as incident_queries;
use crate::error::{IncidentError, IncidentResult};
use crate::services::audit::AuditService;
use crate::services::timeline::TimelineService;
use serde_json::json;
use sqlx_postgres::PgPool;
use tracing::info;

pub struct IncidentService {
    pool: PgPool,
    timeline_service: TimelineService,
    audit_service: AuditService,
}

impl IncidentService {
    pub fn new(pool: PgPool) -> Self {
        let timeline_service = TimelineService::new(pool.clone());
        let audit_service = AuditService::new(pool.clone());
        Self {
            pool,
            timeline_service,
            audit_service,
        }
    }

    pub async fn create_incident(
        &self,
        title: String,
        severity: Severity,
        affected_service: String,
        commander_id: String,
    ) -> IncidentResult<Incident> {
        // Create incident in DB
        let incident = incident_queries::create_incident(
            &self.pool,
            title.clone(),
            severity,
            affected_service.clone(),
            commander_id.clone(),
        )
        .await?;

        // Log to timeline
        self.timeline_service
            .log_event(
                incident.id,
                TimelineEventType::Declared,
                format!("Incident declared: {}", title),
                commander_id.clone(),
            )
            .await?;

        // Log to audit
        self.audit_service
            .log_action(
                Some(incident.id),
                "declare_incident".to_string(),
                commander_id,
                None,
                Some(json!({
                    "title": title,
                    "severity": severity,
                    "service": affected_service,
                })),
                None,
            )
            .await?;

        info!("Incident created: {} ({})", incident.id, title);
        Ok(incident)
    }

    pub async fn update_channel_id(
        &self,
        incident_id: IncidentId,
        channel_id: String,
    ) -> IncidentResult<()> {
        incident_queries::update_channel_id(&self.pool, incident_id, channel_id).await
    }

    pub async fn delete_incident(&self, incident_id: IncidentId) -> IncidentResult<()> {
        incident_queries::delete_incident(&self.pool, incident_id).await
    }

    pub async fn post_status_update(
        &self,
        incident_id: IncidentId,
        message: String,
        posted_by: String,
    ) -> IncidentResult<Incident> {
        // Get incident and validate commander
        let incident = self.get_by_id(incident_id).await?;
        self.validate_commander(&incident, &posted_by).await?;

        // Check if incident is resolved
        if incident.status.is_terminal() {
            return Err(IncidentError::ValidationError {
                field: "status".to_string(),
                reason: "Cannot post status updates to resolved incidents".to_string(),
            });
        }

        // Log to timeline
        self.timeline_service
            .log_event(
                incident_id,
                TimelineEventType::StatusUpdate,
                message.clone(),
                posted_by.clone(),
            )
            .await?;

        // Log to audit
        self.audit_service
            .log_action(
                Some(incident_id),
                "post_status_update".to_string(),
                posted_by,
                None,
                None,
                Some(json!({ "message": message })),
            )
            .await?;

        // Return updated incident
        self.get_by_id(incident_id).await
    }

    pub async fn change_severity(
        &self,
        incident_id: IncidentId,
        new_severity: Severity,
        changed_by: String,
        reason: Option<String>,
    ) -> IncidentResult<(Incident, Severity)> {
        // Get incident and validate commander
        let incident = self.get_by_id(incident_id).await?;
        self.validate_commander(&incident, &changed_by).await?;

        let old_severity = incident.severity;

        // Update severity in DB
        incident_queries::update_severity(&self.pool, incident_id, new_severity).await?;

        // Log to timeline
        let message = if let Some(reason) = &reason {
            format!(
                "Severity changed from {} to {} — {}",
                old_severity.label(),
                new_severity.label(),
                reason
            )
        } else {
            format!(
                "Severity changed from {} to {}",
                old_severity.label(),
                new_severity.label()
            )
        };

        self.timeline_service
            .log_event(
                incident_id,
                TimelineEventType::SeverityChange,
                message,
                changed_by.clone(),
            )
            .await?;

        // Log to audit
        self.audit_service
            .log_action(
                Some(incident_id),
                "change_severity".to_string(),
                changed_by,
                Some(json!({ "severity": old_severity })),
                Some(json!({ "severity": new_severity })),
                reason.map(|r| json!({ "reason": r })),
            )
            .await?;

        // Get updated incident
        let updated_incident = self.get_by_id(incident_id).await?;
        Ok((updated_incident, old_severity))
    }

    pub async fn resolve_incident(
        &self,
        incident_id: IncidentId,
        resolved_by: String,
    ) -> IncidentResult<Incident> {
        // Get incident and validate commander
        let incident = self.get_by_id(incident_id).await?;
        self.validate_commander(&incident, &resolved_by).await?;

        // Check if already resolved
        if incident.status.is_terminal() {
            return Ok(incident); // Idempotent
        }

        // Update status in DB (sets resolved_at, duration_minutes)
        let resolved_incident = incident_queries::resolve_incident(&self.pool, incident_id).await?;

        // Log to timeline
        let duration_text = if let Some(duration) = resolved_incident.duration_minutes {
            let hours = duration / 60;
            let mins = duration % 60;
            if hours > 0 {
                format!("{}h {}min", hours, mins)
            } else {
                format!("{}min", mins)
            }
        } else {
            "unknown".to_string()
        };

        self.timeline_service
            .log_event(
                incident_id,
                TimelineEventType::Resolved,
                format!("Incident resolved (duration: {})", duration_text),
                resolved_by.clone(),
            )
            .await?;

        // Log to audit
        self.audit_service
            .log_action(
                Some(incident_id),
                "resolve_incident".to_string(),
                resolved_by,
                Some(json!({ "status": incident.status })),
                Some(json!({ "status": IncidentStatus::Resolved })),
                Some(json!({ "duration_minutes": resolved_incident.duration_minutes })),
            )
            .await?;

        info!("Incident resolved: {}", incident_id);
        Ok(resolved_incident)
    }

    pub async fn get_by_id(&self, incident_id: IncidentId) -> IncidentResult<Incident> {
        incident_queries::get_incident_by_id(&self.pool, incident_id).await
    }

    pub async fn get_by_channel(&self, channel_id: &str) -> IncidentResult<Incident> {
        incident_queries::get_incident_by_channel(&self.pool, channel_id).await
    }

    pub async fn get_latest_by_channel(&self, channel_id: &str) -> IncidentResult<Incident> {
        incident_queries::get_latest_incident_by_channel(&self.pool, channel_id).await
    }

    pub async fn validate_commander(
        &self,
        incident: &Incident,
        user_id: &str,
    ) -> IncidentResult<()> {
        if incident.commander_id != user_id {
            return Err(IncidentError::PermissionDenied {
                user_id: user_id.to_string(),
                action: "modify this incident".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_all_valid_transitions() {
        use IncidentStatus::*;

        // From Declared
        assert!(Declared.can_transition_to(&Investigating));
        assert!(Declared.can_transition_to(&Identified));
        assert!(Declared.can_transition_to(&Monitoring));
        assert!(Declared.can_transition_to(&Resolved));

        // From Investigating
        assert!(Investigating.can_transition_to(&Identified));
        assert!(Investigating.can_transition_to(&Monitoring));
        assert!(Investigating.can_transition_to(&Resolved));
        assert!(!Investigating.can_transition_to(&Declared));

        // From Identified
        assert!(Identified.can_transition_to(&Monitoring));
        assert!(Identified.can_transition_to(&Resolved));
        assert!(!Identified.can_transition_to(&Declared));
        assert!(!Identified.can_transition_to(&Investigating));

        // From Monitoring
        assert!(Monitoring.can_transition_to(&Resolved));
        assert!(!Monitoring.can_transition_to(&Declared));
        assert!(!Monitoring.can_transition_to(&Investigating));
        assert!(!Monitoring.can_transition_to(&Identified));

        // From Resolved (terminal)
        assert!(!Resolved.can_transition_to(&Declared));
        assert!(!Resolved.can_transition_to(&Investigating));
        assert!(!Resolved.can_transition_to(&Identified));
        assert!(!Resolved.can_transition_to(&Monitoring));
    }

    #[test]
    fn test_terminal_states() {
        use IncidentStatus::*;

        assert!(!Declared.is_terminal());
        assert!(!Investigating.is_terminal());
        assert!(!Identified.is_terminal());
        assert!(!Monitoring.is_terminal());
        assert!(Resolved.is_terminal());
    }

    #[test]
    fn test_severity_parsing() {
        assert_eq!("P1".parse::<Severity>().unwrap(), Severity::P1);
        assert_eq!("p1".parse::<Severity>().unwrap(), Severity::P1);
        assert_eq!("P2".parse::<Severity>().unwrap(), Severity::P2);
        assert_eq!("P3".parse::<Severity>().unwrap(), Severity::P3);
        assert_eq!("P4".parse::<Severity>().unwrap(), Severity::P4);

        assert!("P5".parse::<Severity>().is_err());
        assert!("invalid".parse::<Severity>().is_err());
    }

    #[test]
    fn test_severity_labels() {
        assert_eq!(Severity::P1.label(), "P1 (Critical)");
        assert_eq!(Severity::P2.label(), "P2 (High)");
        assert_eq!(Severity::P3.label(), "P3 (Medium)");
        assert_eq!(Severity::P4.label(), "P4 (Low)");
    }

    #[test]
    fn test_severity_emojis() {
        assert_eq!(Severity::P1.emoji(), "🔴");
        assert_eq!(Severity::P2.emoji(), "🟡");
        assert_eq!(Severity::P3.emoji(), "🟢");
        assert_eq!(Severity::P4.emoji(), "🟢");
    }
}
