pub mod claude;
pub mod gemini;
pub mod minimax;
pub mod openai;
pub mod prompt;
pub mod zai;

use crate::db::Message;
use async_trait::async_trait;

#[async_trait]
pub trait AIService: Send + Sync {
    async fn generate_summary(&self, messages: &[Message]) -> anyhow::Result<String>;
}

pub fn create_ai_service(
    provider: crate::config::AIProvider,
    claude_api_key: Option<String>,
    claude_model: String,
    openai_api_key: Option<String>,
    openai_model: String,
    gemini_api_key: Option<String>,
    gemini_model: String,
    minimax_api_key: Option<String>,
    minimax_model: String,
    zai_api_key: Option<String>,
    zai_model: String,
) -> anyhow::Result<Box<dyn AIService>> {
    match provider {
        crate::config::AIProvider::Claude => {
            let api_key =
                claude_api_key.ok_or_else(|| anyhow::anyhow!("Claude API key is required"))?;
            Ok(Box::new(claude::ClaudeService::new(api_key, claude_model)?))
        }
        crate::config::AIProvider::OpenAI => {
            let api_key =
                openai_api_key.ok_or_else(|| anyhow::anyhow!("OpenAI API key is required"))?;
            Ok(Box::new(openai::OpenAIService::new(api_key, openai_model)?))
        }
        crate::config::AIProvider::Gemini => {
            let api_key =
                gemini_api_key.ok_or_else(|| anyhow::anyhow!("Gemini API key is required"))?;
            Ok(Box::new(gemini::GeminiService::new(api_key, gemini_model)?))
        }
        crate::config::AIProvider::Minimax => {
            let api_key =
                minimax_api_key.ok_or_else(|| anyhow::anyhow!("Minimax API key is required"))?;
            Ok(Box::new(minimax::MinimaxService::new(
                api_key,
                minimax_model,
            )?))
        }
        crate::config::AIProvider::Zai => {
            let api_key =
                zai_api_key.ok_or_else(|| anyhow::anyhow!("Zai API key is required"))?;
            Ok(Box::new(zai::ZaiService::new(api_key, zai_model)?))
        }
    }
}

pub use claude::ClaudeService;
pub use gemini::GeminiService;
pub use minimax::MinimaxService;
pub use openai::OpenAIService;
pub use prompt::{get_summary_prompt, get_title_prompt};
pub use zai::ZaiService;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AIProvider;

    fn models() -> (String, String, String, String, String) {
        (
            "claude-model".to_string(),
            "openai-model".to_string(),
            "gemini-model".to_string(),
            "minimax-model".to_string(),
            "zai-model".to_string(),
        )
    }

    #[test]
    fn create_ai_service_requires_claude_key() {
        let (claude_model, openai_model, gemini_model, minimax_model, zai_model) = models();
        let result = create_ai_service(
            AIProvider::Claude,
            None,
            claude_model,
            None,
            openai_model,
            None,
            gemini_model,
            None,
            minimax_model,
            None,
            zai_model,
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_ai_service_requires_openai_key() {
        let (claude_model, openai_model, gemini_model, minimax_model, zai_model) = models();
        let result = create_ai_service(
            AIProvider::OpenAI,
            None,
            claude_model,
            None,
            openai_model,
            None,
            gemini_model,
            None,
            minimax_model,
            None,
            zai_model,
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_ai_service_requires_gemini_key() {
        let (claude_model, openai_model, gemini_model, minimax_model, zai_model) = models();
        let result = create_ai_service(
            AIProvider::Gemini,
            None,
            claude_model,
            None,
            openai_model,
            None,
            gemini_model,
            None,
            minimax_model,
            None,
            zai_model,
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_ai_service_requires_minimax_key() {
        let (claude_model, openai_model, gemini_model, minimax_model, zai_model) = models();
        let result = create_ai_service(
            AIProvider::Minimax,
            None,
            claude_model,
            None,
            openai_model,
            None,
            gemini_model,
            None,
            minimax_model,
            None,
            zai_model,
        );
        assert!(result.is_err());
    }

    #[test]
    fn create_ai_service_requires_zai_key() {
        let (claude_model, openai_model, gemini_model, minimax_model, zai_model) = models();
        let result = create_ai_service(
            AIProvider::Zai,
            None,
            claude_model,
            None,
            openai_model,
            None,
            gemini_model,
            None,
            minimax_model,
            None,
            zai_model,
        );
        assert!(result.is_err());
    }

}
