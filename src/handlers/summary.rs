use anyhow::Result;
use sqlx::PgPool;

use crate::ai::AIService;
use crate::db::{Message, SourceType};
use crate::line::LineClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummaryCommandType {
    Summarize,
    SummarizeThai,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummaryParameter {
    MessageCount(i32),
    TimeRange(i32), // in minutes
    ThreadByTs(String),
    ThreadByUrl(String),
}

#[derive(Debug)]
pub struct SummaryCommand {
    pub command_type: SummaryCommandType,
    pub parameter: Option<SummaryParameter>,
}

impl SummaryCommand {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let (command_type, remaining) = if text.starts_with("!summarize") {
            (SummaryCommandType::Summarize, &text["!summarize".len()..])
        } else if text.starts_with("/สรุป") {
            (SummaryCommandType::SummarizeThai, &text["/สรุป".len()..])
        } else {
            return None;
        };

        let parameter = match parse_parameter(remaining) {
            Some(param) => Some(param),
            None => Some(SummaryParameter::MessageCount(50)), // Default: last 50 messages
        };

        Some(SummaryCommand {
            command_type,
            parameter,
        })
    }

    pub async fn execute(
        &self,
        pool: &PgPool,
        line_client: &LineClient,
        ai_service: &dyn AIService,
        source_type: SourceType,
        source_id: &str,
        reply_token: &str,
    ) -> Result<()> {
        let messages = match &self.parameter {
            Some(SummaryParameter::MessageCount(count)) => {
                Message::get_recent_messages(pool, source_type, source_id, *count).await?
            }
            Some(SummaryParameter::TimeRange(minutes)) => {
                Message::get_messages_by_time_range(pool, source_type, source_id, *minutes).await?
            }
            Some(SummaryParameter::ThreadByTs(thread_ts)) => {
                Message::get_thread_messages(pool, thread_ts).await?
            }
            Some(SummaryParameter::ThreadByUrl(url)) => {
                // Parse URL to get thread info, then get thread messages
                if let Ok(thread_info) = crate::slack::parse_thread_permalink(url) {
                    Message::get_thread_messages(pool, &thread_info.thread_ts).await?
                } else {
                    return Ok(());
                }
            }
            None => Message::get_recent_messages(pool, source_type, source_id, 50).await?,
        };

        if messages.is_empty() {
            let response = "ยังไม่มีข้อความให้สรุปครับ";
            line_client.reply_markdown(reply_token, response).await?;
            return Ok(());
        }

        let summary = ai_service.generate_summary(&messages).await?;
        line_client.reply_markdown(reply_token, &summary).await?;

        Ok(())
    }
}

fn parse_parameter(text: &str) -> Option<SummaryParameter> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // Check if it's a number followed by time unit (m, h, d)
    if let Some(pos) = text.find(|c| c == 'm' || c == 'h' || c == 'd') {
        let num_str = &text[..pos];
        let unit = text.chars().nth(pos)?;

        if let Ok(num) = num_str.trim().parse::<i32>() {
            return match unit {
                'm' => Some(SummaryParameter::TimeRange(num)),
                'h' => Some(SummaryParameter::TimeRange(num * 60)),
                'd' => Some(SummaryParameter::TimeRange(num * 60 * 24)),
                _ => None,
            };
        }
    }

    // Check if it's just a number (message count)
    if let Ok(count) = text.parse::<i32>() {
        return Some(SummaryParameter::MessageCount(count));
    }

    // Check if it's a thread timestamp format
    if text.len() > 16 && text.chars().nth(10) == Some('.') {
        if let Ok(timestamp) = text.parse::<String>() {
            return Some(SummaryParameter::ThreadByTs(timestamp));
        }
    }

    // Check if it's a Slack thread URL
    if text.contains("slack.com") {
        return Some(SummaryParameter::ThreadByUrl(text.to_string()));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_summarize() {
        let cmd = SummaryCommand::parse("!summarize");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().command_type, SummaryCommandType::Summarize);
    }

    #[test]
    fn test_parse_summarize_thai() {
        let cmd = SummaryCommand::parse("/สรุป");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().command_type, SummaryCommandType::SummarizeThai);
    }

    #[test]
    fn test_parse_summarize_with_count() {
        let cmd = SummaryCommand::parse("!summarize 100");
        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap().parameter,
            Some(SummaryParameter::MessageCount(100))
        ));
    }

    #[test]
    fn test_parse_summarize_with_time_range() {
        let cmd = SummaryCommand::parse("!summarize 2h");
        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap().parameter,
            Some(SummaryParameter::TimeRange(120))
        ));
    }

    #[test]
    fn test_parse_thai_with_time_range() {
        let cmd = SummaryCommand::parse("/สรุป 30m");
        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap().parameter,
            Some(SummaryParameter::TimeRange(30))
        ));
    }

    #[test]
    fn test_parse_invalid_command() {
        let cmd = SummaryCommand::parse("hello world");
        assert!(cmd.is_none());
    }

    #[test]
    fn test_parse_thread_ts() {
        let cmd = SummaryCommand::parse("!summarize 1234567890.123456");
        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap().parameter,
            Some(SummaryParameter::ThreadByTs(..))
        ));
    }

    #[test]
    fn test_parse_thread_url() {
        let cmd = SummaryCommand::parse(
            "!summarize https://workspace.slack.com/archives/C123/p123456/123456",
        );
        assert!(cmd.is_some());
        assert!(matches!(
            cmd.unwrap().parameter,
            Some(SummaryParameter::ThreadByUrl(..))
        ));
    }

    #[test]
    fn test_parse_default_message_count() {
        let cmd = SummaryCommand::parse("!summarize");
        assert!(matches!(
            cmd.unwrap().parameter,
            Some(SummaryParameter::MessageCount(50))
        ));
    }

    #[test]
    fn test_parse_time_range_days() {
        let cmd = SummaryCommand::parse("!summarize 2d");
        assert!(matches!(
            cmd.unwrap().parameter,
            Some(SummaryParameter::TimeRange(2880))
        ));
    }
}
