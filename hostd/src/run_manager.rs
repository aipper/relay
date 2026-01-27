use anyhow::Context;
use chrono::Utc;
use portable_pty::{CommandBuilder, PtySize};
use regex::Regex;
use relay_protocol::{WsEnvelope, redaction::Redactor};
use serde_json::Value as JsonValue;
use serde_json::json;
use std::{
    collections::HashMap,
    collections::HashSet,
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    sync::{
        Arc,
        Mutex as StdMutex,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Mutex, RwLock, broadcast, oneshot};

use crate::tool_mode_cache::{ToolModeCache, ToolRunMode};

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
    codex_mcp: Mutex<Option<CodexMcpState>>,
    codex_rpc_waiters: StdMutex<HashMap<i64, oneshot::Sender<JsonValue>>>,
    codex_call_lock: Mutex<()>,
}

#[derive(Clone)]
struct PendingPermission {
    request_id: String,
    reason: String,
    prompt: String,
    approve_text: String,
    deny_text: String,
    rpc_request_id: Option<i64>,
}

#[derive(Clone)]
struct CodexMcpState {
    // JSON-RPC sequence for requests we send to codex.
    next_id: i64,
    // A stable conversation/session identifier.
    conversation_id: String,
    // Whether we saw a successful initialize response.
    initialized: bool,
    // Whether we've started a session (first prompt already sent).
    session_started: bool,
    // How to start the codex MCP server (e.g. ["mcp-server"] or ["mcp","serve"]).
    mcp_args: Vec<String>,
}

impl Run {
    fn next_seq(&self) -> i64 {
        self.seq.fetch_add(1, Ordering::SeqCst) + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexModeSetting {
    Tui,
    Structured,
    Auto,
}

fn codex_mode_setting() -> CodexModeSetting {
    let v = std::env::var("RELAY_CODEX_MODE")
        .ok()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match v.as_str() {
        "auto" => CodexModeSetting::Auto,
        "structured" | "mcp" => CodexModeSetting::Structured,
        _ => CodexModeSetting::Tui,
    }
}

fn codex_probe_timeout() -> Duration {
    let ms = std::env::var("RELAY_CODEX_PROBE_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(5_000);
    Duration::from_millis(ms.clamp(250, 60_000))
}

async fn codex_rpc_request(
    run: &Arc<Run>,
    method: &str,
    params: JsonValue,
    timeout: Option<Duration>,
) -> anyhow::Result<JsonValue> {
    let id = {
        let mut state = run.codex_mcp.lock().await;
        let Some(state) = state.as_mut() else {
            return Err(anyhow::anyhow!("codex mcp state missing"));
        };
        let id = state.next_id;
        state.next_id += 1;
        id
    };

    let (tx, rx) = oneshot::channel::<JsonValue>();
    {
        let mut waiters = run
            .codex_rpc_waiters
            .lock()
            .map_err(|_| anyhow::anyhow!("codex rpc waiters lock poisoned"))?;
        waiters.insert(id, tx);
    }

    let req = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });

    {
        let mut w = run.writer.lock().await;
        w.write_all(req.to_string().as_bytes())
            .context("write codex rpc request")?;
        w.write_all(b"\n").context("write newline")?;
        w.flush().ok();
    }

    let resp = match timeout {
        Some(t) => match tokio::time::timeout(t, rx).await {
            Ok(v) => v.context("codex rpc response channel closed")?,
            Err(_) => {
                let mut waiters = run
                    .codex_rpc_waiters
                    .lock()
                    .map_err(|_| anyhow::anyhow!("codex rpc waiters lock poisoned"))?;
                waiters.remove(&id);
                return Err(anyhow::anyhow!("codex rpc timeout"));
            }
        },
        None => rx.await.context("codex rpc response channel closed")?,
    };
    Ok(resp)
}

async fn codex_rpc_notify(run: &Arc<Run>, method: &str, params: JsonValue) -> anyhow::Result<()> {
    let msg = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });
    let mut w = run.writer.lock().await;
    w.write_all(msg.to_string().as_bytes())
        .context("write codex rpc notify")?;
    w.write_all(b"\n").context("write newline")?;
    w.flush().ok();
    Ok(())
}

