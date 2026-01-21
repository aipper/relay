mod config;
mod fs_git;
mod local_api;
mod run_manager;
mod runners;
mod spool;

use futures_util::{SinkExt, StreamExt};
use relay_protocol::WsEnvelope;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};

use crate::config::Config;
use crate::run_manager::RunManager;
use crate::spool::Spool;
use serde_json::json;

fn truncate_chars(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let mut out = String::with_capacity(std::cmp::min(s.len(), max_chars));
    for (i, ch) in s.chars().enumerate() {
        if i >= max_chars {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

fn rpc_requires_permission(rpc_type: &str) -> bool {
    matches!(rpc_type, "rpc.fs.write" | "rpc.bash")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (cfg, cfg_path) = Config::from_env_and_file()?;
    if let Some(p) = cfg_path.as_ref() {
        tracing::info!(config_path=%p.display(), "loaded hostd config");
    }
    if cfg.host_token == "dev-token"
        && !cfg.server_base_url.contains("127.0.0.1")
        && !cfg.server_base_url.contains("localhost")
    {
        tracing::warn!(
            "HOST_TOKEN is using the default 'dev-token'; set it in ~/.config/abrelay/hostd.json or HOST_TOKEN for production use"
        );
    }
    tracing::info!(host_id=%cfg.host_id, server_base=%cfg.server_base_url, sock=%cfg.local_unix_socket, "hostd starting");

    let spool = Spool::new(cfg.spool_db_path.clone());
    tokio::task::spawn_blocking({
        let spool = spool.clone();
        move || spool.init()
    })
    .await??;

    // Periodic spool pruning (3 days).
    {
        let spool = spool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                let cutoff = (chrono::Utc::now() - chrono::Duration::days(3)).to_rfc3339();
                let spool = spool.clone();
                let _ =
                    tokio::task::spawn_blocking(move || spool.prune_older_than_rfc3339(&cutoff))
                        .await;
            }
        });
    }

    let redactor = Arc::new(relay_protocol::redaction::Redactor::new(
        &cfg.redaction_extra_regex,
    )?);
    let (events_tx, _) = broadcast::channel::<WsEnvelope>(2048);
    let rm = RunManager::new(
        cfg.host_id.clone(),
        cfg.local_unix_socket.clone(),
        redactor,
        events_tx.clone(),
    );

    // Persist outgoing events to spool for offline replay.
    {
        let spool = spool.clone();
        let mut rx = events_tx.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(env) => {
                        if env.run_id.is_some() && env.seq.is_some() {
                            let spool = spool.clone();
                            let _ =
                                tokio::task::spawn_blocking(move || spool.insert_event(&env)).await;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        });
    }

    let pending_tool_permissions: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Local unix API server.
    let local = Arc::new(local_api::LocalState {
        rm: rm.clone(),
        pending_tool_permissions: pending_tool_permissions.clone(),
    });
    let local_app = local_api::router(local);
    let sock_path = cfg.local_unix_socket.clone();
    tokio::spawn(async move {
        let _ = std::fs::remove_file(&sock_path);
        if let Err(err) = serve_unix(sock_path, local_app).await {
            tracing::error!(error=%err, "local unix api stopped");
        }
    });

    // Outbound WS to central server.
    let mut ws_url = url::Url::parse(&format!(
        "{}/ws/host",
        cfg.server_base_url.trim_end_matches('/')
    ))?;
    ws_url
        .query_pairs_mut()
        .append_pair("host_id", &cfg.host_id)
        .append_pair("host_token", &cfg.host_token);

    loop {
        if let Err(err) = connect_and_run(
            ws_url.clone(),
            cfg.clone(),
            rm.clone(),
            events_tx.subscribe(),
            spool.clone(),
            pending_tool_permissions.clone(),
        )
        .await
        {
            tracing::warn!(error=%err, "server ws disconnected; retrying");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }
}

async fn serve_unix(sock_path: String, app: axum::Router) -> anyhow::Result<()> {
    use hyper::server::conn::http1;
    use hyper_util::{rt::TokioIo, service::TowerToHyperService};

    // Ensure the parent directory exists; otherwise the bind will fail and the dev scripts
    // will keep waiting for a socket that never appears.
    let sock_path = std::path::PathBuf::from(sock_path);
    if let Some(parent) = sock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_file(&sock_path);
    let listener = tokio::net::UnixListener::bind(&sock_path)?;
    loop {
        let (stream, _) = listener.accept().await?;
        let service = TowerToHyperService::new(app.clone());
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let _ = http1::Builder::new().serve_connection(io, service).await;
        });
    }
}

