use anyhow::Context;
use portable_pty::{CommandBuilder, PtySize};
use regex::Regex;
use relay_protocol::{WsEnvelope, redaction::Redactor};
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
    redactor: Arc<Redactor>,
    events: broadcast::Sender<WsEnvelope>,
    runs: Arc<RwLock<HashMap<String, Arc<Run>>>>,
    prompt_regex: Arc<Regex>,
}

struct Run {
    run_id: String,
    seq: AtomicI64,
    writer: Mutex<Box<dyn Write + Send>>,
    pid: i32,
    awaiting_input: Mutex<bool>,
    processed_input_ids: Mutex<HashSet<String>>,
}

impl Run {
    fn next_seq(&self) -> i64 {
        self.seq.fetch_add(1, Ordering::SeqCst) + 1
    }
}

impl RunManager {
    pub fn new(
        host_id: String,
        redactor: Arc<Redactor>,
        events: broadcast::Sender<WsEnvelope>,
    ) -> Self {
        // MVP: heuristic patterns for interactive prompts.
        let prompt_regex = Regex::new(
            r"(?ix)
            (proceed\\?|continue\\?|are\\s+you\\s+sure\\?|confirm\\b)
            |(\\(\\s*y\\s*/\\s*n\\s*\\))
            |(\\[\\s*y\\s*/\\s*n\\s*\\])
            |(\\(\\s*y\\s*/\\s*N\\s*\\))
            |(\\[\\s*y\\s*/\\s*N\\s*\\])
            ",
        )
        .expect("valid prompt regex");

        Self {
            host_id,
            redactor,
            events,
            runs: Arc::new(RwLock::new(HashMap::new())),
            prompt_regex: Arc::new(prompt_regex),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsEnvelope> {
        self.events.subscribe()
    }

    pub async fn start_run(
        &self,
        tool: String,
        cmd: String,
        cwd: Option<String>,
    ) -> anyhow::Result<String> {
        let run_id = format!("run-{}", uuid::Uuid::new_v4());

        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("openpty")?;

        let mut command = CommandBuilder::new("bash");
        command.arg("-lc");
        command.arg(&cmd);
        if let Some(cwd) = cwd.as_deref() {
            command.cwd(cwd);
        }

        let mut child = pair.slave.spawn_command(command).context("spawn_command")?;

        let pid = child.process_id().context("process_id")? as i32;

        let reader = pair.master.try_clone_reader().context("clone reader")?;
        let writer = pair.master.take_writer().context("take writer")?;

        let run = Arc::new(Run {
            run_id: run_id.clone(),
            seq: AtomicI64::new(0),
            writer: Mutex::new(writer),
            pid,
            awaiting_input: Mutex::new(false),
            processed_input_ids: Mutex::new(HashSet::new()),
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
                "cwd": cwd,
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
        let prompt_regex = self.prompt_regex.clone();
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        let is_prompt = prompt_regex.is_match(&text);
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
                                    let mut p = WsEnvelope::new(
                                        "run.awaiting_input",
                                        json!({
                                            "reason": "prompt",
                                            "prompt": prompt
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
        std::thread::spawn(move || {
            let exit = child.wait();
            let exit_code = exit.map(|s| s.exit_code() as i64).unwrap_or(-1);
            let mut env = WsEnvelope::new("run.exited", json!({ "exit_code": exit_code }));
            env.host_id = Some(host_id);
            env.run_id = Some(run_for_thread.run_id.clone());
            env.seq = Some(run_for_thread.next_seq());
            let _ = events.send(env);
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
}
