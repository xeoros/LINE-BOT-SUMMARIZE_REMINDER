use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Teams Bot Activity Type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivityType {
    Message,
    ConversationUpdate,
    InstallationUpdate,
    Event,
    Invoke,
    #[serde(other)]
    Unknown,
}

impl fmt::Display for ActivityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActivityType::Message => write!(f, "message"),
            ActivityType::ConversationUpdate => write!(f, "conversationUpdate"),
            ActivityType::InstallationUpdate => write!(f, "installationUpdate"),
            ActivityType::Event => write!(f, "event"),
            ActivityType::Invoke => write!(f, "invoke"),
            ActivityType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Teams Bot Activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "type")]
    pub activity_type: ActivityType,
    pub id: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub channel_id: Option<String>,
    pub from: Option<ChannelAccount>,
    pub conversation: Option<ConversationAccount>,
    pub recipient: Option<ChannelAccount>,
    pub text: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
    pub entities: Option<Vec<Entity>>,
    pub channel_data: Option<serde_json::Value>,
    pub action: Option<String>,
    pub reply_to_id: Option<String>,
    pub value: Option<serde_json::Value>,
    pub name: Option<String>,
}

/// Channel Account
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAccount {
    pub id: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub aad_object_id: Option<String>,
}

/// Conversation Account
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationAccount {
    pub id: String,
    pub name: Option<String>,
    #[serde(rename = "isGroup")]
    pub is_group: Option<bool>,
    pub conversation_type: Option<String>,
    pub tenant_id: Option<String>,
}

/// Entity (for mentions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub mentioned: Option<ChannelAccount>,
    pub text: Option<String>,
}

/// Attachment (for Adaptive Cards)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub content: Option<serde_json::Value>,
    pub url: Option<String>,
}

/// OneSiam Incident Data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentData {
    // Reporter Info
    pub reporter_name: String,
    pub reporter_team: String,
    pub reporter_contact: String,

    // Affected User / Account
    pub user_name: String,
    pub user_account: String,

    // Feature / Module
    pub module: String,
    pub screen: Option<String>,

    // Steps to Reproduce
    pub steps: String,

    // Expected vs Actual
    pub expected: String,
    pub actual: String,
    pub error_message: Option<String>,

    // Scope & Impact
    pub environment: String,
    pub platform: Option<String>,
    pub network: Option<String>,
    pub severity: String,
    pub users_affected: Option<i32>,
    pub time_of_issue: Option<DateTime<Utc>>,

    // Extra Notes
    pub notes: Option<String>,

    // Teams Metadata
    pub teams_conversation_id: String,
    pub teams_message_id: Option<String>,
    pub submitted_by: String,
    pub submitted_at: DateTime<Utc>,
}

impl IncidentData {
    pub fn new(teams_conversation_id: String, submitted_by: String) -> Self {
        Self {
            reporter_name: String::new(),
            reporter_team: String::new(),
            reporter_contact: String::new(),
            user_name: String::new(),
            user_account: String::new(),
            module: String::new(),
            screen: None,
            steps: String::new(),
            expected: String::new(),
            actual: String::new(),
            error_message: None,
            environment: String::new(),
            platform: None,
            network: None,
            severity: String::new(),
            users_affected: None,
            time_of_issue: None,
            notes: None,
            teams_conversation_id,
            teams_message_id: None,
            submitted_by,
            submitted_at: Utc::now(),
        }
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.reporter_name.is_empty() {
            errors.push("Reporter name is required".to_string());
        }
        if self.reporter_team.is_empty() {
            errors.push("Reporter team is required".to_string());
        }
        if self.reporter_contact.is_empty() {
            errors.push("Reporter contact is required".to_string());
        }
        if self.user_name.is_empty() {
            errors.push("User name is required".to_string());
        }
        if self.user_account.is_empty() {
            errors.push("User account is required".to_string());
        }
        if self.module.is_empty() {
            errors.push("Module is required".to_string());
        }
        if self.steps.is_empty() {
            errors.push("Steps to reproduce are required".to_string());
        }
        if self.expected.is_empty() {
            errors.push("Expected behavior is required".to_string());
        }
        if self.actual.is_empty() {
            errors.push("Actual behavior is required".to_string());
        }
        if self.environment.is_empty() {
            errors.push("Environment is required".to_string());
        }
        if self.severity.is_empty() {
            errors.push("Severity is required".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Adaptive Card Action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveCardAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub title: String,
    pub data: Option<serde_json::Value>,
}

/// n8n Webhook Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N8nResponse {
    pub success: bool,
    pub jira_ticket_id: Option<String>,
    pub jira_ticket_url: Option<String>,
    pub message: Option<String>,
}

/// Send Message Request
#[derive(Debug, Clone, Serialize)]
pub struct SendMessageRequest {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(rename = "from")]
    pub from: ChannelAccount,
    pub conversation: ConversationAccount,
    pub recipient: ChannelAccount,
    pub text: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
}

/// Adaptive Card Content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveCard {
    #[serde(rename = "type")]
    pub card_type: String,
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub body: Vec<serde_json::Value>,
    pub actions: Option<Vec<AdaptiveCardAction>>,
}

/// Adaptive Card Input Text
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputText {
    #[serde(rename = "type")]
    pub input_type: String,
    pub id: String,
    pub placeholder: Option<String>,
    pub is_required: Option<bool>,
    pub value: Option<String>,
    pub max_length: Option<i32>,
    pub style: Option<String>,
}

/// Adaptive Card Input Multiline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputMultiline {
    #[serde(rename = "type")]
    pub input_type: String,
    pub id: String,
    pub placeholder: Option<String>,
    pub is_required: Option<bool>,
    pub value: Option<String>,
    pub max_length: Option<i32>,
    pub is_multiline: bool,
}

/// Adaptive Card Choice Input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputChoice {
    #[serde(rename = "type")]
    pub input_type: String,
    pub id: String,
    pub placeholder: Option<String>,
    pub is_required: Option<bool>,
    pub value: Option<String>,
    pub choices: Vec<Choice>,
}

/// Adaptive Card Choice
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub title: String,
    pub value: String,
}

/// Adaptive Card Date Input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDate {
    #[serde(rename = "type")]
    pub input_type: String,
    pub id: String,
    pub placeholder: Option<String>,
    pub is_required: Option<bool>,
    pub value: Option<String>,
}

/// Adaptive Card Number Input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputNumber {
    #[serde(rename = "type")]
    pub input_type: String,
    pub id: String,
    pub placeholder: Option<String>,
    pub is_required: Option<bool>,
    pub value: Option<i32>,
    pub min: Option<i32>,
}
