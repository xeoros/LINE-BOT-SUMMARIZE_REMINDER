use crate::ai::{get_summary_prompt, AIService};
use crate::db::Message;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

fn zai_api_base() -> String {
    std::env::var("ZAI_API_BASE").unwrap_or_else(|_| "https://api.zai.ai".to_string())
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
struct ZaiRequest {
    model: String,
    messages: Vec<ZaiMessage>,
    max_tokens: i32,
}

#[derive(Debug, Serialize)]
struct ZaiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ZaiResponse {
    choices: Vec<ZaiChoice>,
}

#[derive(Debug, Deserialize)]
struct ZaiChoice {
    message: ZaiMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ZaiMessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ZaiError {
    error: ZaiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ZaiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

pub struct ZaiService {
    client: Client,
    api_key: String,
    model: String,
}

impl ZaiService {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        Ok(Self {
            client: build_client()?,
            api_key,
            model,
        })
    }

    #[cfg(not(test))]
    async fn send_request(&self, messages: Vec<ZaiMessage>) -> Result<String> {
        let request = ZaiRequest {
            model: self.model.clone(),
            messages,
            max_tokens: 4096,
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", zai_api_base()))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Zai API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if let Ok(zai_error) = serde_json::from_str::<ZaiError>(&error_text) {
                anyhow::bail!(
                    "Zai API error ({}): {} - {}",
                    status,
                    zai_error.error.error_type,
                    zai_error.error.message
                );
            }

            anyhow::bail!("Zai API error ({}): {}", status, error_text);
        }

        let zai_response: ZaiResponse = response
            .json()
            .await
            .context("Failed to parse Zai API response")?;

        if zai_response.choices.is_empty() {
            anyhow::bail!("Zai API returned empty choices");
        }

        Ok(zai_response.choices[0].message.content.clone())
    }

    #[cfg(test)]
    async fn send_request(&self, _messages: Vec<ZaiMessage>) -> Result<String> {
        if let Ok(value) = std::env::var("ZAI_TEST_RESPONSE") {
            return Ok(value);
        }
        anyhow::bail!("ZAI_TEST_RESPONSE not set")
    }
}

#[async_trait]
impl AIService for ZaiService {
    async fn generate_summary(&self, messages: &[Message]) -> Result<String> {
        let conversation = Message::format_conversation(messages);
        let prompt = get_summary_prompt(&conversation);

        let zai_messages = vec![
            ZaiMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant that summarizes conversations in Thai."
                    .to_string(),
            },
            ZaiMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        self.send_request(zai_messages).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_zai_test_response(value: &str) {
        std::env::set_var("ZAI_TEST_RESPONSE", value);
    }

    fn clear_zai_test_response() {
        std::env::remove_var("ZAI_TEST_RESPONSE");
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
        set_zai_test_response("summary");
        let service = ZaiService::new("key".to_string(), "zai-test".to_string()).unwrap();
        let result = service.generate_summary(&sample_messages()).await.unwrap();
        clear_zai_test_response();
        assert_eq!(result, "summary");
    }
}
