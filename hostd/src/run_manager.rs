use anyhow::Context;
use portable_pty::{CommandBuilder, PtySize};
use regex::Regex;
use relay_protocol::{WsEnvelope, redaction::Redactor};
use serde_json::Value as JsonValue;
use serde_json::json;
use std::{
    collections::HashMap,
    collections::HashSet,
    io::{Read, Write},
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};
use tokio::sync::{Mutex, RwLock, broadcast};

#[derive(Clone)]
pub struct RunManager {
    host_id: String,
    local_unix_socket: String,
    redactor: Arc<Redactor>,
    events: broadcast::Sender<WsEnvelope>,
    runs: Arc<RwLock<HashMap<String, Arc<Run>>>>,
}

struct Run {
    run_id: String,
    seq: AtomicI64,
    writer: Mutex<Box<dyn Write + Send>>,
    pid: i32,
    cwd: String,
    tool: String,
    prompt_regex: Arc<Regex>,
    awaiting_input: Mutex<bool>,
    stdin_line_buf: Mutex<Vec<u8>>,
    processed_input_ids: Mutex<HashSet<String>>,
    pending_permission: Mutex<Option<PendingPermission>>,
}

#[derive(Clone)]
struct PendingPermission {
    request_id: String,
    reason: String,
    prompt: String,
    approve_text: String,
    deny_text: String,
}

impl Run {
    fn next_seq(&self) -> i64 {
        self.seq.fetch_add(1, Ordering::SeqCst) + 1
    }
}

