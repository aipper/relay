use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::fs_git;
use crate::run_manager::{RunManager, RunSummary};

#[derive(Clone)]
pub struct LocalState {
    pub rm: RunManager,
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
                "actor": "local",
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
                        "actor": "local",
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
                        "actor": "local",
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
                        "actor": "local",
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
                "actor": "local",
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
                        "actor": "local",
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
                        "actor": "local",
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
                        "actor": "local",
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

async fn git_status(
    State(state): State<Arc<LocalState>>,
    Path(run_id): Path<String>,
) -> Result<Json<GitTextResponse>, (StatusCode, String)> {
    let cwd = state
        .rm
        .get_run_cwd(&run_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    const MAX: usize = 200_000;
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
                "actor": "local",
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
                        "actor": "local",
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
                        "actor": "local",
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
                        "actor": "local",
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
                "actor": "local",
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
                        "actor": "local",
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
                        "actor": "local",
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
                        "actor": "local",
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

pub fn router(state: Arc<LocalState>) -> Router {
    Router::new()
        .route("/runs", post(start_run).get(list_runs))
        .route("/runs/:run_id/input", post(send_input))
        .route("/runs/:run_id/stop", post(stop_run))
        .route("/runs/:run_id/fs/read", get(fs_read))
        .route("/runs/:run_id/fs/search", get(fs_search))
        .route("/runs/:run_id/git/status", get(git_status))
        .route("/runs/:run_id/git/diff", get(git_diff))
        .with_state(state)
}
