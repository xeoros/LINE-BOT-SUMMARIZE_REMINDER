// Teams Bot Integration Module
// Provides Microsoft Teams bot functionality with Adaptive Cards and n8n integration

pub mod auth;
pub mod cards;
pub mod client;
pub mod command;
pub mod models;
pub mod n8n;
pub mod webhook;

// Re-export commonly used types
pub use auth::TeamsAuth;
pub use cards::{
    create_error_card, create_help_card, create_incident_card, create_success_card,
    create_validation_error_card, create_welcome_card,
    extract_action_type as cards_extract_action_type, extract_form_data,
};
pub use client::{extract_conversation_id, extract_sender_id, extract_service_url, TeamsClient};
pub use command::{
    clean_text, extract_action_type, extract_command_from_mention, is_bot_mentioned,
    is_card_action, is_conversation_update, is_message, parse_command, parse_incident_data,
    TeamsCommand,
};
pub use models::{
    Activity, ActivityType, AdaptiveCardAction, Attachment, ChannelAccount, ConversationAccount,
    Entity, IncidentData, N8nResponse,
};
pub use n8n::N8nClient;
pub use webhook::{extract_auth_token, validate_incoming_request, TeamsWebhookHandler};
