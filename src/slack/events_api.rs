use anyhow::{Context, Result};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct SlackEvent {
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub user: Option<String>,
    pub channel: Option<String>,
    pub ts: Option<String>,
    pub text: Option<String>,
    pub thread_ts: Option<String>,
    pub subtype: Option<String>,
    #[serde(rename = "client_msg_id")]
    pub client_msg_id: Option<String>,
    pub message: Option<SlackMessage>,
    #[serde(rename = "parent_user_message_id")]
    pub parent_user_message_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SlackMessage {
    #[serde(rename = "type")]
    pub message_type: Option<String>,
    pub user: Option<String>,
    pub channel: Option<String>,
    pub ts: Option<String>,
    pub text: Option<String>,
    pub thread_ts: Option<String>,
    pub subtype: Option<String>,
    #[serde(rename = "parent_user_message_id")]
    pub parent_user_message_id: Option<String>,
    #[serde(rename = "user_id")]
    pub user_id_field: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SlackEventsAPI {
    app_token: String,
    connected: Arc<RwLock<bool>>,
}

impl SlackEventsAPI {
    pub fn new(app_token: String) -> Self {
        Self {
            app_token,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn connect(&self) -> Result<()> {
        // In a full implementation, this would establish WebSocket connection
        // For this implementation, we'll use the structure but mark as simulated
        info!("Slack Events API connection initiated (simulated)");
        *self.connected.write().await = true;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        info!("Slack Events API disconnecting");
        *self.connected.write().await = false;
        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    pub fn parse_event(&self, data: &[u8]) -> Result<SlackEvent> {
        let text = String::from_utf8_lossy(data).to_string();
        let event: SlackEvent = serde_json::from_str(&text)
            .with_context(|| "Failed to parse Slack Events API event".to_string())?;

        Ok(event)
    }
}
