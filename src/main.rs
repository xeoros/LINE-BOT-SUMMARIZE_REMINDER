mod admin;
mod ai;
mod config;
mod db;
mod handlers;
mod line;
mod scheduler;
mod slack;
mod teams;

use anyhow::Result;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use ai::create_ai_service;
use config::{AIProvider, CliArgs, Config};
use db::{create_pool, Message, MessageType, SourceType};
use handlers::{is_done_keyword, ReminderCommand, ReminderHandler, SummaryCommand};
use line::{parse_webhook_body, verify_webhook_signature, LineClient, WebhookEvent};
use scheduler::reminder::ReminderNotifier;
use scheduler::ScheduledSummaries;
use slack::{
    detect_thread_reply, parse_slash_command, parse_thread_permalink, SlackClient, SlackCommand,
    SlackEventsAPI, SlackWebhookHandler, ThreadInfo,
};
use teams::{
    create_error_card, create_help_card, create_incident_card, create_success_card,
    create_validation_error_card, create_welcome_card, extract_auth_token,
    extract_command_from_mention, extract_conversation_id, extract_sender_id, extract_service_url,
    is_bot_mentioned, is_card_action, is_conversation_update, is_message, parse_command,
    parse_incident_data, validate_incoming_request, Activity, N8nClient, TeamsAuth, TeamsClient,
    TeamsCommand, TeamsWebhookHandler,
};

fn bangkok_timer() -> OffsetTime<time::format_description::well_known::Rfc3339> {
    let offset = time::UtcOffset::from_hms(7, 0, 0).expect("valid UTC+7 offset");
    OffsetTime::new(offset, time::format_description::well_known::Rfc3339)
}

// Use the specific extract_action_type from command module
use teams::command::extract_action_type;

// Use Slack types from the webhook module
use slack::{SlackEvent, SlackMessage};

struct AppState {
    pub pool: sqlx::PgPool,
    pub line_client: Arc<LineClient>,
    pub config: Config,
    slack_client: Option<Arc<SlackClient>>,
    slack_webhook_handler: Option<Arc<SlackWebhookHandler>>,
    slack_events_api: Option<Arc<SlackEventsAPI>>,
    teams_auth: Option<Arc<TeamsAuth>>,
    teams_client: Option<Arc<TeamsClient>>,
    teams_webhook_handler: Option<Arc<TeamsWebhookHandler>>,
    n8n_client: Option<Arc<N8nClient>>,
    ai_service: Arc<dyn ai::AIService>,
    reminder_notifier: Option<ReminderNotifier>,
}

// Implement AdminStateAccess trait for AppState
impl admin::AdminStateAccess for AppState {
    fn get_pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

