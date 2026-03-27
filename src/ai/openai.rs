use crate::ai::{get_summary_prompt, AIService};
use crate::db::Message;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

fn openai_api_base() -> String {
    std::env::var("OPENAI_API_BASE").unwrap_or_else(|_| "https://api.openai.com".to_string())
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
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: i32,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageResponse,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    error: OpenAIErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

pub struct OpenAIService {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenAIService {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        Ok(Self {
            client: build_client()?,
            api_key,
            model,
        })
    }

    #[cfg(not(test))]
    async fn send_request(&self, messages: Vec<OpenAIMessage>) -> Result<String> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            max_tokens: 4096,
        };

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", openai_api_base()))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if let Ok(openai_error) = serde_json::from_str::<OpenAIError>(&error_text) {
                anyhow::bail!(
                    "OpenAI API error ({}): {} - {}",
                    status,
                    openai_error.error.error_type,
                    openai_error.error.message
                );
            }

            anyhow::bail!("OpenAI API error ({}): {}", status, error_text);
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI API response")?;

        if openai_response.choices.is_empty() {
            anyhow::bail!("OpenAI API returned empty choices");
        }

        Ok(openai_response.choices[0].message.content.clone())
    }

    #[cfg(test)]
    async fn send_request(&self, _messages: Vec<OpenAIMessage>) -> Result<String> {
        if let Ok(value) = std::env::var("OPENAI_TEST_RESPONSE") {
            return Ok(value);
        }
        anyhow::bail!("OPENAI_TEST_RESPONSE not set")
    }
}

#[async_trait]
impl AIService for OpenAIService {
    async fn generate_summary(&self, messages: &[Message]) -> Result<String> {
        let conversation = Message::format_conversation(messages);
        let prompt = get_summary_prompt(&conversation);

        let openai_messages = vec![
            OpenAIMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant that summarizes conversations in Thai."
                    .to_string(),
            },
            OpenAIMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        self.send_request(openai_messages).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_openai_test_response(value: &str) {
        std::env::set_var("OPENAI_TEST_RESPONSE", value);
    }

    fn clear_openai_test_response() {
        std::env::remove_var("OPENAI_TEST_RESPONSE");
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
        set_openai_test_response("summary");
        let service = OpenAIService::new("key".to_string(), "gpt-test".to_string()).unwrap();
        let result = service.generate_summary(&sample_messages()).await.unwrap();
        clear_openai_test_response();
        assert_eq!(result, "summary");
    }
}
