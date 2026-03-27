use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;

use super::models::{AdaptiveCardAction, Attachment};

/// Create the OneSiam Incident Adaptive Card
pub fn create_incident_card() -> Result<Attachment> {
    let card = json!({
        "type": "AdaptiveCard",
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "version": "1.4",
        "body": [
            {
                "type": "TextBlock",
                "text": "🔴 OneSiam Incident Report",
                "size": "Large",
                "weight": "Bolder",
                "color": "Attention"
            },
            {
                "type": "TextBlock",
                "text": "Please fill out the incident report below. All fields marked with * are required.",
                "wrap": true,
                "size": "Small",
                "color": "Good",
                "isSubtle": true
            },
            {
                "type": "TextBlock",
                "text": "1) Reporter Info",
                "size": "Medium",
                "weight": "Bolder",
                "spacing": "Medium"
            },
            {
                "type": "Input.Text",
                "id": "reporter_name",
                "placeholder": "Your name",
                "label": "Name *",
                "isRequired": true,
                "maxLength": 100
            },
            {
                "type": "Input.Text",
                "id": "reporter_team",
                "placeholder": "Your team or department",
                "label": "Team / Role *",
                "isRequired": true,
                "maxLength": 100
            },
            {
                "type": "Input.Text",
                "id": "reporter_contact",
                "placeholder": "email@example.com",
                "label": "Contact *",
                "isRequired": true,
                "maxLength": 200
            },
            {
                "type": "TextBlock",
                "text": "2) Affected User / Account",
                "size": "Medium",
                "weight": "Bolder",
                "spacing": "Medium"
            },
            {
                "type": "Input.Text",
                "id": "user_name",
                "placeholder": "Customer or user name",
                "label": "Customer name *",
                "isRequired": true,
                "maxLength": 100
            },
            {
                "type": "Input.Text",
                "id": "user_account",
                "placeholder": "Account ID or identifier",
                "label": "Account *",
                "isRequired": true,
                "maxLength": 100
            },
            {
                "type": "TextBlock",
                "text": "3) Feature / Module",
                "size": "Medium",
                "weight": "Bolder",
                "spacing": "Medium"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "module",
                "placeholder": "Select module",
                "label": "Module *",
                "isRequired": true,
                "choices": [
                    { "title": "Authentication", "value": "Authentication" },
                    { "title": "User Management", "value": "User Management" },
                    { "title": "Payment Processing", "value": "Payment Processing" },
                    { "title": "Inventory", "value": "Inventory" },
                    { "title": "Reporting", "value": "Reporting" },
                    { "title": "Notifications", "value": "Notifications" },
                    { "title": "API / Integration", "value": "API / Integration" },
                    { "title": "Database", "value": "Database" },
                    { "title": "Network / Infrastructure", "value": "Network / Infrastructure" },
                    { "title": "Other", "value": "Other" }
                ]
            },
            {
                "type": "Input.Text",
                "id": "screen",
                "placeholder": "e.g., Login Page, Dashboard",
                "label": "Screen",
                "maxLength": 200
            },
            {
                "type": "TextBlock",
                "text": "4) Steps to Reproduce",
                "size": "Medium",
                "weight": "Bolder",
                "spacing": "Medium"
            },
            {
                "type": "Input.Text",
                "id": "steps",
                "placeholder": "1. Navigate to...\n2. Click on...\n3. Enter...",
                "label": "Steps to reproduce *",
                "isRequired": true,
                "isMultiline": true,
                "maxLength": 2000
            },
            {
                "type": "TextBlock",
                "text": "5) Expected vs Actual",
                "size": "Medium",
                "weight": "Bolder",
                "spacing": "Medium"
            },
            {
                "type": "Input.Text",
                "id": "expected",
                "placeholder": "What should happen?",
                "label": "Expected *",
                "isRequired": true,
                "isMultiline": true,
                "maxLength": 2000
            },
            {
                "type": "Input.Text",
                "id": "actual",
                "placeholder": "What actually happened?",
                "label": "Actual *",
                "isRequired": true,
                "isMultiline": true,
                "maxLength": 2000
            },
            {
                "type": "Input.Text",
                "id": "error_message",
                "placeholder": "Copy the error message here",
                "label": "Error message",
                "maxLength": 500
            },
            {
                "type": "TextBlock",
                "text": "6) Scope & Impact",
                "size": "Medium",
                "weight": "Bolder",
                "spacing": "Medium"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "environment",
                "placeholder": "Select environment",
                "label": "Environment *",
                "isRequired": true,
                "choices": [
                    { "title": "Production", "value": "Production" },
                    { "title": "Staging", "value": "Staging" },
                    { "title": "Development", "value": "Development" },
                    { "title": "UAT", "value": "UAT" }
                ]
            },
            {
                "type": "Input.Text",
                "id": "platform",
                "placeholder": "e.g., Web, Mobile, Desktop",
                "label": "Platform",
                "maxLength": 100
            },
            {
                "type": "Input.Text",
                "id": "network",
                "placeholder": "e.g., Corporate VPN, Public Internet",
                "label": "Network",
                "maxLength": 100
            },
            {
                "type": "Input.ChoiceSet",
                "id": "severity",
                "placeholder": "Select severity level",
                "label": "Severity *",
                "isRequired": true,
                "choices": [
                    { "title": "🔴 Critical - System down, no workaround", "value": "Critical" },
                    { "title": "🟠 High - Major functionality impacted", "value": "High" },
                    { "title": "🟡 Medium - Partial impact, workaround exists", "value": "Medium" },
                    { "title": "🟢 Low - Minor issue, cosmetic", "value": "Low" }
                ]
            },
            {
                "type": "Input.Number",
                "id": "users_affected",
                "placeholder": "0",
                "label": "Users affected",
                "min": 0
            },
            {
                "type": "Input.Text",
                "id": "time_of_issue",
                "placeholder": "When did the issue occur?",
                "label": "Time of issue",
                "maxLength": 100
            },
            {
                "type": "TextBlock",
                "text": "7) Extra Notes",
                "size": "Medium",
                "weight": "Bolder",
                "spacing": "Medium"
            },
            {
                "type": "Input.Text",
                "id": "notes",
                "placeholder": "Any additional context, screenshots, or logs...",
                "label": "Notes",
                "isMultiline": true,
                "maxLength": 5000
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "📤 Submit Incident Report",
                "data": {
                    "action": "submit_incident"
                },
                "style": "positive"
            },
            {
                "type": "Action.Submit",
                "title": "❌ Cancel",
                "data": {
                    "action": "cancel"
                },
                "style": "destructive"
            }
        ]
    });

    Ok(Attachment {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        content: Some(card),
        url: None,
    })
}