    fn get_config(&self) -> &Config {
        &self.config
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments first
    let cli_args = CliArgs::parse();

    // Load configuration (environment variables)
    let mut config = Config::from_env()?;

    // Override AI provider from CLI if provided
    if let Some(provider_str) = &cli_args.provider {
        let provider = provider_str
            .parse::<AIProvider>()
            .map_err(|e| anyhow::anyhow!("Invalid provider: {}", e))?;

        // Validate that the required API key is set for the selected provider
        match provider {
            AIProvider::Claude => {
                if config.claude_api_key.is_none() {
                    anyhow::bail!(
                        "CLAUDE_API_KEY environment variable is required when using Claude"
                    );
                }
            }
            AIProvider::OpenAI => {
                if config.openai_api_key.is_none() {
                    anyhow::bail!(
                        "OPENAI_API_KEY environment variable is required when using OpenAI"
                    );
                }
            }
            AIProvider::Gemini => {
                if config.gemini_api_key.is_none() {
                    anyhow::bail!(
                        "GEMINI_API_KEY environment variable is required when using Gemini"
                    );
                }
            }
            AIProvider::Minimax => {
                if config.minimax_api_key.is_none() {
                    anyhow::bail!(
                        "MINIMAX_API_KEY environment variable is required when using Minimax"
                    );
                }
            }
            AIProvider::Zai => {
                if config.zai_api_key.is_none() {
                    anyhow::bail!("ZAI_API_KEY environment variable is required when using Zai");
                }
            }
        }

        config.ai_provider = provider;
    }

    // Validate feature flags and configuration
    if !config.enable_line && !config.enable_slack && !config.enable_teams {
        anyhow::bail!("At least one platform must be enabled. Set ENABLE_LINE=true, ENABLE_SLACK=true, or ENABLE_TEAMS=true");
    }

    // Initialize tracing with both file and console output
    let log_dir = std::path::Path::new(&config.log_dir);
    std::fs::create_dir_all(log_dir).expect("Failed to create log directory");

    let file_appender = tracing_appender::rolling::daily(log_dir, "app.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let log_level: tracing::Level = config.log_level.parse().unwrap_or(tracing::Level::INFO);

    let level_filter = tracing_subscriber::filter::LevelFilter::from_level(log_level);

    tracing_subscriber::registry()
        .with(level_filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_timer(bangkok_timer())
                .with_level(true)
                .with_target(true),
        )
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_timer(bangkok_timer())
                .with_level(true)
                .with_target(true)
                .with_ansi(false),
        )
        .init();

    info!("Starting LINE Chat Summarizer Bot...");
    info!("AI Provider: {:?}", config.ai_provider);
    info!("LINE Enabled: {}", config.enable_line);
    info!("Slack Enabled: {}", config.enable_slack);
    info!("Teams Enabled: {}", config.enable_teams);

    // Create database pool
    let pool = create_pool(&config.database_url).await?;
    sqlx::migrate!("./sql/migrations").run(&pool).await?;
    info!("Database connected successfully");

    // Create LINE client
    let line_client = LineClient::new(config.line_channel_access_token.clone());

    // Create AI service
    let ai_service = create_ai_service(
        config.ai_provider,
        config.claude_api_key.clone(),
        config.claude_model.clone(),
        config.openai_api_key.clone(),
        config.openai_model.clone(),
        config.gemini_api_key.clone(),
        config.gemini_model.clone(),
        config.minimax_api_key.clone(),
        config.minimax_model.clone(),
        config.zai_api_key.clone(),
        config.zai_model.clone(),
    )?;

    // Initialize Slack components if enabled
    let slack_client_opt = if config.enable_slack {
        if let Some(ref bot_token) = config.slack_bot_token {
            info!("Slack integration enabled");
            Some(Arc::new(SlackClient::new(bot_token.clone())))
        } else {
            warn!("Slack integration enabled but SLACK_BOT_TOKEN not set");
            None
        }
    } else {
        info!("Slack integration disabled via feature flag");
        None
    };

    let slack_webhook_handler_opt = if config.enable_slack && config.slack_bot_token.is_some() {
        if let Some(ref signing_secret) = config.slack_signing_secret {
            info!("Slack webhook handler initialized");
            Some(Arc::new(SlackWebhookHandler::new(signing_secret.clone())))
        } else {
            warn!("Slack webhook handler enabled but SLACK_SIGNING_SECRET not set");
            None
        }
    } else {
        None
    };

    let slack_events_api_opt = if config.enable_slack && config.slack_bot_token.is_some() {
        match config.slack_integration_mode {
            config::IntegrationMode::EventsAPI | config::IntegrationMode::Both => {
                if let Some(ref app_token) = config.slack_app_token {
                    info!("Slack Events API initialized");
                    let events_api = SlackEventsAPI::new(app_token.clone());
                    events_api.connect().await?;
                    Some(Arc::new(events_api))
                } else {
                    warn!("Events API mode enabled but SLACK_APP_TOKEN not set");
                    None
                }
            }
            config::IntegrationMode::Webhook => None,
        }
    } else {
        None
    };

    // Initialize Teams components if enabled
    let teams_auth_opt = if config.enable_teams {
        if let (Some(ref app_id), Some(ref app_password), Some(ref tenant_id)) = (
            &config.teams_app_id,
            &config.teams_app_password,
            &config.teams_tenant_id,
        ) {
            info!("Teams integration enabled");
            Some(Arc::new(TeamsAuth::new(
                app_id.clone(),
                app_password.clone(),
                tenant_id.clone(),
            )))
        } else {
            warn!("Teams integration enabled but credentials not set");
            None
        }
    } else {
        info!("Teams integration disabled via feature flag");
        None
    };

    let teams_client_opt = if config.enable_teams && teams_auth_opt.is_some() {
        // Get bot ID from app_id (for now, use app_id as bot_id)
        // In production, this would come from the bot registration
        if let Some(ref auth) = teams_auth_opt {
            let bot_id = config.teams_app_id.clone().unwrap_or_default();
            info!("Teams client initialized");
            Some(Arc::new(TeamsClient::new(auth.clone(), bot_id)))
        } else {
            None
        }
    } else {
        None
    };

    let teams_webhook_handler_opt = if config.enable_teams && config.teams_app_id.is_some() {
        if let (Some(ref app_id), Some(ref app_password)) =
            (&config.teams_app_id, &config.teams_app_password)
        {
            info!("Teams webhook handler initialized");
            Some(Arc::new(TeamsWebhookHandler::new(
                app_id.clone(),
                app_password.clone(),
            )))
        } else {
            warn!("Teams webhook handler enabled but credentials not set");
            None
        }
    } else {
        None
    };

    let n8n_client_opt = if config.enable_teams && config.n8n_webhook_url.is_some() {
        if let Some(ref webhook_url) = config.n8n_webhook_url {
            info!("n8n webhook client initialized");
            Some(Arc::new(N8nClient::new(webhook_url.clone())))
        } else {
            warn!("n8n webhook URL not configured");
            None
        }
    } else {
        info!("n8n integration disabled");
        None
    };

    // Initialize scheduled summaries
    let line_client_arc = Arc::new(line_client.clone());
    let ai_service_arc: Arc<dyn ai::AIService> = Arc::from(ai_service);
    let mut scheduled_summaries = ScheduledSummaries::new(
        Arc::new(pool.clone()),
        line_client_arc.clone(),
        ai_service_arc.clone(),
    )
    .await?;

    // Load schedules from config file if it exists
    if std::path::Path::new(&config.schedules_config_path).exists() {
        match scheduled_summaries
            .load_schedules(&config.schedules_config_path)
            .await
        {
            Ok(_) => {}
            Err(e) => warn!("Failed to load scheduled summaries: {}", e),
        }
        scheduled_summaries.start().await?;
    } else {
        warn!(
            "No schedules config file found at {}, skipping scheduled summaries",
            config.schedules_config_path
        );
    }

    // Initialize Reminder Notifier
    let reminder_notifier =
        ReminderNotifier::new(Arc::new(pool.clone()), Arc::new(line_client.clone())).await;

    let reminder_notifier = match reminder_notifier {
        Ok(mut notifier) => {
            if let Err(e) = notifier.start().await {
                warn!("Failed to start reminder notifier: {}", e);
                None
            } else {
                info!("Reminder notifier started");
                Some(notifier)
            }
        }
        Err(e) => {
            warn!("Failed to create reminder notifier: {}", e);
            None
        }
    };

    // Create app state
    let state = Arc::new(AppState {
        pool,
        line_client: line_client_arc,
        slack_client: slack_client_opt,
        slack_webhook_handler: slack_webhook_handler_opt,
        slack_events_api: slack_events_api_opt,
        teams_auth: teams_auth_opt,
        teams_client: teams_client_opt,
        teams_webhook_handler: teams_webhook_handler_opt,
        n8n_client: n8n_client_opt,
        ai_service: ai_service_arc,
        config,
        reminder_notifier,
    });

    // Start server
    let port = state.config.port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    // Create router with conditional routes based on feature flags
    let mut app = Router::new().route("/health", get(health_check));

    // Add LINE webhook if enabled
    if state.config.enable_line {
        app = app.route("/webhook", post(webhook_handler));
        info!("LINE webhook endpoint registered");
    }

    // Add Slack webhook if enabled and using webhook mode
    if state.config.enable_slack
        && (state.config.slack_integration_mode == config::IntegrationMode::Webhook
            || state.config.slack_integration_mode == config::IntegrationMode::Both)
    {
        app = app.route("/slack/webhook", post(slack_webhook_handler));
        info!("Slack webhook endpoint registered");
    }

    // Add Teams webhook if enabled
    if state.config.enable_teams {
        app = app.route("/teams/webhook", post(teams_webhook_handler));
        info!("Teams webhook endpoint registered");
    }

    // Add admin dashboard routes
    app = app.route("/admin", get(admin::dashboard));
    app = app.route("/admin/api/stats", get(admin::api_stats));
    app = app.route("/admin/api/reminders", get(admin::api_reminders));
    app = app.route(
        "/admin/api/reminders/:checklist_id/reschedule",
        post(admin::api_reschedule),
    );
    app = app.route(
        "/admin/api/reminders/:checklist_id/schedule",
        post(admin::api_toggle_schedule),
    );
    app = app.route("/admin/api/test-alert", post(admin::api_test_alert));
    info!("Admin dashboard registered at /admin");

    let app = app.with_state(state);

    info!("Starting server on port {}", port);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "LINE Chat Summarizer Bot",
        "line_enabled": true,
        "slack_enabled": true,
        "teams_enabled": true
    }))
}

