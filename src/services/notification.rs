use crate::config::AppConfig;
use crate::db::models::{Incident, IncidentId, NotificationStatus, NotificationType, Severity};
use crate::db::queries::notifications;
use crate::error::IncidentResult;
use crate::slack::client::SlackClient;
use serde_json::Value;
use sqlx_postgres::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

type NotificationThrottleKey = (String, IncidentId);
type NotificationThrottleMap = HashMap<NotificationThrottleKey, chrono::DateTime<chrono::Utc>>;

pub struct NotificationService {
    pool: PgPool,
    slack_client: SlackClient,
    config: Arc<AppConfig>,
    // Throttle map: (recipient, incident_id) -> last notification timestamp
    throttle_map: Arc<Mutex<NotificationThrottleMap>>,
}

impl NotificationService {
    pub fn new(pool: PgPool, slack_client: SlackClient, config: Arc<AppConfig>) -> Self {
        Self {
            pool,
            slack_client,
            config,
            throttle_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn notify_incident_declared(
        &self,
        incident: &Incident,
        blocks: Vec<Value>,
    ) -> IncidentResult<()> {
        self.route_by_severity(incident, blocks, "incident_declared")
            .await
    }

    pub async fn notify_status_update(
        &self,
        incident: &Incident,
        blocks: Vec<Value>,
    ) -> IncidentResult<()> {
        // Status updates only go to incident channel
        if let Some(channel_id) = &incident.slack_channel_id {
            self.send_to_channel(incident.id, channel_id, &blocks)
                .await?;
        }
        Ok(())
    }

    pub async fn notify_severity_change(
        &self,
        incident: &Incident,
        old_severity: Severity,
        blocks: Vec<Value>,
    ) -> IncidentResult<()> {
        // If escalating TO P1 or P2, send broader notifications
        let escalating_to_p1 = incident.severity == Severity::P1 && old_severity != Severity::P1;
        let escalating_to_p2 = incident.severity == Severity::P2 && old_severity != Severity::P2;

        if escalating_to_p1 || escalating_to_p2 {
            self.route_by_severity(incident, blocks, "severity_escalation")
                .await
        } else {
            // Downgrade or same severity: only incident channel
            if let Some(channel_id) = &incident.slack_channel_id {
                self.send_to_channel(incident.id, channel_id, &blocks)
                    .await?;
            }
            Ok(())
        }
    }

    pub async fn notify_resolution(
        &self,
        incident: &Incident,
        blocks: Vec<Value>,
    ) -> IncidentResult<()> {
        // Resolution notifications go to same channels as initial declaration
        self.route_by_severity(incident, blocks, "incident_resolved")
            .await
    }

    async fn route_by_severity(
        &self,
        incident: &Incident,
        blocks: Vec<Value>,
        _event_type: &str,
    ) -> IncidentResult<()> {
        match incident.severity {
            Severity::P1 => {
                // P1: incident channel + #general + DM execs
                if let Some(channel_id) = &incident.slack_channel_id {
                    self.send_to_channel(incident.id, channel_id, &blocks)
                        .await?;
                }

                // Post to all P1 channels
                for channel_id in &self.config.p1_channels {
                    self.send_to_channel(incident.id, channel_id, &blocks)
                        .await?;
                }

                // DM all P1 recipients
                for user_id in &self.config.p1_users {
                    if self.should_send_dm(user_id, incident.id).await {
                        self.send_dm(incident.id, user_id, &blocks).await?;
                    } else {
                        info!("Throttling DM to {} for incident {}", user_id, incident.id);
                        // Log throttled notification to database for audit trail
                        notifications::log_notification(
                            &self.pool,
                            incident.id,
                            NotificationType::SlackDm,
                            user_id.to_string(),
                            NotificationStatus::Throttled,
                            None,
                        )
                        .await?;
                    }
                }
            }
            Severity::P2 => {
                // P2: incident channel + #engineering
                if let Some(channel_id) = &incident.slack_channel_id {
                    self.send_to_channel(incident.id, channel_id, &blocks)
                        .await?;
                }

                for channel_id in &self.config.p2_channels {
                    self.send_to_channel(incident.id, channel_id, &blocks)
                        .await?;
                }
            }
            Severity::P3 | Severity::P4 => {
                // P3/P4: incident channel only
                if let Some(channel_id) = &incident.slack_channel_id {
                    self.send_to_channel(incident.id, channel_id, &blocks)
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn should_send_dm(&self, user_id: &str, incident_id: IncidentId) -> bool {
        let mut throttle_map = self.throttle_map.lock().await;

        // Cleanup: Remove entries older than 10 minutes (2x throttle window)
        // This prevents unbounded memory growth
        let now = chrono::Utc::now();
        throttle_map
            .retain(|_, last_sent| now.signed_duration_since(*last_sent).num_seconds() < 600);

        let key = (user_id.to_string(), incident_id);

        if let Some(last_sent) = throttle_map.get(&key) {
            let elapsed = now.signed_duration_since(*last_sent);

            // Throttle: no more than 1 DM per 5 minutes
            if elapsed.num_seconds() < 300 {
                return false;
            }
        }

        // Update throttle map
        throttle_map.insert(key, now);
        true
    }

    async fn send_to_channel(
        &self,
        incident_id: IncidentId,
        channel_id: &str,
        blocks: &[Value],
    ) -> IncidentResult<()> {
        // Clone only when actually sending to reduce memory allocations
        match self
            .slack_client
            .post_message(channel_id, blocks.to_vec())
            .await
        {
            Ok(_) => {
                notifications::log_notification(
                    &self.pool,
                    incident_id,
                    NotificationType::SlackChannel,
                    channel_id.to_string(),
                    NotificationStatus::Sent,
                    None,
                )
                .await?;
                Ok(())
            }
            Err(e) => {
                error!("Failed to post to channel {}: {}", channel_id, e);
                notifications::log_notification(
                    &self.pool,
                    incident_id,
                    NotificationType::SlackChannel,
                    channel_id.to_string(),
                    NotificationStatus::Failed,
                    Some(e.to_string()),
                )
                .await?;
                Err(e)
            }
        }
    }

    async fn send_dm(
        &self,
        incident_id: IncidentId,
        user_id: &str,
        blocks: &[Value],
    ) -> IncidentResult<()> {
        // Clone only when actually sending to reduce memory allocations
        match self.slack_client.send_dm(user_id, blocks.to_vec()).await {
            Ok(_) => {
                notifications::log_notification(
                    &self.pool,
                    incident_id,
                    NotificationType::SlackDm,
                    user_id.to_string(),
                    NotificationStatus::Sent,
                    None,
                )
                .await?;
                Ok(())
            }
            Err(e) => {
                warn!("Failed to send DM to {}: {}", user_id, e);
                notifications::log_notification(
                    &self.pool,
                    incident_id,
                    NotificationType::SlackDm,
                    user_id.to_string(),
                    NotificationStatus::Failed,
                    Some(e.to_string()),
                )
                .await?;
                // Don't fail the whole operation if one DM fails
                Ok(())
            }
        }
    }
}
