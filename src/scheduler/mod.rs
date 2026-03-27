pub mod reminder;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use crate::ai::AIService;
use crate::db::{Message, SourceType};
use crate::line::LineClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub schedules: Vec<Schedule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub source_type: String,
    pub source_id: String,
    pub cron: String,
    #[serde(default)]
    pub message_count: Option<i32>,
    #[serde(default)]
    pub time_range: Option<String>,
    #[serde(default)]
    pub thread_id: Option<String>,
}

impl ScheduleConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read schedules config from {}", path))?;

        toml::from_str(&content).context("Failed to parse schedules TOML config")
    }
}

pub struct ScheduledSummaries {
    scheduler: JobScheduler,
    pool: Arc<PgPool>,
    line_client: Arc<LineClient>,
    ai_service: Arc<dyn AIService>,
}

impl ScheduledSummaries {
    pub async fn new(
        pool: Arc<PgPool>,
        line_client: Arc<LineClient>,
        ai_service: Arc<dyn AIService>,
    ) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;

        Ok(Self {
            scheduler,
            pool,
            line_client,
            ai_service,
        })
    }

    pub async fn load_schedules(&mut self, config_path: &str) -> Result<()> {
        let config = ScheduleConfig::from_file(config_path)?;

        for schedule in &config.schedules {
            self.add_schedule(schedule).await?;
        }

        info!("Loaded {} scheduled summaries", config.schedules.len());
        Ok(())
    }

    async fn add_schedule(&mut self, schedule: &Schedule) -> Result<()> {
        let pool = Arc::clone(&self.pool);
        let line_client = Arc::clone(&self.line_client);
        let ai_service = Arc::clone(&self.ai_service);

        let source_type = schedule.source_type.clone();
        let source_id = schedule.source_id.clone();
        let message_count = schedule.message_count;
        let time_range = schedule.time_range.clone();
        let thread_id = schedule.thread_id.clone();

        let job = Job::new_async(schedule.cron.as_str(), move |_uuid, _l| {
            let pool = Arc::clone(&pool);
            let line_client = Arc::clone(&line_client);
            let ai_service = Arc::clone(&ai_service);
            let source_type = source_type.clone();
            let source_id = source_id.clone();
            let message_count = message_count;
            let time_range = time_range.clone();
            let thread_id = thread_id.clone();

            Box::pin(async move {
                info!(
                    "Running scheduled summary for {} {}",
                    source_type, source_id
                );

                match execute_scheduled_summary(
                    &pool,
                    &line_client,
                    ai_service.as_ref(),
                    &source_type,
                    &source_id,
                    message_count,
                    time_range.as_deref(),
                    thread_id.as_deref(),
                )
                .await
                {
                    Ok(_) => info!(
                        "Scheduled summary sent successfully for {} {}",
                        source_type, source_id
                    ),
                    Err(e) => error!(
                        "Failed to send scheduled summary for {} {}: {}",
                        source_type, source_id, e
                    ),
                }
            })
        })?;

        self.scheduler.add(job).await?;
        info!(
            "Added scheduled summary for {} {} with cron: {}",
            schedule.source_type, schedule.source_id, schedule.cron
        );
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        self.scheduler.start().await?;
        info!("Scheduled summaries started");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.scheduler.shutdown().await?;
        Ok(())
    }
}

async fn execute_scheduled_summary(
    pool: &PgPool,
    line_client: &LineClient,
    ai_service: &dyn AIService,
    source_type: &str,
    source_id: &str,
    message_count: Option<i32>,
    time_range: Option<&str>,
    thread_id: Option<&str>,
) -> Result<()> {
    let source_type = SourceType::from_str(source_type)?;

    let messages = if let Some(thread_ts) = thread_id {
        Message::get_thread_messages(pool, thread_ts).await?
    } else if let Some(range) = time_range {
        let minutes = parse_time_range(range)?;
        Message::get_messages_by_time_range(pool, source_type, source_id, minutes).await?
    } else {
        let count = message_count.unwrap_or(100);
        Message::get_recent_messages(pool, source_type, source_id, count).await?
    };

    if messages.is_empty() {
        info!("No messages to summarize for {} {}", source_type, source_id);
        return Ok(());
    }

    let summary = ai_service.generate_summary(&messages).await?;
    line_client.push_markdown(source_id, &summary).await?;

    Ok(())
}

fn parse_time_range(range: &str) -> Result<i32> {
    let range = range.trim().to_lowercase();

    if range.ends_with('m') {
        let num: i32 = range
            .trim_end_matches('m')
            .parse()
            .context("Invalid time range format")?;
        Ok(num)
    } else if range.ends_with('h') {
        let num: i32 = range
            .trim_end_matches('h')
            .parse()
            .context("Invalid time range format")?;
        Ok(num * 60)
    } else if range.ends_with('d') {
        let num: i32 = range
            .trim_end_matches('d')
            .parse()
            .context("Invalid time range format")?;
        Ok(num * 60 * 24)
    } else {
        anyhow::bail!("Invalid time range format. Use format like '30m', '2h', or '1d'");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use sqlx::PgPool;

    struct DummyAI;

    #[async_trait::async_trait]
    impl AIService for DummyAI {
        async fn generate_summary(&self, _messages: &[Message]) -> anyhow::Result<String> {
            Ok("summary".to_string())
        }
    }

    #[test]
    fn parse_time_range_minutes() {
        assert_eq!(parse_time_range("15m").unwrap(), 15);
    }

    #[test]
    fn parse_time_range_hours() {
        assert_eq!(parse_time_range("2h").unwrap(), 120);
    }

    #[test]
    fn parse_time_range_days() {
        assert_eq!(parse_time_range("1d").unwrap(), 1440);
    }

    #[test]
    fn parse_time_range_invalid() {
        let err = parse_time_range("abc").unwrap_err();
        assert!(err.to_string().contains("Invalid time range format"));
    }

    #[test]
    fn schedule_config_from_file_parses() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"[[schedules]]
source_type = "user"
source_id = "U1"
cron = "0 0 * * * *"
message_count = 50
"#
        )
        .unwrap();

        let config = ScheduleConfig::from_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.schedules.len(), 1);
        assert_eq!(config.schedules[0].source_id, "U1");
        assert_eq!(config.schedules[0].message_count, Some(50));
    }

    async fn setup_pool() -> PgPool {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");
        crate::db::test_utils::setup_db(&pool)
            .await
            .expect("Failed to setup schema");
        sqlx::query("DELETE FROM messages").execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn execute_scheduled_summary_with_count() {
        let _lock = crate::db::test_utils::lock_db();
        let pool = setup_pool().await;
        Message::save(
            &pool,
            "m1",
            SourceType::Group,
            "G1",
            Some("U1"),
            Some("Alice"),
            crate::db::MessageType::Text,
            Some("Hello"),
            None,
            None,
        )
        .await
        .unwrap();

        let client = LineClient::new("token".to_string());
        let ai = DummyAI;
        let result = execute_scheduled_summary(
            &pool,
            &client,
            &ai,
            "group",
            "G1",
            Some(10),
            None,
            None,
        )
        .await;
        assert!(result.is_ok());
    }
}
