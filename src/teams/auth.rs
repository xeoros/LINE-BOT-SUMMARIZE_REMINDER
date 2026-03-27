use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

const TEAMS_AUTHORITY: &str = "https://login.microsoftonline.com";
const TEAMS_SCOPE: &str = "https://api.botframework.com/.default";

/// Teams Authentication Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub expires_on: DateTime<Utc>,
    pub not_before: Option<DateTime<Utc>>,
    pub resource: String,
}

impl TeamsToken {
    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        // Consider token expired 5 minutes before actual expiration
        let expiry_buffer = Duration::minutes(5);
        now + expiry_buffer >= self.expires_on
    }
}

/// Teams Bot Authentication
#[derive(Clone)]
pub struct TeamsAuth {
    client_id: String,
    client_secret: String,
    tenant_id: String,
    token: Arc<RwLock<Option<TeamsToken>>>,
}

impl TeamsAuth {
    pub fn new(client_id: String, client_secret: String, tenant_id: String) -> Self {
        Self {
            client_id,
            client_secret,
            tenant_id,
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// Get a valid access token, refreshing if necessary
    pub async fn get_access_token(&self) -> Result<String> {
        // Check if we have a valid token
        {
            let token_read = self.token.read().await;
            if let Some(ref token) = *token_read {
                if !token.is_expired() {
                    return Ok(token.access_token.clone());
                }
            }
        }

        // Token is expired or not present, acquire a new one
        let new_token = self.acquire_token().await?;

        // Store the new token
        {
            let mut token_write = self.token.write().await;
            *token_write = Some(new_token.clone());
        }

        Ok(new_token.access_token)
    }

    /// Acquire a new access token using OAuth2 client credentials flow
    async fn acquire_token(&self) -> Result<TeamsToken> {
        let client = Client::new();

        let token_url = format!("{}/{}/oauth2/v2.0/token", TEAMS_AUTHORITY, self.tenant_id);

        let mut params = std::collections::HashMap::new();
        params.insert("grant_type", "client_credentials");
        params.insert("client_id", &self.client_id);
        params.insert("client_secret", &self.client_secret);
        params.insert("scope", TEAMS_SCOPE);

        let response = client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .context("Failed to exchange client credentials for access token")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Token request failed: {} - {}", status, body);
        }

        let token_response: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse token response")?;

        let access_token = token_response
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No access_token in response"))?
            .to_string();

        let expires_in = token_response
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        // Calculate expiration time
        let expires_on = Utc::now() + chrono::Duration::seconds(expires_in as i64);

        Ok(TeamsToken {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in,
            expires_on,
            not_before: None,
            resource: TEAMS_SCOPE.to_string(),
        })
    }

    /// Verify JWT token signature (placeholder for actual implementation)
    pub fn verify_token(token: &str) -> Result<bool> {
        // TODO: Implement proper JWT verification
        // For now, just check if token exists and is not empty
        Ok(!token.is_empty())
    }

    /// Extract tenant ID from JWT token (placeholder for actual implementation)
    pub fn extract_tenant_id(token: &str) -> Result<String> {
        // TODO: Implement proper JWT parsing
        // For now, return a placeholder
        Ok("placeholder_tenant".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_expiry() {
        let mut token = TeamsToken {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            expires_on: Utc::now() + chrono::Duration::seconds(3600),
            not_before: None,
            resource: TEAMS_SCOPE.to_string(),
        };

        assert!(!token.is_expired());

        token.expires_on = Utc::now() - chrono::Duration::seconds(3600);
        assert!(token.is_expired());
    }

    #[test]
    fn test_incident_data_validation() {
        use crate::teams::models::IncidentData;

        let incident = IncidentData::new("conversation_id".to_string(), "user_id".to_string());

        let errors = incident.validate().unwrap_err();
        assert!(!errors.is_empty());
    }
}
