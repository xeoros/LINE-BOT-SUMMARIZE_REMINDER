use crate::ai::{get_summary_prompt, AIService};
use crate::db::Message;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

fn minimax_api_base() -> String {
    std::env::var("MINIMAX_API_BASE").unwrap_or_else(|_| "https://api.minimax.chat".to_string())
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
struct MinimaxRequest {
    model: String,
    messages: Vec<MinimaxMessage>,
    max_tokens: i32,
    #[serde(rename = "temperature")]
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct MinimaxMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MinimaxResponse {
    choices: Vec<MinimaxChoice>,
    #[serde(rename = "base_resp")]
    base_resp: Option<MinimaxBaseResp>,
}

#[derive(Debug, Deserialize)]
struct MinimaxChoice {
    message: MinimaxMessageResponse,
}

#[derive(Debug, Deserialize)]
struct MinimaxMessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct MinimaxBaseResp {
    #[serde(rename = "status_code")]
    status_code: i32,
    #[serde(rename = "status_msg")]
    status_msg: String,
}

pub struct MinimaxService {
    client: Client,
    api_key: String,
    model: String,
}

impl MinimaxService {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        Ok(Self {
            client: build_client()?,
            api_key,
            model,
        })
    }

    #[cfg(not(test))]
    async fn send_request(&self, messages: Vec<MinimaxMessage>) -> Result<String> {
        let request = MinimaxRequest {
            model: self.model.clone(),
            messages,
            max_tokens: 4096,
            temperature: 0.7,
        };

        let response = self
            .client
            .post(format!("{}/v1/text/chatcompletion_v2", minimax_api_base()))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Minimax API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Minimax API error ({}): {}", status, error_text);
        }

        let minimax_response: MinimaxResponse = response
            .json()
            .await
            .context("Failed to parse Minimax API response")?;

        // Check for API-level errors
        if let Some(base_resp) = minimax_response.base_resp {
            if base_resp.status_code != 0 {
                anyhow::bail!(
                    "Minimax API error: {} - {}",
                    base_resp.status_code,
                    base_resp.status_msg
                );
            }
        }

        if minimax_response.choices.is_empty() {
            anyhow::bail!("Minimax API returned empty choices");
        }

        Ok(minimax_response.choices[0].message.content.clone())
    }

    #[cfg(test)]
    async fn send_request(&self, _messages: Vec<MinimaxMessage>) -> Result<String> {
        if let Ok(value) = std::env::var("MINIMAX_TEST_RESPONSE") {
            return Ok(value);
        }
        anyhow::bail!("MINIMAX_TEST_RESPONSE not set")
    }
}

#[async_trait]
impl AIService for MinimaxService {
    async fn generate_summary(&self, messages: &[Message]) -> Result<String> {
        let conversation = Message::format_conversation(messages);
        let prompt = get_summary_prompt(&conversation);

        let minimax_messages = vec![
            MinimaxMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant that summarizes conversations in Thai."
                    .to_string(),
            },
            MinimaxMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        self.send_request(minimax_messages).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_minimax_test_response(value: &str) {
        std::env::set_var("MINIMAX_TEST_RESPONSE", value);
    }

    fn clear_minimax_test_response() {
        std::env::remove_var("MINIMAX_TEST_RESPONSE");
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
        set_minimax_test_response("summary");
        let service = MinimaxService::new("key".to_string(), "minimax-test".to_string()).unwrap();
        let result = service.generate_summary(&sample_messages()).await.unwrap();
        clear_minimax_test_response();
        assert_eq!(result, "summary");
    }
}