async fn connect_and_run(
    ws_url: url::Url,
    cfg: Config,
    rm: RunManager,
    mut events_rx: broadcast::Receiver<WsEnvelope>,
    spool: Spool,
    pending_tool_permissions: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
) -> anyhow::Result<()> {
    let (ws, _) = tokio_tungstenite::connect_async(ws_url.to_string()).await?;
    tracing::info!("connected to server ws");

    let (ws_sender, mut ws_receiver) = ws.split();
    let (out_tx, mut out_rx) = mpsc::channel::<tokio_tungstenite::tungstenite::Message>(2048);
    let _send_task = tokio::spawn(async move {
        let mut ws_sender = ws_sender;
        while let Some(msg) = out_rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(10));

    async fn flush_spool(
        out_tx: &mpsc::Sender<tokio_tungstenite::tungstenite::Message>,
        spool: &Spool,
        limit: usize,
    ) -> anyhow::Result<()> {
        let pending = tokio::task::spawn_blocking({
            let spool = spool.clone();
            move || spool.pending_events(limit)
        })
        .await??;
        for env in pending {
            let text = serde_json::to_string(&env)?;
            out_tx
                .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
                .await
                .map_err(|_| anyhow::anyhow!("ws sender closed"))?;
        }
        Ok(())
    }

    // Replay pending events first (best-effort).
    let _ = flush_spool(&out_tx, &spool, 10_000).await;

    let mut task_set = tokio::task::JoinSet::<()>::new();

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                let msg = serde_json::to_string(&WsEnvelope::new("host.heartbeat", serde_json::json!({})))?;
                out_tx
                    .send(tokio_tungstenite::tungstenite::Message::Text(msg.into()))
                    .await
                    .map_err(|_| anyhow::anyhow!("ws sender closed"))?;
                // Periodically try to drain any remaining spool backlog.
                let _ = flush_spool(&out_tx, &spool, 500).await;
            }
            ev = events_rx.recv() => {
                match ev {
                    Ok(env) => {
                        let text = serde_json::to_string(&env)?;
                        out_tx
                            .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
                            .await
                            .map_err(|_| anyhow::anyhow!("ws sender closed"))?;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            res = task_set.join_next(), if task_set.len() > 0 => {
                if let Some(Err(err)) = res {
                    tracing::warn!(error=%err, "rpc task join error");
                }
            }
            incoming = ws_receiver.next() => {
                let Some(incoming) = incoming else { break; };
                let msg = incoming?;
                match msg {
                    tokio_tungstenite::tungstenite::Message::Text(text) => {
                        let Ok(env) = serde_json::from_str::<WsEnvelope>(&text) else { continue; };
                        if env.r#type == "rpc.host.info" || env.r#type == "rpc.host.doctor" {
                            let request_id = env
                                .data
                                .get("request_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if request_id.is_empty() {
                                continue;
                            }

                            let tool_status = |tool: &str| -> serde_json::Value {
                                let (env_var, default_bin) = match tool {
                                    "codex" => ("RELAY_CODEX_BIN", "codex"),
                                    "claude" => ("RELAY_CLAUDE_BIN", "claude"),
                                    "iflow" => ("RELAY_IFLOW_BIN", "iflow"),
                                    "gemini" => ("RELAY_GEMINI_BIN", "gemini"),
                                    _ => ("", tool),
                                };
                                let resolved = if env_var.is_empty() {
                                    default_bin.to_string()
                                } else {
                                    crate::runners::resolve_tool_bin(tool, env_var, default_bin)
                                };
                                let err = crate::runners::validate_bin_exists(&resolved, tool)
                                    .err()
                                    .map(|e| e.to_string());
                                json!({ "tool": tool, "bin": resolved, "ok": err.is_none(), "error": err })
                            };

                            let tools = ["codex", "claude", "iflow", "gemini"]
                                .into_iter()
                                .map(tool_status)
                                .collect::<Vec<_>>();

                            let base = json!({
                                "host_id": rm.host_id_value(),
                                "pid": std::process::id(),
                                "os": std::env::consts::OS,
                                "arch": std::env::consts::ARCH,
                                "version": env!("CARGO_PKG_VERSION"),
                                "tools": tools
                            });

                            let result = if env.r#type == "rpc.host.info" {
                                base
                            } else {
                                let mut obj = base.as_object().cloned().unwrap_or_default();
                                obj.insert(
                                    "deps".to_string(),
                                    json!([
                                        { "name": "rg", "ok": crate::fs_git::has_cmd("rg") },
                                        { "name": "git", "ok": crate::fs_git::has_cmd("git") }
                                    ]),
                                );

                                let bin_map = std::env::var("HOME")
                                    .ok()
                                    .filter(|s| !s.trim().is_empty())
                                    .map(|home| format!("{}/.relay/bin-map.json", home.trim_end_matches('/')));
                                let bin_map_meta = bin_map.as_deref().and_then(|p| std::fs::metadata(p).ok());
                                let mut bm = serde_json::Map::new();
                                bm.insert("path".to_string(), json!(bin_map));
                                bm.insert("exists".to_string(), json!(bin_map_meta.is_some()));
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    if let Some(meta) = bin_map_meta {
                                        let mode = meta.permissions().mode() & 0o777;
                                        bm.insert("mode".to_string(), json!(format!("{mode:o}")));
                                        bm.insert("ok".to_string(), json!((mode & 0o077) == 0));
                                    }
                                }
                                obj.insert("bin_map".to_string(), serde_json::Value::Object(bm));
                                serde_json::Value::Object(obj)
                            };

                            let resp = WsEnvelope::new(
                                "rpc.response",
                                json!({
                                    "request_id": request_id,
                                    "ok": true,
                                    "rpc_type": env.r#type,
                                    "result": result
                                }),
                            );
                            let _ = out_tx
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::to_string(&resp)?.into(),
                                ))
                                .await;
                        } else if env.r#type == "rpc.host.capabilities" {
                            let request_id = env
                                .data
                                .get("request_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if request_id.is_empty() {
                                continue;
                            }

                            let tools = ["codex", "claude", "iflow", "gemini"]
                                .into_iter()
                                .map(|tool| {
                                    let (env_var, default_bin) = match tool {
                                        "codex" => ("RELAY_CODEX_BIN", "codex"),
                                        "claude" => ("RELAY_CLAUDE_BIN", "claude"),
                                        "iflow" => ("RELAY_IFLOW_BIN", "iflow"),
                                        "gemini" => ("RELAY_GEMINI_BIN", "gemini"),
                                        _ => ("", tool),
                                    };
                                    let resolved = crate::runners::resolve_tool_bin(tool, env_var, default_bin);
                                    let err = crate::runners::validate_bin_exists(&resolved, tool)
                                        .err()
                                        .map(|e| e.to_string());
                                    json!({ "tool": tool, "bin": resolved, "ok": err.is_none(), "error": err })
                                })
                                .collect::<Vec<_>>();

                            let result = json!({
                                "host_id": rm.host_id_value(),
                                "pid": std::process::id(),
                                "os": std::env::consts::OS,
                                "arch": std::env::consts::ARCH,
                                "version": env!("CARGO_PKG_VERSION"),
                                "server_base_url": cfg.server_base_url,
                                "local_unix_socket": cfg.local_unix_socket,
                                "spool_db_path": cfg.spool_db_path,
                                "log_path": cfg.log_path,
                                "supported_rpc": [
                                    "rpc.run.start",
                                    "rpc.fs.read",
                                    "rpc.fs.search",
                                    "rpc.fs.list",
                                    "rpc.fs.write",
                                    "rpc.git.status",
                                    "rpc.git.diff",
                                    "rpc.bash",
                                    "rpc.run.stop",
                                    "rpc.runs.list",
                                    "rpc.host.info",
                                    "rpc.host.doctor",
                                    "rpc.host.capabilities",
                                    "rpc.host.logs.tail"
                                ],
                                "tools": tools,
                                "deps": [
                                    { "name": "rg", "ok": crate::fs_git::has_cmd("rg") },
                                    { "name": "git", "ok": crate::fs_git::has_cmd("git") }
                                ]
                            });

                            let resp = WsEnvelope::new(
                                "rpc.response",
                                json!({
                                    "request_id": request_id,
                                    "ok": true,
                                    "rpc_type": env.r#type,
                                    "result": result
                                }),
                            );
                            let _ = out_tx
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::to_string(&resp)?.into(),
                                ))
                                .await;
                        } else if env.r#type == "rpc.host.logs.tail" {
                            let request_id = env
                                .data
                                .get("request_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if request_id.is_empty() {
                                continue;
                            }
                            let lines = env
                                .data
                                .get("lines")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(200)
                                .clamp(1, 2000) as usize;
                            let max_bytes = env
                                .data
                                .get("max_bytes")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(200_000)
                                .clamp(1, 2_000_000) as usize;

                            let Some(path) = cfg.log_path.clone() else {
                                let resp = WsEnvelope::new(
                                    "rpc.response",
                                    json!({
                                        "request_id": request_id,
                                        "ok": false,
                                        "rpc_type": env.r#type,
                                        "error": "HOSTD_LOG_PATH is not set on this host"
                                    }),
                                );
                                let _ = out_tx
                                    .send(tokio_tungstenite::tungstenite::Message::Text(
                                        serde_json::to_string(&resp)?.into(),
                                    ))
                                    .await;
                                continue;
                            };

                            let read = std::fs::read(&path).map_err(|e| e.to_string());
                            let Ok(mut bytes) = read else {
                                let resp = WsEnvelope::new(
                                    "rpc.response",
                                    json!({
                                        "request_id": request_id,
                                        "ok": false,
                                        "rpc_type": env.r#type,
                                        "error": format!("failed to read log file: {path}")
                                    }),
                                );
                                let _ = out_tx
                                    .send(tokio_tungstenite::tungstenite::Message::Text(
                                        serde_json::to_string(&resp)?.into(),
                                    ))
                                    .await;
                                continue;
                            };
                            let truncated = bytes.len() > max_bytes;
                            if truncated {
                                bytes = bytes[bytes.len() - max_bytes..].to_vec();
                            }
                            let text = String::from_utf8_lossy(&bytes).to_string();
                            let mut parts = text.lines().collect::<Vec<_>>();
                            if parts.len() > lines {
                                parts = parts[parts.len() - lines..].to_vec();
                            }
                            let out_text = parts.join("\n");

                            let resp = WsEnvelope::new(
                                "rpc.response",
                                json!({
                                    "request_id": request_id,
                                    "ok": true,
                                    "rpc_type": env.r#type,
                                    "result": { "path": path, "text": out_text, "truncated": truncated }
                                }),
                            );
                            let _ = out_tx
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::to_string(&resp)?.into(),
                                ))
                                .await;
                        } else if env.r#type == "run.ack" {
                            let run_id = env
                                .data
                                .get("run_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let last_seq = env.data.get("last_seq").and_then(|v| v.as_i64()).unwrap_or(0);
                            if !run_id.is_empty() && last_seq > 0 {
                                let spool = spool.clone();
                                let _ = tokio::task::spawn_blocking(move || spool.apply_ack(&run_id, last_seq)).await;
                            }
                            // After ack, attempt to flush more pending data.
                            let _ = flush_spool(&out_tx, &spool, 500).await;
                        } else if env.r#type == "run.send_input" {
                            let Some(run_id) = env.run_id.as_deref() else { continue; };
                            let actor = env.data.get("actor").and_then(|v| v.as_str()).unwrap_or("web");
                            let input_id = env.data.get("input_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let text = env.data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            let _ = rm.send_input(run_id, actor, input_id, text).await;
                        } else if env.r#type == "run.permission.approve" || env.r#type == "run.permission.deny" {
                            let Some(run_id) = env.run_id.as_deref() else { continue; };
                            let actor = env.data.get("actor").and_then(|v| v.as_str()).unwrap_or("web");
                            let request_id = env.data.get("request_id").and_then(|v| v.as_str()).unwrap_or("");
                            if request_id.is_empty() {
                                continue;
                            }
                            let approved = env.r#type == "run.permission.approve";
                            let key = format!("{run_id}:{request_id}");
                            let tx = {
                                let mut map = pending_tool_permissions.lock().await;
                                map.remove(&key)
                            };
                            if let Some(tx) = tx {
                                let _ = tx.send(approved);
                            } else {
                                let decision = if approved { "approve" } else { "deny" };
                                let _ = rm.decide_permission(run_id, actor, request_id, decision).await;
                            }
                        } else if env.r#type == "run.stop" {
                            let Some(run_id) = env.run_id.as_deref() else { continue; };
                            let signal = env.data.get("signal").and_then(|v| v.as_str()).unwrap_or("term");
                            let _ = rm.stop_run(run_id, signal).await;
                        } else if env.r#type == "rpc.run.start" {
                            let request_id = env.data.get("request_id").and_then(|v| v.as_str()).unwrap_or("");
                            if request_id.is_empty() {
                                continue;
                            }
                            let tool = env.data.get("tool").and_then(|v| v.as_str()).unwrap_or("codex").to_string();
                            let cmd = env
                                .data
                                .get("cmd")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .filter(|s| !s.trim().is_empty())
                                .unwrap_or_else(|| tool.clone());

                            let cwd = env.data.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let run_id = match rm.start_run(tool, cmd, cwd).await {
                                Ok(id) => id,
                                Err(err) => {
                                    let resp = WsEnvelope::new(
                                        "rpc.response",
                                        json!({
                                            "request_id": request_id,
                                            "ok": false,
                                            "rpc_type": "rpc.run.start",
                                            "error": err.to_string()
                                        }),
                                    );
                                    let _ = out_tx
                                        .send(tokio_tungstenite::tungstenite::Message::Text(
                                            serde_json::to_string(&resp)?.into(),
                                        ))
                                        .await;
                                    continue;
                                }
                            };

                            let mut resp = WsEnvelope::new(
                                "rpc.response",
                                json!({
                                    "request_id": request_id,
                                    "ok": true,
                                    "rpc_type": "rpc.run.start",
                                    "result": { "run_id": run_id }
                                }),
                            );
                            resp.run_id = resp.data.get("result").and_then(|v| v.get("run_id")).and_then(|v| v.as_str()).map(|s| s.to_string());
                            let _ = out_tx
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::to_string(&resp)?.into(),
                                ))
                                .await;
                        } else if env.r#type.starts_with("rpc.") {
                            let Some(run_id) = env.run_id.as_deref() else { continue; };
                            let request_id = env.data.get("request_id").and_then(|v| v.as_str()).unwrap_or("");
                            if request_id.is_empty() {
                                continue;
                            }

                            let cwd = match rm.get_run_cwd(run_id).await {
                                Ok(c) => c,
                                Err(err) => {
                                    let mut resp = WsEnvelope::new(
                                        "rpc.response",
                                        json!({
                                            "request_id": request_id,
                                            "ok": false,
                                            "rpc_type": env.r#type,
                                            "error": err.to_string()
                                        }),
                                    );
                                    resp.run_id = Some(run_id.to_string());
                                    let _ = out_tx
                                        .send(tokio_tungstenite::tungstenite::Message::Text(
                                            serde_json::to_string(&resp)?.into(),
                                        ))
                                        .await;
                                    continue;
                                }
                            };

                            let actor = env
                                .data
                                .get("actor")
                                .and_then(|v| v.as_str())
                                .unwrap_or("web");
                            let rpc_type = env.r#type.clone();
                            let rpc_type_for_exec = rpc_type.clone();
                            let data = env.data.clone();
                            let started = std::time::Instant::now();

                            let args_for_event = match rpc_type_for_exec.as_str() {
                                "rpc.fs.write" => {
                                    let path = data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                                    let content = data.get("content").and_then(|v| v.as_str()).unwrap_or("");
                                    let bytes = content.as_bytes().len() as i64;
                                    let preview_limit = 2000;
                                    let preview_raw = truncate_chars(content, preview_limit);
                                    let preview_redacted = rm.redact_string(&preview_raw);
                                    let content_truncated = content.chars().count() > preview_limit;
                                    json!({
                                        "path": path,
                                        "bytes": bytes,
                                        "content_preview": preview_redacted,
                                        "content_truncated": content_truncated
                                    })
                                }
                                "rpc.bash" => {
                                    let cmd = data.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
                                    json!({ "cmd": rm.redact_string(cmd) })
                                }
                                _ => rm.redact_json_value(&data),
                            };

                            let _ = rm
                                .emit_run_event(
                                    run_id,
                                    "tool.call",
                                    json!({
                                        "request_id": request_id,
                                        "tool": rpc_type.clone(),
                                        "actor": actor,
                                        "args": args_for_event.clone()
                                    }),
                                )
                                .await;

                            if rpc_requires_permission(rpc_type_for_exec.as_str()) {
                                let (tx, rx) = oneshot::channel::<bool>();
                                let key = format!("{run_id}:{request_id}");
                                {
                                    let mut map = pending_tool_permissions.lock().await;
                                    map.insert(key.clone(), tx);
                                }

                                let (op_tool, op_args, op_args_summary) = match rpc_type_for_exec.as_str() {
                                    "rpc.fs.write" => {
                                        let path = data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                                        let content = data.get("content").and_then(|v| v.as_str()).unwrap_or("");
                                        let bytes = content.as_bytes().len() as i64;
                                        let summary = truncate_chars(&format!("path={path} bytes={bytes}"), 80);
                                        (rpc_type_for_exec.as_str(), args_for_event, summary)
                                    }
                                    "rpc.bash" => {
                                        let cmd = data.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
                                        let cmd = rm.redact_string(cmd);
                                        let summary = truncate_chars(&format!("cmd={cmd}"), 80);
                                        ("bash", json!({ "cmd": cmd }), summary)
                                    }
                                    _ => {
                                        let summary = truncate_chars(&serde_json::to_string(&args_for_event).unwrap_or_default(), 80);
                                        (rpc_type_for_exec.as_str(), args_for_event, summary)
                                    }
                                };

                                let prompt = if op_args_summary.trim().is_empty() {
                                    format!("需要审批：{op_tool}")
                                } else {
                                    format!("需要审批：{op_tool} {op_args_summary}")
                                };
                                let _ = rm
                                    .emit_run_event(
                                        run_id,
                                        "run.permission_requested",
                                        json!({
                                            "request_id": request_id,
                                            "reason": "permission",
                                            "prompt": prompt,
                                            "op_tool": op_tool,
                                            "op_args": op_args,
                                            "op_args_summary": op_args_summary,
                                            "approve_text": "",
                                            "deny_text": ""
                                        }),
                                    )
                                    .await;

                                let rm_task = rm.clone();
                                let out_tx_task = out_tx.clone();
                                let run_id_task = run_id.to_string();
                                let request_id_task = request_id.to_string();
                                let actor_task = actor.to_string();
                                let rpc_type_task = rpc_type.clone();
                                let rpc_type_for_exec_task = rpc_type_for_exec.clone();
                                let data_task = data.clone();
                                let cwd_task = cwd.clone();

                                task_set.spawn(async move {
                                    let approved = rx.await.unwrap_or(false);

                                    let (ok, payload) = if !approved {
                                        (false, json!({ "error": "denied" }))
                                    } else {
                                        let exec_started = std::time::Instant::now();
                                        let result = tokio::task::spawn_blocking(move || {
                                            match rpc_type_for_exec_task.as_str() {
                                                "rpc.fs.write" => {
                                                    let path = data_task.get("path").and_then(|v| v.as_str()).unwrap_or("");
                                                    let content = data_task.get("content").and_then(|v| v.as_str()).unwrap_or("");
                                                    let (bytes_written, truncated) = crate::fs_git::write_utf8_file(&cwd_task, path, content, 1024 * 1024)?;
                                                    Ok(json!({ "path": path, "bytes_written": bytes_written, "truncated": truncated }))
                                                }
                                                "rpc.bash" => {
                                                    let cmd = data_task.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
                                                    let (stdout, stderr, exit_code, truncated) =
                                                        crate::fs_git::bash_exec(&cwd_task, cmd, 200_000, 200_000)?;
                                                    Ok(json!({ "stdout": stdout, "stderr": stderr, "exit_code": exit_code, "truncated": truncated }))
                                                }
                                                _ => Err((axum::http::StatusCode::NOT_IMPLEMENTED, "unknown rpc type".into())),
                                            }
                                        })
                                        .await
                                        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));

                                        match result {
                                            Ok(Ok(v)) => (true, json!({ "result": v, "duration_ms": exec_started.elapsed().as_millis() as i64 })),
                                            Ok(Err((_, msg))) => (false, json!({ "error": msg, "duration_ms": exec_started.elapsed().as_millis() as i64 })),
                                            Err((_, msg)) => (false, json!({ "error": msg, "duration_ms": exec_started.elapsed().as_millis() as i64 })),
                                        }
                                    };

                                    let duration_ms = payload.get("duration_ms").and_then(|v| v.as_i64()).unwrap_or(0);
                                    let result_value = payload.get("result").cloned().unwrap_or(serde_json::Value::Null);
                                    let error_value = payload.get("error").cloned().unwrap_or(serde_json::Value::Null);
                                    let request_id_for_event = request_id_task.clone();
                                    let rpc_type_for_event = rpc_type_task.clone();
                                    let actor_for_event = actor_task.clone();
                                    let result_for_event = result_value.clone();
                                    let error_for_event = error_value.clone();
                                    let _ = rm_task
                                        .emit_run_event(
                                            &run_id_task,
                                            "tool.result",
                                            json!({
                                                "request_id": request_id_for_event,
                                                "tool": rpc_type_for_event,
                                                "actor": actor_for_event,
                                                "ok": ok,
                                                "duration_ms": duration_ms,
                                                "result": result_for_event,
                                                "error": error_for_event
                                            }),
                                        )
                                        .await;

                                    let mut resp_data = json!({
                                        "request_id": request_id_task,
                                        "ok": ok,
                                        "rpc_type": rpc_type_task,
                                    });
                                    if ok {
                                        resp_data["result"] = result_value;
                                    } else {
                                        resp_data["error"] = error_value;
                                    }
                                    let mut resp = WsEnvelope::new("rpc.response", resp_data);
                                    resp.run_id = Some(run_id_task);
                                    if let Ok(text) = serde_json::to_string(&resp) {
                                        let _ = out_tx_task
                                            .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
                                            .await;
                                    }
                                });

                                continue;
                            }

                            let result = if rpc_type_for_exec == "rpc.run.stop" {
                                let signal = data.get("signal").and_then(|v| v.as_str()).unwrap_or("term");
                                match rm.stop_run(run_id, signal).await {
                                    Ok(()) => Ok(Ok(json!({ "signal": signal }))),
                                    Err(err) => Ok(Err((axum::http::StatusCode::BAD_REQUEST, err.to_string()))),
                                }
                            } else if rpc_type_for_exec == "rpc.runs.list" {
                                let runs = rm.list_runs().await;
                                Ok(Ok(json!({ "runs": runs })))
                            } else {
                                tokio::task::spawn_blocking(move || match rpc_type_for_exec.as_str() {
                                    "rpc.fs.read" => {
                                        let path = data.get("path").and_then(|v| v.as_str()).unwrap_or("");
                                        let (content, truncated) =
                                            crate::fs_git::read_utf8_file(&cwd, path, 1024 * 1024)?;
                                        Ok(json!({ "path": path, "content": content, "truncated": truncated }))
                                    }
                                    "rpc.fs.search" => {
                                        let q = data.get("q").and_then(|v| v.as_str()).unwrap_or("");
                                        let (rows, truncated) = crate::fs_git::rg_search(&cwd, q, 200)?;
                                        let matches = rows
                                            .into_iter()
                                            .map(|(path, line, column, text)| {
                                                json!({ "path": path, "line": line, "column": column, "text": text })
                                            })
                                            .collect::<Vec<_>>();
                                        Ok(json!({ "matches": matches, "truncated": truncated }))
                                    }
                                    "rpc.fs.list" => {
                                        let path = data.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                                        let (rows, truncated) = crate::fs_git::list_dir(&cwd, path, 500)?;
                                        let entries = rows
                                            .into_iter()
                                            .map(|(name, is_dir, size_bytes)| {
                                                json!({ "name": name, "is_dir": is_dir, "size_bytes": size_bytes })
                                            })
                                            .collect::<Vec<_>>();
                                        Ok(json!({ "path": path, "entries": entries, "truncated": truncated }))
                                    }
                                    "rpc.git.status" => {
                                        let (stdout, truncated) = crate::fs_git::git_status(&cwd, 200_000)?;
                                        Ok(json!({ "stdout": stdout, "truncated": truncated }))
                                    }
                                    "rpc.git.diff" => {
                                        let path = data.get("path").and_then(|v| v.as_str());
                                        let (stdout, truncated) =
                                            crate::fs_git::git_diff(&cwd, path, 400_000)?;
                                        Ok(json!({ "stdout": stdout, "truncated": truncated }))
                                    }
                                    _ => Err((
                                        axum::http::StatusCode::NOT_IMPLEMENTED,
                                        "unknown rpc type".into(),
                                    )),
                                })
                                .await
                                .map_err(|e| {
                                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                                })
                            };

                            let (ok, payload) = match result {
                                Ok(Ok(v)) => (true, json!({ "result": v })),
                                Ok(Err((_, msg))) => (false, json!({ "error": msg })),
                                Err((_, msg)) => (false, json!({ "error": msg })),
                            };
                            let result_value = payload.get("result").cloned().unwrap_or(serde_json::Value::Null);
                            let error_value = payload.get("error").cloned().unwrap_or(serde_json::Value::Null);
                            let duration_ms = started.elapsed().as_millis() as i64;
                            let _ = rm
                                .emit_run_event(
                                    run_id,
                                    "tool.result",
                                    json!({
                                        "request_id": request_id,
                                        "tool": rpc_type,
                                        "actor": actor,
                                        "ok": ok,
                                        "duration_ms": duration_ms,
                                        "result": result_value,
                                        "error": error_value
                                    }),
                                )
                                .await;

                            let mut resp_data = json!({
                                "request_id": request_id,
                                "ok": ok,
                                "rpc_type": rpc_type,
                            });
                            if let Some(map) = resp_data.as_object_mut() {
                                if let Some(obj) = payload.as_object() {
                                    for (k, v) in obj {
                                        map.insert(k.clone(), v.clone());
                                    }
                                }
                            }

                            let mut resp = WsEnvelope::new("rpc.response", resp_data);
                            resp.run_id = Some(run_id.to_string());
                            let _ = out_tx
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::to_string(&resp)?.into(),
                                ))
                                .await;
                        }
                    }
                    tokio_tungstenite::tungstenite::Message::Ping(p) => {
                        let _ = out_tx.send(tokio_tungstenite::tungstenite::Message::Pong(p)).await;
                    }
                    tokio_tungstenite::tungstenite::Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
