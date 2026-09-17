//! Leading-edge output coalescer for the server push path (Phase B).
//!
//! Semantics copied from paseo's `TerminalOutputCoalescer`
//! (`DEFAULT_FLUSH_DELAY_MS = 5`):
//! - idle-then-first-chunk flushes **immediately** (leading edge) and opens a
//!   short trailing window for the run;
//! - further `run.output` chunks arriving inside the window are **buffered**
//!   and merged (text concatenated) into a single trailing flush;
//! - windows are **per-run independent**.
//!
//! The coalescer only shapes what is pushed to PWA clients via the broadcast
//! fan-out. Raw chunks are still persisted per-chunk by `db::insert_event`
//! before this point, so spool replay stays complete.

use relay_protocol::WsEnvelope;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default trailing-window delay, matching paseo's `DEFAULT_FLUSH_DELAY_MS`.
pub const DEFAULT_OUTPUT_COALESCE_DELAY_MS: u64 = 5;

/// Env var to override the trailing-window delay (parsed as milliseconds).
pub const OUTPUT_COALESCE_MS_ENV: &str = "RELAY_OUTPUT_COALESCE_MS";

/// What `push` decided for one envelope.
pub enum PushOutcome {
    /// First chunk after idle: send immediately and open a trailing window.
    EmitNow { envelope: WsEnvelope },
    /// Absorbed into the run's trailing buffer; send nothing yet.
    Buffered,
}

struct RunWindow {
    opened_at: Instant,
    buffered: Option<WsEnvelope>,
}

pub struct OutputCoalescer {
    delay: Duration,
    windows: HashMap<String, RunWindow>,
}

impl OutputCoalescer {
    pub fn new() -> Self {
        Self::with_delay(Duration::from_millis(DEFAULT_OUTPUT_COALESCE_DELAY_MS))
    }

    pub fn with_delay(delay: Duration) -> Self {
        Self {
            delay,
            windows: HashMap::new(),
        }
    }

    /// Build from `RELAY_OUTPUT_COALESCE_MS`, falling back to the default.
    pub fn from_env() -> Self {
        let ms = std::env::var(OUTPUT_COALESCE_MS_ENV)
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_OUTPUT_COALESCE_DELAY_MS);
        Self::with_delay(Duration::from_millis(ms))
    }

    pub fn delay(&self) -> Duration {
        self.delay
    }

    /// Only `run.output` envelopes carrying a `run_id` are coalesced.
    /// Everything else passes straight through to fan-out.
    pub fn should_coalesce(env: &WsEnvelope) -> bool {
        env.r#type == "run.output" && env.run_id.is_some()
    }

    /// Feed one envelope. `now` is a parameter (instead of `Instant::now()`)
    /// so unit tests can control time deterministically.
    pub fn push(&mut self, env: WsEnvelope, now: Instant) -> PushOutcome {
        let run_id = env.run_id.clone().unwrap_or_default();
        match self.windows.get_mut(&run_id) {
            None => {
                self.windows.insert(
                    run_id,
                    RunWindow {
                        opened_at: now,
                        buffered: None,
                    },
                );
                PushOutcome::EmitNow { envelope: env }
            }
            Some(window) => {
                match window.buffered.as_mut() {
                    None => window.buffered = Some(env),
                    Some(buffered) => merge_output(buffered, &env),
                }
                PushOutcome::Buffered
            }
        }
    }

    /// Drain trailing buffers whose window has expired. Call with
    /// `Instant::now()` from the flush timer.
    pub fn collect_due(&mut self, now: Instant) -> Vec<WsEnvelope> {
        let mut due = Vec::new();
        let delay = self.delay;
        self.windows.retain(|_, window| {
            if now.saturating_duration_since(window.opened_at) >= delay {
                if let Some(env) = window.buffered.take() {
                    due.push(env);
                }
                false
            } else {
                true
            }
        });
        due
    }

    #[cfg(test)]
    fn window_count(&self) -> usize {
        self.windows.len()
    }
}

impl Default for OutputCoalescer {
    fn default() -> Self {
        Self::new()
    }
}

fn output_text(env: &WsEnvelope) -> &str {
    env.data.get("text").and_then(|v| v.as_str()).unwrap_or("")
}