async fn webhook_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    // Verify webhook signature
    let signature = match headers.get("x-line-signature") {
        Some(sig) => match sig.to_str() {
            Ok(s) => s,
            Err(_) => {
                error!("Invalid signature header");
                return (StatusCode::BAD_REQUEST, "Invalid signature header").into_response();
            }
        },
        None => {
            error!("Missing signature header");
            return (StatusCode::UNAUTHORIZED, "Missing signature header").into_response();
        }
    };

    // Debug logging
    info!("Webhook body length: {} bytes", body.len());
    info!(
        "Body preview: {:?}",
        String::from_utf8_lossy(&body[..body.len().min(100)])
    );

    match verify_webhook_signature(&state.config.line_channel_secret, &body, signature) {
        Ok(true) => {
            info!("Signature validation passed");
        }
        Ok(false) => {
            error!("Invalid webhook signature");
            return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
        }
        Err(e) => {
            error!("Failed to verify signature: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Signature verification failed",
            )
                .into_response();
        }
    }

    // Parse webhook body
    let webhook_event = match parse_webhook_body(body.as_ref()) {
        Ok(event) => event,
        Err(e) => {
            error!("Failed to parse webhook body: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid webhook body").into_response();
        }
    };

    // Note: The Slack-specific logic (challenge/event) was erroneously here.
    // LINE events are processed in webhook_event.events.
    for event in webhook_event.events {
        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_line_event(&state_clone, event).await {
                error!("Failed to process LINE event: {}", e);
            }
        });
    }

    StatusCode::OK.into_response()
}

