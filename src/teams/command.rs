use anyhow::Result;
use regex::Regex;
use tracing::debug;

use super::models::{Activity, ActivityType, IncidentData};

/// Teams Command
#[derive(Debug, Clone, PartialEq)]
pub enum TeamsCommand {
    Incident,
    Help,
    Cancel,
    Unknown(String),
}

/// Parse a Teams command from text
pub fn parse_command(text: &str) -> Option<TeamsCommand> {
    let text = text.trim();

    // Match slash commands
    let slash_cmd_regex = Regex::new(r"^/(\w+)(?:\s+(.*))?$").ok()?;
    if let Some(captures) = slash_cmd_regex.captures(text) {
        if let Some(cmd) = captures.get(1) {
            let args = captures
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let command_str = cmd.as_str().to_lowercase();
            return Some(match command_str.as_str() {
                "incident" | "report" | "bug" => TeamsCommand::Incident,
                "help" | "?" => TeamsCommand::Help,
                "cancel" => TeamsCommand::Cancel,
                _ => {
                    let cmd_str = cmd.as_str();
                    if args.trim().is_empty() {
                        TeamsCommand::Unknown(format!("/{}", cmd_str))
                    } else {
                        TeamsCommand::Unknown(format!("/{cmd_str} {args}"))
                    }
                }
            });
        }
    }

    None
}

/// Check if the bot is mentioned in the message
pub fn is_bot_mentioned(activity: &Activity, bot_name: &str) -> bool {
    if let Some(text) = &activity.text {
        let mention_pattern = format!(r"<at>(@{}</at>)", regex::escape(bot_name));
        if let Ok(re) = Regex::new(&mention_pattern) {
            if re.is_match(text) {
                debug!("Bot mentioned in message: {}", text);
                return true;
            }
        }
    }

    // Also check entities for mentions
    if let Some(entities) = &activity.entities {
        for entity in entities {
            if entity.entity_type == "mention" {
                if let Some(mentioned) = &entity.mentioned {
                    if let Some(name) = &mentioned.name {
                        if name.contains(bot_name) {
                            debug!("Bot entity mentioned: {}", name);
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Extract command from a mention
pub fn extract_command_from_mention(text: &str, bot_name: &str) -> Option<String> {
    let mention_pattern = format!(r"<at>@{}</at>\s*(.*)", regex::escape(bot_name));
    if let Ok(re) = Regex::new(&mention_pattern) {
        if let Some(captures) = re.captures(text) {
            return captures.get(1).map(|m| m.as_str().trim().to_string());
        }
    }

    None
}

/// Parse form data from activity value (Adaptive Card submission)
pub fn parse_incident_data(
    activity: &Activity,
    conversation_id: String,
    submitted_by: String,
) -> Result<IncidentData> {
    let mut incident = IncidentData::new(conversation_id, submitted_by);

    if let Some(value) = &activity.value {
        if let Some(obj) = value.as_object() {
            // Extract fields from the form data
            incident.reporter_name = obj
                .get("reporter_name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.reporter_team = obj
                .get("reporter_team")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.reporter_contact = obj
                .get("reporter_contact")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.user_name = obj
                .get("user_name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.user_account = obj
                .get("user_account")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.module = obj
                .get("module")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.screen = obj
                .get("screen")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            incident.steps = obj
                .get("steps")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.expected = obj
                .get("expected")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.actual = obj
                .get("actual")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.error_message = obj
                .get("error_message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            incident.environment = obj
                .get("environment")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.platform = obj
                .get("platform")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            incident.network = obj
                .get("network")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            incident.severity = obj
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            incident.users_affected = obj
                .get("users_affected")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32);

            incident.notes = obj
                .get("notes")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Store the activity ID for reference
            incident.teams_message_id = Some(activity.id.clone());
        }
    }

    Ok(incident)
}

/// Extract action type from Adaptive Card submission
pub fn extract_action_type(activity: &Activity) -> Option<String> {
    activity
        .value
        .as_ref()
        .and_then(|v| v.get("action"))
        .and_then(|a| a.as_str())
        .map(|s| s.to_string())
}

/// Check if activity is an Adaptive Card action
pub fn is_card_action(activity: &Activity) -> bool {
    activity.activity_type == ActivityType::Invoke
        || (activity.activity_type == ActivityType::Message && activity.value.is_some())
}

/// Check if activity is a conversation update (bot added to channel)
pub fn is_conversation_update(activity: &Activity) -> bool {
    activity.activity_type == ActivityType::ConversationUpdate
}

/// Check if activity is a message
pub fn is_message(activity: &Activity) -> bool {
    activity.activity_type == ActivityType::Message
}

/// Clean text by removing mentions
pub fn clean_text(text: &str) -> String {
    let re = Regex::new(r"<at>.*?</at>").unwrap();
    re.replace_all(text, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_incident() {
        let cmd = parse_command("/incident");
        assert_eq!(cmd, Some(TeamsCommand::Incident));
    }

    #[test]
    fn test_parse_command_help() {
        let cmd = parse_command("/help");
        assert_eq!(cmd, Some(TeamsCommand::Help));
    }

    #[test]
    fn test_parse_command_cancel() {
        let cmd = parse_command("/cancel");
        assert_eq!(cmd, Some(TeamsCommand::Cancel));
    }

    #[test]
    fn test_parse_command_with_args() {
        let cmd = parse_command("/incident urgent");
        assert_eq!(cmd, Some(TeamsCommand::Incident));
    }

    #[test]
    fn test_parse_command_unknown() {
        let cmd = parse_command("/unknown");
        assert_eq!(cmd, Some(TeamsCommand::Unknown("/unknown".to_string())));
    }

    #[test]
    fn test_parse_command_no_slash() {
        let cmd = parse_command("incident");
        assert_eq!(cmd, None);
    }

    #[test]
    fn test_extract_command_from_mention() {
        let text = "<at>@OneSiam Bot</at> incident";
        let cmd = extract_command_from_mention(text, "OneSiam Bot");
        assert_eq!(cmd, Some("incident".to_string()));
    }

    #[test]
    fn test_extract_action_type() {
        let mut activity = Activity {
            activity_type: ActivityType::Message,
            id: "123".to_string(),
            timestamp: None,
            channel_id: None,
            from: None,
            conversation: None,
            recipient: None,
            text: None,
            attachments: None,
            entities: None,
            channel_data: None,
            action: None,
            reply_to_id: None,
            value: Some(serde_json::json!({"action": "submit_incident"})),
            name: None,
        };

        let action = extract_action_type(&activity);
        assert_eq!(action, Some("submit_incident".to_string()));
    }

    #[test]
    fn test_is_card_action() {
        let mut activity = Activity {
            activity_type: ActivityType::Message,
            id: "123".to_string(),
            timestamp: None,
            channel_id: None,
            from: None,
            conversation: None,
            recipient: None,
            text: None,
            attachments: None,
            entities: None,
            channel_data: None,
            action: None,
            reply_to_id: None,
            value: Some(serde_json::json!({"action": "submit"})),
            name: None,
        };

        assert!(is_card_action(&activity));
    }

    #[test]
    fn test_clean_text() {
        let text = "<at>@OneSiam Bot</at> incident report";
        let cleaned = clean_text(text);
        assert_eq!(cleaned, "incident report");
    }
}
