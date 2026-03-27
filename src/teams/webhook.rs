use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, error, info, warn};

use super::models::Activity;

/// Teams Webhook Event
#[derive(Debug, Deserialize)]
pub struct TeamsWebhookEvent {
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub id: Option<String>,
    pub timestamp: Option<String>,
    pub channel_data: Option<serde_json::Value>,
    #[serde(flatten)]
    pub activity: Option<Activity>,
}

/// Teams Webhook Handler
pub struct TeamsWebhookHandler {
    app_id: String,
    app_password: String,
}

impl TeamsWebhookHandler {
    pub fn new(app_id: String, app_password: String) -> Self {
        Self {
            app_id,
            app_password,
        }
    }

    /// Verify the authentication from the incoming request
    /// Note: Microsoft Bot Framework uses a different verification method than Slack/LINE
    /// The actual verification happens through the JWT token in the Authorization header
    pub fn verify_auth_token(&self, token: &str) -> Result<bool> {
        // TODO: Implement proper JWT token verification
        // For now, basic check that token exists
        if token.is_empty() {
            anyhow::bail!("Missing authentication token");
        }

        // Extract and verify JWT token
        if let Some(bearer_token) = token.strip_prefix("Bearer ") {
            // Basic format check
            let parts: Vec<&str> = bearer_token.split('.').collect();
            if parts.len() != 3 {
                warn!("Invalid JWT token format");
                return Ok(false);
            }

            // TODO: Verify signature and claims
            // For now, accept the token
            debug!("Token format verified (signature verification TODO)");
            Ok(true)
        } else {
            anyhow::bail!("Invalid Authorization header format");
        }
    }

    /// Parse the incoming webhook body
    pub fn parse_webhook_body(&self, body: &str) -> Result<TeamsWebhookEvent> {
        debug!("Parsing Teams webhook body");

        let event: serde_json::Value =
            serde_json::from_str(body).context("Failed to parse Teams webhook body as JSON")?;

        debug!("Parsed webhook event: {}", event);

        // Try to parse as Activity first
        if let Ok(activity) = serde_json::from_value::<Activity>(event.clone()) {
            return Ok(TeamsWebhookEvent {
                event_type: Some(activity.activity_type.clone().to_string()),
                id: Some(activity.id.clone()),
                timestamp: activity.timestamp.map(|dt| dt.to_rfc3339()),
                channel_data: activity.channel_data.clone(),
                activity: Some(activity),
            });
        }

        // If not an activity, return basic event info
        Ok(TeamsWebhookEvent {
            event_type: event
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            id: event
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            timestamp: event
                .get("timestamp")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            channel_data: event.get("channelData").cloned(),
            activity: None,
        })
    }

    /// Handle URL verification if present (Microsoft Bot Framework doesn't use this like Slack)
    pub fn handle_url_verification(&self, event: &TeamsWebhookEvent) -> Option<String> {
        // Teams doesn't use URL verification like Slack
        None
    }
}

/// Extract the Authorization token from headers
pub fn extract_auth_token(auth_header: Option<&str>) -> Result<String> {
    auth_header
        .map(|h| h.to_string())
        .ok_or_else(|| anyhow::anyhow!("Missing Authorization header"))
}

/// Validate the incoming request
pub fn validate_incoming_request(
    auth_header: Option<&str>,
    content_type: Option<&str>,
) -> Result<()> {
    // Check Authorization header
    let auth_token = extract_auth_token(auth_header)?;
    if auth_token.is_empty() {
        anyhow::bail!("Missing or empty Authorization header");
    }

    // Check Content-Type
    if let Some(ct) = content_type {
        if !ct.contains("application/json") {
            anyhow::bail!("Invalid Content-Type: {}. Expected application/json", ct);
        }
    } else {
        anyhow::bail!("Missing Content-Type header");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_auth_token() {
        let header = Some("Bearer some_token_here");
        let result = extract_auth_token(header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Bearer some_token_here");
    }

    #[test]
    fn test_extract_auth_token_missing() {
        let result = extract_auth_token(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_incoming_request() {
        let result = validate_incoming_request(Some("Bearer token123"), Some("application/json"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_incoming_request_no_auth() {
        let result = validate_incoming_request(None, Some("application/json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_incoming_request_invalid_content_type() {
        let result = validate_incoming_request(Some("Bearer token123"), Some("text/plain"));
        assert!(result.is_err());
    }
}
