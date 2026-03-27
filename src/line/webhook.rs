use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "replyToken")]
    pub reply_token: Option<String>,
    pub timestamp: i64,
    pub source: EventSource,
    pub message: Option<EventMessage>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EventSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[serde(rename = "groupId")]
    pub group_id: Option<String>,
    #[serde(rename = "roomId")]
    pub room_id: Option<String>,
}

impl EventSource {
    pub fn get_type_and_id(&self) -> Option<(&str, &str)> {
        match self.source_type.as_str() {
            "user" => self.user_id.as_ref().map(|id| ("user", id.as_str())),
            "group" => self.group_id.as_ref().map(|id| ("group", id.as_str())),
            "room" => self.room_id.as_ref().map(|id| ("room", id.as_str())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EventMessage {
    #[serde(rename = "id")]
    pub message_id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(rename = "text")]
    pub text: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WebhookBody {
    pub destination: Option<String>,
    pub events: Vec<WebhookEvent>,
}

pub fn verify_webhook_signature(
    channel_secret: &str,
    body: &[u8],
    signature: &str,
) -> Result<bool> {
    let mut mac =
        HmacSha256::new_from_slice(channel_secret.as_bytes()).context("Failed to create HMAC")?;

    mac.update(body);

    let expected_signature = BASE64.encode(mac.finalize().into_bytes());
    Ok(expected_signature == signature)
}

pub fn parse_webhook_body(body: &[u8]) -> Result<WebhookBody> {
    serde_json::from_slice(body).context("Failed to parse webhook body")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    #[test]
    fn event_source_get_type_and_id_user() {
        let source = EventSource {
            source_type: "user".to_string(),
            user_id: Some("U123".to_string()),
            group_id: None,
            room_id: None,
        };
        let result = source.get_type_and_id();
        assert_eq!(result, Some(("user", "U123")));
    }

    #[test]
    fn event_source_get_type_and_id_group() {
        let source = EventSource {
            source_type: "group".to_string(),
            user_id: None,
            group_id: Some("G123".to_string()),
            room_id: None,
        };
        let result = source.get_type_and_id();
        assert_eq!(result, Some(("group", "G123")));
    }

    #[test]
    fn event_source_get_type_and_id_room() {
        let source = EventSource {
            source_type: "room".to_string(),
            user_id: None,
            group_id: None,
            room_id: Some("R123".to_string()),
        };
        let result = source.get_type_and_id();
        assert_eq!(result, Some(("room", "R123")));
    }

    #[test]
    fn event_source_get_type_and_id_unknown() {
        let source = EventSource {
            source_type: "unknown".to_string(),
            user_id: Some("U123".to_string()),
            group_id: None,
            room_id: None,
        };
        let result = source.get_type_and_id();
        assert!(result.is_none());
    }

    #[test]
    fn verify_webhook_signature_matches() {
        let secret = "test-secret";
        let body = br#"{"events":[]}"#;

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected_signature =
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        let verified = verify_webhook_signature(secret, body, &expected_signature).unwrap();
        assert!(verified);
    }

    #[test]
    fn verify_webhook_signature_mismatch() {
        let verified = verify_webhook_signature("secret", b"body", "invalid").unwrap();
        assert!(!verified);
    }

    #[test]
    fn parse_webhook_body_parses_events() {
        let payload = r#"
        {
          "destination": "U123",
          "events": [
            {
              "type": "message",
              "replyToken": "token",
              "timestamp": 1710000000000,
              "source": { "type": "user", "userId": "U123" },
              "message": { "id": "1", "type": "text", "text": "hello" }
            }
          ]
        }
        "#;

        let parsed = parse_webhook_body(payload.as_bytes()).unwrap();
        assert_eq!(parsed.destination.as_deref(), Some("U123"));
        assert_eq!(parsed.events.len(), 1);
        let event = &parsed.events[0];
        assert_eq!(event.event_type, "message");
        assert_eq!(event.reply_token.as_deref(), Some("token"));
        assert_eq!(event.source.source_type, "user");
        assert_eq!(event.message.as_ref().unwrap().text.as_deref(), Some("hello"));
    }
}
