use anyhow::Result;
use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ai::{get_title_prompt, AIService};
use crate::db::{Checklist, Reminder, SourceType};
use crate::line::LineClient;

#[derive(Debug, Clone)]
pub enum ReminderCommand {
    AddChecklist {
        title: Option<String>,
        tasks: Vec<String>,
        schedule: Option<ScheduleInfo>,
    },
    MarkDone {
        checklist_id: String,
        task_number: i32,
    },
    ShowChecklist {
        checklist_id: Option<String>,
    },
    DeleteChecklist {
        checklist_id: String,
    },
    NotifyComplete {
        checklist_id: Option<String>,
        new_time: Option<String>,
    },
    Help,
}

#[derive(Debug, Clone)]
pub struct ScheduleInfo {
    pub kind: ScheduleKind,
}

#[derive(Debug, Clone)]
pub enum ScheduleKind {
    RelativeMinutes(i64),
    AbsoluteTime(DateTime<Utc>),
}

impl ScheduleInfo {
    pub fn minutes(&self) -> Option<i64> {
        match &self.kind {
            ScheduleKind::RelativeMinutes(minutes) => Some(*minutes),
            ScheduleKind::AbsoluteTime(_) => None,
        }
    }

    pub fn is_absolute_time(&self) -> bool {
        matches!(self.kind, ScheduleKind::AbsoluteTime(_))
    }

    pub fn absolute_time(&self) -> Option<DateTime<Utc>> {
        match &self.kind {
            ScheduleKind::AbsoluteTime(dt) => Some(dt.clone()),
            ScheduleKind::RelativeMinutes(_) => None,
        }
    }
}

fn extract_time_from_text(text: &str) -> Option<String> {
    let lowercase = text.to_lowercase();

    if lowercase.contains("tomorrow") {
        let tomorrow = bangkok_now() + Duration::days(1);
        return Some(tomorrow.format("%Y-%m-%d %H:%M").to_string());
    }

    if lowercase.contains("tonight") {
        let tonight = bangkok_now() + Duration::hours(2);
        return Some(tonight.format("%Y-%m-%d %H:%M").to_string());
    }

    if let Some(time_match) = text.find(|c: char| c.is_ascii_digit()) {
        let time_str = &text[time_match..];
        if let Some(colon_pos) = time_str.find(':') {
            let time_part = &time_str[..colon_pos + 3];
            let date_part = &text[..time_match];
            if !date_part.is_empty() {
                return Some(format!("{} {}", date_part.trim(), time_part.trim()));
            }
        }
    }

    None
}

fn parse_absolute_bangkok_time(text: &str, now: DateTime<FixedOffset>) -> Option<DateTime<Utc>> {
    let text = text.trim();
    let text = text.strip_prefix("at ").unwrap_or(text).trim();

    let time_pattern = Regex::new(r"^(\d{1,2})[.:](\d{2})\b").unwrap();
    let caps = time_pattern.captures(text)?;
    let hour: u32 = caps.get(1)?.as_str().parse().ok()?;
    let minute: u32 = caps.get(2)?.as_str().parse().ok()?;

    if hour > 23 || minute > 59 {
        return None;
    }

    let local_now = now.with_timezone(&bangkok_offset());
    let local_date = local_now.date_naive();
    let candidate_naive = local_date.and_hms_opt(hour, minute, 0)?;
    let candidate = bangkok_offset()
        .from_local_datetime(&candidate_naive)
        .single()
        .expect("Bangkok time is a fixed offset");

    let candidate = if candidate <= local_now {
        let next_day = local_date + chrono::Duration::days(1);
        let next_candidate_naive = next_day.and_hms_opt(hour, minute, 0)?;
        bangkok_offset()
            .from_local_datetime(&next_candidate_naive)
            .single()
            .expect("Bangkok time is a fixed offset")
    } else {
        candidate
    };

    Some(candidate.with_timezone(&Utc))
}

fn bangkok_offset() -> FixedOffset {
    FixedOffset::east_opt(7 * 60 * 60).expect("valid UTC+7 offset")
}

fn bangkok_now() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&bangkok_offset())
}

fn parse_bangkok_datetime(datetime_str: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
        return Some(dt.with_timezone(&Utc));
    }

    let naive = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M"))
        .ok()?;

    Some(
        bangkok_offset()
            .from_local_datetime(&naive)
            .single()
            .expect("Bangkok time is a fixed offset")
            .with_timezone(&Utc),
    )
}

fn find_uncompleted_reminder<'a>(
    reminders: &'a [Reminder],
    task_number: i32,
) -> Option<&'a Reminder> {
    reminders
        .iter()
        .find(|r| r.task_number == task_number && !r.is_completed)
}

fn sort_recent_checklist_ids(mut reminders: Vec<Reminder>) -> Vec<String> {
    reminders.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    reminders
        .into_iter()
        .filter_map(|r| r.checklist_id)
        .collect()
}