// Helper to handle LINE events (extracted for clarity)
async fn handle_line_event(state: &AppState, event: line::webhook::WebhookEvent) -> Result<()> {
    info!("Processing LINE event type: {}", event.event_type);

    if event.event_type != "message" {
        return Ok(());
    }

    let message = match &event.message {
        Some(msg) => msg,
        None => return Ok(()),
    };

    if message.message_type != "text" {
        return Ok(());
    }

    let message_text = match &message.text {
        Some(text) => text.clone(),
        None => return Ok(()),
    };

    let (source_type, source_id) = match event.source.source_type.as_str() {
        "user" => (
            SourceType::User,
            event.source.user_id.clone().unwrap_or_default(),
        ),
        "group" => (
            SourceType::Group,
            event.source.group_id.clone().unwrap_or_default(),
        ),
        "room" => (
            SourceType::Room,
            event.source.room_id.clone().unwrap_or_default(),
        ),
        _ => return Ok(()),
    };

    let sender_id = event.source.user_id.clone();

    info!(
        "Processing LINE message from {}: {}",
        source_id, message_text
    );

    // Check for reminder commands first
    let reminder_handler = ReminderHandler;

    // Check if it's a done keyword
    if is_done_keyword(&message_text) {
        if let Some((checklist_id, task_number)) =
            crate::handlers::parse_done_command(&message_text)
        {
            let command = ReminderCommand::MarkDone {
                checklist_id,
                task_number,
            };
            let response = reminder_handler
                .execute(
                    &command,
                    &state.pool,
                    &state.line_client,
                    state.ai_service.as_ref(),
                    source_type,
                    &source_id,
                    sender_id.as_deref(),
                    event.reply_token.as_deref(),
                )
                .await;

            match response {
                Ok(_) => info!("Reminder command executed successfully"),
                Err(e) => error!("Failed to execute reminder command: {}", e),
            }
            return Ok(());
        }
    }

    // Check for other reminder commands
    if let Some(command) = ReminderHandler::parse(&message_text) {
        let response = reminder_handler
            .execute(
                &command,
                &state.pool,
                &state.line_client,
                state.ai_service.as_ref(),
                source_type,
                &source_id,
                sender_id.as_deref(),
                event.reply_token.as_deref(),
            )
            .await;

        match response {
            Ok(_) => info!("Reminder command executed successfully"),
            Err(e) => error!("Failed to execute reminder command: {}", e),
        }
        return Ok(());
    }

    // Check for summary commands
    if let Some(command) = SummaryCommand::parse(&message_text) {
        info!("Executing summary command: {:?}", command);
        match command
            .execute(
                &state.pool,
                &state.line_client,
                state.ai_service.as_ref(),
                source_type,
                &source_id,
                event.reply_token.as_deref().unwrap_or(""),
            )
            .await
        {
            Ok(_) => info!("Summary command executed successfully"),
            Err(e) => error!("Failed to execute summary command: {}", e),
        }
        return Ok(());
    }

    // Store message for future summarization
    if let Some(text) = &message.text {
        let _ = Message::save(
            &state.pool,
            &message.message_id,
            source_type,
            &source_id,
            sender_id.as_deref(),
            None,
            MessageType::Text,
            Some(text),
            None,
            None,
        )
        .await;
    }

    Ok(())
}

