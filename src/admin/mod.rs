use axum::{
    extract::{Path, Query, State},
    response::Html,
    Json,
};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

use crate::config::Config;
use crate::db::Checklist;

// AppState is defined in main.rs, we'll use it directly in the functions

pub async fn dashboard() -> Html<&'static str> {
    Html(ADMIN_HTML)
}

pub async fn api_stats<S>(State(state): State<Arc<S>>) -> Json<StatsResponse>
where
    S: AdminStateAccess,
{
    let pool = state.get_pool();
    let pending = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM reminders r
        LEFT JOIN checklists c ON c.checklist_id = r.checklist_id
        WHERE r.is_completed = FALSE
          AND (r.checklist_id IS NULL OR COALESCE(c.schedule_enabled, TRUE))
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let completed_today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM reminders WHERE is_completed = TRUE AND completed_at::date = CURRENT_DATE"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let total_checklists =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(DISTINCT checklist_id) FROM checklists")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    Json(StatsResponse {
        pending_count: pending,
        completed_today,
        total_checklists,
    })
}

pub async fn api_reminders<S>(
    State(state): State<Arc<S>>,
    Query(filter): Query<ReminderFilter>,
) -> Json<Vec<ReminderResponse>>
where
    S: AdminStateAccess,
{
    let limit = filter.limit.unwrap_or(20);

    #[derive(sqlx::FromRow)]
    struct ReminderRow {
        checklist_id: String,
        source_type: String,
        source_id: String,
        title: Option<String>,
        group_name: Option<String>,
        pending_tasks: i64,
        total_tasks: i64,
        next_notify: Option<DateTime<Utc>>,
        schedule_enabled: bool,
        created_at: DateTime<Utc>,
    }

    let pool = state.get_pool();
    let rows: Vec<ReminderRow> = match sqlx::query_as(
        r#"
        SELECT
               c.checklist_id,
               c.source_type,
               c.source_id,
               c.title,
               c.group_name,
               COALESCE(agg.pending_tasks, 0) as pending_tasks,
               COALESCE(agg.total_tasks, 0) as total_tasks,
               agg.next_notify,
               c.schedule_enabled as schedule_enabled,
               c.created_at as created_at
        FROM checklists c
        LEFT JOIN LATERAL (
            SELECT
                COUNT(CASE WHEN r.is_completed = FALSE THEN 1 END) as pending_tasks,
                COUNT(r.id) as total_tasks,
                MIN(CASE
                    WHEN r.is_completed = FALSE
                         AND r.notify_at IS NOT NULL
                         AND (r.last_notified_at IS NULL OR r.last_notified_at < r.notify_at)
                         AND c.schedule_enabled = TRUE
                    THEN r.notify_at
                END) as next_notify
            FROM reminders r
            WHERE r.checklist_id = c.checklist_id
        ) agg ON TRUE
        WHERE COALESCE(agg.pending_tasks, 0) > 0
        ORDER BY (agg.next_notify IS NULL), agg.next_notify ASC, c.created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(err) => {
            warn!("Failed to load admin reminders: {}", err);
            Vec::new()
        }
    };

    let reminders: Vec<ReminderResponse> = rows
        .into_iter()
        .map(|r| ReminderResponse {
            checklist_id: r.checklist_id,
            source_type: r.source_type,
            source_id: r.source_id,
            title: r.title,
            group_name: r.group_name,
            pending_tasks: r.pending_tasks,
            total_tasks: r.total_tasks,
            next_notify: r.next_notify.map(format_bangkok_datetime),
            schedule_enabled: r.schedule_enabled,
            created_at: format_bangkok_datetime(r.created_at),
        })
        .collect();

    Json(reminders)
}

