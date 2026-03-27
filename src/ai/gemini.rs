use crate::ai::{get_summary_prompt, AIService};
use crate::db::Message;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

fn gemini_api_base() -> String {
    std::env::var("GEMINI_API_BASE")
        .unwrap_or_else(|_| "https://generativelanguage.googleapis.com".to_string())
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
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    max_output_tokens: i32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    error: GeminiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorDetail {
    code: i32,
    message: String,
    status: String,
}

pub struct GeminiService {
    client: Client,
    api_key: String,
    model: String,
}

impl GeminiService {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        Ok(Self {
            client: build_client()?,
            api_key,
            model,
        })
    }

    #[cfg(not(test))]
    async fn send_request(&self, prompt: &str) -> Result<String> {
        let generation_config = GenerationConfig {
            max_output_tokens: 4096,
            temperature: 0.7,
        };

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: Some(generation_config),
        };

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            gemini_api_base(),
            self.model,
            self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if let Ok(gemini_error) = serde_json::from_str::<GeminiError>(&error_text) {
                anyhow::bail!(
                    "Gemini API error ({}): {} - {}",
                    status,
                    gemini_error.error.status,
                    gemini_error.error.message
                );
            }

            anyhow::bail!("Gemini API error ({}): {}", status, error_text);
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Gemini API response")?;

        if gemini_response.candidates.is_empty() {
            anyhow::bail!("Gemini API returned no candidates");
        }

        let candidate = &gemini_response.candidates[0];

        if let Some(ref finish_reason) = candidate.finish_reason {
            if finish_reason != "STOP" && finish_reason != "MAX_TOKENS" {
                anyhow::bail!("Gemini generation finished with reason: {}", finish_reason);
            }
        }

        candidate
            .content
            .as_ref()
            .and_then(|c| c.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow::anyhow!("Gemini API returned empty response"))
    }

    #[cfg(test)]
    async fn send_request(&self, _prompt: &str) -> Result<String> {
        if let Ok(value) = std::env::var("GEMINI_TEST_RESPONSE") {
            return Ok(value);
        }
        anyhow::bail!("GEMINI_TEST_RESPONSE not set")
    }
}

#[async_trait]
impl AIService for GeminiService {
    async fn generate_summary(&self, messages: &[Message]) -> Result<String> {
        let conversation = Message::format_conversation(messages);
        let prompt = get_summary_prompt(&conversation);

        self.send_request(&prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_gemini_test_response(value: &str) {
        std::env::set_var("GEMINI_TEST_RESPONSE", value);
    }

    fn clear_gemini_test_response() {
        std::env::remove_var("GEMINI_TEST_RESPONSE");
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
        set_gemini_test_response("summary");
        let service = GeminiService::new("key".to_string(), "gemini-test".to_string()).unwrap();
        let result = service.generate_summary(&sample_messages()).await.unwrap();
        clear_gemini_test_response();
        assert_eq!(result, "summary");
    }
}