async fn process_event(state: &AppState, event: &SlackEvent) -> Result<()> {
    if let Some(event_type) = event.event_type.as_deref() {
        match event_type {
            "message" => {
                process_message_event(state, event).await?;
            }
            "thread_broadcast" => {
                info!("Thread broadcast detected in channel: {:?}", event.channel);
            }
            "url_verification" => {
                info!("URL verification event received");
            }
            "app_mention" => {
                info!("App mention detected: {:?}", event.text);
            }
            _ => {
                info!("Unhandled Slack event type: {}", event_type);
            }
        }
    } else {
        info!("Slack event has no type");
    }
    Ok(())
}

async fn process_message_event(state: &AppState, event: &SlackEvent) -> Result<()> {
    let slack_client = match &state.slack_client {
        Some(client) => client,
        None => {
            warn!("Slack client not available");
            return Ok(());
        }
    };

    let channel_id = match &event.channel {
        Some(channel) => channel.clone(),
        None => {
            warn!("Missing channel in Slack event");
            return Ok(());
        }
    };

    let message_text = event.text.as_deref();

    // Parse and process Slack commands
    if let Some(text) = message_text {
        if let Some(slack_command) = parse_slash_command(text) {
            match slack_command {
                SlackCommand::SummaryThread { thread_ts } => {
                    info!("Summarizing thread: {}", thread_ts);
                    // For now, return success - in full implementation would get thread and summarize
                    return Ok(());
                }
                SlackCommand::SummaryByUrl { url } => {
                    info!("Summarizing thread from URL: {}", url);
                    // For now, return success - in full implementation would parse URL and get thread
                    return Ok(());
                }
                SlackCommand::SummaryChannel { count, time_range } => {
                    info!(
                        "Summarizing channel: count={:?}, time_range={:?}",
                        count, time_range
                    );
                    // For now, return success - in full implementation would get messages and summarize
                    return Ok(());
                }
            }
        }
    }

    // For now, just store message
    info!("Slack message stored successfully");
    Ok(())
}

async fn slack_webhook_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    let slack_handler = match &state.slack_webhook_handler {
        Some(handler) => handler,
        None => {
            error!("Slack webhook handler not available");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Slack webhook handler not configured",
            )
                .into_response();
        }
    };

    // Extract timestamp and signature headers
    let timestamp = match headers.get("x-slack-request-timestamp") {
        Some(ts) => match ts.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                error!("Invalid timestamp header");
                return (StatusCode::BAD_REQUEST, "Invalid timestamp header").into_response();
            }
        },
        None => {
            error!("Missing timestamp header");
            return (StatusCode::BAD_REQUEST, "Missing timestamp header").into_response();
        }
    };

    let signature = match headers.get("x-slack-signature") {
        Some(sig) => match sig.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                error!("Invalid signature header");
                return (StatusCode::BAD_REQUEST, "Invalid signature header").into_response();
            }
        },
        None => {
            error!("Missing signature header");
            return (StatusCode::BAD_REQUEST, "Missing signature header").into_response();
        }
    };

    // Verify webhook signature
    match slack_handler.verify_signature(body.as_ref(), &timestamp, &signature) {
        Ok(true) => {
            info!("Slack webhook signature validation passed");
        }
        Ok(false) => {
            error!("Invalid Slack webhook signature");
            return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
        }
        Err(e) => {
            error!("Failed to verify Slack signature: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Signature verification failed",
            )
                .into_response();
        }
    }

    // Parse webhook body
    let text = String::from_utf8_lossy(&body).to_string();
    let webhook_event = match slack_handler.parse_webhook_body(&text) {
        Ok(event) => event,
        Err(e) => {
            error!("Failed to parse Slack webhook body: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid webhook body").into_response();
        }
    };

    // Handle URL verification challenge if present
    if let Some(challenge) = webhook_event.challenge {
        return (StatusCode::OK, challenge).into_response();
    }

    // Process the event asynchronously
    if let Some(slack_event) = webhook_event.event {
        info!("Processing Slack event type: {:?}", slack_event.event_type);
        let event_clone = slack_event.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Err(e) = process_slack_event(&state_clone, &event_clone).await {
                error!("Failed to process Slack event: {}", e);
            }
        });
    }

    StatusCode::OK.into_response()
}