pub async fn api_reschedule<S>(
    State(state): State<Arc<S>>,
    Path(checklist_id): Path<String>,
    Json(req): Json<RescheduleRequest>,
) -> Json<serde_json::Value>
where
    S: AdminStateAccess,
{
    let new_notify = if let Some(datetime_str) = &req.datetime {
        parse_bangkok_datetime(datetime_str).unwrap_or_else(|_| bangkok_now().with_timezone(&Utc))
    } else if let Some(minutes) = req.minutes {
        bangkok_now().with_timezone(&Utc) + chrono::Duration::minutes(minutes)
    } else {
        bangkok_now().with_timezone(&Utc) + chrono::Duration::minutes(30)
    };

    let pool = state.get_pool();
    let result = sqlx::query(
        "UPDATE reminders SET notify_at = $1, updated_at = NOW() WHERE checklist_id = $2 AND is_completed = FALSE"
    )
    .bind(new_notify)
    .bind(&checklist_id)
    .execute(pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            let _ = Checklist::update_schedule_enabled(state.get_pool(), &checklist_id, true).await;
            Json(serde_json::json!({
                "success": true,
                "message": format!("Rescheduled to {}", format_bangkok_datetime(new_notify))
            }))
        }
        _ => Json(serde_json::json!({
            "success": false,
            "message": "Failed to reschedule"
        })),
    }
}

pub async fn api_toggle_schedule<S>(
    State(state): State<Arc<S>>,
    Path(checklist_id): Path<String>,
    Json(req): Json<ToggleScheduleRequest>,
) -> Json<serde_json::Value>
where
    S: AdminStateAccess,
{
    let pool = state.get_pool();
    match Checklist::update_schedule_enabled(pool, &checklist_id, req.enabled).await {
        Ok(true) => Json(serde_json::json!({
            "success": true,
            "message": if req.enabled { "Schedule enabled" } else { "Schedule disabled" }
        })),
        _ => Json(serde_json::json!({
            "success": false,
            "message": "Failed to update schedule"
        })),
    }
}

fn bangkok_offset() -> FixedOffset {
    FixedOffset::east_opt(7 * 60 * 60).expect("valid UTC+7 offset")
}

fn bangkok_now() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&bangkok_offset())
}

fn format_bangkok_datetime(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&bangkok_offset())
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

