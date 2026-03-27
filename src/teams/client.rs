use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info};

use super::{
    auth::TeamsAuth,
    models::{Activity, Attachment, ChannelAccount, ConversationAccount, SendMessageRequest},
};

const TEAMS_API_BASE: &str = "https://smba.trafficmanager.net/apis/v3";

/// Teams Bot Client
#[derive(Clone)]
pub struct TeamsClient {
    auth: Arc<TeamsAuth>,
    bot_id: String,
    http_client: Client,
}

impl TeamsClient {
    pub fn new(auth: Arc<TeamsAuth>, bot_id: String) -> Self {
        Self {
            auth,
            bot_id,
            http_client: Client::new(),
        }
    }

    /// Send a message to a conversation
    pub async fn send_message(
        &self,
        conversation_id: &str,
        service_url: &str,
        recipient_id: &str,
        recipient_name: Option<String>,
        text: Option<String>,
        attachments: Option<Vec<Attachment>>,
    ) -> Result<()> {
        let token = self.auth.get_access_token().await?;

        let request = SendMessageRequest {
            message_type: "message".to_string(),
            from: ChannelAccount {
                id: self.bot_id.clone(),
                name: Some("OneSiam Incident Bot".to_string()),
                role: Some("bot".to_string()),
                aad_object_id: None,
            },
            conversation: ConversationAccount {
                id: conversation_id.to_string(),
                name: None,
                is_group: None,
                conversation_type: None,
                tenant_id: None,
            },
            recipient: ChannelAccount {
                id: recipient_id.to_string(),
                name: recipient_name,
                role: None,
                aad_object_id: None,
            },
            text,
            attachments,
        };

        let url = format!(
            "{}/conversations/{}/activities",
            service_url, conversation_id
        );

        debug!("Sending message to Teams conversation: {}", conversation_id);
        debug!("Request body: {}", json!(request));

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send Teams message")?;

        if response.status().is_success() {
            info!(
                "Message sent successfully to Teams conversation: {}",
                conversation_id
            );
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(
                "Failed to send Teams message. Status: {}, Body: {}",
                status, body
            );
            anyhow::bail!("Failed to send Teams message. Status: {}", status);
        }
    }

    /// Reply to a message in a conversation
    pub async fn reply_to_message(
        &self,
        conversation_id: &str,
        activity_id: &str,
        service_url: &str,
        text: Option<String>,
        attachments: Option<Vec<Attachment>>,
    ) -> Result<()> {
        let token = self.auth.get_access_token().await?;

        let request = SendMessageRequest {
            message_type: "message".to_string(),
            from: ChannelAccount {
                id: self.bot_id.clone(),
                name: Some("OneSiam Incident Bot".to_string()),
                role: Some("bot".to_string()),
                aad_object_id: None,
            },
            conversation: ConversationAccount {
                id: conversation_id.to_string(),
                name: None,
                is_group: None,
                conversation_type: None,
                tenant_id: None,
            },
            recipient: ChannelAccount {
                id: conversation_id.to_string(),
                name: None,
                role: None,
                aad_object_id: None,
            },
            text,
            attachments,
        };

        let url = format!(
            "{}/conversations/{}/activities/{}/reply",
            service_url, conversation_id, activity_id
        );

        debug!(
            "Replying to message: {} in conversation: {}",
            activity_id, conversation_id
        );

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to reply to Teams message")?;

        if response.status().is_success() {
            info!("Reply sent successfully to Teams");
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(
                "Failed to reply to Teams message. Status: {}, Body: {}",
                status, body
            );
            anyhow::bail!("Failed to reply to Teams message. Status: {}", status);
        }
    }

    /// Get conversation members
    pub async fn get_conversation_members(
        &self,
        conversation_id: &str,
        service_url: &str,
    ) -> Result<Vec<ChannelAccount>> {
        let token = self.auth.get_access_token().await?;

        let url = format!("{}/conversations/{}/members", service_url, conversation_id);

        debug!("Getting conversation members for: {}", conversation_id);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .send()
            .await
            .context("Failed to get conversation members")?;

        if response.status().is_success() {
            let members: Vec<ChannelAccount> = response
                .json()
                .await
                .context("Failed to parse conversation members")?;
            debug!(
                "Retrieved {} members for conversation {}",
                members.len(),
                conversation_id
            );
            Ok(members)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(
                "Failed to get conversation members. Status: {}, Body: {}",
                status, body
            );
            anyhow::bail!("Failed to get conversation members. Status: {}", status);
        }
    }
}

/// Extract service URL from Teams activity
pub fn extract_service_url(activity: &Activity) -> Result<String> {
    activity
        .channel_data
        .as_ref()
        .and_then(|data| data.get("serviceUrl"))
        .and_then(|url| url.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Service URL not found in activity"))
}

/// Extract conversation ID from Teams activity
pub fn extract_conversation_id(activity: &Activity) -> Result<String> {
    activity
        .conversation
        .as_ref()
        .map(|conv| conv.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Conversation ID not found in activity"))
}

/// Extract sender ID from Teams activity
pub fn extract_sender_id(activity: &Activity) -> Result<String> {
    activity
        .from
        .as_ref()
        .map(|from| from.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Sender ID not found in activity"))
}
