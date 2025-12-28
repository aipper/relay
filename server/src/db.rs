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
  text_sha256 TEXT
);
"#,
    )
    .execute(pool)
    .await?;

    // Best-effort schema upgrades for dev (ignore if already exists).
    let _ = sqlx::query("ALTER TABLE events ADD COLUMN input_id TEXT;")
        .execute(pool)
        .await;
    let _ = sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS events_run_seq_uq ON events(run_id, seq) WHERE seq IS NOT NULL;",
    )
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
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
INSERT OR IGNORE INTO events (run_id, seq, ts, type, stream, actor, input_id, text, text_redacted, text_sha256)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
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
INSERT INTO runs (id, host_id, tool, cwd, status, started_at)
VALUES (?1, ?2, ?3, ?4, 'running', ?5)
ON CONFLICT(id) DO UPDATE SET
  host_id=excluded.host_id,
  tool=excluded.tool,
  cwd=excluded.cwd,
  status='running',
  started_at=excluded.started_at
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

pub async fn mark_run_awaiting_input(pool: &Db, run_id: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE runs SET status='awaiting_input' WHERE id=?1")
        .bind(run_id)
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
    sqlx::query(
        r#"
UPDATE runs
SET status='exited', ended_at=?2, exit_code=?3
WHERE id=?1
"#,
    )
    .bind(run_id)
    .bind(ended_at.to_rfc3339())
    .bind(exit_code)
    .execute(pool)
    .await?;
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
    pub ended_at: Option<String>,
    pub exit_code: Option<i64>,
}

pub async fn list_runs(pool: &Db) -> anyhow::Result<Vec<RunRow>> {
    let rows = sqlx::query_as::<_, RunRow>(
        r#"
SELECT id, host_id, tool, cwd, status, started_at, ended_at, exit_code
FROM runs
ORDER BY started_at DESC
LIMIT 200
"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