fn parse_bangkok_datetime(datetime_str: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    let naive = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M"))?;

    Ok(bangkok_offset()
        .from_local_datetime(&naive)
        .single()
        .expect("Bangkok time is a fixed offset")
        .with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IntegrationMode;
    use sqlx::PgPool;
    use std::sync::Arc;

    #[test]
    fn test_parse_bangkok_datetime_converts_to_utc() {
        let dt = parse_bangkok_datetime("2026-03-27T10:02").unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-03-27 03:02");
    }

    #[test]
    fn test_format_bangkok_datetime_uses_local_time() {
        let dt = DateTime::parse_from_rfc3339("2026-03-27T03:02:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(format_bangkok_datetime(dt), "2026-03-27 10:02");
    }

    #[test]
    fn test_parse_bangkok_datetime_rejects_invalid() {
        let result = parse_bangkok_datetime("invalid");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dashboard_returns_html() {
        let html = dashboard().await;
        assert!(html.0.contains("Admin Dashboard"));
    }

    struct TestState {
        pool: PgPool,
        config: Config,
    }

    impl AdminStateAccess for TestState {
        fn get_pool(&self) -> &sqlx::PgPool {
            &self.pool
        }
        fn get_config(&self) -> &Config {
            &self.config
        }
    }

    async fn setup_pool() -> Option<PgPool> {
        crate::db::test_utils::try_setup_pool(&["DELETE FROM reminders", "DELETE FROM checklists"])
            .await
    }

    fn test_config() -> Config {
        Config {
            database_url: "postgres://user:pass@localhost/db".to_string(),
            line_channel_access_token: "token".to_string(),
            line_channel_secret: "secret".to_string(),
            ai_provider: crate::config::AIProvider::Claude,
            claude_api_key: Some("key".to_string()),
            claude_model: "model".to_string(),
            openai_api_key: None,
            openai_model: "model".to_string(),
            gemini_api_key: None,
            gemini_model: "model".to_string(),
            minimax_api_key: None,
            minimax_model: "model".to_string(),
            zai_api_key: None,
            zai_model: "model".to_string(),
            slack_bot_token: None,
            slack_app_token: None,
            slack_signing_secret: None,
            enable_line: true,
            enable_slack: false,
            slack_integration_mode: IntegrationMode::Webhook,
            teams_app_id: None,
            teams_app_password: None,
            teams_tenant_id: None,
            enable_teams: false,
            n8n_webhook_url: None,
            port: 3000,
            schedules_config_path: "config/schedules.toml".to_string(),
            log_level: "info".to_string(),
            log_dir: "logs".to_string(),
        }
    }

    #[tokio::test]
    async fn api_stats_returns_counts() {
        let _lock = crate::db::test_utils::lock_db();
        let Some(pool) = setup_pool().await else {
            eprintln!("Skipping api_stats_returns_counts: DATABASE_URL unavailable or database unreachable");
            return;
        };
        sqlx::query(
            "INSERT INTO checklists (checklist_id, source_type, source_id) VALUES ('c1','group','G1')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO reminders (reminder_id, source_type, source_id, checklist_id, task_number, task_text, notify_at) VALUES ('r1','group','G1','c1',1,'Task', NOW() + INTERVAL '10 minutes')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let state = Arc::new(TestState {
            pool,
            config: test_config(),
        });
        let Json(stats) = api_stats(State(state)).await;
        assert_eq!(stats.pending_count, 1);
    }

    #[tokio::test]
    async fn api_reminders_returns_rows() {
        let _lock = crate::db::test_utils::lock_db();
        let Some(pool) = setup_pool().await else {
            eprintln!("Skipping api_reminders_returns_rows: DATABASE_URL unavailable or database unreachable");
            return;
        };
        sqlx::query(
            "INSERT INTO checklists (checklist_id, source_type, source_id) VALUES ('c1','group','G1')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO reminders (reminder_id, source_type, source_id, checklist_id, task_number, task_text, notify_at) VALUES ('r1','group','G1','c1',1,'Task', NOW() + INTERVAL '10 minutes')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let state = Arc::new(TestState {
            pool,
            config: test_config(),
        });
        let Query(filter) = Query(ReminderFilter {
            status: None,
            limit: Some(10),
        });
        let Json(reminders) = api_reminders(State(state), Query(filter)).await;
        assert_eq!(reminders.len(), 1);
        assert_eq!(reminders[0].checklist_id, "c1");
    }

    #[tokio::test]
    async fn api_reminders_includes_rows_without_notify_time() {
        let _lock = crate::db::test_utils::lock_db();
        let Some(pool) = setup_pool().await else {
            eprintln!("Skipping api_reminders_includes_rows_without_notify_time: DATABASE_URL unavailable or database unreachable");
            return;
        };
        sqlx::query(
            "INSERT INTO checklists (checklist_id, source_type, source_id) VALUES ('c2','group','G1')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO reminders (reminder_id, source_type, source_id, checklist_id, task_number, task_text) VALUES ('r2','group','G1','c2',1,'Task without time')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let state = Arc::new(TestState {
            pool,
            config: test_config(),
        });
        let Query(filter) = Query(ReminderFilter {
            status: None,
            limit: Some(10),
        });
        let Json(reminders) = api_reminders(State(state), Query(filter)).await;
        assert_eq!(reminders.len(), 1);
        assert_eq!(reminders[0].checklist_id, "c2");
        assert_eq!(reminders[0].next_notify, None);
    }

    #[tokio::test]
    async fn api_reschedule_updates_notify_time() {
        let _lock = crate::db::test_utils::lock_db();
        let Some(pool) = setup_pool().await else {
            eprintln!("Skipping api_reschedule_updates_notify_time: DATABASE_URL unavailable or database unreachable");
            return;
        };
        sqlx::query(
            "INSERT INTO checklists (checklist_id, source_type, source_id) VALUES ('c1','group','G1')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO reminders (reminder_id, source_type, source_id, checklist_id, task_number, task_text, notify_at) VALUES ('r1','group','G1','c1',1,'Task', NOW() + INTERVAL '10 minutes')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let state = Arc::new(TestState {
            pool,
            config: test_config(),
        });
        let Json(resp) = api_reschedule(
            State(state),
            Path("c1".to_string()),
            Json(RescheduleRequest {
                minutes: Some(5),
                datetime: None,
            }),
        )
        .await;
        assert_eq!(resp.get("success").unwrap(), true);
    }

    #[tokio::test]
    async fn api_toggle_schedule_updates_flag() {
        let _lock = crate::db::test_utils::lock_db();
        let Some(pool) = setup_pool().await else {
            eprintln!("Skipping api_toggle_schedule_updates_flag: DATABASE_URL unavailable or database unreachable");
            return;
        };
        sqlx::query(
            "INSERT INTO checklists (checklist_id, source_type, source_id) VALUES ('c1','group','G1')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let state = Arc::new(TestState {
            pool,
            config: test_config(),
        });
        let Json(resp) = api_toggle_schedule(
            State(state),
            Path("c1".to_string()),
            Json(ToggleScheduleRequest { enabled: false }),
        )
        .await;
        assert_eq!(resp.get("success").unwrap(), true);
    }
}

