use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use crate::db::Reminder;
use crate::line::LineClient;

pub struct ReminderNotifier {
    scheduler: JobScheduler,
    pool: Arc<PgPool>,
    line_client: Arc<LineClient>,
}

impl ReminderNotifier {
    pub async fn new(pool: Arc<PgPool>, line_client: Arc<LineClient>) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;

        Ok(Self {
            scheduler,
            pool,
            line_client,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        let pool = Arc::clone(&self.pool);
        let line_client = Arc::clone(&self.line_client);

        let job = Job::new_async("0 * * * * *", move |_uuid, _l| {
            let pool = Arc::clone(&pool);
            let line_client = Arc::clone(&line_client);

            Box::pin(async move {
                if let Err(e) = send_pending_reminders(&pool, &line_client).await {
                    error!("Failed to send reminder notifications: {}", e);
                }
            })
        })?;

        self.scheduler.add(job).await?;
        self.scheduler.start().await?;
        info!("Reminder notifier started");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.scheduler.shutdown().await?;
        Ok(())
    }
}

async fn send_pending_reminders(pool: &PgPool, line_client: &LineClient) -> Result<()> {
    let now = bangkok_now();
    info!(
        "[{}] Cron job triggered - checking for pending reminders",
        now.format("%Y-%m-%d %H:%M:%S %:z")
    );

    let reminders = Reminder::get_pending_reminders(pool).await?;

    if reminders.is_empty() {
        info!("No pending reminders to send");
        return Ok(());
    }

    info!("Found {} pending reminders to send", reminders.len());

    let mut current_checklist: Option<String> = None;
    let mut checklist_reminders: Vec<&Reminder> = Vec::new();

    for reminder in &reminders {
        if current_checklist != reminder.checklist_id {
            if !checklist_reminders.is_empty() {
                if let Some(checklist_id) = &current_checklist {
                    if let Err(e) = send_checklist_reminder(
                        pool,
                        line_client,
                        checklist_id,
                        &checklist_reminders,
                    )
                    .await
                    {
                        error!("Failed to send checklist reminder: {}", e);
                    }
                }
            }
            current_checklist = reminder.checklist_id.clone();
            checklist_reminders.clear();
        }
        checklist_reminders.push(reminder);
    }

    if !checklist_reminders.is_empty() {
        if let Some(checklist_id) = &current_checklist {
            if let Err(e) =
                send_checklist_reminder(pool, line_client, checklist_id, &checklist_reminders).await
            {
                error!("Failed to send checklist reminder: {}", e);
            }
        }
    }

    Ok(())
}

fn bangkok_now() -> DateTime<FixedOffset> {
    let offset = FixedOffset::east_opt(7 * 60 * 60).expect("valid UTC+7 offset");
    Utc::now().with_timezone(&offset)
}

async fn send_checklist_reminder(
    pool: &PgPool,
    line_client: &LineClient,
    checklist_id: &str,
    reminders: &[&Reminder],
) -> Result<()> {
    if reminders.is_empty() {
        return Ok(());
    }

    let source_id = &reminders[0].source_id;

    let pending_count = reminders.iter().filter(|r| !r.is_completed).count();
    let total_count = reminders.len();

    let mut message = String::from("🔔 ถึงเวลาเตือนความจำ!\n\n");

    message.push_str("📋 รายการที่ยังไม่เสร็จ:\n");

    for reminder in reminders {
        if !reminder.is_completed {
            message.push_str(&format!(
                "[ ] {}. {}\n",
                reminder.task_number, reminder.task_text
            ));
        }
    }

    message.push_str(&format!(
        "\n({}/{} เสร็จแล้ว)",
        total_count - pending_count,
        total_count
    ));
    message.push_str("\n\n💡 พิมพ์ `done 1` เพื่อทำเครื่องหมายว่าเสร็จแล้ว");

    info!(
        "Sending reminder to {} for checklist {} (source_type: {:?})",
        source_id, checklist_id, reminders[0].source_type
    );
    line_client.push_message(source_id, &message).await?;
    info!(
        "Reminder sent successfully to {} for checklist {}",
        source_id, checklist_id
    );

    let notified = Reminder::mark_notified_by_checklist(pool, checklist_id).await?;
    info!(
        "Marked {} reminders as notified in checklist {}",
        notified, checklist_id
    );

    let follow_up = "⏰ Reminder sent.\nSend `!notify 30m` to set a new reminder time.";
    line_client.push_message(source_id, follow_up).await?;
    info!(
        "Sent follow-up prompt to {} for checklist {}",
        source_id, checklist_id
    );

    Ok(())
}

pub async fn send_checklist_summary(
    pool: &PgPool,
    line_client: &LineClient,
    source_type: crate::db::SourceType,
    source_id: &str,
) -> Result<()> {
    let reminders = Reminder::get_recent_by_source(pool, source_type, source_id, 10).await?;

    if reminders.is_empty() {
        return Ok(());
    }

    let mut message = String::from("📋 สรุปรายการที่ต้องทำ:\n\n");

    let mut current_checklist: Option<String> = None;
    for reminder in &reminders {
        if current_checklist != reminder.checklist_id {
            current_checklist = reminder.checklist_id.clone();
            message.push_str("---\n");
        }

        let checkbox = if reminder.is_completed { "x" } else { " " };
        let status = if reminder.is_completed { " ✓" } else { "" };

        message.push_str(&format!(
            "[{}] {}. {}{}\n",
            checkbox, reminder.task_number, reminder.task_text, status
        ));
    }

    line_client.push_message(source_id, &message).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Checklist;
    use sqlx::PgPool;

    #[test]
    fn bangkok_now_has_plus7_offset() {
        let now = bangkok_now();
        assert_eq!(now.offset().local_minus_utc(), 7 * 60 * 60);
    }

    async fn setup_pool() -> Option<PgPool> {
        crate::db::test_utils::try_setup_pool(&["DELETE FROM reminders", "DELETE FROM checklists"])
            .await
    }

    #[tokio::test]
    async fn send_checklist_summary_sends_message() {
        let _lock = crate::db::test_utils::lock_db();
        let Some(pool) = setup_pool().await else {
            eprintln!("Skipping send_checklist_summary_sends_message: DATABASE_URL unavailable or database unreachable");
            return;
        };
        let checklist_id = "c1";

        Checklist::save(
            &pool,
            checklist_id,
            crate::db::SourceType::Group,
            "G1",
            Some("U1"),
            Some("Title"),
            Some("Group"),
        )
        .await
        .unwrap();

        Reminder::save(
            &pool,
            "r1",
            crate::db::SourceType::Group,
            "G1",
            Some("U1"),
            Some(checklist_id),
            1,
            "Task one",
            None,
        )
        .await
        .unwrap();

        let client = LineClient::new("token".to_string());
        let result =
            send_checklist_summary(&pool, &client, crate::db::SourceType::Group, "G1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_pending_reminders_runs() {
        let _lock = crate::db::test_utils::lock_db();
        let Some(pool) = setup_pool().await else {
            eprintln!("Skipping send_pending_reminders_runs: DATABASE_URL unavailable or database unreachable");
            return;
        };
        let checklist_id = "c2";

        Checklist::save(
            &pool,
            checklist_id,
            crate::db::SourceType::Group,
            "G2",
            Some("U1"),
            Some("Title"),
            Some("Group"),
        )
        .await
        .unwrap();

        Reminder::save(
            &pool,
            "r2",
            crate::db::SourceType::Group,
            "G2",
            Some("U1"),
            Some(checklist_id),
            1,
            "Task one",
            Some(Utc::now()),
        )
        .await
        .unwrap();

        let client = LineClient::new("token".to_string());
        let result = send_pending_reminders(&pool, &client).await;
        assert!(result.is_ok());
    }
}
