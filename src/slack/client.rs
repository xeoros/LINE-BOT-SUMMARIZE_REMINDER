use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const SLACK_API_BASE: &str = "https://slack.com/api";

#[derive(Debug, Serialize)]
struct PostMessageRequest {
    channel: String,
    text: String,
    thread_ts: Option<String>,
}

#[derive(Debug, Serialize)]
struct ThreadRepliesRequest {
    channel: String,
    ts: String,
}

#[derive(Debug, Serialize)]
struct ConversationHistoryRequest {
    channel: String,
    limit: i32,
}

#[derive(Debug, Deserialize)]
struct SlackApiResponse<T> {
    ok: bool,
    #[serde(default)]
    error: String,
    #[serde(rename = "messages")]
    data: Option<T>,
    message: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct SlackUserProfile {
    pub id: String,
    pub name: String,
    pub real_name: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConversationMessage {
    ts: String,
    thread_ts: Option<String>,
    user: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ThreadReplyMessage {
    ts: String,
    thread_ts: Option<String>,
    user: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PermalinkResponse {
    permalink: String,
}

#[derive(Debug, Clone)]
pub struct SlackClient {
    client: Client,
    bot_token: String,
}

impl SlackClient {
    pub fn new(bot_token: String) -> Self {
        Self {
            client: Client::new(),
            bot_token,
        }
    }

    async fn post_to_endpoint<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        data: &T,
    ) -> Result<R> {
        let response = self
            .client
            .post(&format!("{}/{}", SLACK_API_BASE, endpoint))
            .bearer_auth(&self.bot_token)
            .json(data)
            .send()
            .await
            .context(format!(
                "Failed to post to Slack API endpoint: {}",
                endpoint
            ))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Slack API error: {}", error_text);
        }

        let api_response: SlackApiResponse<R> = response
            .json()
            .await
            .context("Failed to parse Slack API response")?;

        if !api_response.ok {
            anyhow::bail!("Slack API returned error: {}", api_response.error);
        }

        Ok(api_response.message.unwrap_or_else(|| {
            api_response
                .data
                .expect("Expected message or data in response")
        }))
    }

    pub async fn post_message(&self, channel_id: &str, text: &str) -> Result<()> {
        let request = PostMessageRequest {
            channel: channel_id.to_string(),
            text: text.to_string(),
            thread_ts: None,
        };

        let _response: ConversationMessage =
            self.post_to_endpoint("chat.postMessage", &request).await?;

        Ok(())
    }

    pub async fn reply_to_message(
        &self,
        channel_id: &str,
        thread_ts: &str,
        text: &str,
    ) -> Result<()> {
        let request = PostMessageRequest {
            channel: channel_id.to_string(),
            text: text.to_string(),
            thread_ts: Some(thread_ts.to_string()),
        };

        let _response: ConversationMessage =
            self.post_to_endpoint("chat.postMessage", &request).await?;

        Ok(())
    }

    pub async fn get_conversation_history(
        &self,
        channel_id: &str,
        limit: i32,
    ) -> Result<Vec<(String, Option<String>, String, String)>> {
        let request = ConversationHistoryRequest {
            channel: channel_id.to_string(),
            limit,
        };

        let response: Vec<ConversationMessage> = self
            .post_to_endpoint("conversations.history", &request)
            .await?;

        let messages: Vec<(String, Option<String>, String, String)> = response
            .into_iter()
            .map(|msg| {
                let user_id = msg.user.unwrap_or_else(|| "unknown".to_string());
                let text = msg.text.unwrap_or_else(|| "".to_string());
                (msg.ts, msg.thread_ts, user_id, text)
            })
            .collect();

        Ok(messages)
    }

    pub async fn get_thread_replies(
        &self,
        channel_id: &str,
        thread_ts: &str,
    ) -> Result<Vec<(String, String, String)>> {
        let request = ThreadRepliesRequest {
            channel: channel_id.to_string(),
            ts: thread_ts.to_string(),
        };

        let response: Vec<ThreadReplyMessage> = self
            .post_to_endpoint("conversations.replies", &request)
            .await?;

        let replies: Vec<(String, String, String)> = response
            .into_iter()
            .map(|msg| {
                let user_id = msg.user.unwrap_or_else(|| "unknown".to_string());
                let text = msg.text.unwrap_or_else(|| "".to_string());
                (msg.ts, user_id, text)
            })
            .collect();

        Ok(replies)
    }

    pub async fn get_permalink(&self, channel_id: &str, message_ts: &str) -> Result<String> {
        #[derive(Debug, Serialize)]
        struct PermalinkRequest {
            channel: String,
            message_ts: String,
        }

        let request = PermalinkRequest {
            channel: channel_id.to_string(),
            message_ts: message_ts.to_string(),
        };

        let response: PermalinkResponse =
            self.post_to_endpoint("chat.getPermalink", &request).await?;

        Ok(response.permalink)
    }

    pub async fn get_user_info(&self, user_id: &str) -> Result<SlackUserProfile> {
        let url = format!("{}/users.profile", SLACK_API_BASE);

        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.bot_token)
            .query(&[("user", user_id)])
            .send()
            .await
            .context("Failed to get user profile from Slack API")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Slack API error: {}", error_text);
        }

        let profile: SlackUserProfile = response
            .json()
            .await
            .context("Failed to parse user profile")?;

        Ok(profile)
    }
}
