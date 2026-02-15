use crate::db::models::{Incident, Severity, TimelineEvent};
use serde_json::{json, Value};

pub fn incident_declared_blocks(incident: &Incident) -> Vec<Value> {
    vec![
        json!({
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": format!("{} {} - Incident Declared", incident.severity.emoji(), incident.severity.label()),
            }
        }),
        json!({
            "type": "section",
            "fields": [
                {
                    "type": "mrkdwn",
                    "text": format!("*Title:*\n{}", incident.title)
                },
                {
                    "type": "mrkdwn",
                    "text": format!("*Service:*\n{}", incident.affected_service)
                },
                {
                    "type": "mrkdwn",
                    "text": format!("*Commander:*\n<@{}>", incident.commander_id)
                },
                {
                    "type": "mrkdwn",
                    "text": format!("*Started:*\n<!date^{}^{{time}}|{}>",
                        incident.declared_at.timestamp(),
                        incident.declared_at.format("%H:%M %Z"))
                },
            ]
        }),
        json!({
            "type": "context",
            "elements": [
                {
                    "type": "mrkdwn",
                    "text": "⚠️ Do NOT post credentials, customer data, or PII in this channel."
                }
            ]
        }),
    ]
}

pub fn status_update_blocks(severity: Severity, message: &str, posted_by: &str) -> Vec<Value> {
    vec![
        json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("{} *Status Update*\n{}\n_Posted by <@{}>_", severity.emoji(), message, posted_by)
            }
        }),
    ]
}

pub fn severity_change_blocks(
    old_severity: Severity,
    new_severity: Severity,
    changed_by: &str,
    reason: Option<&str>,
) -> Vec<Value> {
    let direction = if new_severity as u8 > old_severity as u8 {
        "⬇️ Downgraded"
    } else {
        "⬆️ Escalated"
    };

    let mut blocks = vec![json!({
        "type": "section",
        "text": {
            "type": "mrkdwn",
            "text": format!("{} *Severity {} from {} to {}*\n_Changed by <@{}>_",
                direction,
                if new_severity as u8 > old_severity as u8 { "downgraded" } else { "escalated" },
                old_severity.label(),
                new_severity.label(),
                changed_by)
        }
    })];

    if let Some(reason) = reason {
        blocks.push(json!({
            "type": "context",
            "elements": [{
                "type": "mrkdwn",
                "text": format!("Reason: {}", reason)
            }]
        }));
    }

    blocks
}

pub fn resolution_blocks(incident: &Incident, resolved_by: &str) -> Vec<Value> {
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

    vec![
        json!({
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": "✅ RESOLVED",
            }
        }),
        json!({
            "type": "section",
            "fields": [
                {
                    "type": "mrkdwn",
                    "text": format!("*Duration:*\n{}", duration_text)
                },
                {
                    "type": "mrkdwn",
                    "text": format!("*Resolved by:*\n<@{}>", resolved_by)
                },
            ]
        }),
    ]
}

pub fn timeline_blocks(events: &[TimelineEvent]) -> Vec<Value> {
    let mut blocks = vec![json!({
        "type": "header",
        "text": {
            "type": "plain_text",
            "text": "📋 Incident Timeline",
        }
    })];

    if events.is_empty() {
        blocks.push(json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": "_No timeline events yet._"
            }
        }));
        return blocks;
    }

    let timeline_text = events
        .iter()
        .map(|e| {
            let event_icon = match e.event_type {
                crate::db::models::TimelineEventType::Declared => "🚨",
                crate::db::models::TimelineEventType::StatusUpdate => "📝",
                crate::db::models::TimelineEventType::SeverityChange => "⚠️",
                crate::db::models::TimelineEventType::Resolved => "✅",
            };
            format!(
                "{} *{}* — {}\n_by <@{}>_",
                event_icon,
                e.timestamp.format("%H:%M"),
                e.message,
                e.posted_by
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    blocks.push(json!({
        "type": "section",
        "text": {
            "type": "mrkdwn",
            "text": timeline_text
        }
    }));

    blocks
}

pub fn error_blocks(message: &str) -> Vec<Value> {
    vec![json!({
        "type": "section",
        "text": {
            "type": "mrkdwn",
            "text": format!("❌ *Error:* {}", message)
        }
    })]
}

pub fn permission_denied_blocks(action: &str) -> Vec<Value> {
    vec![json!({
        "type": "section",
        "text": {
            "type": "mrkdwn",
            "text": format!("❌ *Permission denied:* Only the incident commander can {}.", action)
        }
    })]
}