fn extract_checklist_id(text: &str) -> Option<String> {
    let chars: Vec<char> = text
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if chars.len() >= 8 {
        Some(chars.iter().collect())
    } else {
        None
    }
}

fn extract_title_from_ai_output(output: &str) -> Option<String> {
    fn normalize_title_line(line: &str) -> &str {
        line.trim_start_matches(|c: char| c == '#' || c.is_whitespace())
            .trim_start_matches(|c: char| c == '-' || c == '*')
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
            .trim()
    }

    let mut saw_summary_heading = false;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let without_heading = trimmed.trim_start_matches('#').trim();
        let heading_text = without_heading.trim_end_matches([':', '：']).trim();
        if heading_text == "สรุปย่อ" {
            saw_summary_heading = true;
            continue;
        }

        if saw_summary_heading {
            let candidate = normalize_title_line(without_heading);

            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }

    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(normalize_title_line)
        .map(str::to_string)
}

pub struct ReminderHandler;

impl ReminderHandler {
    pub fn parse(text: &str) -> Option<ReminderCommand> {
        let text = text.trim();

        if text.eq_ignore_ascii_case("help") || text.eq_ignore_ascii_case("ช่วยเหลือ")
        {
            return Some(ReminderCommand::Help);
        }

        if text.eq_ignore_ascii_case("done")
            || text.eq_ignore_ascii_case("เสร็จแล้ว")
            || text.eq_ignore_ascii_case("x")
            || text.eq_ignore_ascii_case("list")
            || text.eq_ignore_ascii_case("รายการ")
        {
            return Some(ReminderCommand::ShowChecklist { checklist_id: None });
        }

        if text.to_lowercase().starts_with("done ")
            || text.to_lowercase().starts_with("เสร็จ ")
            || text.to_lowercase().starts_with("x ")
        {
            if let Some(cmd) = Self::parse_mark_done(text) {
                return Some(cmd);
            }
        }

        if text.eq_ignore_ascii_case("delete") || text.eq_ignore_ascii_case("ลบ") {
            return Some(ReminderCommand::ShowChecklist { checklist_id: None });
        }

        if text.to_lowercase().starts_with("delete ") || text.to_lowercase().starts_with("ลบ ")
        {
            if let Some(cmd) = Self::parse_delete(text) {
                return Some(cmd);
            }
        }

        if text.starts_with("!notify") || text.starts_with("/notify") {
            return Self::parse_notify_complete(text);
        }

        if text.starts_with("!task")
            || text.starts_with("/task")
            || text.starts_with("!remind")
            || text.starts_with("/remind")
        {
            return Self::parse_add_checklist(text);
        }

        None
    }

    fn parse_mark_done(text: &str) -> Option<ReminderCommand> {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let task_input = parts[1];

        if let Some((checklist_id, task_number)) = task_input.split_once('.') {
            let task_num: i32 = task_number.parse().ok()?;
            Some(ReminderCommand::MarkDone {
                checklist_id: checklist_id.to_string(),
                task_number: task_num,
            })
        } else if let Ok(task_number) = task_input.parse::<i32>() {
            Some(ReminderCommand::MarkDone {
                checklist_id: String::new(),
                task_number,
            })
        } else {
            Some(ReminderCommand::MarkDone {
                checklist_id: task_input.to_string(),
                task_number: 0,
            })
        }
    }

    fn parse_delete(text: &str) -> Option<ReminderCommand> {
        let parts: Vec<&str> = text.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return None;
        }