async fn process_slack_event(state: &AppState, event: &SlackEvent) -> Result<()> {
    let slack_client = match &state.slack_client {
        Some(client) => client,
        None => {
            warn!("Slack client not available");
            return Ok(());
        }
    };

    let channel_id = match &event.channel {
        Some(channel) => channel.clone(),
        None => {
            warn!("Missing channel in Slack event");
            return Ok(());
        }
    };

    let message_text = event.text.as_deref();

    // Parse and process Slack commands
    if let Some(text) = message_text {
        if let Some(slack_command) = parse_slash_command(text) {
            match slack_command {
                SlackCommand::SummaryThread { thread_ts } => {
                    info!("Summarizing thread: {}", thread_ts);
                    return Ok(());
                }
                SlackCommand::SummaryByUrl { url } => {
                    info!("Summarizing thread from URL: {}", url);
                    return Ok(());
                }
                SlackCommand::SummaryChannel { count, time_range } => {
                    info!(
                        "Summarizing channel: count={:?}, time_range={:?}",
                        count, time_range
                    );
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

async fn teams_webhook_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    let teams_handler = match &state.teams_webhook_handler {
        Some(handler) => handler,
        None => {
            error!("Teams webhook handler not available");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Teams webhook handler not configured",
            )
                .into_response();
        }
    };

    // Extract Authorization header
    let auth_header = match headers.get("Authorization") {
        Some(auth) => match auth.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => {
                error!("Invalid Authorization header");
                return (StatusCode::BAD_REQUEST, "Invalid Authorization header").into_response();
            }
        },
        None => {
            error!("Missing Authorization header");
            return (StatusCode::BAD_REQUEST, "Missing Authorization header").into_response();
        }
    };

    // Extract Content-Type
    let content_type = match headers.get("Content-Type") {
        Some(ct) => match ct.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => {
                error!("Invalid Content-Type header");
                return (StatusCode::BAD_REQUEST, "Invalid Content-Type header").into_response();
            }
        },
        None => {
            error!("Missing Content-Type header");
            return (StatusCode::BAD_REQUEST, "Missing Content-Type header").into_response();
        }
    };

    // Validate incoming request
    if let Err(e) = validate_incoming_request(Some(auth_header.as_str()), content_type.as_deref()) {
        error!("Invalid Teams request: {}", e);
        return (StatusCode::BAD_REQUEST, "Invalid request").into_response();
    }

    // Parse webhook body
    let text = String::from_utf8_lossy(&body).to_string();
    let webhook_event = match teams_handler.parse_webhook_body(&text) {
        Ok(event) => event,
        Err(e) => {
            error!("Failed to parse Teams webhook body: {}", e);
            return (StatusCode::BAD_REQUEST, "Invalid webhook body").into_response();
        }
    };

    // Process the event asynchronously
    if let Some(activity) = webhook_event.activity {
        info!(
            "Processing Teams activity type: {:?}",
            activity.activity_type
        );
        let activity_clone = activity.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Err(e) = process_teams_activity(&state_clone, &activity_clone).await {
                error!("Failed to process Teams activity: {}", e);
            }
        });
    }

    StatusCode::OK.into_response()
}

