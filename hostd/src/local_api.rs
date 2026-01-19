use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{Mutex, oneshot};

use crate::fs_git;
use crate::run_manager::{RunManager, RunSummary};

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

#[derive(Clone)]
pub struct LocalState {
    pub rm: RunManager,
    pub pending_tool_permissions: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
}

#[derive(Deserialize)]
pub struct StartRunRequest {
    pub tool: String,
    pub cmd: String,
    pub cwd: Option<String>,
}

#[derive(Serialize)]
pub struct StartRunResponse {
    pub run_id: String,
}

#[derive(Deserialize)]
pub struct InputRequest {
    pub input_id: String,
    pub text: String,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Deserialize)]
pub struct StopRequest {
    #[serde(default)]
    pub signal: Option<String>,
}

#[derive(Deserialize)]
pub struct ReadFileQuery {
    pub path: String,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Serialize)]
pub struct ReadFileResponse {
    pub path: String,
    pub content: String,
    pub truncated: bool,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Serialize)]
pub struct SearchMatch {
    pub path: String,
    pub line: i64,
    pub column: i64,
    pub text: String,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub matches: Vec<SearchMatch>,
    pub truncated: bool,
}

#[derive(Serialize)]
pub struct GitTextResponse {
    pub stdout: String,
    pub truncated: bool,
}

async fn start_run(
    State(state): State<Arc<LocalState>>,
    Json(req): Json<StartRunRequest>,
) -> Result<Json<StartRunResponse>, (StatusCode, String)> {
    let cmd = if req.cmd.trim().is_empty() {
        req.tool.clone()
    } else {
        req.cmd
    };
    let run_id = state
        .rm
        .start_run(req.tool, cmd, req.cwd)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(StartRunResponse { run_id }))
}

async fn send_input(
    State(state): State<Arc<LocalState>>,
    Path(run_id): Path<String>,
    Json(req): Json<InputRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let actor = req.actor.as_deref().unwrap_or("cli");
    state
        .rm
        .send_input(&run_id, actor, &req.input_id, &req.text)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn stop_run(
    State(state): State<Arc<LocalState>>,
    Path(run_id): Path<String>,
    Json(req): Json<StopRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let signal = req.signal.as_deref().unwrap_or("term");
    state
        .rm
        .stop_run(&run_id, signal)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_runs(State(state): State<Arc<LocalState>>) -> Json<Vec<RunSummary>> {
    Json(state.rm.list_runs().await)
}

async fn fs_read(
    State(state): State<Arc<LocalState>>,
    Path(run_id): Path<String>,
    Query(q): Query<ReadFileQuery>,
) -> Result<Json<ReadFileResponse>, (StatusCode, String)> {
    let cwd = state
        .rm
        .get_run_cwd(&run_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let actor = q.actor.as_deref().unwrap_or("local");
    const MAX_BYTES: usize = 1024 * 1024;
    let rel = q.path.clone();
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = std::time::Instant::now();
    let _ = state
        .rm
        .emit_run_event(
            &run_id,
            "tool.call",
            json!({
                "request_id": request_id,
                "tool": "fs.read",
                "actor": actor,
                "args": { "path": rel }
            }),
        )
        .await;
    let result = tokio::task::spawn_blocking({
        let cwd = cwd.clone();
        let rel = q.path.clone();
        move || fs_git::read_utf8_file(&cwd, &rel, MAX_BYTES)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    let duration_ms = started.elapsed().as_millis() as i64;
    let (content, truncated) = match result {
        Ok(Ok(v)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "fs.read",
                        "actor": actor,
                        "ok": true,
                        "duration_ms": duration_ms,
                        "result": { "path": q.path, "truncated": v.1 }
                    }),
                )
                .await;
            v
        }
        Ok(Err((_, msg))) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "fs.read",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::BAD_REQUEST, "fs.read failed".into()));
        }
        Err((_, msg)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "fs.read",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, msg));
        }
    };

    Ok(Json(ReadFileResponse {
        path: q.path,
        content,
        truncated,
    }))
}