/// Create the Welcome Card with trigger options
pub fn create_welcome_card() -> Result<Attachment> {
    let card = json!({
        "type": "AdaptiveCard",
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "version": "1.4",
        "body": [
            {
                "type": "TextBlock",
                "text": "🤖 OneSiam Incident Bot",
                "size": "Large",
                "weight": "Bolder"
            },
            {
                "type": "TextBlock",
                "text": "Report incidents quickly and efficiently. Click the button below to start a new incident report.",
                "wrap": true,
                "size": "Medium",
                "color": "Good",
                "isSubtle": true,
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "You can trigger an incident report in three ways:",
                "wrap": true,
                "size": "Small",
                "weight": "Bolder",
                "spacing": "Small"
            },
            {
                "type": "TextBlock",
                "text": "• Click the button below\n• Type /incident\n• Mention @OneSiam Incident Bot",
                "wrap": true,
                "size": "Small",
                "isSubtle": true,
                "spacing": "None"
            },
            {
                "type": "FactSet",
                "facts": [
                    {
                        "title": "⏱️ Response Time",
                        "value": "Jira tickets created within 5 minutes"
                    },
                    {
                        "title": "📊 Priority Levels",
                        "value": "Critical, High, Medium, Low"
                    },
                    {
                        "title": "🎯 Tracking",
                        "value": "Full TSD project integration"
                    }
                ],
                "spacing": "Medium"
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "📋 Report New Incident",
                "data": {
                    "action": "open_incident_form"
                },
                "style": "positive"
            },
            {
                "type": "Action.Submit",
                "title": "❓ Help",
                "data": {
                    "action": "help"
                }
            }
        ]
    });

    Ok(Attachment {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        content: Some(card),
        url: None,
    })
}

/// Create a help card
pub fn create_help_card() -> Result<Attachment> {
    let card = json!({
        "type": "AdaptiveCard",
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "version": "1.4",
        "body": [
            {
                "type": "TextBlock",
                "text": "📖 OneSiam Incident Bot - Help",
                "size": "Large",
                "weight": "Bolder"
            },
            {
                "type": "TextBlock",
                "text": "How to use this bot:",
                "size": "Medium",
                "weight": "Bolder",
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "🚀 Report an Incident\n\nUse any of these methods:\n• Click the 'Report New Incident' button\n• Type /incident\n• Mention @OneSiam Incident Bot\n\nFill out the form with all required fields and submit. A Jira ticket will be created automatically.",
                "wrap": true,
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "⚠️ Severity Levels\n\n• 🔴 Critical: System down, no workaround\n• 🟠 High: Major functionality impacted\n• 🟡 Medium: Partial impact, workaround exists\n• 🟢 Low: Minor issue, cosmetic",
                "wrap": true,
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "💡 Tips\n\n• Include error messages in your report\n• Describe steps to reproduce clearly\n• Specify the correct environment\n• Add screenshots or logs in the notes section",
                "wrap": true,
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "❓ Questions?\n\nContact the DevOps team for support.",
                "wrap": true,
                "size": "Small",
                "isSubtle": true,
                "spacing": "Large"
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "📋 Report New Incident",
                "data": {
                    "action": "open_incident_form"
                },
                "style": "positive"
            }
        ]
    });

    Ok(Attachment {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        content: Some(card),
        url: None,
    })
}