/// Merge `incoming` into `buffered`: concatenate text, advance to the latest
/// seq/ts so web gap detection sees monotonic progress. Other fields
/// (stream, host/run ids) stay from the first chunk of the burst.
fn merge_output(buffered: &mut WsEnvelope, incoming: &WsEnvelope) {
    let mut text = output_text(buffered).to_string();
    text.push_str(output_text(incoming));
    if let Value::Object(map) = &mut buffered.data {
        map.insert("text".to_string(), Value::String(text));
    }
    if incoming.seq.is_some() {
        buffered.seq = incoming.seq;
    }
    if incoming.ts > buffered.ts {
        buffered.ts = incoming.ts;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn output_env(run_id: &str, text: &str, seq: i64) -> WsEnvelope {
        let mut env = WsEnvelope::new("run.output", json!({"stream": "stdout", "text": text}));
        env.run_id = Some(run_id.to_string());
        env.seq = Some(seq);
        env
    }

    #[test]
    fn idle_first_chunk_flushes_immediately() {
        let mut c = OutputCoalescer::with_delay(Duration::from_millis(5));
        let now = Instant::now();
        let outcome = c.push(output_env("run-1", "hello", 1), now);
        assert!(
            matches!(outcome, PushOutcome::EmitNow { .. }),
            "first chunk after idle must flush immediately"
        );
        // Window open but nothing buffered: nothing due even after expiry.
        let due = c.collect_due(now + Duration::from_millis(50));
        assert!(due.is_empty());
        assert_eq!(c.window_count(), 0);
    }

    #[test]
    fn burst_merges_on_trailing_timer() {
        let mut c = OutputCoalescer::with_delay(Duration::from_millis(5));
        let t0 = Instant::now();
        let first = c.push(output_env("run-1", "a", 1), t0);
        assert!(matches!(first, PushOutcome::EmitNow { .. }));
        for (text, seq) in [("b", 2), ("c", 3), ("d", 4)] {
            let o = c.push(
                output_env("run-1", text, seq),
                t0 + Duration::from_millis(1),
            );
            assert!(
                matches!(o, PushOutcome::Buffered),
                "burst chunk must be buffered"
            );
        }
        // Before the window expires: nothing due.
        assert!(c.collect_due(t0 + Duration::from_millis(4)).is_empty());
        // After expiry: exactly one merged flush with concatenated text.
        let due = c.collect_due(t0 + Duration::from_millis(6));
        assert_eq!(due.len(), 1);
        assert_eq!(output_text(&due[0]), "bcd");
        assert_eq!(due[0].seq, Some(4), "merged flush carries latest seq");
        // Window closed: next chunk is a new leading edge.
        let next = c.push(output_env("run-1", "e", 5), t0 + Duration::from_millis(7));
        assert!(matches!(next, PushOutcome::EmitNow { .. }));
    }

    #[test]
    fn runs_are_independent() {
        let mut c = OutputCoalescer::with_delay(Duration::from_millis(5));
        let t0 = Instant::now();
        assert!(matches!(
            c.push(output_env("run-A", "a1", 1), t0),
            PushOutcome::EmitNow { .. }
        ));
        // run-B's first chunk flushes immediately even while run-A's window is open.
        assert!(matches!(
            c.push(output_env("run-B", "b1", 1), t0),
            PushOutcome::EmitNow { .. }
        ));
        assert!(matches!(
            c.push(output_env("run-A", "a2", 2), t0),
            PushOutcome::Buffered
        ));
        let due = c.collect_due(t0 + Duration::from_millis(50));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].run_id.as_deref(), Some("run-A"));
        assert_eq!(output_text(&due[0]), "a2");
    }

    #[test]
    fn should_coalesce_only_run_output_with_run_id() {
        let out = output_env("run-1", "x", 1);
        assert!(OutputCoalescer::should_coalesce(&out));
        let mut no_run = output_env("run-1", "x", 1);
        no_run.run_id = None;
        assert!(!OutputCoalescer::should_coalesce(&no_run));
        let other = WsEnvelope::new("tool.result", json!({}));
        assert!(!OutputCoalescer::should_coalesce(&other));
    }

    #[test]
    fn from_env_defaults_to_five_ms() {
        unsafe { std::env::remove_var(OUTPUT_COALESCE_MS_ENV) };
        assert_eq!(
            OutputCoalescer::from_env().delay(),
            Duration::from_millis(DEFAULT_OUTPUT_COALESCE_DELAY_MS)
        );
    }
}