async fn fs_search(
    State(state): State<Arc<LocalState>>,
    Path(run_id): Path<String>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    let cwd = state
        .rm
        .get_run_cwd(&run_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let qstr = q.q.clone();
    let actor = q.actor.as_deref().unwrap_or("local");
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = std::time::Instant::now();
    let _ = state
        .rm
        .emit_run_event(
            &run_id,
            "tool.call",
            json!({
                "request_id": request_id,
                "tool": "fs.search",
                "actor": actor,
                "args": { "q": qstr }
            }),
        )
        .await;
    let result = tokio::task::spawn_blocking({
        let cwd = cwd.clone();
        let qstr = q.q.clone();
        move || fs_git::rg_search(&cwd, &qstr, 200)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    let duration_ms = started.elapsed().as_millis() as i64;
    let (rows, truncated) = match result {
        Ok(Ok(v)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "fs.search",
                        "actor": actor,
                        "ok": true,
                        "duration_ms": duration_ms,
                        "result": { "truncated": v.1, "count": v.0.len() }
                    }),
                )
                .await;
            v
        }
        Ok(Err((_, msg))) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "fs.search",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::BAD_REQUEST, "fs.search failed".into()));
        }
        Err((_, msg)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "fs.search",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, msg));
        }
    };
    let matches = rows
        .into_iter()
        .map(|(path, line, column, text)| SearchMatch {
            path,
            line,
            column,
            text,
        })
        .collect::<Vec<_>>();

    Ok(Json(SearchResponse { matches, truncated }))
}

#[derive(Deserialize)]
pub struct ActorQuery {
    #[serde(default)]
    pub actor: Option<String>,
}

async fn git_status(
    State(state): State<Arc<LocalState>>,
    Path(run_id): Path<String>,
    Query(q): Query<ActorQuery>,
) -> Result<Json<GitTextResponse>, (StatusCode, String)> {
    let cwd = state
        .rm
        .get_run_cwd(&run_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    const MAX: usize = 200_000;
    let actor = q.actor.as_deref().unwrap_or("local");
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = std::time::Instant::now();
    let _ = state
        .rm
        .emit_run_event(
            &run_id,
            "tool.call",
            json!({
                "request_id": request_id,
                "tool": "git.status",
                "actor": actor,
                "args": {}
            }),
        )
        .await;
    let result = tokio::task::spawn_blocking({
        let cwd = cwd.clone();
        move || fs_git::git_status(&cwd, MAX)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    let duration_ms = started.elapsed().as_millis() as i64;
    let (stdout, truncated) = match result {
        Ok(Ok(v)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "git.status",
                        "actor": actor,
                        "ok": true,
                        "duration_ms": duration_ms,
                        "result": { "truncated": v.1 }
                    }),
                )
                .await;
            v
        }
        Ok(Err((_, msg))) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "git.status",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::BAD_REQUEST, "git.status failed".into()));
        }
        Err((_, msg)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "git.status",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, msg));
        }
    };

    Ok(Json(GitTextResponse { stdout, truncated }))
}

#[derive(Deserialize)]
pub struct GitDiffQuery {
    pub path: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

async fn git_diff(
    State(state): State<Arc<LocalState>>,
    Path(run_id): Path<String>,
    Query(q): Query<GitDiffQuery>,
) -> Result<Json<GitTextResponse>, (StatusCode, String)> {
    let cwd = state
        .rm
        .get_run_cwd(&run_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    const MAX: usize = 400_000;
    let rel = q.path.clone();
    let actor = q.actor.as_deref().unwrap_or("local");
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = std::time::Instant::now();
    let _ = state
        .rm
        .emit_run_event(
            &run_id,
            "tool.call",
            json!({
                "request_id": request_id,
                "tool": "git.diff",
                "actor": actor,
                "args": { "path": rel }
            }),
        )
        .await;
    let result = tokio::task::spawn_blocking({
        let cwd = cwd.clone();
        let rel = q.path.clone();
        move || fs_git::git_diff(&cwd, rel.as_deref(), MAX)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    let duration_ms = started.elapsed().as_millis() as i64;
    let (stdout, truncated) = match result {
        Ok(Ok(v)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "git.diff",
                        "actor": actor,
                        "ok": true,
                        "duration_ms": duration_ms,
                        "result": { "truncated": v.1 }
                    }),
                )
                .await;
            v
        }
        Ok(Err((_, msg))) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "git.diff",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::BAD_REQUEST, "git.diff failed".into()));
        }
        Err((_, msg)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "git.diff",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, msg));
        }
    };

    Ok(Json(GitTextResponse { stdout, truncated }))
}

#[derive(Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Serialize)]
pub struct WriteFileResponse {
    pub path: String,
    pub bytes_written: i64,
    pub truncated: bool,
}