/// Create a success card after incident submission
pub fn create_success_card(jira_ticket_id: String, jira_ticket_url: String) -> Result<Attachment> {
    let card = json!({
        "type": "AdaptiveCard",
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "version": "1.4",
        "body": [
            {
                "type": "TextBlock",
                "text": "✅ Incident Report Submitted Successfully!",
                "size": "Large",
                "weight": "Bolder",
                "color": "Good",
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "Your incident has been logged and a Jira ticket has been created.",
                "wrap": true,
                "size": "Medium",
                "spacing": "Small"
            },
            {
                "type": "FactSet",
                "facts": [
                    {
                        "title": "📋 Ticket ID",
                        "value": jira_ticket_id
                    },
                    {
                        "title": "🔗 Ticket URL",
                        "value": format!("[View Ticket]({})", jira_ticket_url)
                    },
                    {
                        "title": "⏱️ Response Time",
                        "value": "Ticket created within 5 minutes"
                    }
                ],
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "💡 You will receive updates via the Jira ticket. Track progress using the link above.",
                "wrap": true,
                "size": "Small",
                "isSubtle": true,
                "spacing": "Large"
            }
        ],
        "actions": [
            {
                "type": "Action.OpenUrl",
                "title": "🔗 View Jira Ticket",
                "url": jira_ticket_url,
                "style": "positive"
            }
        ]
    });

    Ok(Attachment {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        content: Some(card),
        url: None,
    })
}

/// Create an error card
pub fn create_error_card(error_message: String) -> Result<Attachment> {
    let card = json!({
        "type": "AdaptiveCard",
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "version": "1.4",
        "body": [
            {
                "type": "TextBlock",
                "text": "❌ Error Submitting Incident",
                "size": "Large",
                "weight": "Bolder",
                "color": "Attention",
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "We encountered an error while processing your incident report.",
                "wrap": true,
                "size": "Medium",
                "spacing": "Small"
            },
            {
                "type": "TextBlock",
                "text": format!("🔍 Error Details:\n\n{}", error_message),
                "wrap": true,
                "size": "Small",
                "isSubtle": true,
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "💡 Please try again or contact the DevOps team if the issue persists.",
                "wrap": true,
                "size": "Small",
                "isSubtle": true,
                "spacing": "Large"
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "🔄 Try Again",
                "data": {
                    "action": "open_incident_form"
                },
                "style": "positive"
            }
        ]
    });

    Ok(Attachment {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        content: Some(card),
        url: None,
    })
}

/// Create a validation error card
pub fn create_validation_error_card(errors: Vec<String>) -> Result<Attachment> {
    let error_list = errors
        .iter()
        .enumerate()
        .map(|(i, e)| format!("{}. {}", i + 1, e))
        .collect::<Vec<_>>()
        .join("\n");

    let card = json!({
        "type": "AdaptiveCard",
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "version": "1.4",
        "body": [
            {
                "type": "TextBlock",
                "text": "⚠️ Validation Errors",
                "size": "Large",
                "weight": "Bolder",
                "color": "Attention",
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "Please fix the following errors and resubmit:",
                "wrap": true,
                "size": "Medium",
                "spacing": "Small"
            },
            {
                "type": "TextBlock",
                "text": error_list,
                "wrap": true,
                "size": "Small",
                "isSubtle": true,
                "spacing": "Medium"
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "📋 Try Again",
                "data": {
                    "action": "open_incident_form"
                },
                "style": "positive"
            }
        ]
    });

    Ok(Attachment {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        content: Some(card),
        url: None,
    })
}

/// Extract form data from Adaptive Card action submit
pub fn extract_form_data(value: &serde_json::Value) -> Result<HashMap<String, String>> {
    let mut data = HashMap::new();

    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            if key != "action" {
                let str_val = match val {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => String::new(),
                };
                data.insert(key.clone(), str_val);
            }
        }
    }

    Ok(data)
}

/// Extract action type from Adaptive Card action submit
pub fn extract_action_type(value: &serde_json::Value) -> Option<String> {
    value
        .get("action")
        .and_then(|a| a.as_str())
        .map(|s| s.to_string())
}
