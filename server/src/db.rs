use chrono::{DateTime, Utc};
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::path::Path;
use std::str::FromStr;

pub type Db = Pool<Sqlite>;

pub async fn connect(database_url: &str) -> anyhow::Result<Db> {
    if let Some(path) = sqlite_file_path(database_url) {
        if let Some(parent) = Path::new(&path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
    }
    let opts = SqliteConnectOptions::from_str(database_url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(opts)
        .await?;
    Ok(pool)
}

fn sqlite_file_path(database_url: &str) -> Option<String> {
    if database_url.starts_with("sqlite::memory:") {
        return None;
    }

    // Accept:
    // - sqlite:data.db
    // - sqlite://data.db
    // - sqlite:///abs/path.db
    let rest = database_url.strip_prefix("sqlite:")?;
    let (path, _) = rest.split_once('?').unwrap_or((rest, ""));

    let path = if let Some(stripped) = path.strip_prefix("//") {
        // `sqlite://data.db` => "data.db"
        // `sqlite:///abs.db` => "/abs.db"
        stripped
    } else {
        path
    };

    if path.is_empty() || path == ":memory:" {
        return None;
    }

    Some(path.to_string())
}

pub async fn init(pool: &Db) -> anyhow::Result<()> {
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS hosts (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT,
  token_hash TEXT NOT NULL,
  last_seen_at TEXT
);
"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY NOT NULL,
  host_id TEXT NOT NULL,
  tool TEXT NOT NULL,
  cwd TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  last_active_at TEXT,
  pending_request_id TEXT,
  pending_reason TEXT,
  pending_prompt TEXT,
  pending_op_tool TEXT,
  pending_op_args_summary TEXT,
  ended_at TEXT,
  exit_code INTEGER
);
"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id TEXT NOT NULL,
  seq INTEGER,
  ts TEXT NOT NULL,
  type TEXT NOT NULL,
  stream TEXT,
  actor TEXT,
  input_id TEXT,
  text TEXT,
  text_redacted TEXT,
  text_sha256 TEXT,
  data_json TEXT
);
"#,
    )
    .execute(pool)
    .await?;

    // Best-effort schema upgrades for dev (ignore if already exists).
    let _ = sqlx::query("ALTER TABLE events ADD COLUMN input_id TEXT;")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE events ADD COLUMN data_json TEXT;")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE runs ADD COLUMN last_active_at TEXT;")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE runs ADD COLUMN pending_request_id TEXT;")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE runs ADD COLUMN pending_reason TEXT;")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE runs ADD COLUMN pending_prompt TEXT;")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE runs ADD COLUMN pending_op_tool TEXT;")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE runs ADD COLUMN pending_op_args_summary TEXT;")
        .execute(pool)
        .await;
    let _ = sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS events_run_seq_uq ON events(run_id, seq) WHERE seq IS NOT NULL;",
    )
    .execute(pool)
    .await;
    let _ =
        sqlx::query("UPDATE runs SET last_active_at = started_at WHERE last_active_at IS NULL;")
            .execute(pool)
            .await;

    Ok(())
}

