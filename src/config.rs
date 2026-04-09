use anyhow::{Context, Result};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationMode {
    Webhook,
    EventsAPI,
    Both,
}

impl FromStr for IntegrationMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "webhook" => Ok(IntegrationMode::Webhook),
            "events_api" => Ok(IntegrationMode::EventsAPI),
            "eventsapi" => Ok(IntegrationMode::EventsAPI),
            "both" => Ok(IntegrationMode::Both),
            _ => Err(format!(
                "Invalid integration mode: {}. Must be 'webhook', 'events_api', or 'both'",
                s
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub line_channel_access_token: String,
    pub line_channel_secret: String,
    pub ai_provider: AIProvider,
    pub claude_api_key: Option<String>,
    pub claude_model: String,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
    pub minimax_api_key: Option<String>,
    pub minimax_model: String,
    pub zai_api_key: Option<String>,
    pub zai_model: String,
    pub slack_bot_token: Option<String>,
    pub slack_app_token: Option<String>,
    pub slack_signing_secret: Option<String>,
    pub enable_line: bool,
    pub enable_slack: bool,
    pub slack_integration_mode: IntegrationMode,
    pub teams_app_id: Option<String>,
    pub teams_app_password: Option<String>,
    pub teams_tenant_id: Option<String>,
    pub enable_teams: bool,
    pub n8n_webhook_url: Option<String>,
    pub port: u16,
    pub schedules_config_path: String,
    pub log_level: String,
    pub log_dir: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    Claude,
    OpenAI,
    Gemini,
    Minimax,
    Zai,
}

impl FromStr for AIProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(AIProvider::Claude),
            "openai" => Ok(AIProvider::OpenAI),
            "gemini" => Ok(AIProvider::Gemini),
            "minimax" => Ok(AIProvider::Minimax),
            "zai" => Ok(AIProvider::Zai),
            _ => Err(format!(
                "Invalid AI provider: {}. Must be 'claude', 'openai', 'gemini', 'minimax', or 'zai'",
                s
            )),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, clap::Parser)]
#[command(name = "line_bot_summarize")]
#[command(about = "LINE Chat Summarizer Bot")]
pub struct CliArgs {
    #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(["claude", "openai", "gemini", "minimax", "zai"]))]
    pub provider: Option<String>,
}

impl CliArgs {
    pub fn parse() -> Self {
        clap::Parser::parse()
    }
}