async fn fs_write(
    State(state): State<Arc<LocalState>>,
    Path(run_id): Path<String>,
    Json(req): Json<WriteFileRequest>,
) -> Result<Json<WriteFileResponse>, (StatusCode, String)> {
    let cwd = state
        .rm
        .get_run_cwd(&run_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let actor = req.actor.as_deref().unwrap_or("local");
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = std::time::Instant::now();

    let bytes = req.content.as_bytes().len() as i64;
    let preview_limit = 2000;
    let preview_raw = truncate_chars(&req.content, preview_limit);
    let content_preview = state.rm.redact_string(&preview_raw);
    let content_truncated = req.content.chars().count() > preview_limit;
    let rel = req.path.clone();
    let args_for_event = json!({
        "path": rel.clone(),
        "bytes": bytes,
        "content_preview": content_preview,
        "content_truncated": content_truncated
    });

    let _ = state
        .rm
        .emit_run_event(
            &run_id,
            "tool.call",
            json!({
                "request_id": request_id,
                "tool": "rpc.fs.write",
                "actor": actor,
                "args": args_for_event.clone()
            }),
        )
        .await;

    let op_args_summary = truncate_chars(&format!("path={} bytes={}", req.path, bytes), 80);
    let prompt = if op_args_summary.trim().is_empty() {
        "需要审批：rpc.fs.write".to_string()
    } else {
        format!("需要审批：rpc.fs.write {op_args_summary}")
    };

    let key = format!("{run_id}:{request_id}");
    let (tx, rx) = oneshot::channel::<bool>();
    {
        let mut map = state.pending_tool_permissions.lock().await;
        map.insert(key.clone(), tx);
    }
    let _ = state
        .rm
        .emit_run_event(
            &run_id,
            "run.permission_requested",
            json!({
                "request_id": request_id,
                "reason": "permission",
                "prompt": prompt,
                "op_tool": "rpc.fs.write",
                "op_args": args_for_event,
                "op_args_summary": op_args_summary,
                "approve_text": "",
                "deny_text": ""
            }),
        )
        .await;

    let approved = match tokio::time::timeout(Duration::from_secs(600), rx).await {
        Ok(Ok(v)) => v,
        Ok(Err(_)) => false,
        Err(_) => {
            let mut map = state.pending_tool_permissions.lock().await;
            map.remove(&key);
            let duration_ms = started.elapsed().as_millis() as i64;
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "rpc.fs.write",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": "timeout"
                    }),
                )
                .await;
            return Err((StatusCode::REQUEST_TIMEOUT, "timeout".into()));
        }
    };

    if !approved {
        let duration_ms = started.elapsed().as_millis() as i64;
        let _ = state
            .rm
            .emit_run_event(
                &run_id,
                "tool.result",
                json!({
                    "request_id": request_id,
                    "tool": "rpc.fs.write",
                    "actor": actor,
                    "ok": false,
                    "duration_ms": duration_ms,
                    "error": "denied"
                }),
            )
            .await;
        return Err((StatusCode::FORBIDDEN, "denied".into()));
    }

    let rel_for_exec = rel.clone();
    let content = req.content.clone();
    let result = tokio::task::spawn_blocking(move || {
        crate::fs_git::write_utf8_file(&cwd, &rel_for_exec, &content, 1024 * 1024)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    let duration_ms = started.elapsed().as_millis() as i64;
    let (bytes_written, truncated) = match result {
        Ok(Ok(v)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "rpc.fs.write",
                        "actor": actor,
                        "ok": true,
                        "duration_ms": duration_ms,
                        "result": { "path": rel, "bytes_written": v.0, "truncated": v.1 }
                    }),
                )
                .await;
            v
        }
        Ok(Err((_, msg))) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "rpc.fs.write",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::BAD_REQUEST, "fs.write failed".into()));
        }
        Err((_, msg)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "rpc.fs.write",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, msg));
        }
    };

    Ok(Json(WriteFileResponse {
        path: req.path,
        bytes_written,
        truncated,
    }))
}

#[derive(Deserialize)]
pub struct BashRequest {
    pub cmd: String,
    #[serde(default)]
    pub actor: Option<String>,
}

#[derive(Serialize)]
pub struct BashResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
    pub truncated: bool,
}