impl RunManager {
    pub fn new(
        host_id: String,
        local_unix_socket: String,
        redactor: Arc<Redactor>,
        events: broadcast::Sender<WsEnvelope>,
    ) -> Self {
        Self {
            host_id,
            local_unix_socket,
            redactor,
            events,
            runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEnvelope> {
        self.events.subscribe()
    }

    pub fn host_id_value(&self) -> String {
        self.host_id.clone()
    }

    pub fn redact_string(&self, raw: &str) -> String {
        self.redactor.redact(raw).text_redacted
    }

    pub fn redact_json_value(&self, v: &JsonValue) -> JsonValue {
        fn walk(redactor: &Redactor, v: &JsonValue) -> JsonValue {
            match v {
                JsonValue::String(s) => JsonValue::String(redactor.redact(s).text_redacted),
                JsonValue::Array(arr) => {
                    JsonValue::Array(arr.iter().map(|x| walk(redactor, x)).collect())
                }
                JsonValue::Object(map) => JsonValue::Object(
                    map.iter()
                        .map(|(k, val)| (k.clone(), walk(redactor, val)))
                        .collect(),
                ),
                _ => v.clone(),
            }
        }
        walk(&self.redactor, v)
    }

    pub async fn start_run(
        &self,
        tool: String,
        cmd: String,
        cwd: Option<String>,
    ) -> anyhow::Result<String> {
        let run_id = format!("run-{}", uuid::Uuid::new_v4());
        let resolved_cwd = match cwd.as_deref() {
            Some(c) => c.to_string(),
            None => std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| ".".into()),
        };

        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("openpty")?;

        let spec = crate::runners::for_tool(&tool).build(&cmd, &resolved_cwd)?;
        let mut command: CommandBuilder = spec.command;
        command.env("RELAY_RUN_ID", &run_id);
        command.env("RELAY_TOOL", &tool);
        command.env("RELAY_HOSTD_SOCK", &self.local_unix_socket);
        command.env("RELAY_CWD", &resolved_cwd);

        let mut child = pair.slave.spawn_command(command).context("spawn_command")?;

        let pid = child.process_id().context("process_id")? as i32;

        let reader = pair.master.try_clone_reader().context("clone reader")?;
        let writer = pair.master.take_writer().context("take writer")?;

        let run = Arc::new(Run {
            run_id: run_id.clone(),
            seq: AtomicI64::new(0),
            writer: Mutex::new(writer),
            pid,
            cwd: resolved_cwd,
            tool: tool.clone(),
            prompt_regex: spec.prompt_regex,
            awaiting_input: Mutex::new(false),
            stdin_line_buf: Mutex::new(Vec::new()),
            processed_input_ids: Mutex::new(HashSet::new()),
            pending_permission: Mutex::new(None),
        });

        {
            let mut runs = self.runs.write().await;
            runs.insert(run_id.clone(), run.clone());
        }

        // Emit run.started
        let mut started = WsEnvelope::new(
            "run.started",
            json!({
                "tool": tool,
                "cwd": run.cwd,
                "command": cmd,
            }),
        );
        started.host_id = Some(self.host_id.clone());
        started.run_id = Some(run_id.clone());
        started.seq = Some(run.next_seq());
        let _ = self.events.send(started);

        // Output reader loop (blocking).
        let events = self.events.clone();
        let host_id = self.host_id.clone();
        let run_for_thread = run.clone();
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        let is_prompt = run_for_thread.prompt_regex.is_match(&text);
                        let mut env = WsEnvelope::new(
                            "run.output",
                            json!({
                                "stream": "stdout",
                                "text": text
                            }),
                        );
                        env.host_id = Some(host_id.clone());
                        env.run_id = Some(run_for_thread.run_id.clone());
                        env.seq = Some(run_for_thread.next_seq());
                        let _ = events.send(env);

                        if is_prompt {
                            // Best-effort: avoid spamming awaiting_input for the same run.
                            if let Ok(mut awaiting) = run_for_thread.awaiting_input.try_lock() {
                                if !*awaiting {
                                    *awaiting = true;
                                    let prompt = text.chars().take(200).collect::<String>();
                                    let request_id = uuid::Uuid::new_v4().to_string();

                                    if let Ok(mut pending) =
                                        run_for_thread.pending_permission.try_lock()
                                    {
                                        *pending = Some(PendingPermission {
                                            request_id: request_id.clone(),
                                            reason: "prompt".to_string(),
                                            prompt: prompt.clone(),
                                            approve_text: "y\n".to_string(),
                                            deny_text: "n\n".to_string(),
                                        });
                                    }

                                    let mut pr = WsEnvelope::new(
                                        "run.permission_requested",
                                        json!({
                                            "request_id": request_id,
                                            "reason": "prompt",
                                            "prompt": prompt,
                                            "approve_text": "y\n",
                                            "deny_text": "n\n"
                                        }),
                                    );
                                    pr.host_id = Some(host_id.clone());
                                    pr.run_id = Some(run_for_thread.run_id.clone());
                                    pr.seq = Some(run_for_thread.next_seq());
                                    let _ = events.send(pr);

                                    let mut p = WsEnvelope::new(
                                        "run.awaiting_input",
                                        json!({
                                            "reason": "prompt",
                                            "prompt": prompt,
                                            "request_id": request_id
                                        }),
                                    );
                                    p.host_id = Some(host_id.clone());
                                    p.run_id = Some(run_for_thread.run_id.clone());
                                    p.seq = Some(run_for_thread.next_seq());
                                    let _ = events.send(p);
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Exit waiter (blocking), then emit run.exited.
        let events = self.events.clone();
        let host_id = self.host_id.clone();
        let run_for_thread = run.clone();
        let runs_map = self.runs.clone();
        std::thread::spawn(move || {
            let exit = child.wait();
            let exit_code = exit.map(|s| s.exit_code() as i64).unwrap_or(-1);
            let mut env = WsEnvelope::new("run.exited", json!({ "exit_code": exit_code }));
            env.host_id = Some(host_id);
            env.run_id = Some(run_for_thread.run_id.clone());
            env.seq = Some(run_for_thread.next_seq());
            let _ = events.send(env);

            if let Ok(mut map) = runs_map.try_write() {
                map.remove(&run_for_thread.run_id);
            }
        });

        Ok(run_id)
    }

    pub async fn send_input(
        &self,
        run_id: &str,
        actor: &str,
        input_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        let run = {
            let runs = self.runs.read().await;
            runs.get(run_id).cloned()
        }
        .context("unknown run_id")?;

        // Idempotency: ignore duplicate input_id for the same run.
        {
            let mut processed = run.processed_input_ids.lock().await;
            if processed.contains(input_id) {
                return Ok(());
            }
            processed.insert(input_id.to_string());
        }

        {
            let mut w = run.writer.lock().await;
            w.write_all(text.as_bytes()).context("write stdin")?;
            w.flush().ok();
        }

        // Clear awaiting flag once we write an input.
        {
            let mut awaiting = run.awaiting_input.lock().await;
            *awaiting = false;
        }
        {
            let mut pending = run.pending_permission.lock().await;
            *pending = None;
        }

        let redacted = self.redactor.redact(text);
        let mut env = WsEnvelope::new(
            "run.input",
            json!({
                "actor": actor,
                "input_id": input_id,
                "text_redacted": redacted.text_redacted,
                "text_sha256": redacted.text_sha256
            }),
        );
        env.host_id = Some(self.host_id.clone());
        env.run_id = Some(run.run_id.clone());
        env.seq = Some(run.next_seq());
        let _ = self.events.send(env);

        Ok(())
    }

    pub async fn write_stdin_bytes(
        &self,
        run_id: &str,
        actor: &str,
        bytes: &[u8],
    ) -> anyhow::Result<()> {
        let run = {
            let runs = self.runs.read().await;
            runs.get(run_id).cloned()
        }
        .context("unknown run_id")?;

        {
            let mut w = run.writer.lock().await;
            w.write_all(bytes).context("write stdin")?;
            w.flush().ok();
        }

        // Track per-line input so we can emit `run.input` to clear awaiting state in the server UI.
        // We only emit on newline boundaries to avoid spamming events for every keypress.
        let mut lines: Vec<Vec<u8>> = Vec::new();
        {
            let mut buf = run.stdin_line_buf.lock().await;
            buf.extend_from_slice(bytes);
            if buf.len() > 64 * 1024 {
                // Safety valve: if a client never sends a newline, don't grow unbounded.
                let len = buf.len();
                let drop_len = len - 64 * 1024;
                buf.drain(0..drop_len);
            }

            let mut start = 0usize;
            let mut i = 0usize;
            while i < buf.len() {
                match buf[i] {
                    b'\n' => {
                        lines.push(buf[start..=i].to_vec());
                        start = i + 1;
                    }
                    b'\r' => {
                        if i + 1 < buf.len() && buf[i + 1] == b'\n' {
                            lines.push(buf[start..=i + 1].to_vec());
                            start = i + 2;
                            i += 1;
                        } else {
                            lines.push(buf[start..=i].to_vec());
                            start = i + 1;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            if start > 0 {
                buf.drain(0..start);
            }
        }

        for line in lines {
            let text = String::from_utf8_lossy(&line).to_string();

            {
                let mut awaiting = run.awaiting_input.lock().await;
                *awaiting = false;
            }
            {
                let mut pending = run.pending_permission.lock().await;
                *pending = None;
            }

            let redacted = self.redactor.redact(&text);
            let input_id = uuid::Uuid::new_v4().to_string();
            let mut env = WsEnvelope::new(
                "run.input",
                json!({
                    "actor": actor,
                    "input_id": input_id,
                    "text_redacted": redacted.text_redacted,
                    "text_sha256": redacted.text_sha256
                }),
            );
            env.host_id = Some(self.host_id.clone());
            env.run_id = Some(run.run_id.clone());
            env.seq = Some(run.next_seq());
            let _ = self.events.send(env);
        }

        Ok(())
    }

    pub async fn decide_permission(
        &self,
        run_id: &str,
        actor: &str,
        request_id: &str,
        decision: &str,
    ) -> anyhow::Result<()> {
        let run = {
            let runs = self.runs.read().await;
            runs.get(run_id).cloned()
        }
        .context("unknown run_id")?;

        let pending = { run.pending_permission.lock().await.clone() };
        let Some(pending) = pending else {
            return Ok(());
        };
        if pending.request_id != request_id {
            return Ok(());
        }

        let text = match decision {
            "approve" => pending.approve_text,
            "deny" => pending.deny_text,
            _ => return Err(anyhow::anyhow!("invalid decision")),
        };

        self.send_input(run_id, actor, request_id, &text).await?;
        Ok(())
    }

    pub async fn stop_run(&self, run_id: &str, signal: &str) -> anyhow::Result<()> {
        let run = {
            let runs = self.runs.read().await;
            runs.get(run_id).cloned()
        }
        .context("unknown run_id")?;

        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;
            let sig = match signal {
                "kill" => Signal::SIGKILL,
                _ => Signal::SIGTERM,
            };
            kill(Pid::from_raw(run.pid), sig).context("kill")?;
        }

        Ok(())
    }

    pub async fn get_run_cwd(&self, run_id: &str) -> anyhow::Result<String> {
        let run = {
            let runs = self.runs.read().await;
            runs.get(run_id).cloned()
        }
        .context("unknown run_id")?;
        Ok(run.cwd.clone())
    }

    pub async fn emit_run_event(
        &self,
        run_id: &str,
        event_type: &str,
        data: serde_json::Value,
    ) -> anyhow::Result<()> {
        let run = {
            let runs = self.runs.read().await;
            runs.get(run_id).cloned()
        }
        .context("unknown run_id")?;

        let mut env = WsEnvelope::new(event_type, data);
        env.host_id = Some(self.host_id.clone());
        env.run_id = Some(run.run_id.clone());
        env.seq = Some(run.next_seq());
        let _ = self.events.send(env);
        Ok(())
    }

    pub async fn list_runs(&self) -> Vec<RunSummary> {
        let runs = {
            let map = self.runs.read().await;
            map.values().cloned().collect::<Vec<_>>()
        };

        let mut out = Vec::with_capacity(runs.len());
        for run in runs {
            let awaiting_input = *run.awaiting_input.lock().await;
            let pending_permission = run.pending_permission.lock().await.clone();
            out.push(RunSummary {
                run_id: run.run_id.clone(),
                pid: run.pid,
                tool: run.tool.clone(),
                cwd: run.cwd.clone(),
                awaiting_input,
                pending_request_id: pending_permission.map(|p| p.request_id),
            });
        }
        out
    }
}

#[derive(serde::Serialize, Clone)]
pub struct RunSummary {
    pub run_id: String,
    pub pid: i32,
    pub tool: String,
    pub cwd: String,
    pub awaiting_input: bool,
    pub pending_request_id: Option<String>,
}