pub async fn api_test_alert<S>(
    State(state): State<Arc<S>>,
    Json(req): Json<TestAlertRequest>,
) -> Json<serde_json::Value>
where
    S: AdminStateAccess,
{
    let target = req.target_id.unwrap_or_else(|| "Utestuser".to_string());

    let client = reqwest::Client::new();

    let config = state.get_config();
    let result = client
        .post("https://api.line.me/v2/bot/message/push")
        .bearer_auth(&config.line_channel_access_token)
        .json(&serde_json::json!({
            "to": target,
            "messages": [{
                "type": "text",
                "text": "🔔 Test Alert\n\nThis is a test notification from the admin dashboard."
            }]
        }))
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => Json(serde_json::json!({
            "success": true,
            "message": format!("Test alert sent to {}", target)
        })),
        Ok(resp) => Json(serde_json::json!({
            "success": false,
            "message": format!("LINE API error: {}", resp.status())
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "message": format!("Failed: {}", e)
        })),
    }
}

#[derive(Serialize)]
pub struct StatsResponse {
    pending_count: i64,
    completed_today: i64,
    total_checklists: i64,
}

#[derive(Deserialize)]
pub struct ReminderFilter {
    status: Option<String>,
    limit: Option<i32>,
}

#[derive(Serialize)]
pub struct ReminderResponse {
    checklist_id: String,
    source_type: String,
    source_id: String,
    title: Option<String>,
    group_name: Option<String>,
    pending_tasks: i64,
    total_tasks: i64,
    next_notify: Option<String>,
    schedule_enabled: bool,
    created_at: String,
}

#[derive(Deserialize)]
pub struct RescheduleRequest {
    minutes: Option<i64>,
    datetime: Option<String>,
}