        Some(ReminderCommand::DeleteChecklist {
            checklist_id: parts[1].trim().to_string(),
        })
    }

    fn parse_notify_complete(text: &str) -> Option<ReminderCommand> {
        let remaining = if text.starts_with("!notify") {
            &text["!notify".len()..]
        } else if text.starts_with("/notify") {
            &text["/notify".len()..]
        } else {
            return None;
        };

        let remaining = remaining.trim();
        if remaining.is_empty() {
            return Some(ReminderCommand::NotifyComplete {
                checklist_id: None,
                new_time: None,
            });
        }

        if remaining.eq_ignore_ascii_case("complete")
            || remaining.eq_ignore_ascii_case("เสร็จ")
            || remaining.eq_ignore_ascii_case("done")
        {
            return Some(ReminderCommand::NotifyComplete {
                checklist_id: None,
                new_time: None,
            });
        }

        if let Some(new_time) = extract_time_from_text(remaining) {
            return Some(ReminderCommand::NotifyComplete {
                checklist_id: None,
                new_time: Some(new_time),
            });
        }

        if let Some(checklist_id) = extract_checklist_id(remaining) {
            return Some(ReminderCommand::NotifyComplete {
                checklist_id: Some(checklist_id),
                new_time: None,
            });
        }

        None
    }

    fn parse_add_checklist(text: &str) -> Option<ReminderCommand> {
        let remaining = if text.starts_with("!task") {
            &text["!task".len()..]
        } else if text.starts_with("/task") {
            &text["/task".len()..]
        } else if text.starts_with("!remind") {
            &text["!remind".len()..]
        } else if text.starts_with("/remind") {
            &text["/remind".len()..]
        } else {
            return None;
        };

        let remaining = remaining.trim();
        if remaining.is_empty() {
            return Some(ReminderCommand::Help);
        }

        let (schedule, task_content) = Self::parse_schedule(remaining);

        let lines: Vec<&str> = task_content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();

        if lines.is_empty() {
            return Some(ReminderCommand::Help);
        }

        let title = if lines[0].contains('.')
            && !lines[0].trim().starts_with(|c: char| c.is_ascii_digit())
        {
            let first_line = lines[0].trim();
            if let Some(idx) = first_line.find('.') {
                let title_part = first_line[..idx].trim();
                if !title_part.is_empty()
                    && title_part
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
                {
                    Some(title_part.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let mut tasks: Vec<String> = Vec::new();
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let task_text = if let Some(dot_pos) = trimmed.find(". ") {
                trimmed[dot_pos + 2..].trim().to_string()
            } else if let Some(dot_pos) = trimmed.find('.') {
                let after_dot = &trimmed[dot_pos + 1..];
                if after_dot
                    .chars()
                    .all(|c| c.is_whitespace() || !c.is_alphanumeric())
                {
                    trimmed[dot_pos + 1..].trim().to_string()
                } else {
                    trimmed.to_string()
                }
            } else {
                trimmed.to_string()
            };

            if !task_text.is_empty() {
                tasks.push(task_text);
            }
        }

        if tasks.is_empty() {
            return Some(ReminderCommand::Help);
        }

        Some(ReminderCommand::AddChecklist {
            title,
            tasks,
            schedule,
        })
    }

    fn parse_schedule(text: &str) -> (Option<ScheduleInfo>, &str) {
        let time_pattern =
            Regex::new(r"^(?:in\s+)?(\d+)\s*(m|min|mins|minutes|h|hr|hrs|hours|d|day|days)\b")
                .unwrap();
        let datetime_pattern = Regex::new(
            r"(?s)^(?:at\s+)?(?P<datetime>\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2})(?:\s+(?P<rest>.*))?$",
        )
        .unwrap();

        if let Some(caps) = time_pattern.captures(text) {
            let value: i64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let unit = caps.get(2).unwrap().as_str();

            let minutes = match unit.chars().next() {
                Some('m') => value,
                Some('h') | Some('H') => value * 60,
                Some('d') | Some('D') => value * 60 * 24,
                _ => value,
            };

            let remaining_start = caps.get(0).unwrap().end();
            let remaining = text[remaining_start..].trim();
            (
                Some(ScheduleInfo {
                    kind: ScheduleKind::RelativeMinutes(minutes),
                }),
                remaining,
            )
        } else if let Some(caps) = datetime_pattern.captures(text) {
            let datetime_text = caps.name("datetime").unwrap().as_str();

            if let Some(notify_at) = parse_bangkok_datetime(datetime_text) {
                let remaining = caps.name("rest").map(|m| m.as_str().trim()).unwrap_or("");

                (
                    Some(ScheduleInfo {
                        kind: ScheduleKind::AbsoluteTime(notify_at),
                    }),
                    remaining,
                )
            } else {
                (None, text)
            }
        } else if text.to_lowercase().starts_with("at ")
            || Regex::new(r"^\d{1,2}[.:]\d{2}\b").unwrap().is_match(text)
        {
            if let Some(notify_at) = parse_absolute_bangkok_time(text, bangkok_now()) {
                let remaining = text
                    .strip_prefix("at ")
                    .unwrap_or(text)
                    .split_once(|c: char| c.is_whitespace())
                    .map(|(_, rest)| rest.trim())
                    .unwrap_or("");

                (
                    Some(ScheduleInfo {
                        kind: ScheduleKind::AbsoluteTime(notify_at),
                    }),
                    remaining,
                )
            } else {
                (None, text)
            }
        } else {
            (None, text)
        }
    }

    async fn send_channel_message(
        line_client: &LineClient,
        reply_token: Option<&str>,
        source_id: &str,
        message: &str,
    ) -> Result<()> {
        if let Some(token) = reply_token {
            line_client.reply_message(token, message).await?;
        } else {
            line_client.push_message(source_id, message).await?;
        }

        Ok(())
    }

    pub async fn execute(
        &self,
        command: &ReminderCommand,
        pool: &PgPool,
        line_client: &LineClient,
        ai_service: &dyn AIService,
        source_type: SourceType,
        source_id: &str,
        sender_id: Option<&str>,
        reply_token: Option<&str>,
    ) -> Result<String> {
        match command {
            ReminderCommand::AddChecklist {
                title,
                tasks,
                schedule,
            } => {
                let final_title = if title.is_none() {
                    let generated_title = if !tasks.is_empty() {
                        let prompt = get_title_prompt(tasks);
                        ai_service
                            .generate_summary(&[crate::db::Message {
                                id: 0,
                                message_id: "title_gen".to_string(),
                                source_type,
                                source_id: source_id.to_string(),
                                sender_id: None,
                                display_name: None,
                                message_type: crate::db::MessageType::Text,
                                message_text: Some(prompt),
                                thread_id: None,
                                parent_message_id: None,
                                created_at: Utc::now(),
                            }])
                            .await
                            .ok()
                            .and_then(|t| extract_title_from_ai_output(&t))
                            .filter(|t| !t.is_empty())
                    } else {
                        None
                    };

                    Some(
                        generated_title
                            .unwrap_or_else(|| Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()),
                    )
                } else {
                    title.clone()
                };

                self.add_checklist(
                    pool,
                    line_client,
                    source_type,
                    source_id,
                    sender_id,
                    &final_title,
                    tasks,
                    schedule,
                    reply_token,
                )
                .await
            }
            ReminderCommand::MarkDone {
                checklist_id,
                task_number,
            } => {
                self.mark_done(
                    pool,
                    line_client,
                    source_type,
                    source_id,
                    checklist_id,
                    *task_number,
                    reply_token,
                )
                .await
            }
            ReminderCommand::ShowChecklist { checklist_id } => {
                self.show_checklist(
                    pool,
                    line_client,
                    source_type,
                    source_id,
                    checklist_id.as_deref(),
                    reply_token,
                )
                .await
            }
            ReminderCommand::DeleteChecklist { checklist_id } => {
                self.delete_checklist(pool, line_client, source_id, checklist_id, reply_token)
                    .await
            }
            ReminderCommand::Help => Ok(Self::get_help_message()),
            ReminderCommand::NotifyComplete {
                checklist_id,
                new_time,
            } => {
                self.notify_complete(
                    pool,
                    line_client,
                    source_type,
                    source_id,
                    checklist_id.as_deref(),
                    new_time.as_deref(),
                    reply_token,
                )
                .await
            }
        }
    }

    async fn notify_complete(
        &self,
        pool: &PgPool,
        line_client: &LineClient,
        source_type: SourceType,
        source_id: &str,
        checklist_id: Option<&str>,
        new_time: Option<&str>,
        reply_token: Option<&str>,
    ) -> Result<String> {
        let reminders = if let Some(checklist_id) = checklist_id {
            Reminder::get_by_checklist(pool, checklist_id).await?
        } else {
            Reminder::get_recent_by_source(pool, source_type, source_id, 5).await?
        };

        if reminders.is_empty() {
            let response = "❌ ไม่พบรายการที่ต้องทำครับ";
            Self::send_channel_message(line_client, reply_token, source_id, response).await?;
            return Ok(response.to_string());
        }

        let target_checklist_id = checklist_id
            .map(str::to_string)
            .unwrap_or_else(|| reminders[0].checklist_id.clone().unwrap_or_default());

        let reminders = Reminder::get_by_checklist(pool, &target_checklist_id).await?;
        if reminders.is_empty() {
            let response = "❌ ไม่พบรายการที่ต้องทำครับ";
            Self::send_channel_message(line_client, reply_token, source_id, response).await?;
            return Ok(response.to_string());
        }

        let response = if let Some(time_str) = new_time {
            if let Some(notified_reminder) = reminders.iter().find(|r| !r.is_completed) {
                let notify_at = parse_bangkok_datetime(time_str);

                if let Some(notify_at) = notify_at {
                    Reminder::update_notify_time(
                        pool,
                        &notified_reminder.reminder_id,
                        Some(notify_at),
                    )
                    .await?;

                    let updated_reminders =
                        Reminder::get_by_checklist(pool, &target_checklist_id).await?;
                    let done_count = updated_reminders.iter().filter(|r| r.is_completed).count();
                    let total_count = updated_reminders.len();
                    format!(
                        "✅ รายการถูกอัปเดตแล้ว!\n\n⏰ เวลาเตือนใหม่: {}\n({}/{} เสร็จแล้ว)",
                        notify_at
                            .with_timezone(&bangkok_offset())
                            .format("%Y-%m-%d %H:%M"),
                        done_count,
                        total_count
                    )
                } else {
                    "⚠️ ไม่สามารถอัปเดตเวลาได้ กรุณาลองใหม่อีกครั้ง".to_string()
                }
            } else {
                "⚠️ ไม่พบรายการที่ยังไม่เสร็จสำหรับอัปเดตเวลา".to_string()
            }
        } else if let Some(notified_reminder) = reminders.iter().find(|r| !r.is_completed) {
            Reminder::mark_completed(pool, &notified_reminder.reminder_id).await?;

            let updated_reminders = Reminder::get_by_checklist(pool, &target_checklist_id).await?;
            if updated_reminders.iter().all(|r| r.is_completed) {
                Checklist::update_schedule_enabled(pool, &target_checklist_id, false).await?;
                all_done_follow_up_prompt().to_string()
            } else {
                let done_count = updated_reminders.iter().filter(|r| r.is_completed).count();
                let total_count = updated_reminders.len();
                format!(
                    "✅ รายการถูกอัปเดตแล้ว!\n\n({}/{} เสร็จแล้ว)",
                    done_count, total_count
                )
            }
        } else {
            "⚠️ ไม่สามารถอัปเดตเวลาได้ กรุณาลองใหม่อีกครั้ง".to_string()
        };

        Self::send_channel_message(line_client, reply_token, source_id, &response).await?;
        Ok(response)
    }

    async fn add_checklist(
        &self,
        pool: &PgPool,
        line_client: &LineClient,
        source_type: SourceType,
        source_id: &str,
        sender_id: Option<&str>,
        title: &Option<String>,
        tasks: &[String],
        schedule: &Option<ScheduleInfo>,
        reply_token: Option<&str>,
    ) -> Result<String> {
        let checklist_id = Uuid::new_v4().to_string();

        let group_name = if matches!(source_type, SourceType::Group | SourceType::Room) {
            line_client
                .get_group_summary(source_id)
                .await
                .ok()
                .map(|summary| summary.group_name)
        } else {
            None
        };

        Checklist::save(
            pool,
            &checklist_id,
            source_type,
            source_id,
            sender_id,
            title.as_deref(),
            group_name.as_deref(),
        )
        .await?;

        let notify_at = schedule.as_ref().map(|s| match &s.kind {
            ScheduleKind::RelativeMinutes(minutes) => Utc::now() + Duration::minutes(*minutes),
            ScheduleKind::AbsoluteTime(dt) => dt.clone(),
        });

        for (idx, task_text) in tasks.iter().enumerate() {
            let task_number = (idx + 1) as i32;
            let reminder_id = format!("{}_{}", checklist_id, task_number);

            Reminder::save(
                pool,
                &reminder_id,
                source_type,
                source_id,
                sender_id,
                Some(&checklist_id),
                task_number,
                task_text,
                notify_at,
            )
            .await?;
        }

        let reminders = Reminder::get_by_checklist(pool, &checklist_id).await?;
        let checklist_text = Reminder::format_checklist(&reminders, &checklist_id);

        let schedule_msg = if let Some(s) = schedule {
            match &s.kind {
                ScheduleKind::RelativeMinutes(minutes) => {
                    let time_str = if *minutes >= 60 * 24 {
                        format!("{} วัน", minutes / (60 * 24))
                    } else if *minutes >= 60 {
                        format!("{} ชั่วโมง", minutes / 60)
                    } else {
                        format!("{} นาที", minutes)
                    };
                    format!("\n\n⏰ จะส่งเตือนในอีก {}", time_str)
                }
                ScheduleKind::AbsoluteTime(dt) => format!(
                    "\n\n⏰ จะส่งเตือนเวลา {}",
                    dt.with_timezone(&bangkok_offset()).format("%Y-%m-%d %H:%M")
                ),
            }
        } else {
            String::new()
        };

        let response = format!(
            "✅ สร้างรายการสำเร็จ!\n\n{}{}\n\n💡 พิมพ์ `done 1` หรือ `เสร็จ 1` เพื่อทำเครื่องหมายว่าเสร็จแล้ว{}",
            checklist_text,
            if let Some(t) = title {
                format!("📌 {}", t)
            } else {
                String::new()
            },
            schedule_msg
        );

        Self::send_channel_message(line_client, reply_token, source_id, &response).await?;
        Ok(response)
    }

    async fn mark_done(
        &self,
        pool: &PgPool,
        line_client: &LineClient,
        source_type: SourceType,
        source_id: &str,
        checklist_id: &str,
        task_number: i32,
        reply_token: Option<&str>,
    ) -> Result<String> {
        let reminders = if checklist_id.is_empty() {
            Reminder::get_recent_by_source(pool, source_type, source_id, 20).await?
        } else {
            Reminder::get_by_checklist(pool, checklist_id).await?
        };

        if reminders.is_empty() {
            let response = "❌ ไม่พบรายการที่ต้องทำครับ";
            Self::send_channel_message(line_client, reply_token, source_id, response).await?;
            return Ok(response.to_string());
        }

        let actual_task_number = if checklist_id.is_empty() && task_number == 0 {
            1
        } else {
            task_number
        };

        let mut candidate_checklists = if checklist_id.is_empty() {
            sort_recent_checklist_ids(reminders.clone())
        } else {
            vec![checklist_id.to_string()]
        };

        if candidate_checklists.is_empty() {
            let response = "❌ ไม่พบรายการที่ต้องทำครับ";
            Self::send_channel_message(line_client, reply_token, source_id, response).await?;
            return Ok(response.to_string());
        }

        let mut target_checklist_id = String::new();
        let mut reminder = None;

        for candidate_checklist_id in candidate_checklists.drain(..) {
            let checklist_reminders =
                Reminder::get_by_checklist(pool, &candidate_checklist_id).await?;
            if let Some(found) = find_uncompleted_reminder(&checklist_reminders, actual_task_number)
            {
                target_checklist_id = candidate_checklist_id;
                reminder = Some(found.clone());
                break;
            }
        }

        let response = if let Some(rem) = reminder {
            Reminder::mark_completed(pool, &rem.reminder_id).await?;

            let updated_reminders = Reminder::get_by_checklist(pool, &target_checklist_id).await?;
            let checklist_text =
                Reminder::format_checklist(&updated_reminders, &target_checklist_id);

            let done_count = updated_reminders.iter().filter(|r| r.is_completed).count();
            let total_count = updated_reminders.len();
            let all_done = updated_reminders.iter().all(|r| r.is_completed);

            if all_done {
                Checklist::update_schedule_enabled(pool, &target_checklist_id, false).await?;
                format!(
                    "✅ ทำเครื่องหมาย '{}' เสร็จแล้ว!\n\n({}/{} เสร็จแล้ว)\n\n{}\n\n{}",
                    rem.task_text,
                    done_count,
                    total_count,
                    checklist_text,
                    all_done_follow_up_prompt()
                )
            } else {
                format!(
                    "✅ ทำเครื่องหมาย '{}' เสร็จแล้ว!\n\n({}/{} เสร็จแล้ว)\n\n{}",
                    rem.task_text, done_count, total_count, checklist_text
                )
            }
        } else {
            let reminder_text = if task_number == 0 {
                "รายการนี้".to_string()
            } else {
                format!("รายการที่ {}", actual_task_number)
            };
            format!("⚠️ {} ทำเครื่องหมายไว้แล้วหรือไม่พบรายการครับ", reminder_text)
        };

        Self::send_channel_message(line_client, reply_token, source_id, &response).await?;
        Ok(response)
    }

    async fn show_checklist(
        &self,
        pool: &PgPool,
        line_client: &LineClient,
        source_type: SourceType,
        source_id: &str,
        checklist_id: Option<&str>,
        reply_token: Option<&str>,
    ) -> Result<String> {
        let reminders = if let Some(id) = checklist_id {
            Reminder::get_by_checklist(pool, id).await?
        } else {
            Reminder::get_recent_by_source(pool, source_type, source_id, 10).await?
        };

        if reminders.is_empty() {
            let response = "📋 ไม่มีรายการที่ต้องทำครับ\n\nพิมพ์ `!task` เพื่อสร้างรายการใหม่";
            Self::send_channel_message(line_client, reply_token, source_id, response).await?;
            return Ok(response.to_string());
        }

        if reminders.len() == 1 && reminders[0].checklist_id.is_some() {
            let checklist = reminders[0].checklist_id.clone().unwrap();
            let all_reminders = Reminder::get_by_checklist(pool, &checklist).await?;
            let checklist_text = Reminder::format_checklist(&all_reminders, &checklist);

            let done_count = all_reminders.iter().filter(|r| r.is_completed).count();
            let total_count = all_reminders.len();

            let response = format!(
                "{}\n\n({}/{} เสร็จแล้ว)",
                checklist_text, done_count, total_count
            );

            Self::send_channel_message(line_client, reply_token, source_id, &response).await?;
            return Ok(response);
        }

        let mut current_checklist: Option<String> = None;
        let mut output = String::from("📋 รายการที่ต้องทำ:\n\n");

        for rem in &reminders {
            if current_checklist != rem.checklist_id {
                current_checklist = rem.checklist_id.clone();
                output.push_str("---\n");
            }

            let checkbox = if rem.is_completed { "x" } else { " " };
            output.push_str(&format!(
                "[{}] {}. {}",
                checkbox, rem.task_number, rem.task_text
            ));
            if rem.is_completed {
                output.push_str(" ✓");
            }
            output.push('\n');
        }

        Self::send_channel_message(line_client, reply_token, source_id, &output).await?;
        Ok(output)
    }

    async fn delete_checklist(
        &self,
        pool: &PgPool,
        line_client: &LineClient,
        source_id: &str,
        checklist_id: &str,
        reply_token: Option<&str>,
    ) -> Result<String> {
        let deleted = Reminder::delete_checklist(pool, checklist_id).await?;
        Checklist::delete(pool, checklist_id).await?;

        let response = format!("🗑️ ลบรายการสำเร็จ ({} รายการ)", deleted);
        Self::send_channel_message(line_client, reply_token, source_id, &response).await?;
        Ok(response)
    }

    pub fn get_help_message() -> String {
        String::from(
            r#"📝 คำสั่ง Reminder & Checklist

➕ สร้างรายการ:
!task หรือ /task
1. ซื้ออาหารสุนัข
2. จ่ายบิลมือถือ
3. ซื้อของชำ

⏰ ตั้งเวลาเตือน:
!task in 30m
1. ทำรายงาน
2. โทรหาลูกค้า

!task at 16:30
1. เตรียมประชุม
2. ส่งสรุป

!task 2026-03-31 16:30
1. เตรียมประชุม
2. ส่งสรุป

✅ ทำเครื่องหมายว่าเสร็จแล้ว:
done 1 หรือ เสร็จ 1 หรือ x 1

📋 ดูรายการ:
list หรือ รายการ

🗑️ ลบรายการ:
delete <checklist_id>
"#,
        )
    }
}

fn all_done_follow_up_prompt() -> &'static str {
    "✅ All reminders are completed.\nSend `!notify 30m` to set a new reminder time."
}

pub fn is_done_keyword(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    lower == "done"
        || lower == "เสร็จแล้ว"
        || lower == "เสร็จ"
        || lower == "x"
        || lower == "finished"
        || lower == "complete"
}

pub fn parse_done_command(text: &str) -> Option<(String, i32)> {
    let lower = text.trim().to_lowercase();

    if !is_done_keyword(&lower)
        && !lower.starts_with("done ")
        && !lower.starts_with("เสร็จ ")
        && !lower.starts_with("เสร็จแล้ว ")
        && !lower.starts_with("x ")
    {
        return None;
    }

    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let task_input = parts[1];

    if let Ok(task_num) = task_input.parse::<i32>() {
        return Some((String::new(), task_num));
    }

    if let Some((_, task_num_str)) = task_input.split_once('.') {
        if let Ok(task_num) = task_num_str.parse::<i32>() {
            return Some((String::new(), task_num));
        }
    }

    if task_input.len() > 2 {
        if let Some((checklist_id, task_num_str)) = task_input.split_once('_') {
            if let Ok(task_num) = task_num_str.parse::<i32>() {
                return Some((checklist_id.to_string(), task_num));
            }
        }
    }

    Some((String::new(), 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_add_checklist() {
        let cmd = ReminderHandler::parse("!task\n1. Buy pet food\n2. Bill Monthly Mobile internet");
        assert!(matches!(cmd, Some(ReminderCommand::AddChecklist { .. })));
    }

    #[test]
    fn test_parse_add_checklist_with_schedule() {
        let cmd = ReminderHandler::parse(
            "!task in 30m\n1. Buy pet food\n2. Bill Monthly Mobile internet",
        );
        match cmd {
            Some(ReminderCommand::AddChecklist { schedule, .. }) => {
                assert!(schedule.is_some());
                assert_eq!(schedule.unwrap().minutes().unwrap(), 30);
            }
            _ => panic!("Expected AddChecklist command"),
        }
    }

    #[test]
    fn test_parse_add_checklist_with_absolute_time_schedule() {
        let cmd = ReminderHandler::parse(
            "!task at 16.30\n1. Buy pet food\n2. Bill Monthly Mobile internet",
        );
        match cmd {
            Some(ReminderCommand::AddChecklist { schedule, .. }) => {
                let schedule = schedule.expect("expected schedule");
                assert!(schedule.is_absolute_time());
                assert_eq!(
                    schedule
                        .absolute_time()
                        .unwrap()
                        .with_timezone(&bangkok_offset())
                        .format("%H:%M")
                        .to_string(),
                    "16:30"
                );
            }
            _ => panic!("Expected AddChecklist command"),
        }
    }

    #[test]
    fn test_parse_add_checklist_with_absolute_datetime_schedule() {
        let cmd = ReminderHandler::parse(
            "!task 2026-03-31 16:30\n1. Buy pet food\n2. Bill Monthly Mobile internet",
        );
        match cmd {
            Some(ReminderCommand::AddChecklist { schedule, .. }) => {
                let schedule = schedule.expect("expected schedule");
                assert!(schedule.is_absolute_time());
                assert_eq!(
                    schedule
                        .absolute_time()
                        .unwrap()
                        .with_timezone(&bangkok_offset())
                        .format("%Y-%m-%d %H:%M")
                        .to_string(),
                    "2026-03-31 16:30"
                );
            }
            _ => panic!("Expected AddChecklist command"),
        }
    }

    #[test]
    fn test_extract_title_from_summary_section_only_uses_first_summary_line() {
        let output = r#"
# สรุปย่อ
- ซื้ออาหารสุนัข
- จ่ายบิลมือถือ

## รายละเอียด
something else
"#;

        assert_eq!(
            extract_title_from_ai_output(output).as_deref(),
            Some("ซื้ออาหารสุนัข")
        );
    }

    #[test]
    fn test_extract_title_from_summary_section_accepts_colon_heading() {
        let output = r#"
## สรุปย่อ:
1. เตรียมประชุมทีม
2. ส่งสรุป
"#;

        assert_eq!(
            extract_title_from_ai_output(output).as_deref(),
            Some("เตรียมประชุมทีม")
        );
    }

    #[test]
    fn test_parse_absolute_time_schedule_rolls_over_to_tomorrow_when_time_has_passed() {
        let now = bangkok_offset()
            .with_ymd_and_hms(2026, 3, 31, 17, 0, 0)
            .single()
            .unwrap();

        let scheduled = parse_absolute_bangkok_time("16.30", now).expect("expected scheduled time");

        assert_eq!(
            scheduled
                .with_timezone(&bangkok_offset())
                .format("%Y-%m-%d %H:%M")
                .to_string(),
            "2026-04-01 16:30"
        );
    }

    #[test]
    fn test_parse_done_command() {
        let (checklist_id, task_num) = parse_done_command("done 1").unwrap();
        assert!(checklist_id.is_empty());
        assert_eq!(task_num, 1);
    }

    #[test]
    fn test_parse_done_command_with_checklist_id() {
        let (checklist_id, task_num) = parse_done_command("done abc_2").unwrap();
        assert_eq!(checklist_id, "abc");
        assert_eq!(task_num, 2);
    }

    #[test]
    fn test_parse_done_command_with_dot_syntax() {
        let (checklist_id, task_num) = parse_done_command("done list.3").unwrap();
        assert!(checklist_id.is_empty());
        assert_eq!(task_num, 3);
    }

    #[test]
    fn test_parse_thai_done() {
        assert!(is_done_keyword("เสร็จแล้ว"));
        assert!(is_done_keyword("เสร็จ"));
    }

    #[test]
    fn test_is_done_keyword() {
        assert!(is_done_keyword("done"));
        assert!(is_done_keyword("x"));
        assert!(is_done_keyword("เสร็จแล้ว"));
    }

    #[test]
    fn test_all_done_prompt_mentions_new_alert_time() {
        let prompt = all_done_follow_up_prompt();
        assert!(prompt.contains("set a new reminder time"));
        assert!(prompt.contains("!notify"));
    }

    #[test]
    fn test_parse_bangkok_datetime_treats_plain_datetime_as_local_time() {
        let dt = parse_bangkok_datetime("2026-03-27 10:02").unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-03-27 03:02");
    }

    #[test]
    fn test_find_uncompleted_reminder_scans_entire_checklist() {
        let reminders = vec![
            Reminder {
                id: 1,
                reminder_id: "check_1".to_string(),
                source_type: SourceType::User,
                source_id: "U123".to_string(),
                sender_id: None,
                checklist_id: Some("check".to_string()),
                task_number: 1,
                task_text: "First task".to_string(),
                is_completed: true,
                notify_at: None,
                last_notified_at: None,
                completed_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Reminder {
                id: 2,
                reminder_id: "check_2".to_string(),
                source_type: SourceType::User,
                source_id: "U123".to_string(),
                sender_id: None,
                checklist_id: Some("check".to_string()),
                task_number: 2,
                task_text: "Second task".to_string(),
                is_completed: false,
                notify_at: None,
                last_notified_at: None,
                completed_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let reminder = find_uncompleted_reminder(&reminders, 2).unwrap();
        assert_eq!(reminder.task_text, "Second task");
    }

    #[test]
    fn test_sort_recent_checklist_ids_prefers_newest_checklist() {
        let reminders = vec![
            Reminder {
                id: 1,
                reminder_id: "old_1".to_string(),
                source_type: SourceType::User,
                source_id: "U123".to_string(),
                sender_id: None,
                checklist_id: Some("old".to_string()),
                task_number: 1,
                task_text: "Old".to_string(),
                is_completed: false,
                notify_at: None,
                last_notified_at: None,
                completed_at: None,
                created_at: Utc::now() - chrono::Duration::minutes(10),
                updated_at: Utc::now() - chrono::Duration::minutes(10),
            },
            Reminder {
                id: 2,
                reminder_id: "new_1".to_string(),
                source_type: SourceType::User,
                source_id: "U123".to_string(),
                sender_id: None,
                checklist_id: Some("new".to_string()),
                task_number: 1,
                task_text: "New".to_string(),
                is_completed: false,
                notify_at: None,
                last_notified_at: None,
                completed_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let ids = sort_recent_checklist_ids(reminders);
        assert_eq!(ids, vec!["new".to_string(), "old".to_string()]);
    }

    #[test]
    fn test_parse_notify_complete_command() {
        let cmd = ReminderHandler::parse("!notify checklist_1");
        assert!(matches!(
            cmd,
            Some(ReminderCommand::NotifyComplete { checklist_id: Some(ref id), .. }) if id == "checklist_1"
        ));
    }

    #[test]
    fn test_parse_delete_command() {
        let cmd = ReminderHandler::parse("delete abc");
        assert!(matches!(
            cmd,
            Some(ReminderCommand::DeleteChecklist { checklist_id }) if checklist_id == "abc"
        ));
    }

    #[test]
    fn test_parse_show_list_command() {
        let cmd = ReminderHandler::parse("list");
        assert!(matches!(
            cmd,
            Some(ReminderCommand::ShowChecklist { checklist_id: None })
        ));
    }
}
