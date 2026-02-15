use crate::db::models::{IncidentId, TimelineEvent, TimelineEventType};
use crate::db::queries::timeline as timeline_queries;
use crate::error::IncidentResult;
use sqlx_postgres::PgPool;

pub struct TimelineService {
    pool: PgPool,
}

impl TimelineService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn log_event(
        &self,
        incident_id: IncidentId,
        event_type: TimelineEventType,
        message: String,
        posted_by: String,
    ) -> IncidentResult<TimelineEvent> {
        timeline_queries::log_event(&self.pool, incident_id, event_type, message, posted_by).await
    }

    pub async fn get_timeline(
        &self,
        incident_id: IncidentId,
    ) -> IncidentResult<Vec<TimelineEvent>> {
        timeline_queries::get_timeline(&self.pool, incident_id).await
    }

    pub fn format_as_markdown(&self, events: &[TimelineEvent]) -> String {
        if events.is_empty() {
            return "_No timeline events yet._".to_string();
        }

        events
            .iter()
            .map(|e| {
                let event_icon = match e.event_type {
                    TimelineEventType::Declared => "🚨",
                    TimelineEventType::StatusUpdate => "📝",
                    TimelineEventType::SeverityChange => "⚠️",
                    TimelineEventType::Resolved => "✅",
                };
                format!(
                    "**{}** — {} {}\n→ {}\n",
                    e.timestamp.format("%H:%M"),
                    event_icon,
                    format!("{:?}", e.event_type).replace("_", " "),
                    e.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