async fn bash_run(
    State(state): State<Arc<LocalState>>,
    Path(run_id): Path<String>,
    Json(req): Json<BashRequest>,
) -> Result<Json<BashResponse>, (StatusCode, String)> {
    let cwd = state
        .rm
        .get_run_cwd(&run_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let actor = req.actor.as_deref().unwrap_or("local");
    let request_id = uuid::Uuid::new_v4().to_string();
    let started = std::time::Instant::now();

    let cmd_redacted = state.rm.redact_string(&req.cmd);
    let args_for_event = json!({ "cmd": cmd_redacted.clone() });
    let _ = state
        .rm
        .emit_run_event(
            &run_id,
            "tool.call",
            json!({
                "request_id": request_id,
                "tool": "rpc.bash",
                "actor": actor,
                "args": args_for_event.clone()
            }),
        )
        .await;

    let op_args_summary = truncate_chars(&format!("cmd={cmd_redacted}"), 80);
    let prompt = if op_args_summary.trim().is_empty() {
        "需要审批：bash".to_string()
    } else {
        format!("需要审批：bash {op_args_summary}")
    };

    let key = format!("{run_id}:{request_id}");
    let (tx, rx) = oneshot::channel::<bool>();
    {
        let mut map = state.pending_tool_permissions.lock().await;
        map.insert(key.clone(), tx);
    }
    let _ = state
        .rm
        .emit_run_event(
            &run_id,
            "run.permission_requested",
            json!({
                "request_id": request_id,
                "reason": "permission",
                "prompt": prompt,
                "op_tool": "bash",
                "op_args": args_for_event,
                "op_args_summary": op_args_summary,
                "approve_text": "",
                "deny_text": ""
            }),
        )
        .await;

    let approved = match tokio::time::timeout(Duration::from_secs(600), rx).await {
        Ok(Ok(v)) => v,
        Ok(Err(_)) => false,
        Err(_) => {
            let mut map = state.pending_tool_permissions.lock().await;
            map.remove(&key);
            let duration_ms = started.elapsed().as_millis() as i64;
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "rpc.bash",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": "timeout"
                    }),
                )
                .await;
            return Err((StatusCode::REQUEST_TIMEOUT, "timeout".into()));
        }
    };

    if !approved {
        let duration_ms = started.elapsed().as_millis() as i64;
        let _ = state
            .rm
            .emit_run_event(
                &run_id,
                "tool.result",
                json!({
                    "request_id": request_id,
                    "tool": "rpc.bash",
                    "actor": actor,
                    "ok": false,
                    "duration_ms": duration_ms,
                    "error": "denied"
                }),
            )
            .await;
        return Err((StatusCode::FORBIDDEN, "denied".into()));
    }

    let cmd = req.cmd.clone();
    let result =
        tokio::task::spawn_blocking(move || crate::fs_git::bash_exec(&cwd, &cmd, 200_000, 200_000))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    let duration_ms = started.elapsed().as_millis() as i64;

    let (stdout, stderr, exit_code, truncated) = match result {
        Ok(Ok(v)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "rpc.bash",
                        "actor": actor,
                        "ok": true,
                        "duration_ms": duration_ms,
                        "result": { "exit_code": v.2, "truncated": v.3, "stdout_len": v.0.len(), "stderr_len": v.1.len() }
                    }),
                )
                .await;
            v
        }
        Ok(Err((_, msg))) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "rpc.bash",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::BAD_REQUEST, "bash failed".into()));
        }
        Err((_, msg)) => {
            let _ = state
                .rm
                .emit_run_event(
                    &run_id,
                    "tool.result",
                    json!({
                        "request_id": request_id,
                        "tool": "rpc.bash",
                        "actor": actor,
                        "ok": false,
                        "duration_ms": duration_ms,
                        "error": msg
                    }),
                )
                .await;
            return Err((StatusCode::INTERNAL_SERVER_ERROR, msg));
        }
    };

    Ok(Json(BashResponse {
        stdout,
        stderr,
        exit_code,
        truncated,
    }))
}

pub fn router(state: Arc<LocalState>) -> Router {
    Router::new()
        .route("/runs", post(start_run).get(list_runs))
        .route("/runs/:run_id/input", post(send_input))
        .route("/runs/:run_id/stop", post(stop_run))
        .route("/runs/:run_id/fs/read", get(fs_read))
        .route("/runs/:run_id/fs/search", get(fs_search))
        .route("/runs/:run_id/fs/write", post(fs_write))
        .route("/runs/:run_id/git/status", get(git_status))
        .route("/runs/:run_id/git/diff", get(git_diff))
        .route("/runs/:run_id/bash", post(bash_run))
        .with_state(state)
}