fn mcp_tool_result_text(result: &JsonValue) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(content) = result.get("content").and_then(|v| v.as_array()) {
        for item in content {
            let t = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if t == "text" {
                if let Some(s) = item.get("text").and_then(|v| v.as_str()) {
                    parts.push(s.to_string());
                }
            } else if let Some(s) = item.get("text").and_then(|v| v.as_str()) {
                parts.push(s.to_string());
            }
        }
    }
    parts.join("")
}

fn mcp_tool_result_thread_id(result: &JsonValue) -> Option<String> {
    let sc = result.get("structuredContent")?;
    sc.get("threadId")
        .and_then(|v| v.as_str())
        .or_else(|| sc.get("conversationId").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
}

async fn codex_mcp_submit_prompt(
    run: Arc<Run>,
    events: broadcast::Sender<WsEnvelope>,
    host_id: String,
    prompt: String,
) -> anyhow::Result<()> {
    let _guard = run.codex_call_lock.lock().await;

    let (tool_name, args, is_first) = {
        let state = run.codex_mcp.lock().await;
        let Some(state) = state.as_ref() else {
            return Err(anyhow::anyhow!("codex mcp state missing"));
        };
        if !state.initialized {
            return Err(anyhow::anyhow!("codex mcp not initialized"));
        }

        if state.session_started && !state.conversation_id.trim().is_empty() {
            (
                "codex-reply".to_string(),
                json!({
                    "threadId": state.conversation_id.clone(),
                    "conversationId": state.conversation_id.clone(),
                    "prompt": prompt,
                }),
                false,
            )
        } else {
            (
                "codex".to_string(),
                json!({
                    "prompt": prompt,
                    "cwd": run.cwd.clone(),
                }),
                true,
            )
        }
    };

    let resp = codex_rpc_request(
        &run,
        "tools/call",
        json!({ "name": tool_name, "arguments": args }),
        None,
    )
    .await?;

    if let Some(err) = resp.get("error") {
        let mut env = WsEnvelope::new(
            "run.output",
            json!({ "stream": "stderr", "text": format!("codex tools/call error: {err}") }),
        );
        env.host_id = Some(host_id);
        env.run_id = Some(run.run_id.clone());
        env.seq = Some(run.next_seq());
        let _ = events.send(env);
        return Ok(());
    }

    let result = resp.get("result").cloned().unwrap_or(JsonValue::Null);
    let is_error = result.get("isError").and_then(|v| v.as_bool()).unwrap_or(false);
    let text = mcp_tool_result_text(&result);
    let thread_id = mcp_tool_result_thread_id(&result);

    if is_first {
        if let Some(thread_id) = thread_id {
            let mut state = run.codex_mcp.lock().await;
            if let Some(state) = state.as_mut() {
                state.conversation_id = thread_id;
                state.session_started = true;
            }
        }
    }

    if !text.trim().is_empty() || is_error {
        let stream = if is_error { "stderr" } else { "stdout" };
        let mut env = WsEnvelope::new("run.output", json!({ "stream": stream, "text": text }));
        env.host_id = Some(host_id);
        env.run_id = Some(run.run_id.clone());
        env.seq = Some(run.next_seq());
        let _ = events.send(env);
    }

    Ok(())
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

        if tool == "codex" {
            match codex_mode_setting() {
                CodexModeSetting::Tui => {
                    return self
                        .start_run_pty_with_id(run_id, tool, cmd, resolved_cwd)
                        .await;
                }
                CodexModeSetting::Structured => {
                    let args = self.probe_codex_mcp_args(&resolved_cwd).await?;
                    return self
                        .start_run_codex_mcp_with_id(run_id, cmd, resolved_cwd, args)
                        .await;
                }
                CodexModeSetting::Auto => {
                    return self
                        .start_run_codex_auto(run_id, cmd, resolved_cwd)
                        .await;
                }
            }
        }

        self.start_run_pty_with_id(run_id, tool, cmd, resolved_cwd)
            .await
    }

    async fn start_run_pty_with_id(
        &self,
        run_id: String,
        tool: String,
        cmd: String,
        cwd: String,
    ) -> anyhow::Result<String> {
        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("openpty")?;

        let spec = crate::runners::for_tool(&tool).build(&cmd, &cwd)?;
        let mut command: CommandBuilder = spec.command;
        command.env("RELAY_RUN_ID", &run_id);
        command.env("RELAY_TOOL", &tool);
        command.env("RELAY_HOSTD_SOCK", &self.local_unix_socket);
        command.env("RELAY_CWD", &cwd);
        if std::env::var_os("TERM").is_none() {
            command.env("TERM", "xterm-256color");
        }
        if std::env::var_os("COLORTERM").is_none() {
            command.env("COLORTERM", "truecolor");
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
            cwd,
            tool: tool.clone(),
            prompt_regex: spec.prompt_regex,
            awaiting_input: Mutex::new(false),
            stdin_line_buf: Mutex::new(Vec::new()),
            processed_input_ids: Mutex::new(HashSet::new()),
            pending_permission: Mutex::new(None),
            codex_mcp: Mutex::new(None),
            codex_rpc_waiters: StdMutex::new(HashMap::new()),
            codex_call_lock: Mutex::new(()),
        });

        {
            let mut runs = self.runs.write().await;
            runs.insert(run_id.clone(), run.clone());
        }

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
                                            rpc_request_id: None,
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

    async fn start_run_codex_auto(
        &self,
        run_id: String,
        cmd: String,
        cwd: String,
    ) -> anyhow::Result<String> {
        let now = Utc::now();

        let mut cache = ToolModeCache::load()?;
        let should_probe = cache
            .as_ref()
            .map(|c| c.should_probe("codex", now))
            .unwrap_or(true);

        if should_probe {
            match self.probe_codex_mcp_args(&cwd).await {
                Ok(args) => {
                    if let Some(c) = cache.as_mut() {
                        c.record_probe_result(
                            "codex",
                            ToolRunMode::Structured,
                            Some(args),
                            None,
                            now,
                        );
                        let _ = c.save();
                    }
                }
                Err(e) => {
                    let msg = e.to_string();
                    if let Some(c) = cache.as_mut() {
                        c.record_probe_result("codex", ToolRunMode::Tui, None, Some(msg), now);
                        let _ = c.save();
                    }
                }
            }
        }

        let entry = cache.as_ref().map(|c| c.get("codex")).unwrap_or_default();
        let selected_mode = entry.parsed_mode().unwrap_or(ToolRunMode::Tui);

        let run_id = if selected_mode == ToolRunMode::Structured {
            if let Some(args) = entry.mcp_args.clone() {
                match self
                    .start_run_codex_mcp_with_id(run_id.clone(), cmd.clone(), cwd.clone(), args)
                    .await
                {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::warn!(error=%e, "codex structured start failed; falling back to PTY");
                        if let Some(c) = cache.as_mut() {
                            c.record_probe_result(
                                "codex",
                                ToolRunMode::Tui,
                                None,
                                Some(format!("structured start failed: {e:#}")),
                                now,
                            );
                            let _ = c.save();
                        }
                        self.start_run_pty_with_id(run_id, "codex".to_string(), cmd, cwd)
                            .await?
                    }
                }
            } else {
                self.start_run_pty_with_id(run_id, "codex".to_string(), cmd, cwd)
                    .await?
            }
        } else {
            self.start_run_pty_with_id(run_id, "codex".to_string(), cmd, cwd)
                .await?
        };

        if let Some(c) = cache.as_mut() {
            c.touch_run("codex");
            let _ = c.save();
        }

        Ok(run_id)
    }

    async fn start_run_codex_mcp_with_id(
        &self,
        run_id: String,
        cmd: String,
        cwd: String,
        mcp_args: Vec<String>,
    ) -> anyhow::Result<String> {
        fn escape_toml_basic_string(s: &str) -> String {
            s.replace('\\', "\\\\").replace('\"', "\\\"")
        }

        fn resolve_relay_mcp_command() -> String {
            if let Ok(v) = std::env::var("RELAY_MCP_COMMAND") {
                let v = v.trim().to_string();
                if !v.is_empty() {
                    return v;
                }
            }

            let exe = match std::env::current_exe() {
                Ok(p) => p,
                Err(_) => return "relay".to_string(),
            };
            let Some(dir) = exe.parent() else {
                return "relay".to_string();
            };

            #[cfg(windows)]
            let candidate = dir.join("relay.exe");
            #[cfg(not(windows))]
            let candidate = dir.join("relay");

            if candidate.is_file() {
                return candidate.to_string_lossy().to_string();
            }
            "relay".to_string()
        }

        fn env_truthy(name: &str) -> bool {
            let v = match std::env::var(name) {
                Ok(v) => v,
                Err(_) => return false,
            };
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            )
        }

        fn env_falsy(name: &str) -> bool {
            let v = match std::env::var(name) {
                Ok(v) => v,
                Err(_) => return false,
            };
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "n" | "off"
            )
        }

        let bin = crate::runners::resolve_tool_bin("codex", "RELAY_CODEX_BIN", "codex");
        crate::runners::validate_bin_exists(
            &bin,
            "codex (set RELAY_CODEX_BIN=/path/to/codex or install shims to record real path)",
        )?;

        let mut child_cmd = Command::new(&bin);
        for a in &mcp_args {
            child_cmd.arg(a);
        }

        if !env_falsy("RELAY_CODEX_ENABLE_RELAY_MCP") && !env_truthy("RELAY_CODEX_DISABLE_RELAY_MCP")
        {
            let relay_cmd = escape_toml_basic_string(&resolve_relay_mcp_command());
            child_cmd.arg("--config");
            child_cmd.arg(format!(
                r#"mcp_servers.relay={{command="{relay_cmd}", args=["mcp"], startup_timeout_sec=20, tool_timeout_sec=600, enabled=true}}"#,
            ));
            child_cmd.arg("--config");
            child_cmd.arg(
                r#"mcp_servers.relay.env={RELAY_RUN_ID="${RELAY_RUN_ID}", RELAY_HOSTD_SOCK="${RELAY_HOSTD_SOCK}", RELAY_TOOL="${RELAY_TOOL}"}"#,
            );
        }

        child_cmd.current_dir(&cwd);
        child_cmd.env("RELAY_RUN_ID", &run_id);
        child_cmd.env("RELAY_TOOL", "codex");
        child_cmd.env("RELAY_HOSTD_SOCK", &self.local_unix_socket);
        child_cmd.env("RELAY_CWD", &cwd);
        child_cmd.stdin(Stdio::piped());
        child_cmd.stdout(Stdio::piped());
        child_cmd.stderr(Stdio::piped());

        let mut child = child_cmd.spawn().context("spawn codex mcp server")?;
        let pid = child.id() as i32;

        let stdin = child.stdin.take().context("take stdin")?;
        let stdout = child.stdout.take().context("take stdout")?;
        let stderr = child.stderr.take().context("take stderr")?;

        let run = Arc::new(Run {
            run_id: run_id.clone(),
            seq: AtomicI64::new(0),
            writer: Mutex::new(Box::new(stdin)),
            pid,
            cwd,
            tool: "codex".to_string(),
            prompt_regex: crate::runners::base_prompt_regex("codex"),
            awaiting_input: Mutex::new(false),
            stdin_line_buf: Mutex::new(Vec::new()),
            processed_input_ids: Mutex::new(HashSet::new()),
            pending_permission: Mutex::new(None),
            codex_mcp: Mutex::new(Some(CodexMcpState {
                next_id: 1,
                conversation_id: String::new(),
                initialized: false,
                session_started: false,
                mcp_args: mcp_args.clone(),
            })),
            codex_rpc_waiters: StdMutex::new(HashMap::new()),
            codex_call_lock: Mutex::new(()),
        });

        {
            let mut runs = self.runs.write().await;
            runs.insert(run_id.clone(), run.clone());
        }

        let mut started = WsEnvelope::new(
            "run.started",
            json!({
                "tool": "codex",
                "cwd": run.cwd,
                "command": cmd,
                "runner_mode": "structured",
                "mcp_args": mcp_args,
            }),
        );
        started.host_id = Some(self.host_id.clone());
        started.run_id = Some(run_id.clone());
        started.seq = Some(run.next_seq());
        let _ = self.events.send(started);

        // Stdout JSON-RPC reader.
        {
            let events = self.events.clone();
            let host_id = self.host_id.clone();
            let run_for_thread = run.clone();
            std::thread::spawn(move || {
                let mut r = BufReader::new(stdout);
                let mut line = String::new();
                loop {
                    line.clear();
                    match r.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let raw = line.clone();
                            let parsed = serde_json::from_str::<JsonValue>(&raw).ok();

                            if let Some(v) = parsed.as_ref() {
                                let rpc_id = v.get("id").and_then(|v| v.as_i64());
                                let is_response = rpc_id.is_some()
                                    && (v.get("result").is_some() || v.get("error").is_some());
                                if is_response {
                                    let id = rpc_id.unwrap_or_default();
                                    if let Ok(mut waiters) = run_for_thread.codex_rpc_waiters.lock()
                                    {
                                        if let Some(tx) = waiters.remove(&id) {
                                            let _ = tx.send(v.clone());
                                            continue;
                                        }
                                    }
                                }

                                let method = v.get("method").and_then(|m| m.as_str()).unwrap_or("");
                                if method == "elicitation/create" {
                                    if let Some(rpc_request_id) = v.get("id").and_then(|v| v.as_i64()) {
                                        let prompt = v
                                            .get("params")
                                            .and_then(|p| p.get("prompt"))
                                            .and_then(|p| p.as_str())
                                            .unwrap_or("permission requested")
                                            .to_string();
                                        let request_id = uuid::Uuid::new_v4().to_string();

                                        if let Ok(mut pending) =
                                            run_for_thread.pending_permission.try_lock()
                                        {
                                            *pending = Some(PendingPermission {
                                                request_id: request_id.clone(),
                                                reason: "permission".to_string(),
                                                prompt: prompt.clone(),
                                                approve_text: "".to_string(),
                                                deny_text: "".to_string(),
                                                rpc_request_id: Some(rpc_request_id),
                                            });
                                        }

                                        if let Ok(mut awaiting) = run_for_thread.awaiting_input.try_lock() {
                                            *awaiting = true;
                                        }

                                        let mut pr = WsEnvelope::new(
                                            "run.permission_requested",
                                            json!({
                                                "request_id": request_id,
                                                "reason": "permission",
                                                "prompt": prompt,
                                                "op_tool": "codex",
                                                "approve_text": "",
                                                "deny_text": ""
                                            }),
                                        );
                                        pr.host_id = Some(host_id.clone());
                                        pr.run_id = Some(run_for_thread.run_id.clone());
                                        pr.seq = Some(run_for_thread.next_seq());
                                        let _ = events.send(pr);

                                        let mut p = WsEnvelope::new(
                                            "run.awaiting_input",
                                            json!({
                                                "reason": "permission",
                                                "prompt": prompt,
                                                "request_id": request_id
                                            }),
                                        );
                                        p.host_id = Some(host_id.clone());
                                        p.run_id = Some(run_for_thread.run_id.clone());
                                        p.seq = Some(run_for_thread.next_seq());
                                        let _ = events.send(p);
                                        continue;
                                    }
                                }
                            }

                            let mut env = WsEnvelope::new(
                                "run.output",
                                json!({
                                    "stream": "stdout",
                                    "text": raw
                                }),
                            );
                            env.host_id = Some(host_id.clone());
                            env.run_id = Some(run_for_thread.run_id.clone());
                            env.seq = Some(run_for_thread.next_seq());
                            let _ = events.send(env);
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        // Stderr reader (logs).
        {
            let events = self.events.clone();
            let host_id = self.host_id.clone();
            let run_for_thread = run.clone();
            std::thread::spawn(move || {
                let mut r = BufReader::new(stderr);
                let mut line = String::new();
                loop {
                    line.clear();
                    match r.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let mut env = WsEnvelope::new(
                                "run.output",
                                json!({
                                    "stream": "stderr",
                                    "text": line
                                }),
                            );
                            env.host_id = Some(host_id.clone());
                            env.run_id = Some(run_for_thread.run_id.clone());
                            env.seq = Some(run_for_thread.next_seq());
                            let _ = events.send(env);
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        // Initialize MCP server (fast-fail).
        let timeout = Some(codex_probe_timeout());
        let init_result: anyhow::Result<()> = async {
            let init_resp = codex_rpc_request(
                &run,
                "initialize",
                json!({
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": { "name": "relay-hostd", "version": env!("CARGO_PKG_VERSION") }
                }),
                timeout,
            )
            .await?;
            if let Some(err) = init_resp.get("error") {
                return Err(anyhow::anyhow!("codex initialize failed: {}", err));
            }
            codex_rpc_notify(&run, "notifications/initialized", json!({})).await?;

            let tools_resp = codex_rpc_request(&run, "tools/list", json!({}), timeout).await?;
            let tools = tools_resp
                .get("result")
                .and_then(|r| r.get("tools"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut has_codex = false;
            let mut has_reply = false;
            for t in tools {
                let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name == "codex" {
                    has_codex = true;
                }
                if name == "codex-reply" {
                    has_reply = true;
                }
            }
            if !has_codex {
                return Err(anyhow::anyhow!("codex mcp server missing tool: codex"));
            }
            if !has_reply {
                tracing::warn!("codex mcp server missing tool: codex-reply (will retry without threadId)");
            }

            let mut state = run.codex_mcp.lock().await;
            if let Some(state) = state.as_mut() {
                state.initialized = true;
            }
            Ok(())
        }
        .await;

        if let Err(e) = init_result {
            let mut env = WsEnvelope::new(
                "run.output",
                json!({
                    "stream": "stderr",
                    "text": format!("codex mcp init failed: {e:#}")
                }),
            );
            env.host_id = Some(self.host_id.clone());
            env.run_id = Some(run_id.clone());
            env.seq = Some(run.next_seq());
            let _ = self.events.send(env);

            let _ = child.kill();
            let _ = child.wait();

            {
                let mut map = self.runs.write().await;
                map.remove(&run_id);
            }

            let mut exited = WsEnvelope::new("run.exited", json!({ "exit_code": -1 }));
            exited.host_id = Some(self.host_id.clone());
            exited.run_id = Some(run_id.clone());
            exited.seq = Some(run.next_seq());
            let _ = self.events.send(exited);

            return Err(e);
        }

        // Exit waiter thread.
        {
            let events = self.events.clone();
            let host_id = self.host_id.clone();
            let run_for_thread = run.clone();
            let runs_map = self.runs.clone();
            std::thread::spawn(move || {
                let exit = child.wait();
                let exit_code = exit.map(|s| s.code().unwrap_or(-1) as i64).unwrap_or(-1);
                let mut env = WsEnvelope::new("run.exited", json!({ "exit_code": exit_code }));
                env.host_id = Some(host_id);
                env.run_id = Some(run_for_thread.run_id.clone());
                env.seq = Some(run_for_thread.next_seq());
                let _ = events.send(env);

                if let Ok(mut map) = runs_map.try_write() {
                    map.remove(&run_for_thread.run_id);
                }
            });
        }

        Ok(run_id)
    }

    async fn probe_codex_mcp_args(&self, cwd: &str) -> anyhow::Result<Vec<String>> {
        let cwd = cwd.to_string();
        let timeout = codex_probe_timeout();
        tokio::task::spawn_blocking(move || {
            fn try_args(
                bin: &str,
                cwd: &str,
                args: &[String],
                timeout: Duration,
            ) -> anyhow::Result<()> {
                use std::sync::mpsc;
                use std::time::Instant;

                let mut child_cmd = Command::new(bin);
                for a in args {
                    child_cmd.arg(a);
                }
                child_cmd.current_dir(cwd);
                child_cmd.stdin(Stdio::piped());
                child_cmd.stdout(Stdio::piped());
                child_cmd.stderr(Stdio::piped());

                let mut child = child_cmd.spawn().context("spawn codex mcp probe")?;
                let mut stdin = child.stdin.take().context("take stdin")?;
                let stdout = child.stdout.take().context("take stdout")?;

                let (tx, rx) = mpsc::channel::<String>();
                std::thread::spawn(move || {
                    let mut r = BufReader::new(stdout);
                    let mut line = String::new();
                    loop {
                        line.clear();
                        match r.read_line(&mut line) {
                            Ok(0) => break,
                            Ok(_) => {
                                let _ = tx.send(line.clone());
                            }
                            Err(_) => break,
                        }
                    }
                });

                let init = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18",
                        "capabilities": {},
                        "clientInfo": { "name": "relay-probe", "version": "0" }
                    }
                });
                let inited = json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized",
                    "params": {}
                });
                let tools = json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/list",
                    "params": {}
                });
                stdin.write_all(init.to_string().as_bytes())?;
                stdin.write_all(b"\n")?;
                stdin.write_all(inited.to_string().as_bytes())?;
                stdin.write_all(b"\n")?;
                stdin.write_all(tools.to_string().as_bytes())?;
                stdin.write_all(b"\n")?;
                stdin.flush().ok();

                let deadline = Instant::now() + timeout;
                let mut got_init = false;
                let mut got_tools = false;

                while Instant::now() < deadline {
                    let rem = deadline.saturating_duration_since(Instant::now());
                    let line = match rx.recv_timeout(rem) {
                        Ok(l) => l,
                        Err(mpsc::RecvTimeoutError::Timeout) => break,
                        Err(_) => break,
                    };
                    let Ok(v) = serde_json::from_str::<JsonValue>(&line) else {
                        continue;
                    };
                    let id = v.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                    if id == 1 && v.get("result").is_some() {
                        got_init = true;
                    }
                    if id == 2 {
                        let tools = v
                            .get("result")
                            .and_then(|r| r.get("tools"))
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let mut has_codex = false;
                        for t in tools {
                            let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            if name == "codex" {
                                has_codex = true;
                            }
                        }
                        if has_codex {
                            got_tools = true;
                        }
                    }
                    if got_init && got_tools {
                        break;
                    }
                }

                let _ = child.kill();
                let _ = child.wait();

                if got_init && got_tools {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("probe did not receive expected responses"))
                }
            }

            let bin = crate::runners::resolve_tool_bin("codex", "RELAY_CODEX_BIN", "codex");
            crate::runners::validate_bin_exists(
                &bin,
                "codex (set RELAY_CODEX_BIN=/path/to/codex or install shims to record real path)",
            )?;

            let candidates = vec![
                vec!["mcp-server".to_string()],
                vec!["mcp".to_string(), "serve".to_string()],
            ];
            let mut last_err: Option<anyhow::Error> = None;
            for args in candidates {
                match try_args(&bin, &cwd, &args, timeout) {
                    Ok(()) => return Ok(args),
                    Err(e) => last_err = Some(e),
                }
            }
            Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no compatible codex mcp server args found")))
        })
        .await?
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

        let is_codex_mcp = { run.codex_mcp.lock().await.is_some() };
        if !is_codex_mcp {
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

        if is_codex_mcp {
            let prompt = text.trim_end_matches(&['\r', '\n'][..]).to_string();
            if !prompt.trim().is_empty() {
                let run2 = run.clone();
                let events = self.events.clone();
                let host_id = self.host_id.clone();
                tokio::spawn(async move {
                    let events2 = events.clone();
                    let host_id2 = host_id.clone();
                    if let Err(e) = codex_mcp_submit_prompt(run2.clone(), events2, host_id2, prompt).await {
                        let mut env = WsEnvelope::new(
                            "run.output",
                            json!({ "stream": "stderr", "text": format!("codex mcp prompt failed: {e:#}") }),
                        );
                        env.host_id = Some(host_id);
                        env.run_id = Some(run2.run_id.clone());
                        env.seq = Some(run2.next_seq());
                        let _ = events.send(env);
                    }
                });
            }
        }

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

        let is_codex_mcp = { run.codex_mcp.lock().await.is_some() };
        if !is_codex_mcp {
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

            if is_codex_mcp {
                let prompt = text.trim_end_matches(&['\r', '\n'][..]).to_string();
                if !prompt.trim().is_empty() {
                    let run2 = run.clone();
                    let events = self.events.clone();
                    let host_id = self.host_id.clone();
                    tokio::spawn(async move {
                        let events2 = events.clone();
                        let host_id2 = host_id.clone();
                        if let Err(e) =
                            codex_mcp_submit_prompt(run2.clone(), events2, host_id2, prompt).await
                        {
                            let mut env = WsEnvelope::new(
                                "run.output",
                                json!({ "stream": "stderr", "text": format!("codex mcp prompt failed: {e:#}") }),
                            );
                            env.host_id = Some(host_id);
                            env.run_id = Some(run2.run_id.clone());
                            env.seq = Some(run2.next_seq());
                            let _ = events.send(env);
                        }
                    });
                }
            }
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

        if let Some(rpc_id) = pending.rpc_request_id {
            // Idempotency: ignore duplicate request_id decisions.
            {
                let mut processed = run.processed_input_ids.lock().await;
                if processed.contains(request_id) {
                    return Ok(());
                }
                processed.insert(request_id.to_string());
            }

            let approved = match decision {
                "approve" => true,
                "deny" => false,
                _ => return Err(anyhow::anyhow!("invalid decision")),
            };

            // MCP elicitation: respond to the JSON-RPC request id.
            let result = json!({
                "action": if approved { "accept" } else { "decline" },
                "content": { "approved": approved }
            });
            let resp = json!({
                "jsonrpc": "2.0",
                "id": rpc_id,
                "result": result
            });
            {
                let mut w = run.writer.lock().await;
                w.write_all(resp.to_string().as_bytes())
                    .context("write elicitation response")?;
                w.write_all(b"\n").context("write newline")?;
                w.flush().ok();
            }

            {
                let mut awaiting = run.awaiting_input.lock().await;
                *awaiting = false;
            }
            {
                let mut pending = run.pending_permission.lock().await;
                *pending = None;
            }

            let decision_text = if approved { "approve" } else { "deny" };
            let redacted = self.redactor.redact(decision_text);
            let mut env = WsEnvelope::new(
                "run.input",
                json!({
                    "actor": actor,
                    "input_id": request_id,
                    "text_redacted": redacted.text_redacted,
                    "text_sha256": redacted.text_sha256
                }),
            );
            env.host_id = Some(self.host_id.clone());
            env.run_id = Some(run.run_id.clone());
            env.seq = Some(run.next_seq());
            let _ = self.events.send(env);

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
                "int" => Signal::SIGINT,
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
