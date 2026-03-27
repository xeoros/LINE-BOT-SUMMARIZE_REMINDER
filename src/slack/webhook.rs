use anyhow::{Context, Result};
use hex::FromHex;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, Clone)]
pub struct SlackWebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub challenge: Option<String>,
    pub event: Option<SlackEvent>,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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
pub struct SlackWebhookHandler {
    signing_secret: String,
}

impl SlackWebhookHandler {
    pub fn new(signing_secret: String) -> Self {
        Self { signing_secret }
    }

    pub fn verify_signature(&self, body: &[u8], timestamp: &str, signature: &str) -> Result<bool> {
        let basestring = format!("v0:{}:{}", timestamp, {
            String::from_utf8_lossy(body).to_string()
        });

        let mut hasher = Sha256::new();
        hasher.update(basestring.as_bytes());
        let hash = hasher.finalize();
        let hex_hash = format!("{:x}", hash);

        let decoded_signature = <[u8; 32]>::from_hex(signature)
            .map_err(|e| anyhow::anyhow!("Failed to decode signature: {}", e))?;

        let constant_time_eq = hex_hash
            .as_bytes()
            .iter()
            .zip(decoded_signature.iter())
            .all(|(a, b)| a == b);

        Ok(constant_time_eq)
    }

    pub fn parse_webhook_body(&self, body: &str) -> Result<SlackWebhookEvent> {
        let event: SlackWebhookEvent =
            serde_json::from_str(body).context("Failed to parse Slack webhook body")?;

        Ok(event)
    }
}
