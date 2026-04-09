use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

use super::SourceType;

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: i32,
    pub reminder_id: String,
    pub source_type: SourceType,
    pub source_id: String,
    pub sender_id: Option<String>,
    pub checklist_id: Option<String>,
    pub task_number: i32,
    pub task_text: String,
    pub is_completed: bool,
    pub notify_at: Option<DateTime<Utc>>,
    pub last_notified_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Checklist {
    pub id: i32,
    pub checklist_id: String,
    pub source_type: SourceType,
    pub source_id: String,
    pub sender_id: Option<String>,
    pub title: Option<String>,
    pub group_name: Option<String>,
    pub schedule_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Reminder {
    pub async fn save(
        pool: &PgPool,
        reminder_id: &str,
        source_type: SourceType,
        source_id: &str,
        sender_id: Option<&str>,
        checklist_id: Option<&str>,
        task_number: i32,
        task_text: &str,
        notify_at: Option<DateTime<Utc>>,
    ) -> Result<i32> {
        let row = sqlx::query(
            r#"
            INSERT INTO reminders (reminder_id, source_type, source_id, sender_id, checklist_id, task_number, task_text, notify_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(reminder_id)
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(sender_id)
        .bind(checklist_id)
        .bind(task_number)
        .bind(task_text)
        .bind(notify_at)
        .fetch_one(pool)
        .await?;

        Ok(row
            .try_get("id")
            .context("Failed to get id from INSERT result")?)
    }

    pub async fn get_by_id(pool: &PgPool, reminder_id: &str) -> Result<Option<Reminder>> {
        #[derive(sqlx::FromRow)]
        struct ReminderRow {
            id: i32,
            reminder_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            checklist_id: Option<String>,
            task_number: i32,
            task_text: String,
            is_completed: bool,
            notify_at: Option<DateTime<Utc>>,
            last_notified_at: Option<DateTime<Utc>>,
            completed_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<ReminderRow> = sqlx::query_as(
            r#"
            SELECT id, reminder_id, source_type, source_id, sender_id, checklist_id,
                   task_number, task_text, is_completed, notify_at, last_notified_at, completed_at,
                   created_at, updated_at
            FROM reminders
            WHERE reminder_id = $1
            "#,
        )
        .bind(reminder_id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Reminder {
            id: r.id,
            reminder_id: r.reminder_id,
            source_type: SourceType::from_str(&r.source_type).unwrap(),
            source_id: r.source_id,
            sender_id: r.sender_id,
            checklist_id: r.checklist_id,
            task_number: r.task_number,
            task_text: r.task_text,
            is_completed: r.is_completed,
            notify_at: r.notify_at,
            last_notified_at: r.last_notified_at,
            completed_at: r.completed_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    pub async fn get_by_task_number(
        pool: &PgPool,
        checklist_id: &str,
        task_number: i32,
    ) -> Result<Option<Reminder>> {
        #[derive(sqlx::FromRow)]
        struct ReminderRow {
            id: i32,
            reminder_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            checklist_id: Option<String>,
            task_number: i32,
            task_text: String,
            is_completed: bool,
            notify_at: Option<DateTime<Utc>>,
            last_notified_at: Option<DateTime<Utc>>,
            completed_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<ReminderRow> = sqlx::query_as(
            r#"
            SELECT id, reminder_id, source_type, source_id, sender_id, checklist_id,
                   task_number, task_text, is_completed, notify_at, last_notified_at, completed_at,
                   created_at, updated_at
            FROM reminders
            WHERE checklist_id = $1 AND task_number = $2
            "#,
        )
        .bind(checklist_id)
        .bind(task_number)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Reminder {
            id: r.id,
            reminder_id: r.reminder_id,
            source_type: SourceType::from_str(&r.source_type).unwrap(),
            source_id: r.source_id,
            sender_id: r.sender_id,
            checklist_id: r.checklist_id,
            task_number: r.task_number,
            task_text: r.task_text,
            is_completed: r.is_completed,
            notify_at: r.notify_at,
            last_notified_at: r.last_notified_at,
            completed_at: r.completed_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    pub async fn mark_completed(pool: &PgPool, reminder_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE reminders
            SET is_completed = TRUE, completed_at = NOW(), updated_at = NOW()
            WHERE reminder_id = $1 AND is_completed = FALSE
            "#,
        )
        .bind(reminder_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn mark_notified_by_checklist(pool: &PgPool, checklist_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE reminders
            SET last_notified_at = NOW(), updated_at = NOW()
            WHERE checklist_id = $1 AND notify_at IS NOT NULL
            "#,
        )
        .bind(checklist_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn update_notify_time(
        pool: &PgPool,
        reminder_id: &str,
        notify_at: Option<DateTime<Utc>>,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE reminders
            SET notify_at = $1, last_notified_at = NULL, updated_at = NOW()
            WHERE reminder_id = $2
            "#,
        )
        .bind(notify_at)
        .bind(reminder_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn clear_notify_time_by_checklist(pool: &PgPool, checklist_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE reminders
            SET notify_at = NULL, updated_at = NOW()
            WHERE checklist_id = $1
            "#,
        )
        .bind(checklist_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_by_checklist(pool: &PgPool, checklist_id: &str) -> Result<Vec<Reminder>> {
        #[derive(sqlx::FromRow)]
        struct ReminderRow {
            id: i32,
            reminder_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            checklist_id: Option<String>,
            task_number: i32,
            task_text: String,
            is_completed: bool,
            notify_at: Option<DateTime<Utc>>,
            last_notified_at: Option<DateTime<Utc>>,
            completed_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<ReminderRow> = sqlx::query_as(
            r#"
            SELECT id, reminder_id, source_type, source_id, sender_id, checklist_id,
                   task_number, task_text, is_completed, notify_at, last_notified_at, completed_at,
                   created_at, updated_at
            FROM reminders
            WHERE checklist_id = $1
            ORDER BY task_number ASC
            "#,
        )
        .bind(checklist_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Reminder {
                id: r.id,
                reminder_id: r.reminder_id,
                source_type: SourceType::from_str(&r.source_type).unwrap(),
                source_id: r.source_id,
                sender_id: r.sender_id,
                checklist_id: r.checklist_id,
                task_number: r.task_number,
                task_text: r.task_text,
                is_completed: r.is_completed,
                notify_at: r.notify_at,
                last_notified_at: r.last_notified_at,
                completed_at: r.completed_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    pub async fn get_pending_reminders(pool: &PgPool) -> Result<Vec<Reminder>> {
        #[derive(sqlx::FromRow)]
        struct ReminderRow {
            id: i32,
            reminder_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            checklist_id: Option<String>,
            task_number: i32,
            task_text: String,
            is_completed: bool,
            notify_at: Option<DateTime<Utc>>,
            last_notified_at: Option<DateTime<Utc>>,
            completed_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<ReminderRow> = sqlx::query_as(
            r#"
            SELECT r.id, r.reminder_id, r.source_type, r.source_id, r.sender_id, r.checklist_id,
                   r.task_number, r.task_text, r.is_completed, r.notify_at, r.last_notified_at, r.completed_at,
                   r.created_at, r.updated_at
            FROM reminders r
            LEFT JOIN checklists c ON c.checklist_id = r.checklist_id
            WHERE r.is_completed = FALSE
              AND r.notify_at IS NOT NULL
              AND r.notify_at <= NOW()
              AND (r.last_notified_at IS NULL OR r.last_notified_at < r.notify_at)
              AND (r.checklist_id IS NULL OR COALESCE(c.schedule_enabled, TRUE))
            ORDER BY r.notify_at ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Reminder {
                id: r.id,
                reminder_id: r.reminder_id,
                source_type: SourceType::from_str(&r.source_type).unwrap(),
                source_id: r.source_id,
                sender_id: r.sender_id,
                checklist_id: r.checklist_id,
                task_text: r.task_text,
                task_number: r.task_number,
                is_completed: r.is_completed,
                notify_at: r.notify_at,
                last_notified_at: r.last_notified_at,
                completed_at: r.completed_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    pub async fn get_recent_by_source(
        pool: &PgPool,
        source_type: SourceType,
        source_id: &str,
        limit: i32,
    ) -> Result<Vec<Reminder>> {
        #[derive(sqlx::FromRow)]
        struct ReminderRow {
            id: i32,
            reminder_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            checklist_id: Option<String>,
            task_number: i32,
            task_text: String,
            is_completed: bool,
            notify_at: Option<DateTime<Utc>>,
            last_notified_at: Option<DateTime<Utc>>,
            completed_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<ReminderRow> = sqlx::query_as(
            r#"
            SELECT DISTINCT ON (checklist_id) id, reminder_id, source_type, source_id, sender_id,
                   checklist_id, task_number, task_text, is_completed, notify_at, last_notified_at, completed_at,
                   created_at, updated_at
            FROM reminders
            WHERE source_type = $1 AND source_id = $2 AND checklist_id IS NOT NULL
            ORDER BY checklist_id, created_at DESC
            LIMIT $3
            "#,
        )
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Reminder {
                id: r.id,
                reminder_id: r.reminder_id,
                source_type: SourceType::from_str(&r.source_type).unwrap(),
                source_id: r.source_id,
                sender_id: r.sender_id,
                checklist_id: r.checklist_id,
                task_number: r.task_number,
                task_text: r.task_text,
                is_completed: r.is_completed,
                notify_at: r.notify_at,
                last_notified_at: r.last_notified_at,
                completed_at: r.completed_at,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    pub fn format_checklist(reminders: &[Reminder], checklist_id: &str) -> String {
        let mut output = String::new();
        output.push_str("📋 Checklist:\n");

        for reminder in reminders {
            let checkbox = if reminder.is_completed { "x" } else { " " };
            let text = if reminder.is_completed {
                format!("~~{}~~", reminder.task_text)
            } else {
                reminder.task_text.clone()
            };
            output.push_str(&format!(
                "[{}] {}. {}\n",
                checkbox, reminder.task_number, text
            ));
        }

        output
    }

    pub async fn delete_checklist(pool: &PgPool, checklist_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM reminders WHERE checklist_id = $1
            "#,
        )
        .bind(checklist_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}

impl Checklist {
    pub async fn save(
        pool: &PgPool,
        checklist_id: &str,
        source_type: SourceType,
        source_id: &str,
        sender_id: Option<&str>,
        title: Option<&str>,
        group_name: Option<&str>,
    ) -> Result<i32> {
        let row = sqlx::query(
            r#"
            INSERT INTO checklists (checklist_id, source_type, source_id, sender_id, title, group_name)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (checklist_id) DO UPDATE SET
                updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(checklist_id)
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(sender_id)
        .bind(title)
        .bind(group_name)
        .fetch_one(pool)
        .await?;

        Ok(row
            .try_get("id")
            .context("Failed to get id from INSERT result")?)
    }

    pub async fn update_schedule_enabled(
        pool: &PgPool,
        checklist_id: &str,
        schedule_enabled: bool,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE checklists
            SET schedule_enabled = $1, updated_at = NOW()
            WHERE checklist_id = $2
            "#,
        )
        .bind(schedule_enabled)
        .bind(checklist_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_by_id(pool: &PgPool, checklist_id: &str) -> Result<Option<Checklist>> {
        #[derive(sqlx::FromRow)]
        struct ChecklistRow {
            id: i32,
            checklist_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            title: Option<String>,
            group_name: Option<String>,
            schedule_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<ChecklistRow> = sqlx::query_as(
            r#"
            SELECT id, checklist_id, source_type, source_id, sender_id, title, group_name, schedule_enabled, created_at, updated_at
            FROM checklists
            WHERE checklist_id = $1
            "#,
        )
        .bind(checklist_id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Checklist {
            id: r.id,
            checklist_id: r.checklist_id,
            source_type: SourceType::from_str(&r.source_type).unwrap(),
            source_id: r.source_id,
            sender_id: r.sender_id,
            title: r.title,
            group_name: r.group_name,
            schedule_enabled: r.schedule_enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }

    pub async fn get_recent_by_source(
        pool: &PgPool,
        source_type: SourceType,
        source_id: &str,
        limit: i32,
    ) -> Result<Vec<Checklist>> {
        #[derive(sqlx::FromRow)]
        struct ChecklistRow {
            id: i32,
            checklist_id: String,
            source_type: String,
            source_id: String,
            sender_id: Option<String>,
            title: Option<String>,
            group_name: Option<String>,
            schedule_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<ChecklistRow> = sqlx::query_as(
            r#"
            SELECT id, checklist_id, source_type, source_id, sender_id, title, group_name, schedule_enabled, created_at, updated_at
            FROM checklists
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

        Ok(rows
            .into_iter()
            .map(|r| Checklist {
                id: r.id,
                checklist_id: r.checklist_id,
                source_type: SourceType::from_str(&r.source_type).unwrap(),
                source_id: r.source_id,
                sender_id: r.sender_id,
                title: r.title,
                group_name: r.group_name,
                schedule_enabled: r.schedule_enabled,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    pub async fn delete(pool: &PgPool, checklist_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM checklists WHERE checklist_id = $1
            "#,
        )
        .bind(checklist_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDateTime, Utc};
    use sqlx::PgPool;

    fn sample_reminder(task_number: i32, text: &str, completed: bool) -> Reminder {
        Reminder {
            id: 1,
            reminder_id: "r1".to_string(),
            source_type: SourceType::User,
            source_id: "U1".to_string(),
            sender_id: Some("U1".to_string()),
            checklist_id: Some("c1".to_string()),
            task_number,
            task_text: text.to_string(),
            is_completed: completed,
            notify_at: None,
            last_notified_at: None,
            completed_at: None,
            created_at: DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
                Utc,
            ),
            updated_at: DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
                Utc,
            ),
        }
    }

    #[test]
    fn format_checklist_marks_completed_items() {
        let reminders = vec![
            sample_reminder(1, "First", false),
            sample_reminder(2, "Second", true),
        ];

        let output = Reminder::format_checklist(&reminders, "c1");
        assert!(output.contains("[ ] 1. First"));
        assert!(output.contains("[x] 2. ~~Second~~"));
    }

    async fn setup_pool() -> Option<PgPool> {
        crate::db::test_utils::try_setup_pool(&["DELETE FROM reminders", "DELETE FROM checklists"])
            .await
    }

    #[tokio::test]
    async fn db_reminder_and_checklist_queries_work() {
        let _lock = crate::db::test_utils::lock_db();
        let Some(pool) = setup_pool().await else {
            eprintln!("Skipping db_reminder_and_checklist_queries_work: DATABASE_URL unavailable or database unreachable");
            return;
        };

        let checklist_id = "c1";
        let _checklist_row = Checklist::save(
            &pool,
            checklist_id,
            SourceType::Group,
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
            SourceType::Group,
            "G1",
            Some("U1"),
            Some(checklist_id),
            1,
            "Task one",
            None,
        )
        .await
        .unwrap();

        Reminder::save(
            &pool,
            "r2",
            SourceType::Group,
            "G1",
            Some("U1"),
            Some(checklist_id),
            2,
            "Task two",
            Some(Utc::now()),
        )
        .await
        .unwrap();

        let by_id = Reminder::get_by_id(&pool, "r1").await.unwrap();
        assert!(by_id.is_some());

        let by_task = Reminder::get_by_task_number(&pool, checklist_id, 2)
            .await
            .unwrap();
        assert!(by_task.is_some());

        let updated = Reminder::update_notify_time(&pool, checklist_id, Some(Utc::now()))
            .await
            .unwrap();
        assert!(updated);

        let pending = Reminder::get_pending_reminders(&pool).await.unwrap();
        assert!(!pending.is_empty());

        let recent = Reminder::get_recent_by_source(&pool, SourceType::Group, "G1", 10)
            .await
            .unwrap();
        assert_eq!(recent.len(), 2);

        let marked = Reminder::mark_notified_by_checklist(&pool, checklist_id)
            .await
            .unwrap();
        assert!(marked > 0);

        let cleared = Reminder::clear_notify_time_by_checklist(&pool, checklist_id)
            .await
            .unwrap();
        assert!(cleared > 0);

        let completed = Reminder::mark_completed(&pool, "r1").await.unwrap();
        assert!(completed);

        let checklist = Checklist::get_by_id(&pool, checklist_id).await.unwrap();
        assert!(checklist.is_some());

        let updated_schedule = Checklist::update_schedule_enabled(&pool, checklist_id, false)
            .await
            .unwrap();
        assert!(updated_schedule);

        let deleted = Reminder::delete_checklist(&pool, checklist_id)
            .await
            .unwrap();
        assert!(deleted > 0);

        let deleted_checklist = Checklist::delete(&pool, checklist_id).await.unwrap();
        assert!(deleted_checklist > 0);
    }
}
