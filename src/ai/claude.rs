use crate::ai::{get_summary_prompt, AIService};
use crate::db::Message;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

fn claude_api_base() -> String {
    std::env::var("CLAUDE_API_BASE").unwrap_or_else(|_| "https://api.anthropic.com".to_string())
}

fn build_client() -> Result<Client> {
    #[cfg(test)]
    {
        Client::builder()
            .no_proxy()
            .build()
            .context("Failed to build reqwest client")
    }
    #[cfg(not(test))]
    {
        Ok(Client::new())
    }
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: i32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    id: String,
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

pub struct ClaudeService {
    client: Client,
    api_key: String,
    model: String,
}

impl ClaudeService {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        Ok(Self {
            client: build_client()?,
            api_key,
            model,
        })
    }

    #[cfg(not(test))]
    async fn send_request(&self, messages: Vec<ClaudeMessage>) -> Result<String> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages,
        };

        let response = self
            .client
            .post(format!("{}/v1/messages", claude_api_base()))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if let Ok(claude_error) = serde_json::from_str::<ClaudeError>(&error_text) {
                anyhow::bail!(
                    "Claude API error ({}): {} - {}",
                    status,
                    claude_error.error_type,
                    claude_error.message
                );
            }

            anyhow::bail!("Claude API error ({}): {}", status, error_text);
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .context("Failed to parse Claude API response")?;

        if claude_response.content.is_empty() {
            anyhow::bail!("Claude API returned empty response");
        }

        Ok(claude_response.content[0].text.clone())
    }

    #[cfg(test)]
    async fn send_request(&self, _messages: Vec<ClaudeMessage>) -> Result<String> {
        if let Ok(value) = std::env::var("CLAUDE_TEST_RESPONSE") {
            return Ok(value);
        }
        anyhow::bail!("CLAUDE_TEST_RESPONSE not set")
    }
}

#[async_trait]
impl AIService for ClaudeService {
    async fn generate_summary(&self, messages: &[Message]) -> Result<String> {
        let conversation = Message::format_conversation(messages);
        let prompt = get_summary_prompt(&conversation);

        let claude_messages = vec![ClaudeMessage {
            role: "user".to_string(),
            content: prompt,
        }];

        self.send_request(claude_messages).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_claude_test_response(value: &str) {
        std::env::set_var("CLAUDE_TEST_RESPONSE", value);
    }

    fn clear_claude_test_response() {
        std::env::remove_var("CLAUDE_TEST_RESPONSE");
    }

    fn sample_messages() -> Vec<Message> {
        vec![Message {
            id: 1,
            message_id: "m1".to_string(),
            source_type: crate::db::SourceType::User,
            source_id: "U1".to_string(),
            sender_id: Some("U1".to_string()),
            display_name: Some("Alice".to_string()),
            message_type: crate::db::MessageType::Text,
            message_text: Some("Hello".to_string()),
            thread_id: None,
            parent_message_id: None,
            created_at: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
                chrono::Utc,
            ),
        }]
    }

    #[tokio::test]
    async fn generate_summary_uses_test_response() {
        set_claude_test_response("summary");
        let service = ClaudeService::new("key".to_string(), "claude-test".to_string()).unwrap();
        let result = service.generate_summary(&sample_messages()).await.unwrap();
        clear_claude_test_response();
        assert_eq!(result, "summary");
    }
}