pub async fn insert_event(
    pool: &Db,
    run_id: &str,
    seq: Option<i64>,
    ts: DateTime<Utc>,
    r#type: &str,
    stream: Option<&str>,
    actor: Option<&str>,
    input_id: Option<&str>,
    text: Option<&str>,
    text_redacted: Option<&str>,
    text_sha256: Option<&str>,
    data_json: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
INSERT OR IGNORE INTO events (run_id, seq, ts, type, stream, actor, input_id, text, text_redacted, text_sha256, data_json)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
"#,
    )
    .bind(run_id)
    .bind(seq)
    .bind(ts.to_rfc3339())
    .bind(r#type)
    .bind(stream)
    .bind(actor)
    .bind(input_id)
    .bind(text)
    .bind(text_redacted)
    .bind(text_sha256)
    .bind(data_json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_run_started(
    pool: &Db,
    run_id: &str,
    host_id: &str,
    tool: &str,
    cwd: &str,
    started_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
INSERT INTO runs (
  id,
  host_id,
  tool,
  cwd,
  status,
  started_at,
  last_active_at,
  pending_request_id,
  pending_reason,
  pending_prompt,
  pending_op_tool,
  pending_op_args_summary,
  ended_at,
  exit_code
)
VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?5, NULL, NULL, NULL, NULL, NULL, NULL, NULL)
ON CONFLICT(id) DO UPDATE SET
  host_id=excluded.host_id,
  tool=excluded.tool,
  cwd=excluded.cwd,
  status='running',
  started_at=excluded.started_at,
  last_active_at=excluded.last_active_at,
  ended_at=NULL,
  exit_code=NULL,
  pending_request_id=NULL,
  pending_reason=NULL,
  pending_prompt=NULL,
  pending_op_tool=NULL,
  pending_op_args_summary=NULL
"#,
    )
    .bind(run_id)
    .bind(host_id)
    .bind(tool)
    .bind(cwd)
    .bind(started_at.to_rfc3339())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn touch_run_last_active(
    pool: &Db,
    run_id: &str,
    ts: DateTime<Utc>,
) -> anyhow::Result<()> {
    let ts = ts.to_rfc3339();
    sqlx::query(
        r#"
UPDATE runs
SET last_active_at = CASE
  WHEN last_active_at IS NULL OR last_active_at < ?2 THEN ?2
  ELSE last_active_at
END
WHERE id=?1
"#,
    )
    .bind(run_id)
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_run_awaiting_input(
    pool: &Db,
    run_id: &str,
    ts: DateTime<Utc>,
) -> anyhow::Result<()> {
    let ts = ts.to_rfc3339();
    sqlx::query(
        r#"
UPDATE runs
SET status='awaiting_input',
    last_active_at = CASE
      WHEN last_active_at IS NULL OR last_active_at < ?2 THEN ?2
      ELSE last_active_at
    END
WHERE id=?1
"#,
    )
    .bind(run_id)
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_run_awaiting_approval(
    pool: &Db,
    run_id: &str,
    ts: DateTime<Utc>,
) -> anyhow::Result<()> {
    let ts = ts.to_rfc3339();
    sqlx::query(
        r#"
UPDATE runs
SET status='awaiting_approval',
    last_active_at = CASE
      WHEN last_active_at IS NULL OR last_active_at < ?2 THEN ?2
      ELSE last_active_at
    END
WHERE id=?1
"#,
    )
    .bind(run_id)
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_run_pending_permission(
    pool: &Db,
    run_id: &str,
    ts: DateTime<Utc>,
    request_id: &str,
    reason: Option<&str>,
    prompt: Option<&str>,
    op_tool: Option<&str>,
    op_args_summary: Option<&str>,
) -> anyhow::Result<()> {
    let ts = ts.to_rfc3339();
    sqlx::query(
        r#"
UPDATE runs
SET status='awaiting_approval',
    last_active_at = CASE
      WHEN last_active_at IS NULL OR last_active_at < ?2 THEN ?2
      ELSE last_active_at
    END,
    pending_request_id=?3,
    pending_reason=?4,
    pending_prompt=?5,
    pending_op_tool=?6,
    pending_op_args_summary=?7
WHERE id=?1
"#,
    )
    .bind(run_id)
    .bind(ts)
    .bind(request_id)
    .bind(reason)
    .bind(prompt)
    .bind(op_tool)
    .bind(op_args_summary)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_run_running(pool: &Db, run_id: &str, ts: DateTime<Utc>) -> anyhow::Result<()> {
    let ts = ts.to_rfc3339();
    sqlx::query(
        r#"
UPDATE runs
SET status='running',
    last_active_at = CASE
      WHEN last_active_at IS NULL OR last_active_at < ?2 THEN ?2
      ELSE last_active_at
    END,
    pending_request_id=NULL,
    pending_reason=NULL,
    pending_prompt=NULL,
    pending_op_tool=NULL,
    pending_op_args_summary=NULL
WHERE id=?1
"#,
    )
    .bind(run_id)
    .bind(ts)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn clear_run_pending_permission_by_request_id(
    pool: &Db,
    run_id: &str,
    ts: DateTime<Utc>,
    request_id: &str,
) -> anyhow::Result<()> {
    let ts = ts.to_rfc3339();
    sqlx::query(
        r#"
UPDATE runs
SET status='running',
    last_active_at = CASE
      WHEN last_active_at IS NULL OR last_active_at < ?2 THEN ?2
      ELSE last_active_at
    END,
    pending_request_id=NULL,
    pending_reason=NULL,
    pending_prompt=NULL,
    pending_op_tool=NULL,
    pending_op_args_summary=NULL
WHERE id=?1 AND pending_request_id=?3
"#,
    )
    .bind(run_id)
    .bind(ts)
    .bind(request_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn finish_run(
    pool: &Db,
    run_id: &str,
    ended_at: DateTime<Utc>,
    exit_code: i64,
) -> anyhow::Result<()> {
    let ended_at_str = ended_at.to_rfc3339();
    sqlx::query(
        r#"
UPDATE runs
SET status='exited', ended_at=?2, exit_code=?3
WHERE id=?1
"#,
    )
    .bind(run_id)
    .bind(&ended_at_str)
    .bind(exit_code)
    .execute(pool)
    .await?;
    let _ = sqlx::query(
        r#"
UPDATE runs
SET last_active_at = CASE
  WHEN last_active_at IS NULL OR last_active_at < ?2 THEN ?2
  ELSE last_active_at
END,
pending_request_id=NULL,
pending_reason=NULL,
pending_prompt=NULL,
pending_op_tool=NULL,
pending_op_args_summary=NULL
WHERE id=?1
"#,
    )
    .bind(run_id)
    .bind(&ended_at_str)
    .execute(pool)
    .await;
    Ok(())
}

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct RunRow {
    pub id: String,
    pub host_id: String,
    pub tool: String,
    pub cwd: String,
    pub status: String,
    pub started_at: String,
    pub last_active_at: Option<String>,
    pub pending_request_id: Option<String>,
    pub pending_reason: Option<String>,
    pub pending_prompt: Option<String>,
    pub pending_op_tool: Option<String>,
    pub pending_op_args_summary: Option<String>,
    pub ended_at: Option<String>,
    pub exit_code: Option<i64>,
}

pub async fn list_runs(pool: &Db) -> anyhow::Result<Vec<RunRow>> {
    let rows = sqlx::query_as::<_, RunRow>(
        r#"
SELECT
  id,
  host_id,
  tool,
  cwd,
  status,
  started_at,
  last_active_at,
  pending_request_id,
  pending_reason,
  pending_prompt,
  pending_op_tool,
  pending_op_args_summary,
  ended_at,
  exit_code
FROM runs
ORDER BY COALESCE(last_active_at, started_at) DESC
LIMIT 200
"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_run(pool: &Db, run_id: &str) -> anyhow::Result<Option<RunRow>> {
    let row = sqlx::query_as::<_, RunRow>(
        r#"
SELECT
  id,
  host_id,
  tool,
  cwd,
  status,
  started_at,
  last_active_at,
  pending_request_id,
  pending_reason,
  pending_prompt,
  pending_op_tool,
  pending_op_args_summary,
  ended_at,
  exit_code
FROM runs
WHERE id = ?1
LIMIT 1
"#,
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_recent_runs(pool: &Db, limit: i64) -> anyhow::Result<Vec<RunRow>> {
    let limit = limit.clamp(1, 200);
    let rows = sqlx::query_as::<_, RunRow>(
        r#"
SELECT
  id,
  host_id,
  tool,
  cwd,
  status,
  started_at,
  last_active_at,
  pending_request_id,
  pending_reason,
  pending_prompt,
  pending_op_tool,
  pending_op_args_summary,
  ended_at,
  exit_code
FROM runs
ORDER BY COALESCE(last_active_at, started_at) DESC
LIMIT ?1
"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct HostRow {
    pub id: String,
    pub name: Option<String>,
    pub last_seen_at: Option<String>,
}

pub async fn list_hosts(pool: &Db) -> anyhow::Result<Vec<HostRow>> {
    let rows = sqlx::query_as::<_, HostRow>(
        r#"
SELECT id, name, last_seen_at
FROM hosts
ORDER BY id ASC
LIMIT 200
"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[derive(sqlx::FromRow)]
pub struct MessageEventRow {
    pub id: i64,
    pub ts: String,
    pub r#type: String,
    pub actor: Option<String>,
    pub input_id: Option<String>,
    pub text: Option<String>,
    pub text_redacted: Option<String>,
    pub data_json: Option<String>,
}

pub async fn list_message_events(
    pool: &Db,
    run_id: &str,
    before_id: Option<i64>,
    limit: i64,
) -> anyhow::Result<Vec<MessageEventRow>> {
    let limit = limit.clamp(1, 500);
    let rows = sqlx::query_as::<_, MessageEventRow>(
        r#"
SELECT id, ts, type, actor, input_id, text, text_redacted, data_json
FROM events
WHERE run_id=?1
  AND type IN (
    'run.started',
    'run.output',
    'run.permission_requested',
    'run.input',
    'run.exited',
    'tool.call',
    'tool.result'
  )
  AND (?2 IS NULL OR id < ?2)
ORDER BY id DESC
LIMIT ?3
"#,
    )
    .bind(run_id)
    .bind(before_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