#[allow(deprecated)]
fn load_env_file_override() -> Result<()> {
    if std::env::var("SKIP_DOTENV").is_ok() {
        return Ok(());
    }

    let iter = dotenv::from_filename_iter(".env")?;

    for item in iter {
        let (key, value) = item?;
        std::env::set_var(key, value);
    }

    Ok(())
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load local .env values on top of any already-exported variables.
        // This keeps the file as the source of truth for local development.
        load_env_file_override().ok();

        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL environment variable is required")?;

        let line_channel_access_token = std::env::var("LINE_CHANNEL_ACCESS_TOKEN")
            .context("LINE_CHANNEL_ACCESS_TOKEN environment variable is required")?;

        let line_channel_secret = std::env::var("LINE_CHANNEL_SECRET")
            .context("LINE_CHANNEL_SECRET environment variable is required")?;

        let ai_provider_str = std::env::var("AI_PROVIDER").unwrap_or_else(|_| "claude".to_string());
        let ai_provider = match ai_provider_str.to_lowercase().as_str() {
            "claude" => AIProvider::Claude,
            "openai" => AIProvider::OpenAI,
            "gemini" => AIProvider::Gemini,
            "minimax" => AIProvider::Minimax,
            "zai" => AIProvider::Zai,
            _ => anyhow::bail!(
                "Invalid AI_PROVIDER: {}. Must be 'claude', 'openai', 'gemini', 'minimax', or 'zai'",
                ai_provider_str
            ),
        };

        let claude_api_key = std::env::var("CLAUDE_API_KEY").ok();
        let openai_api_key = std::env::var("OPENAI_API_KEY").ok();
        let gemini_api_key = std::env::var("GEMINI_API_KEY").ok();
        let minimax_api_key = std::env::var("MINIMAX_API_KEY").ok();
        let zai_api_key = std::env::var("ZAI_API_KEY").ok();

        // Slack configuration (optional)
        let slack_bot_token = std::env::var("SLACK_BOT_TOKEN").ok();
        let slack_app_token = std::env::var("SLACK_APP_TOKEN").ok();
        let slack_signing_secret = std::env::var("SLACK_SIGNING_SECRET").ok();

        // Teams configuration (optional)
        let teams_app_id = std::env::var("TEAMS_APP_ID").ok();
        let teams_app_password = std::env::var("TEAMS_APP_PASSWORD").ok();
        let teams_tenant_id = std::env::var("TEAMS_TENANT_ID").ok();
        let n8n_webhook_url = std::env::var("N8N_WEBHOOK_URL").ok();

        // Feature flags
        let enable_line: bool = std::env::var("ENABLE_LINE")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let enable_slack: bool = std::env::var("ENABLE_SLACK")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let enable_teams: bool = std::env::var("ENABLE_TEAMS")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let slack_integration_mode_str =
            std::env::var("SLACK_INTEGRATION_MODE").unwrap_or_else(|_| "both".to_string());
        let slack_integration_mode = slack_integration_mode_str
            .parse::<IntegrationMode>()
            .map_err(|e| anyhow::anyhow!("Invalid SLACK_INTEGRATION_MODE: {}", e))?;

        if ai_provider == AIProvider::Claude && claude_api_key.is_none() {
            anyhow::bail!("CLAUDE_API_KEY environment variable is required when using Claude");
        }

        if ai_provider == AIProvider::OpenAI && openai_api_key.is_none() {
            anyhow::bail!("OPENAI_API_KEY environment variable is required when using OpenAI");
        }

        if ai_provider == AIProvider::Gemini && gemini_api_key.is_none() {
            anyhow::bail!("GEMINI_API_KEY environment variable is required when using Gemini");
        }

        if ai_provider == AIProvider::Minimax && minimax_api_key.is_none() {
            anyhow::bail!("MINIMAX_API_KEY environment variable is required when using Minimax");
        }

        if ai_provider == AIProvider::Zai && zai_api_key.is_none() {
            anyhow::bail!("ZAI_API_KEY environment variable is required when using Zai");
        }

        if enable_slack && slack_bot_token.is_none() {
            anyhow::bail!(
                "SLACK_BOT_TOKEN environment variable is required when enable_slack=true"
            );
        }

        if enable_slack && slack_signing_secret.is_none() {
            anyhow::bail!(
                "SLACK_SIGNING_SECRET environment variable is required when enable_slack=true"
            );
        }

        if enable_slack
            && (slack_integration_mode == IntegrationMode::EventsAPI
                || slack_integration_mode == IntegrationMode::Both)
            && slack_app_token.is_none()
        {
            anyhow::bail!(
                "SLACK_APP_TOKEN environment variable is required when using Events API mode"
            );
        }

        if enable_teams && teams_app_id.is_none() {
            anyhow::bail!("TEAMS_APP_ID environment variable is required when enable_teams=true");
        }

        if enable_teams && teams_app_password.is_none() {
            anyhow::bail!(
                "TEAMS_APP_PASSWORD environment variable is required when enable_teams=true"
            );
        }

        if enable_teams && teams_tenant_id.is_none() {
            anyhow::bail!(
                "TEAMS_TENANT_ID environment variable is required when enable_teams=true"
            );
        }

        let openai_model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
        let claude_model =
            std::env::var("CLAUDE_MODEL").unwrap_or_else(|_| "claude-sonnet-4-6".to_string());
        let gemini_model =
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash".to_string());
        let minimax_model =
            std::env::var("MINIMAX_MODEL").unwrap_or_else(|_| "abab6.5s-chat".to_string());
        let zai_model = std::env::var("ZAI_MODEL").unwrap_or_else(|_| "zai-7b".to_string());

        let port: u16 = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .context("PORT must be a valid number")?;

        let schedules_config_path = std::env::var("SCHEDULES_CONFIG_PATH")
            .unwrap_or_else(|_| "config/schedules.toml".to_string());

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string());

        Ok(Config {
            database_url,
            line_channel_access_token,
            line_channel_secret,
            ai_provider,
            claude_api_key,
            claude_model,
            openai_api_key,
            openai_model,
            gemini_api_key,
            gemini_model,
            minimax_api_key,
            minimax_model,
            zai_api_key,
            zai_model,
            slack_bot_token,
            slack_app_token,
            slack_signing_secret,
            enable_line,
            enable_slack,
            slack_integration_mode,
            teams_app_id,
            teams_app_password,
            teams_tenant_id,
            enable_teams,
            n8n_webhook_url,
            port,
            schedules_config_path,
            log_level,
            log_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn set_env(key: &str, value: &str) {
        std::env::set_var(key, value);
    }

    fn clear_env(key: &str) {
        std::env::remove_var(key);
    }

    fn set_min_required_env() {
        set_env("SKIP_DOTENV", "1");
        set_env("DATABASE_URL", "postgres://user:pass@localhost/db");
        set_env("LINE_CHANNEL_ACCESS_TOKEN", "line-token");
        set_env("LINE_CHANNEL_SECRET", "line-secret");
        set_env("AI_PROVIDER", "claude");
        set_env("CLAUDE_API_KEY", "claude-key");
        set_env("ENABLE_SLACK", "true");
        set_env("SLACK_BOT_TOKEN", "slack-bot");
        set_env("SLACK_SIGNING_SECRET", "slack-sign");
        set_env("SLACK_INTEGRATION_MODE", "webhook");
    }

    fn clear_min_required_env() {
        for key in [
            "SKIP_DOTENV",
            "DATABASE_URL",
            "LINE_CHANNEL_ACCESS_TOKEN",
            "LINE_CHANNEL_SECRET",
            "AI_PROVIDER",
            "CLAUDE_API_KEY",
            "ENABLE_SLACK",
            "SLACK_BOT_TOKEN",
            "SLACK_SIGNING_SECRET",
            "SLACK_INTEGRATION_MODE",
        ] {
            clear_env(key);
        }
    }

    #[test]
    fn integration_mode_from_str_variants() {
        assert_eq!(
            IntegrationMode::from_str("webhook").unwrap(),
            IntegrationMode::Webhook
        );
        assert_eq!(
            IntegrationMode::from_str("events_api").unwrap(),
            IntegrationMode::EventsAPI
        );
        assert_eq!(
            IntegrationMode::from_str("eventsapi").unwrap(),
            IntegrationMode::EventsAPI
        );
        assert_eq!(
            IntegrationMode::from_str("both").unwrap(),
            IntegrationMode::Both
        );
    }

    #[test]
    fn integration_mode_from_str_invalid() {
        let err = IntegrationMode::from_str("invalid").unwrap_err();
        assert!(err.contains("Invalid integration mode"));
    }

    #[test]
    fn ai_provider_from_str_variants() {
        assert_eq!(AIProvider::from_str("claude").unwrap(), AIProvider::Claude);
        assert_eq!(AIProvider::from_str("openai").unwrap(), AIProvider::OpenAI);
        assert_eq!(AIProvider::from_str("gemini").unwrap(), AIProvider::Gemini);
        assert_eq!(
            AIProvider::from_str("minimax").unwrap(),
            AIProvider::Minimax
        );
        assert_eq!(AIProvider::from_str("zai").unwrap(), AIProvider::Zai);
    }

    #[test]
    fn ai_provider_from_str_invalid() {
        let err = AIProvider::from_str("invalid").unwrap_err();
        assert!(err.contains("Invalid AI provider"));
    }

    #[test]
    fn config_from_env_succeeds_with_minimum_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        set_min_required_env();
        let config = Config::from_env().unwrap();
        clear_min_required_env();

        assert_eq!(config.ai_provider, AIProvider::Claude);
        assert_eq!(config.enable_slack, true);
        assert_eq!(config.slack_integration_mode, IntegrationMode::Webhook);
        assert_eq!(config.line_channel_access_token, "line-token");
    }

    #[test]
    fn config_from_env_requires_slack_bot_token_when_enabled() {
        let _lock = ENV_LOCK.lock().unwrap();
        set_min_required_env();
        clear_env("SLACK_BOT_TOKEN");
        let result = Config::from_env();
        clear_min_required_env();
        assert!(result.is_err());
    }

    #[test]
    fn config_from_env_disables_slack_by_default() {
        let _lock = ENV_LOCK.lock().unwrap();
        set_min_required_env();
        clear_env("ENABLE_SLACK");
        clear_env("SLACK_BOT_TOKEN");
        clear_env("SLACK_SIGNING_SECRET");
        clear_env("SLACK_INTEGRATION_MODE");

        let config = Config::from_env().unwrap();

        clear_min_required_env();
        assert!(!config.enable_slack);
    }
}
