use crate::error::{IncidentError, IncidentResult};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, error};

#[derive(Clone)]
pub struct SlackClient {
    http_client: Client,
    bot_token: String,
}

#[derive(Debug, Deserialize)]
struct SlackResponse<T> {
    ok: bool,
    error: Option<String>,
    #[serde(flatten)]
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct ChannelsListResponse {
    channels: Vec<Channel>,
    response_metadata: Option<ResponseMetadata>,
}

#[derive(Debug, Deserialize)]
struct ResponseMetadata {
    next_cursor: Option<String>,
}

impl SlackClient {
    pub fn new(bot_token: String) -> Self {
        // Set 30-second timeout to prevent hanging requests to Slack API
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            bot_token,
        }
    }

    async fn call_api<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        payload: Value,
    ) -> IncidentResult<T> {
        debug!("Calling Slack API: {}", method);

        let response = self
            .http_client
            .post(format!("https://slack.com/api/{}", method))
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&payload)
            .send()
            .await?;

        let slack_response: SlackResponse<T> = response.json().await?;

        if !slack_response.ok {
            let error_code = slack_response
                .error
                .unwrap_or_else(|| "unknown".to_string());
            error!("Slack API error: {}", error_code);
            return Err(IncidentError::SlackAPIError {
                message: format!("API call failed: {}", method),
                slack_error_code: error_code,
            });
        }

        slack_response
            .data
            .ok_or_else(|| IncidentError::SlackAPIError {
                message: "No data in response".to_string(),
                slack_error_code: "no_data".to_string(),
            })
    }

    pub async fn create_conversation(&self, name: &str) -> IncidentResult<String> {
        #[derive(Deserialize)]
        struct CreateResponse {
            channel: Channel,
        }

        let response: CreateResponse = self
            .call_api(
                "conversations.create",
                json!({
                    "name": name,
                    "is_private": false,
                }),
            )
            .await?;

        Ok(response.channel.id)
    }

    pub async fn list_conversations(&self) -> IncidentResult<Vec<Channel>> {
        // Implement cursor-based pagination for workspaces with >1000 channels
        let mut all_channels = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut params = json!({
                "exclude_archived": true,
                "limit": 1000,
            });

            if let Some(ref c) = cursor {
                params["cursor"] = json!(c);
            }

            let response: ChannelsListResponse =
                self.call_api("conversations.list", params).await?;

            all_channels.extend(response.channels);

            // Check if there are more pages
            match response.response_metadata.and_then(|m| m.next_cursor) {
                Some(next) if !next.is_empty() => cursor = Some(next),
                _ => break,
            }
        }

        Ok(all_channels)
    }

    pub async fn invite_users(
        &self,
        channel_id: &str,
        user_ids: Vec<String>,
    ) -> IncidentResult<()> {
        if user_ids.is_empty() {
            return Ok(());
        }

        let _: Value = self
            .call_api(
                "conversations.invite",
                json!({
                    "channel": channel_id,
                    "users": user_ids.join(","),
                }),
            )
            .await?;

        Ok(())
    }

    pub async fn archive_channel(&self, channel_id: &str) -> IncidentResult<()> {
        let _: Value = self
            .call_api(
                "conversations.archive",
                json!({
                    "channel": channel_id,
                }),
            )
            .await?;

        Ok(())
    }

    pub async fn post_message(
        &self,
        channel_id: &str,
        blocks: Vec<Value>,
    ) -> IncidentResult<String> {
        #[derive(Deserialize)]
        struct PostResponse {
            ts: String,
        }

        let response: PostResponse = self
            .call_api(
                "chat.postMessage",
                json!({
                    "channel": channel_id,
                    "blocks": blocks,
                }),
            )
            .await?;

        Ok(response.ts)
    }

    pub async fn pin_message(&self, channel_id: &str, timestamp: &str) -> IncidentResult<()> {
        let _: Value = self
            .call_api(
                "pins.add",
                json!({
                    "channel": channel_id,
                    "timestamp": timestamp,
                }),
            )
            .await?;

        Ok(())
    }

    pub async fn send_dm(&self, user_id: &str, blocks: Vec<Value>) -> IncidentResult<()> {
        // First open a DM conversation
        #[derive(Deserialize)]
        struct OpenResponse {
            channel: Channel,
        }

        let open_response: OpenResponse = self
            .call_api(
                "conversations.open",
                json!({
                    "users": user_id,
                }),
            )
            .await?;

        // Then post the message
        self.post_message(&open_response.channel.id, blocks).await?;

        Ok(())
    }

    pub async fn open_modal(&self, trigger_id: &str, view: Value) -> IncidentResult<()> {
        let _: Value = self
            .call_api(
                "views.open",
                json!({
                    "trigger_id": trigger_id,
                    "view": view,
                }),
            )
            .await?;

        Ok(())
    }

    pub async fn post_to_response_url(
        &self,
        response_url: &str,
        blocks: Vec<Value>,
    ) -> IncidentResult<()> {
        let response = self
            .http_client
            .post(response_url)
            .json(&json!({
                "blocks": blocks,
                "response_type": "ephemeral",
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(IncidentError::SlackAPIError {
                message: "Failed to post to response_url".to_string(),
                slack_error_code: response.status().to_string(),
            });
        }

        Ok(())
    }
}