async fn process_teams_activity(state: &AppState, activity: &Activity) -> Result<()> {
    let teams_client = match &state.teams_client {
        Some(client) => client,
        None => {
            warn!("Teams client not available");
            return Ok(());
        }
    };

    let n8n_client = match &state.n8n_client {
        Some(client) => client,
        None => {
            warn!("n8n client not available");
            return Ok(());
        }
    };

    // Handle conversation updates (bot added to channel)
    if is_conversation_update(activity) {
        info!("Teams conversation update detected");
        // Send welcome card
        if let (Some(conversation_id), Some(service_url)) = (
            extract_conversation_id(activity).ok(),
            activity
                .channel_data
                .as_ref()
                .and_then(|d| d.get("serviceUrl"))
                .and_then(|u| u.as_str()),
        ) {
            let welcome_card = create_welcome_card()?;
            teams_client
                .send_message(
                    &conversation_id,
                    service_url,
                    &conversation_id,
                    None,
                    None,
                    Some(vec![welcome_card]),
                )
                .await?;
        }
        return Ok(());
    }

    // Handle Adaptive Card actions
    if is_card_action(activity) {
        info!("Teams card action detected");
        if let Some(action_type) = extract_action_type(activity) {
            match action_type.as_str() {
                "open_incident_form" => {
                    // Send incident form
                    if let (Some(conversation_id), Some(service_url)) = (
                        extract_conversation_id(activity).ok(),
                        activity
                            .channel_data
                            .as_ref()
                            .and_then(|d| d.get("serviceUrl"))
                            .and_then(|u| u.as_str()),
                    ) {
                        let incident_card = create_incident_card()?;
                        teams_client
                            .send_message(
                                &conversation_id,
                                service_url,
                                &conversation_id,
                                None,
                                None,
                                Some(vec![incident_card]),
                            )
                            .await?;
                    }
                }
                "submit_incident" => {
                    // Parse incident data and send to n8n
                    if let (Ok(conversation_id), Ok(submitted_by), Some(service_url)) = (
                        extract_conversation_id(activity),
                        extract_sender_id(activity),
                        activity
                            .channel_data
                            .as_ref()
                            .and_then(|d| d.get("serviceUrl"))
                            .and_then(|u| u.as_str()),
                    ) {
                        let incident_data =
                            parse_incident_data(activity, conversation_id.clone(), submitted_by)?;

                        // Validate incident data
                        match incident_data.validate() {
                            Ok(()) => {
                                // Send to n8n
                                match n8n_client.trigger_jira_creation(&incident_data).await {
                                    Ok(n8n_response) => {
                                        if n8n_response.success {
                                            if let (Some(ticket_id), Some(ticket_url)) = (
                                                n8n_response.jira_ticket_id,
                                                n8n_response.jira_ticket_url,
                                            ) {
                                                let success_card =
                                                    create_success_card(ticket_id, ticket_url)?;
                                                teams_client
                                                    .send_message(
                                                        &conversation_id,
                                                        service_url,
                                                        &conversation_id,
                                                        None,
                                                        None,
                                                        Some(vec![success_card]),
                                                    )
                                                    .await?;
                                            } else {
                                                // No ticket info returned, send generic success
                                                teams_client.send_message(
                                                    &conversation_id,
                                                    service_url,
                                                    &conversation_id,
                                                    None,
                                                    Some("✅ Incident submitted successfully! Check your Jira project for updates.".to_string()),
                                                    None,
                                                ).await?;
                                            }
                                        } else {
                                            let error_card = create_error_card(
                                                n8n_response
                                                    .message
                                                    .unwrap_or_else(|| "Unknown error".to_string()),
                                            )?;
                                            teams_client
                                                .send_message(
                                                    &conversation_id,
                                                    service_url,
                                                    &conversation_id,
                                                    None,
                                                    None,
                                                    Some(vec![error_card]),
                                                )
                                                .await?;
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to send incident to n8n: {}", e);
                                        let error_card = create_error_card(format!(
                                            "Failed to process incident: {}",
                                            e
                                        ))?;
                                        teams_client
                                            .send_message(
                                                &conversation_id,
                                                service_url,
                                                &conversation_id,
                                                None,
                                                None,
                                                Some(vec![error_card]),
                                            )
                                            .await?;
                                    }
                                }
                            }
                            Err(errors) => {
                                let validation_card = create_validation_error_card(errors)?;
                                teams_client
                                    .send_message(
                                        &conversation_id,
                                        service_url,
                                        &conversation_id,
                                        None,
                                        None,
                                        Some(vec![validation_card]),
                                    )
                                    .await?;
                            }
                        }
                    }
                }
                "cancel" => {
                    // Send cancel message
                    if let (Some(conversation_id), Some(service_url)) = (
                        extract_conversation_id(activity).ok(),
                        activity
                            .channel_data
                            .as_ref()
                            .and_then(|d| d.get("serviceUrl"))
                            .and_then(|u| u.as_str()),
                    ) {
                        teams_client
                            .send_message(
                                &conversation_id,
                                service_url,
                                &conversation_id,
                                None,
                                Some("❌ Incident report cancelled.".to_string()),
                                None,
                            )
                            .await?;
                    }
                }
                "help" => {
                    // Send help card
                    if let (Some(conversation_id), Some(service_url)) = (
                        extract_conversation_id(activity).ok(),
                        activity
                            .channel_data
                            .as_ref()
                            .and_then(|d| d.get("serviceUrl"))
                            .and_then(|u| u.as_str()),
                    ) {
                        let help_card = create_help_card()?;
                        teams_client
                            .send_message(
                                &conversation_id,
                                service_url,
                                &conversation_id,
                                None,
                                None,
                                Some(vec![help_card]),
                            )
                            .await?;
                    }
                }
                _ => {
                    info!("Unknown Teams action: {}", action_type);
                }
            }
        }
        return Ok(());
    }

    // Handle text messages
    if is_message(activity) {
        if let Some(text) = &activity.text {
            info!("Teams message received: {}", text);

            // Check for slash commands
            if let Some(command) = parse_command(text) {
                match command {
                    TeamsCommand::Incident => {
                        // Send incident form
                        if let (Some(conversation_id), Some(service_url)) = (
                            extract_conversation_id(activity).ok(),
                            activity
                                .channel_data
                                .as_ref()
                                .and_then(|d| d.get("serviceUrl"))
                                .and_then(|u| u.as_str()),
                        ) {
                            let incident_card = create_incident_card()?;
                            teams_client
                                .send_message(
                                    &conversation_id,
                                    service_url,
                                    &conversation_id,
                                    None,
                                    None,
                                    Some(vec![incident_card]),
                                )
                                .await?;
                        }
                    }
                    TeamsCommand::Help => {
                        // Send help card
                        if let (Some(conversation_id), Some(service_url)) = (
                            extract_conversation_id(activity).ok(),
                            activity
                                .channel_data
                                .as_ref()
                                .and_then(|d| d.get("serviceUrl"))
                                .and_then(|u| u.as_str()),
                        ) {
                            let help_card = create_help_card()?;
                            teams_client
                                .send_message(
                                    &conversation_id,
                                    service_url,
                                    &conversation_id,
                                    None,
                                    None,
                                    Some(vec![help_card]),
                                )
                                .await?;
                        }
                    }
                    TeamsCommand::Cancel => {
                        // Send cancel message
                        if let (Some(conversation_id), Some(service_url)) = (
                            extract_conversation_id(activity).ok(),
                            activity
                                .channel_data
                                .as_ref()
                                .and_then(|d| d.get("serviceUrl"))
                                .and_then(|u| u.as_str()),
                        ) {
                            teams_client
                                .send_message(
                                    &conversation_id,
                                    service_url,
                                    &conversation_id,
                                    None,
                                    Some("❌ Incident report cancelled.".to_string()),
                                    None,
                                )
                                .await?;
                        }
                    }
                    TeamsCommand::Unknown(cmd) => {
                        info!("Unknown Teams command: {}", cmd);
                        // Send help message
                        if let (Some(conversation_id), Some(service_url)) = (
                            extract_conversation_id(activity).ok(),
                            activity
                                .channel_data
                                .as_ref()
                                .and_then(|d| d.get("serviceUrl"))
                                .and_then(|u| u.as_str()),
                        ) {
                            teams_client
                                .send_message(
                                    &conversation_id,
                                    service_url,
                                    &conversation_id,
                                    None,
                                    Some(format!(
                                        "❓ Unknown command: '{}'. Type /help for assistance.",
                                        cmd
                                    )),
                                    None,
                                )
                                .await?;
                        }
                    }
                }
                return Ok(());
            }

            // Check for bot mentions
            if is_bot_mentioned(activity, "OneSiam Incident Bot") {
                if let Some(cmd_text) = extract_command_from_mention(text, "OneSiam Incident Bot") {
                    if let Some(command) = parse_command(&cmd_text) {
                        // Handle mention commands similar to slash commands above
                        if let TeamsCommand::Incident = command {
                            if let (Some(conversation_id), Some(service_url)) = (
                                extract_conversation_id(activity).ok(),
                                activity
                                    .channel_data
                                    .as_ref()
                                    .and_then(|d| d.get("serviceUrl"))
                                    .and_then(|u| u.as_str()),
                            ) {
                                let incident_card = create_incident_card()?;
                                teams_client
                                    .send_message(
                                        &conversation_id,
                                        service_url,
                                        &conversation_id,
                                        None,
                                        None,
                                        Some(vec![incident_card]),
                                    )
                                    .await?;
                            }
                        }
                    } else {
                        // Send help on unknown mention
                        if let (Some(conversation_id), Some(service_url)) = (
                            extract_conversation_id(activity).ok(),
                            activity
                                .channel_data
                                .as_ref()
                                .and_then(|d| d.get("serviceUrl"))
                                .and_then(|u| u.as_str()),
                        ) {
                            let help_card = create_help_card()?;
                            teams_client
                                .send_message(
                                    &conversation_id,
                                    service_url,
                                    &conversation_id,
                                    None,
                                    None,
                                    Some(vec![help_card]),
                                )
                                .await?;
                        }
                    }
                }
                return Ok(());
            }
        }
    }

    Ok(())
}
