use anyhow::Result;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadInfo {
    pub channel_id: String,
    pub thread_ts: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlackCommand {
    SummaryThread {
        thread_ts: String,
    },
    SummaryByUrl {
        url: String,
    },
    SummaryChannel {
        count: Option<i32>,
        time_range: Option<i32>,
    },
}

lazy_static::lazy_static! {
    static ref SLACK_PERMALINK_REGEX: Regex = Regex::new(
        r"(?i)https://[^/]+\.slack\.com/archives/([^/]+)/p\d+/(\d+)(?:/thread/[^/]+/)?"
    ).expect("Invalid Slack permalink regex");

    static ref SLACK_WORKSPACE_PERMALINK_REGEX: Regex = Regex::new(
        r"(?i)https://[^/]+\.slack\.com/archives/([^/]+)/p\d+/(\d+)(?:/thread/[^/]+/)?"
    ).expect("Invalid Slack workspace permalink regex");

    static ref THREAD_TS_REGEX: Regex = Regex::new(
        r"(\d{10}\.\d{6})"
    ).expect("Invalid thread timestamp regex");
}

pub fn parse_thread_permalink(url: &str) -> Result<ThreadInfo> {
    // Try workspace format first
    if let Some(captures) = SLACK_WORKSPACE_PERMALINK_REGEX.captures(url) {
        let channel_id = captures
            .get(1)
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let thread_ts = captures
            .get(2)
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        if channel_id.is_empty() || thread_ts.is_empty() {
            anyhow::bail!("Invalid Slack permalink: could not extract channel and thread info");
        }

        return Ok(ThreadInfo {
            channel_id,
            thread_ts,
        });
    }

    // Try standard archives format
    if let Some(captures) = SLACK_PERMALINK_REGEX.captures(url) {
        let channel_id = captures
            .get(1)
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let thread_ts = captures
            .get(2)
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        if channel_id.is_empty() || thread_ts.is_empty() {
            anyhow::bail!("Invalid Slack permalink: could not extract channel and thread info");
        }

        return Ok(ThreadInfo {
            channel_id,
            thread_ts,
        });
    }

    anyhow::bail!("Invalid Slack permalink format: {}", url)
}

pub fn parse_slash_command(text: &str) -> Option<SlackCommand> {
    let text = text.trim();
    let lower_text = text.to_lowercase();

    // Parse /summary commands
    if lower_text.starts_with("/summary") || lower_text.starts_with("/summarize") {
        let mut parts = text.split_whitespace();
        let _command = parts.next();
        let remaining = parts.collect::<Vec<_>>().join(" ");
        let remaining = remaining.trim();

        // Check for thread timestamp
        if let Some(thread_ts_match) = THREAD_TS_REGEX.find(remaining) {
            return Some(SlackCommand::SummaryThread {
                thread_ts: thread_ts_match.as_str().to_string(),
            });
        }

        // Check for URL
        if let Ok(thread_info) = parse_thread_permalink(remaining) {
            return Some(SlackCommand::SummaryByUrl {
                url: remaining.to_string(),
            });
        }

        // Check for parameters (count or time range)
        if let Some(param) = parse_summary_parameter(remaining) {
            return Some(param);
        }

        // Default to recent messages
        return Some(SlackCommand::SummaryChannel {
            count: None,
            time_range: None,
        });
    }

    None
}

pub fn detect_thread_reply(text: &str) -> Option<ThreadInfo> {
    let text = text.trim();

    // Check for thread TS format
    if let Some(thread_ts_match) = THREAD_TS_REGEX.find(text) {
        return Some(ThreadInfo {
            channel_id: String::new(), // Will be filled by channel context
            thread_ts: thread_ts_match.as_str().to_string(),
        });
    }

    None
}

fn parse_summary_parameter(text: &str) -> Option<SlackCommand> {
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
                'm' => Some(SlackCommand::SummaryChannel {
                    count: None,
                    time_range: Some(num),
                }),
                'h' => Some(SlackCommand::SummaryChannel {
                    count: None,
                    time_range: Some(num * 60),
                }),
                'd' => Some(SlackCommand::SummaryChannel {
                    count: None,
                    time_range: Some(num * 60 * 24),
                }),
                _ => None,
            };
        }
    }

    // Check if it's just a number (message count)
    if let Ok(count) = text.parse::<i32>() {
        return Some(SlackCommand::SummaryChannel {
            count: Some(count),
            time_range: None,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_thread_permalink() {
        let url = "https://myworkspace.slack.com/archives/C123456789/p1234567890123456/1234567890";
        let result = parse_thread_permalink(url);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.channel_id, "C123456789");
        assert_eq!(info.thread_ts, "1234567890");
    }

    #[test]
    fn test_parse_slash_command_thread_ts() {
        let text = "/summary 1234567890.123456";
        let result = parse_slash_command(text);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            SlackCommand::SummaryThread { .. }
        ));
    }

    #[test]
    fn test_parse_slash_command_url() {
        let text = "/summary https://workspace.slack.com/archives/C123/p123456/123456";
        let result = parse_slash_command(text);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), SlackCommand::SummaryByUrl { .. }));
    }

    #[test]
    fn test_parse_slash_command_count() {
        let text = "/summary 100";
        let result = parse_slash_command(text);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            SlackCommand::SummaryChannel {
                count: Some(100),
                ..
            }
        ));
    }

    #[test]
    fn test_parse_slash_command_time_range() {
        let text = "/summary 2h";
        let result = parse_slash_command(text);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            SlackCommand::SummaryChannel {
                time_range: Some(120),
                ..
            }
        ));
    }

    #[test]
    fn test_detect_thread_reply() {
        let text = "Check out 1234567890.123456";
        let result = detect_thread_reply(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap().thread_ts, "1234567890.123456");
    }
}
