use anyhow::Result;
use sqlx::{Executor, PgPool};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub async fn setup_db(pool: &PgPool) -> Result<()> {
    let schema = std::fs::read_to_string("sql/schema.sql")?;
    for statement in schema.split(';') {
        let stmt = statement.trim();
        if stmt.is_empty() {
            continue;
        }
        pool.execute(stmt).await?;
    }

    // Apply migration 002 inline to add missing columns if needed.
    pool.execute(
        "ALTER TABLE checklists ADD COLUMN IF NOT EXISTS schedule_enabled BOOLEAN NOT NULL DEFAULT TRUE",
    )
    .await?;
    pool.execute(
        "ALTER TABLE reminders ADD COLUMN IF NOT EXISTS last_notified_at TIMESTAMPTZ NULL",
    )
    .await?;

    Ok(())
}

pub fn lock_db() -> MutexGuard<'static, ()> {
    static DB_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    DB_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

pub async fn try_setup_pool(clear_statements: &[&str]) -> Option<PgPool> {
    let database_url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&database_url).await.ok()?;

    setup_db(&pool).await.ok()?;

    for statement in clear_statements {
        sqlx::query(statement).execute(&pool).await.ok()?;
    }

    Some(pool)
}