#[derive(Deserialize)]
pub struct TestAlertRequest {
    target_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ToggleScheduleRequest {
    enabled: bool,
}

// Trait to abstract admin state access
// This allows the admin functions to work with both AppState and AdminState
pub trait AdminStateAccess: Send + Sync {
    fn get_pool(&self) -> &sqlx::PgPool;
    fn get_config(&self) -> &Config;
}

const ADMIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="th">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Admin Dashboard - Cronjob Monitor</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; color: #333; }
        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px 30px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .header h1 { font-size: 24px; margin-bottom: 5px; }
        .header p { opacity: 0.8; font-size: 14px; }
        .container { max-width: 1200px; margin: 0 auto; padding: 20px; }
        .stats-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin-bottom: 30px; }
        .stat-card { background: white; padding: 20px; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
        .stat-card h3 { font-size: 14px; color: #666; margin-bottom: 8px; }
        .stat-card .value { font-size: 32px; font-weight: bold; }
        .stat-card.pending .value { color: #e74c3c; }
        .stat-card.completed .value { color: #27ae60; }
        .stat-card.total .value { color: #3498db; }
        .card { background: white; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); margin-bottom: 20px; overflow: hidden; }
        .card-header { padding: 15px 20px; border-bottom: 1px solid #eee; display: flex; justify-content: space-between; align-items: center; }
        .card-header h2 { font-size: 16px; }
        .card-body { padding: 20px; }
        table { width: 100%; border-collapse: collapse; }
        th, td { padding: 12px; text-align: left; border-bottom: 1px solid #eee; }
        th { background: #f8f9fa; font-weight: 600; color: #555; font-size: 13px; }
        td { font-size: 14px; }
        .badge { display: inline-block; padding: 4px 10px; border-radius: 20px; font-size: 12px; font-weight: 500; }
        .badge-group { background: #e8f4fd; color: #1976d2; }
        .badge-user { background: #fce4ec; color: #c2185b; }
        .badge-room { background: #fff3e0; color: #f57c00; }
        button { padding: 8px 16px; border: none; border-radius: 6px; cursor: pointer; font-size: 13px; transition: all 0.2s; }
        .btn-primary { background: #667eea; color: white; }
        .btn-primary:hover { background: #5568d3; }
        .btn-secondary { background: #6c757d; color: white; }
        .btn-sm { padding: 6px 12px; font-size: 12px; }
        .actions { display: flex; gap: 8px; flex-wrap: wrap; }
        .input-group { display: flex; gap: 10px; align-items: center; margin-bottom: 15px; }
        .input-group input { padding: 10px 14px; border: 1px solid #ddd; border-radius: 6px; font-size: 14px; width: 200px; }
        .input-group input:focus { outline: none; border-color: #667eea; }
        .alert { padding: 12px 20px; border-radius: 8px; margin-bottom: 15px; display: none; }
        .alert.success { background: #d4edda; color: #155724; border: 1px solid #c3e6cb; }
        .alert.error { background: #f8d7da; color: #721c24; border: 1px solid #f5c6cb; }
        .loading, .empty { text-align: center; padding: 40px; color: #666; }
        .cron-info { background: #f8f9fa; padding: 15px; border-radius: 8px; margin-bottom: 15px; }
        .cron-info p { margin: 5px 0; font-size: 14px; }
        .cron-info code { background: #e9ecef; padding: 2px 6px; border-radius: 4px; font-family: monospace; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Admin Dashboard</h1>
        <p>Monitor and manage reminder cronjobs</p>
    </div>
    <div class="container">
        <div class="stats-grid">
            <div class="stat-card pending"><h3>Pending Reminders</h3><div class="value" id="pendingCount">-</div></div>
            <div class="stat-card completed"><h3>Completed Today</h3><div class="value" id="completedToday">-</div></div>
            <div class="stat-card total"><h3>Total Checklists</h3><div class="value" id="totalChecklists">-</div></div>
        </div>
        <div class="card">
            <div class="card-header"><h2>Schedule Configuration</h2></div>
            <div class="card-body">
                <div class="cron-info">
                    <p><strong>Current Cron:</strong> <code>0 * * * * *</code> (every minute)</p>
                    <p><strong>Status:</strong> <span class="badge badge-group">Active</span></p>
                </div>
                <div class="input-group">
                    <input type="text" id="targetUserId" placeholder="LINE User ID for test alert">
                    <button class="btn-primary" onclick="triggerTestAlert()">Send Test Alert</button>
                </div>
                <div id="testAlertResult"></div>
            </div>
        </div>
        <div class="card">
            <div class="card-header">
                <h2>Pending Reminders</h2>
                <button class="btn-secondary btn-sm" onclick="loadReminders()">Refresh</button>
            </div>
            <div class="card-body">
                <div id="alertMessage"></div>
                <table>
                    <thead><tr><th>Checklist ID</th><th>Source</th><th>Group Name</th><th>Title</th><th>Tasks</th><th>Next Notify</th><th>Schedule</th><th>Actions</th></tr></thead>
                    <tbody id="remindersTable"><tr><td colspan="8" class="loading">Loading...</td></tr></tbody>
                </table>
            </div>
        </div>
    </div>
    <script>
        const API_BASE = '/admin/api';
        async function fetchJSON(url, options = {}) {
            const response = await fetch(url, { ...options, headers: { 'Content-Type': 'application/json', ...options.headers } });
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            return response.json();
        }
        async function loadStats() {
            try {
                const stats = await fetchJSON(`${API_BASE}/stats`);
                document.getElementById('pendingCount').textContent = stats.pending_count;
                document.getElementById('completedToday').textContent = stats.completed_today;
                document.getElementById('totalChecklists').textContent = stats.total_checklists;
            } catch (e) { console.error('Failed to load stats:', e); }
        }
        async function loadReminders() {
            const tbody = document.getElementById('remindersTable');
            try {
                const reminders = await fetchJSON(`${API_BASE}/reminders?limit=50`);
                if (reminders.length === 0) { tbody.innerHTML = '<tr><td colspan="8" class="empty">No pending reminders</td></tr>'; return; }
                tbody.innerHTML = reminders.map(r => `<tr>
                    <td><code style="font-size:11px">${r.checklist_id.slice(0,8)}...</code></td>
                    <td><span class="badge badge-${r.source_type}">${r.source_type}</span></td>
                    <td>${r.group_name || '-'}</td>
                    <td>${r.title || '-'}</td>
                    <td>${r.pending_tasks} / ${r.total_tasks}</td>
                    <td>${r.next_notify || 'N/A'}</td>
                    <td><span class="badge" style="background:${r.schedule_enabled ? '#d4edda' : '#f8d7da'}; color:${r.schedule_enabled ? '#155724' : '#721c24'};">${r.schedule_enabled ? 'Enabled' : 'Disabled'}</span></td>
                    <td class="actions">
                        <button class="btn-primary btn-sm" onclick="reschedule('${r.checklist_id}', 30)">+30m</button>
                        <button class="btn-primary btn-sm" onclick="reschedule('${r.checklist_id}', 60)">+1h</button>
                        <button class="btn-secondary btn-sm" onclick="reschedule('${r.checklist_id}', 1440)">+1d</button>
                        <button class="btn-secondary btn-sm" onclick="openRescheduleModal('${r.checklist_id}')">📅</button>
                        <button class="btn-secondary btn-sm" onclick="toggleSchedule('${r.checklist_id}', ${!r.schedule_enabled})">${r.schedule_enabled ? 'Disable' : 'Enable'}</button>
                    </td>
                </tr>`).join('');
            } catch (e) { tbody.innerHTML = `<tr><td colspan="8" class="empty">Error: ${e.message}</td></tr>`; }
        }
        async function reschedule(checklistId, minutes) {
            try {
                const result = await fetchJSON(`${API_BASE}/reminders/${checklistId}/reschedule`, { method: 'POST', body: JSON.stringify({ minutes }) });
                showAlert(result.success ? 'success' : 'error', result.message);
                if (result.success) { loadReminders(); loadStats(); }
            } catch (e) { showAlert('error', `Failed: ${e.message}`); }
        }

        async function toggleSchedule(checklistId, enabled) {
            try {
                const result = await fetchJSON(`${API_BASE}/reminders/${checklistId}/schedule`, {
                    method: 'POST',
                    body: JSON.stringify({ enabled })
                });
                showAlert(result.success ? 'success' : 'error', result.message);
                if (result.success) { loadReminders(); loadStats(); }
            } catch (e) { showAlert('error', `Failed: ${e.message}`); }
        }

        function openRescheduleModal(checklistId) {
            const modal = document.getElementById('rescheduleModal');
            const checklistIdInput = document.getElementById('rescheduleChecklistId');
            const datetimeInput = document.getElementById('rescheduleDatetime');
            checklistIdInput.value = checklistId;
            datetimeInput.value = '';
            modal.style.display = 'flex';
        }

        function closeRescheduleModal() {
            document.getElementById('rescheduleModal').style.display = 'none';
        }

        async function customReschedule() {
            const checklistId = document.getElementById('rescheduleChecklistId').value;
            const datetime = document.getElementById('rescheduleDatetime').value;

            if (!datetime) {
                showAlert('error', 'Please select a date and time');
                return;
            }

            try {
                const result = await fetchJSON(`${API_BASE}/reminders/${checklistId}/reschedule`, {
                    method: 'POST',
                    body: JSON.stringify({ datetime })
                });
                showAlert(result.success ? 'success' : 'error', result.message);
                if (result.success) {
                    closeRescheduleModal();
                    loadReminders();
                    loadStats();
                }
            } catch (e) { showAlert('error', `Failed: ${e.message}`); }
        }
        async function triggerTestAlert() {
            const targetId = document.getElementById('targetUserId').value;
            const resultDiv = document.getElementById('testAlertResult');
            try {
                resultDiv.innerHTML = '<div class="loading">Sending...</div>';
                const result = await fetchJSON(`${API_BASE}/test-alert`, { method: 'POST', body: JSON.stringify({ target_id: targetId || null }) });
                resultDiv.innerHTML = `<div class="alert ${result.success ? 'success' : 'error'}" style="display:block">${result.message}</div>`;
            } catch (e) { resultDiv.innerHTML = `<div class="alert error" style="display:block">Failed: ${e.message}</div>`; }
        }
        function showAlert(type, message) {
            const el = document.getElementById('alertMessage');
            el.innerHTML = `<div class="alert ${type}" style="display:block">${message}</div>`;
            setTimeout(() => el.innerHTML = '', 5000);
        }

        // Close modal when clicking outside
        window.onclick = function(event) {
            const modal = document.getElementById('rescheduleModal');
            if (event.target == modal) {
                closeRescheduleModal();
            }
        }
        loadStats(); loadReminders();
        setInterval(() => { loadStats(); loadReminders(); }, 30000);
    </script>
</body>
</html>

<!-- Reschedule Modal -->
<div id="rescheduleModal" style="display:none; position:fixed; top:0; left:0; width:100%; height:100%; background:rgba(0,0,0,0.5); justify-content:center; align-items:center; z-index:1000;">
    <div style="background:white; padding:30px; border-radius:12px; max-width:400px; width:90%; box-shadow:0 4px 20px rgba(0,0,0,0.2);">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:20px;">
            <h2 style="margin:0; font-size:20px;">Reschedule Reminder</h2>
            <button onclick="closeRescheduleModal()" style="background:none; border:none; font-size:24px; cursor:pointer;">&times;</button>
        </div>
        <input type="hidden" id="rescheduleChecklistId" />
        <div style="margin-bottom:15px;">
            <label style="display:block; margin-bottom:8px; font-weight:500;">Select Date & Time:</label>
            <input type="datetime-local" id="rescheduleDatetime" style="width:100%; padding:12px; border:1px solid #ddd; border-radius:6px; font-size:14px;" />
        </div>
        <div style="display:flex; gap:10px; justify-content:flex-end;">
            <button onclick="closeRescheduleModal()" style="padding:10px 20px; border:1px solid #ddd; background:white; border-radius:6px; cursor:pointer; font-size:14px;">Cancel</button>
            <button onclick="customReschedule()" style="padding:10px 20px; border:none; background:#667eea; color:white; border-radius:6px; cursor:pointer; font-size:14px;">Reschedule</button>
        </div>
    </div>
</div>
    </script>
</body>
</html>"#;
