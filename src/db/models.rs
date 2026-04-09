use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

#[derive(Debug, Clone)]
pub struct Message {
    pub id: i32,
    pub message_id: String,
    pub source_type: SourceType,
    pub source_id: String,
    pub sender_id: Option<String>,
    pub display_name: Option<String>,
    pub message_type: MessageType,
    pub message_text: Option<String>,
    pub thread_id: Option<String>,
    pub parent_message_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    User,
    Group,
    Room,
    SlackChannel,
    SlackUser,
}

impl SourceType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "user" => Ok(SourceType::User),
            "group" => Ok(SourceType::Group),
            "room" => Ok(SourceType::Room),
            "slack_channel" => Ok(SourceType::SlackChannel),
            "slack_user" => Ok(SourceType::SlackUser),
            _ => anyhow::bail!("Invalid source type: {}", s),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::User => "user",
            SourceType::Group => "group",
            SourceType::Room => "room",
            SourceType::SlackChannel => "slack_channel",
            SourceType::SlackUser => "slack_user",
        }
    }
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Text,
    Image,
    Video,
    Audio,
    File,
    Location,
    Sticker,
    Unknown,
}

impl MessageType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "text" => MessageType::Text,
            "image" => MessageType::Image,
            "video" => MessageType::Video,
            "audio" => MessageType::Audio,
            "file" => MessageType::File,
            "location" => MessageType::Location,
            "sticker" => MessageType::Sticker,
            _ => MessageType::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Text => "text",
            MessageType::Image => "image",
            MessageType::Video => "video",
            MessageType::Audio => "audio",
            MessageType::File => "file",
            MessageType::Location => "location",
            MessageType::Sticker => "sticker",
            MessageType::Unknown => "unknown",
        }
    }
}

