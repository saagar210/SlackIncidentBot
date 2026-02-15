use crate::db::models::{Incident, IncidentId};
use crate::error::IncidentResult;
use crate::services::timeline::TimelineService;
use sqlx::PgPool;

pub struct PostmortemService {
    pool: PgPool,
    timeline_service: TimelineService,
}

impl PostmortemService {
    pub fn new(pool: PgPool) -> Self {
        let timeline_service = TimelineService::new(pool.clone());
        Self {
            pool,
            timeline_service,
        }
    }

    pub async fn generate(&self, incident: &Incident) -> IncidentResult<String> {
        let events = self.timeline_service.get_timeline(incident.id).await?;

        let duration_text = if let Some(duration) = incident.duration_minutes {
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

        let timeline_md = self.timeline_service.format_as_markdown(&events);

        let template = format!(
            r#"# Postmortem: {} ({})

## Incident Summary
- **Duration**: {} ({} - {})
- **Severity**: {}
- **Status**: Resolved
- **Affected Service**: {}
- **Incident Commander**: <@{}>
- **Impact**: [TO BE FILLED BY TEAM]
- **Root Cause**: [TO BE FILLED BY TEAM]

## Timeline

{}

## Action Items
- [ ] [TO BE ADDED BY TEAM]

## Lessons Learned
- [TO BE FILLED BY TEAM]

---
*Generated on {} by Incident Bot*
*Edit this postmortem and use `/incident postmortem publish` to post to Confluence (Phase 2)*
"#,
            incident.title,
            incident.declared_at.format("%Y-%m-%d"),
            duration_text,
            incident.declared_at.format("%Y-%m-%d %H:%M %Z"),
            incident.resolved_at.expect("Resolved incidents must have resolved_at timestamp").format("%Y-%m-%d %H:%M %Z").to_string(),
            incident.severity.label(),
            incident.affected_service,
            incident.commander_id,
            timeline_md,
            chrono::Utc::now().format("%Y-%m-%d %H:%M %Z"),
        );

        Ok(template)
    }
}
