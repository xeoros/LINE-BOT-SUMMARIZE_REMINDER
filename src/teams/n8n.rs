use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use tracing::{debug, error, info};

use super::models::{IncidentData, N8nResponse};

/// n8n Webhook Client
#[derive(Clone)]
pub struct N8nClient {
    webhook_url: String,
    http_client: Client,
}

impl N8nClient {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            http_client: Client::new(),
        }
    }

    /// Trigger Jira ticket creation via n8n webhook
    pub async fn trigger_jira_creation(&self, incident_data: &IncidentData) -> Result<N8nResponse> {
        debug!("Sending incident data to n8n webhook: {}", self.webhook_url);
        debug!("Incident data: {}", json!(incident_data));

        let payload = json!({
            "reporter_name": incident_data.reporter_name,
            "reporter_team": incident_data.reporter_team,
            "reporter_contact": incident_data.reporter_contact,
            "user_name": incident_data.user_name,
            "user_account": incident_data.user_account,
            "module": incident_data.module,
            "screen": incident_data.screen,
            "steps": incident_data.steps,
            "expected": incident_data.expected,
            "actual": incident_data.actual,
            "error_message": incident_data.error_message,
            "environment": incident_data.environment,
            "platform": incident_data.platform,
            "network": incident_data.network,
            "severity": incident_data.severity,
            "users_affected": incident_data.users_affected,
            "time_of_issue": incident_data.time_of_issue.map(|dt| dt.to_rfc3339()),
            "notes": incident_data.notes,
            "teams_conversation_id": incident_data.teams_conversation_id,
            "teams_message_id": incident_data.teams_message_id,
            "submitted_by": incident_data.submitted_by,
            "submitted_at": incident_data.submitted_at.to_rfc3339(),
        });

        let response = self
            .http_client
            .post(&self.webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to n8n webhook")?;

        let status = response.status();

        if status.is_success() {
            info!("Successfully sent incident data to n8n webhook");

            // Parse the response
            let response_body = response
                .json::<serde_json::Value>()
                .await
                .context("Failed to parse n8n response")?;

            debug!("n8n response: {}", response_body);

            // Extract ticket information from response
            let jira_ticket_id = response_body
                .get("jira_ticket_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let jira_ticket_url = response_body
                .get("jira_ticket_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let message = response_body
                .get("message")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Ok(N8nResponse {
                success: true,
                jira_ticket_id,
                jira_ticket_url,
                message,
            })
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read response body".to_string());

            error!(
                "Failed to send incident data to n8n. Status: {}, Body: {}",
                status, body
            );

            // Still return a response object with success: false
            Ok(N8nResponse {
                success: false,
                jira_ticket_id: None,
                jira_ticket_url: None,
                message: Some(format!(
                    "Failed to create Jira ticket: {} - {}",
                    status, body
                )),
            })
        }
    }

    /// Check if the webhook URL is valid
    pub async fn health_check(&self) -> Result<bool> {
        debug!("Checking n8n webhook health: {}", self.webhook_url);

        // Note: Many webhooks don't respond to HEAD/OPTIONS requests
        // This is a basic connectivity check
        let response = self
            .http_client
            .head(&self.webhook_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;

        match response {
            Ok(resp) => {
                // Even if we get a 405 Method Not Allowed, the endpoint exists
                let is_reachable = resp.status().is_success()
                    || resp.status().as_u16() == 405
                    || resp.status().as_u16() == 404; // 404 might still mean the service is up
                Ok(is_reachable)
            }
            Err(e) => {
                // Don't log as error, just return false
                debug!("n8n webhook health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_data_serialization() {
        let incident = IncidentData::new("conv_id".to_string(), "user_id".to_string());

        let json = json!(incident);
        assert_eq!(json["teams_conversation_id"], "conv_id");
        assert_eq!(json["submitted_by"], "user_id");
    }

    #[test]
    fn test_n8n_response_deserialization() {
        let json_str = r#"{
            "success": true,
            "jira_ticket_id": "TSD-123",
            "jira_ticket_url": "https://jira.company.com/browse/TSD-123",
            "message": "Ticket created successfully"
        }"#;

        let response: N8nResponse = serde_json::from_str(json_str).unwrap();
        assert!(response.success);
        assert_eq!(response.jira_ticket_id, Some("TSD-123".to_string()));
        assert_eq!(
            response.jira_ticket_url,
            Some("https://jira.company.com/browse/TSD-123".to_string())
        );
    }
}