impl Message {
    pub async fn save(
        pool: &PgPool,
        message_id: &str,
        source_type: SourceType,
        source_id: &str,
        sender_id: Option<&str>,
        display_name: Option<&str>,
        message_type: MessageType,
        message_text: Option<&str>,
        thread_id: Option<&str>,
        parent_message_id: Option<&str>,
    ) -> Result<i32> {
        let row = sqlx::query(
            r#"
            INSERT INTO messages (message_id, source_type, source_id, sender_id, display_name, message_type, message_text, thread_id, parent_message_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (message_id) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(message_id)
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(sender_id)
        .bind(display_name)
        .bind(message_type.as_str())
        .bind(message_text)
        .bind(thread_id)
        .bind(parent_message_id)
        .fetch_one(pool)
        .await?;

        Ok(row
            .try_get("id")
            .context("Failed to get id from INSERT result")?)
    }

    pub async fn get_recent_messages(
        pool: &PgPool,
        source_type: SourceType,
        source_id: &str,
        limit: i32,
    ) -> Result<Vec<Message>> {
        #[derive(sqlx::FromRow)]
        struct MessageRow {
            id: i32,
            message_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            display_name: Option<String>,
            message_type: String,
            message_text: Option<String>,
            thread_id: Option<String>,
            parent_message_id: Option<String>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<MessageRow> = sqlx::query_as(
            r#"
            SELECT id, message_id, source_type, source_id, sender_id, display_name, message_type, message_text, thread_id, parent_message_id, created_at
            FROM messages
            WHERE source_type = $1 AND source_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let messages = rows
            .into_iter()
            .map(|row| Message {
                id: row.id,
                message_id: row.message_id,
                source_type: SourceType::from_str(&row.source_type).unwrap(),
                source_id: row.source_id,
                sender_id: row.sender_id,
                display_name: row.display_name,
                message_type: MessageType::from_str(&row.message_type),
                message_text: row.message_text,
                thread_id: row.thread_id,
                parent_message_id: row.parent_message_id,
                created_at: row.created_at,
            })
            .collect();

        Ok(messages)
    }

    pub async fn get_messages_by_time_range(
        pool: &PgPool,
        source_type: SourceType,
        source_id: &str,
        minutes: i32,
    ) -> Result<Vec<Message>> {
        #[derive(sqlx::FromRow)]
        struct MessageRow {
            id: i32,
            message_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            display_name: Option<String>,
            message_type: String,
            message_text: Option<String>,
            thread_id: Option<String>,
            parent_message_id: Option<String>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<MessageRow> = sqlx::query_as(
            r#"
            SELECT id, message_id, source_type, source_id, sender_id, display_name, message_type, message_text, thread_id, parent_message_id, created_at
            FROM messages
            WHERE source_type = $1 AND source_id = $2 AND created_at >= NOW() - INTERVAL '1 minute' * $3
            ORDER BY created_at ASC
            "#,
        )
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(minutes)
        .fetch_all(pool)
        .await?;

        let messages = rows
            .into_iter()
            .map(|row| Message {
                id: row.id,
                message_id: row.message_id,
                source_type: SourceType::from_str(&row.source_type).unwrap(),
                source_id: row.source_id,
                sender_id: row.sender_id,
                display_name: row.display_name,
                message_type: MessageType::from_str(&row.message_type),
                message_text: row.message_text,
                thread_id: row.thread_id,
                parent_message_id: row.parent_message_id,
                created_at: row.created_at,
            })
            .collect();

        Ok(messages)
    }

    pub fn format_for_summary(&self) -> Option<String> {
        if self.message_type != MessageType::Text {
            return None;
        }

        let text = self.message_text.as_deref().unwrap_or("");
        if text.is_empty() {
            return None;
        }

        let name = self.display_name.as_deref().unwrap_or("Unknown");

        let time = self.created_at.format("%H:%M");
        Some(format!("[{} {}]: {}", name, time, text))
    }

    pub fn format_conversation(messages: &[Message]) -> String {
        let mut conversation = String::new();

        for msg in messages {
            if let Some(formatted) = msg.format_for_summary() {
                conversation.push_str(&formatted);
                conversation.push('\n');
            }
        }

        conversation
    }

    pub async fn get_thread_messages(pool: &PgPool, thread_id: &str) -> Result<Vec<Message>> {
        #[derive(sqlx::FromRow)]
        struct MessageRow {
            id: i32,
            message_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            display_name: Option<String>,
            message_type: String,
            message_text: Option<String>,
            thread_id: Option<String>,
            parent_message_id: Option<String>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<MessageRow> = sqlx::query_as(
            r#"
            SELECT id, message_id, source_type, source_id, sender_id, display_name, message_type, message_text, thread_id, parent_message_id, created_at
            FROM messages
            WHERE thread_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(thread_id)
        .fetch_all(pool)
        .await?;

        let messages = rows
            .into_iter()
            .map(|row| Message {
                id: row.id,
                message_id: row.message_id,
                source_type: SourceType::from_str(&row.source_type).unwrap(),
                source_id: row.source_id,
                sender_id: row.sender_id,
                display_name: row.display_name,
                message_type: MessageType::from_str(&row.message_type),
                message_text: row.message_text,
                thread_id: row.thread_id,
                parent_message_id: row.parent_message_id,
                created_at: row.created_at,
            })
            .collect();

        Ok(messages)
    }

    pub async fn get_recent_threads(
        pool: &PgPool,
        source_type: SourceType,
        source_id: &str,
        limit: i32,
    ) -> Result<Vec<Message>> {
        #[derive(sqlx::FromRow)]
        struct MessageRow {
            id: i32,
            message_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            display_name: Option<String>,
            message_type: String,
            message_text: Option<String>,
            thread_id: Option<String>,
            parent_message_id: Option<String>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<MessageRow> = sqlx::query_as(
            r#"
            SELECT DISTINCT ON (thread_id) id, message_id, source_type, source_id, sender_id, display_name, message_type, message_text, thread_id, parent_message_id, created_at
            FROM messages
            WHERE source_type = $1 AND source_id = $2 AND thread_id IS NOT NULL
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let messages = rows
            .into_iter()
            .map(|row| Message {
                id: row.id,
                message_id: row.message_id,
                source_type: SourceType::from_str(&row.source_type).unwrap(),
                source_id: row.source_id,
                sender_id: row.sender_id,
                display_name: row.display_name,
                message_type: MessageType::from_str(&row.message_type),
                message_text: row.message_text,
                thread_id: row.thread_id,
                parent_message_id: row.parent_message_id,
                created_at: row.created_at,
            })
            .collect();

        Ok(messages)
    }

    pub fn format_thread_conversation(messages: &[Message]) -> String {
        let mut conversation = String::new();

        // Group messages by thread hierarchy
        let mut root_messages: Vec<&Message> = Vec::new();
        let mut replies: std::collections::HashMap<String, Vec<&Message>> =
            std::collections::HashMap::new();

        for msg in messages {
            if let Some(parent_id) = &msg.parent_message_id {
                replies
                    .entry(parent_id.clone())
                    .or_insert_with(Vec::new)
                    .push(msg);
            } else {
                root_messages.push(msg);
            }
        }

        // Format messages with reply hierarchy
        fn format_message_with_replies(
            msg: &Message,
            replies: &std::collections::HashMap<String, Vec<&Message>>,
            indent: usize,
            conversation: &mut String,
        ) {
            let prefix = "  ".repeat(indent);
            if let Some(formatted) = msg.format_for_summary() {
                conversation.push_str(&format!("{}{}\n", prefix, formatted.trim()));
            }

            // Format replies with increased indentation
            if let Some(message_replies) = replies.get(&msg.message_id) {
                for reply in message_replies {
                    format_message_with_replies(reply, replies, indent + 1, conversation);
                }
            }
        }

        conversation.push_str("🧵 Thread Conversation:\n");
        for root in &root_messages {
            format_message_with_replies(root, &replies, 0, &mut conversation);
            conversation.push('\n');
        }

        conversation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDateTime, Utc};
    use sqlx::PgPool;

    fn sample_message(
        id: i32,
        message_id: &str,
        text: Option<&str>,
        parent_message_id: Option<&str>,
    ) -> Message {
        Message {
            id,
            message_id: message_id.to_string(),
            source_type: SourceType::User,
            source_id: "U1".to_string(),
            sender_id: Some("U1".to_string()),
            display_name: Some("Alice".to_string()),
            message_type: MessageType::Text,
            message_text: text.map(|t| t.to_string()),
            thread_id: None,
            parent_message_id: parent_message_id.map(|p| p.to_string()),
            created_at: DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
                Utc,
            ),
        }
    }

    #[test]
    fn source_type_from_str_and_as_str_round_trip() {
        let ty = SourceType::from_str("group").unwrap();
        assert_eq!(ty.as_str(), "group");
    }

    #[test]
    fn message_type_from_str_and_as_str_round_trip() {
        let ty = MessageType::from_str("image");
        assert_eq!(ty.as_str(), "image");
    }

    #[test]
    fn format_for_summary_skips_non_text() {
        let mut message = sample_message(1, "m1", Some("hi"), None);
        message.message_type = MessageType::Image;
        assert!(message.format_for_summary().is_none());
    }

    #[test]
    fn format_for_summary_skips_empty_text() {
        let message = sample_message(1, "m1", Some(""), None);
        assert!(message.format_for_summary().is_none());
    }

    #[test]
    fn format_for_summary_formats_text() {
        let message = sample_message(1, "m1", Some("hello"), None);
        let formatted = message.format_for_summary().unwrap();
        assert!(formatted.contains("Alice"));
        assert!(formatted.contains("hello"));
    }

    #[test]
    fn format_conversation_skips_non_text_entries() {
        let mut non_text = sample_message(1, "m1", Some("hi"), None);
        non_text.message_type = MessageType::Image;
        let text = sample_message(2, "m2", Some("hello"), None);
        let result = Message::format_conversation(&[non_text, text]);
        assert!(result.contains("hello"));
        assert!(!result.contains("hi"));
    }

    #[test]
    fn format_thread_conversation_formats_hierarchy() {
        let root = sample_message(1, "root", Some("root msg"), None);
        let reply = sample_message(2, "reply", Some("reply msg"), Some("root"));
        let result = Message::format_thread_conversation(&[root, reply]);
        assert!(result.contains("Thread Conversation"));
        assert!(result.contains("root msg"));
        assert!(result.contains("reply msg"));
        assert!(result.contains("  [Alice"));
    }

    async fn setup_pool() -> Option<PgPool> {
        crate::db::test_utils::try_setup_pool(&["DELETE FROM messages"]).await
    }

    #[tokio::test]
    async fn db_message_queries_work() {
        let _lock = crate::db::test_utils::lock_db();
        let Some(pool) = setup_pool().await else {
            eprintln!("Skipping db_message_queries_work: DATABASE_URL unavailable or database unreachable");
            return;
        };

        Message::save(
            &pool,
            "m1",
            SourceType::Group,
            "G1",
            Some("U1"),
            Some("Alice"),
            MessageType::Text,
            Some("Hello"),
            Some("thread1"),
            None,
        )
        .await
        .unwrap();

        Message::save(
            &pool,
            "m2",
            SourceType::Group,
            "G1",
            Some("U2"),
            Some("Bob"),
            MessageType::Text,
            Some("Reply"),
            Some("thread1"),
            Some("m1"),
        )
        .await
        .unwrap();

        let recent = Message::get_recent_messages(&pool, SourceType::Group, "G1", 10)
            .await
            .unwrap();
        assert_eq!(recent.len(), 2);

        let by_range = Message::get_messages_by_time_range(&pool, SourceType::Group, "G1", 60)
            .await
            .unwrap();
        assert_eq!(by_range.len(), 2);

        let thread = Message::get_thread_messages(&pool, "thread1")
            .await
            .unwrap();
        assert_eq!(thread.len(), 2);

        let threads = Message::get_recent_threads(&pool, SourceType::Group, "G1", 5)
            .await
            .unwrap();
        assert_eq!(threads.len(), 1);
    }
}
