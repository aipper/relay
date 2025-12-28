use anyhow::Context;
use relay_protocol::WsEnvelope;
use rusqlite::{Connection, params};

#[derive(Clone)]
pub struct Spool {
    path: String,
}

impl Spool {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub fn init(&self) -> anyhow::Result<()> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch(
            r#"
CREATE TABLE IF NOT EXISTS spool_events (
  run_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  ts TEXT NOT NULL,
  json TEXT NOT NULL,
  PRIMARY KEY(run_id, seq)
);
CREATE TABLE IF NOT EXISTS spool_acks (
  run_id TEXT PRIMARY KEY NOT NULL,
  last_seq INTEGER NOT NULL
);
"#,
        )?;
        Ok(())
    }

    pub fn insert_event(&self, env: &WsEnvelope) -> anyhow::Result<()> {
        let run_id = env.run_id.as_deref().context("spool requires run_id")?;
        let seq = env.seq.context("spool requires seq")?;
        let ts = env.ts.to_rfc3339();
        let json = serde_json::to_string(env)?;

        let conn = Connection::open(&self.path)?;
        conn.execute(
            "INSERT OR IGNORE INTO spool_events (run_id, seq, ts, json) VALUES (?1, ?2, ?3, ?4)",
            params![run_id, seq, ts, json],
        )?;
        Ok(())
    }

    pub fn apply_ack(&self, run_id: &str, last_seq: i64) -> anyhow::Result<()> {
        let conn = Connection::open(&self.path)?;
        conn.execute(
            r#"
INSERT INTO spool_acks (run_id, last_seq) VALUES (?1, ?2)
ON CONFLICT(run_id) DO UPDATE SET last_seq = CASE
  WHEN excluded.last_seq > spool_acks.last_seq THEN excluded.last_seq
  ELSE spool_acks.last_seq
END
"#,
            params![run_id, last_seq],
        )?;
        conn.execute(
            "DELETE FROM spool_events WHERE run_id=?1 AND seq <= ?2",
            params![run_id, last_seq],
        )?;
        Ok(())
    }

    pub fn pending_events(&self, limit: usize) -> anyhow::Result<Vec<WsEnvelope>> {
        let conn = Connection::open(&self.path)?;
        let mut stmt = conn.prepare(
            r#"
SELECT e.json
FROM spool_events e
LEFT JOIN spool_acks a ON a.run_id = e.run_id
WHERE e.seq > COALESCE(a.last_seq, 0)
ORDER BY e.run_id ASC, e.seq ASC
LIMIT ?1
"#,
        )?;

        let mut rows = stmt.query(params![limit as i64])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            let env: WsEnvelope = serde_json::from_str(&json)?;
            out.push(env);
        }
        Ok(out)
    }

    pub fn prune_older_than_rfc3339(&self, cutoff_ts: &str) -> anyhow::Result<()> {
        let conn = Connection::open(&self.path)?;
        conn.execute("DELETE FROM spool_events WHERE ts < ?1", params![cutoff_ts])?;
        Ok(())
    }
}
