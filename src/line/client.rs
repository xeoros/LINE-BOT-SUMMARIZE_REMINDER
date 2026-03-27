use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const LINE_API_BASE: &str = "https://api.line.me/v2/bot";

fn line_api_base() -> String {
    std::env::var("LINE_API_BASE").unwrap_or_else(|_| LINE_API_BASE.to_string())
}

fn build_client() -> Client {
    #[cfg(test)]
    {
        Client::builder()
            .no_proxy()
            .build()
            .expect("Failed to build reqwest client")
    }
    #[cfg(not(test))]
    {
        Client::new()
    }
}

#[derive(Debug, Serialize)]
struct ReplyMessageRequest {
    #[serde(rename = "replyToken")]
    reply_token: String,
    messages: Vec<MessageObject>,
}

#[derive(Debug, Serialize)]
struct PushMessageRequest {
    #[serde(rename = "to")]
    to: String,
    messages: Vec<MessageObject>,
}

#[derive(Debug, Serialize)]
struct MessageObject {
    #[serde(rename = "type")]
    message_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupMemberProfile {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UserProfile {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupSummary {
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "groupName")]
    pub group_name: String,
    #[serde(rename = "pictureUrl")]
    pub picture_url: String,
}

#[derive(Clone)]
pub struct LineClient {
    client: Client,
    access_token: String,
}

impl LineClient {
    pub fn new(access_token: String) -> Self {
        Self {
            client: build_client(),
            access_token,
        }
    }

    #[cfg(not(test))]
    pub async fn reply_message(&self, reply_token: &str, text: &str) -> Result<()> {
        let request = ReplyMessageRequest {
            reply_token: reply_token.to_string(),
            messages: vec![MessageObject {
                message_type: "text".to_string(),
                text: text.to_string(),
            }],
        };

        self.client
            .post(&format!("{}/message/reply", line_api_base()))
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to send reply message")?;

        Ok(())
    }

    #[cfg(test)]
    pub async fn reply_message(&self, _reply_token: &str, _text: &str) -> Result<()> {
        Ok(())
    }

    #[cfg(not(test))]
    pub async fn push_message(&self, to: &str, text: &str) -> Result<()> {
        let request = PushMessageRequest {
            to: to.to_string(),
            messages: vec![MessageObject {
                message_type: "text".to_string(),
                text: text.to_string(),
            }],
        };

        self.client
            .post(&format!("{}/message/push", line_api_base()))
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to send push message")?;

        Ok(())
    }

    #[cfg(test)]
    pub async fn push_message(&self, _to: &str, _text: &str) -> Result<()> {
        Ok(())
    }

    #[cfg(not(test))]
    pub async fn reply_markdown(&self, reply_token: &str, text: &str) -> Result<()> {
        let request = ReplyMessageRequest {
            reply_token: reply_token.to_string(),
            messages: vec![MessageObject {
                message_type: "text".to_string(), // LINE only accepts "text", not "markdown"
                text: text.to_string(),
            }],
        };

        self.client
            .post(&format!("{}/message/reply", line_api_base()))
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to send reply markdown message")?;

        Ok(())
    }

    #[cfg(test)]
    pub async fn reply_markdown(&self, _reply_token: &str, _text: &str) -> Result<()> {
        Ok(())
    }

    #[cfg(not(test))]
    pub async fn push_markdown(&self, to: &str, text: &str) -> Result<()> {
        let request = PushMessageRequest {
            to: to.to_string(),
            messages: vec![MessageObject {
                message_type: "text".to_string(), // LINE only accepts "text", not "markdown"
                text: text.to_string(),
            }],
        };

        self.client
            .post(&format!("{}/message/push", line_api_base()))
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .await
            .context("Failed to send push markdown message")?;

        Ok(())
    }

    #[cfg(test)]
    pub async fn push_markdown(&self, _to: &str, _text: &str) -> Result<()> {
        Ok(())
    }

    #[cfg(not(test))]
    pub async fn get_group_member_profile(
        &self,
        group_id: &str,
        user_id: &str,
    ) -> Result<GroupMemberProfile> {
        let url = format!("{}/group/{}/member/{}", line_api_base(), group_id, user_id);

        let profile = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get group member profile")?
            .json::<GroupMemberProfile>()
            .await
            .context("Failed to parse group member profile")?;

        Ok(profile)
    }

    #[cfg(test)]
    pub async fn get_group_member_profile(
        &self,
        _group_id: &str,
        user_id: &str,
    ) -> Result<GroupMemberProfile> {
        Ok(GroupMemberProfile {
            user_id: user_id.to_string(),
            display_name: "Test User".to_string(),
        })
    }

    #[cfg(not(test))]
    pub async fn get_user_profile(&self, user_id: &str) -> Result<UserProfile> {
        let url = format!("{}/profile/{}", line_api_base(), user_id);

        let profile = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get user profile")?
            .json::<UserProfile>()
            .await
            .context("Failed to parse user profile")?;

        Ok(profile)
    }

    #[cfg(test)]
    pub async fn get_user_profile(&self, user_id: &str) -> Result<UserProfile> {
        Ok(UserProfile {
            user_id: user_id.to_string(),
            display_name: "Test User".to_string(),
        })
    }

    #[cfg(not(test))]
    pub async fn get_group_summary(&self, group_id: &str) -> Result<GroupSummary> {
        let url = format!("{}/group/{}/summary", line_api_base(), group_id);

        let summary = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get group summary")?
            .json::<GroupSummary>()
            .await
            .context("Failed to parse group summary")?;

        Ok(summary)
    }

    #[cfg(test)]
    pub async fn get_group_summary(&self, group_id: &str) -> Result<GroupSummary> {
        Ok(GroupSummary {
            group_id: group_id.to_string(),
            group_name: "Test Group".to_string(),
            picture_url: "https://example.com/pic.png".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reply_message_returns_ok_in_test_mode() {
        let client = LineClient::new("token".to_string());
        let result = client.reply_message("reply", "hi").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_user_profile_parses_response() {
        let client = LineClient::new("token".to_string());
        let profile = client.get_user_profile("U123").await.unwrap();
        assert_eq!(profile.user_id, "U123");
        assert_eq!(profile.display_name, "Test User");
    }
}
