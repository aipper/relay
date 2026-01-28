mod config;
mod db;

use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use argon2::password_hash::SaltString;
use axum::{
    Json, Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use futures_util::{SinkExt, StreamExt};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use rand_core::OsRng;
use relay_protocol::{WsEnvelope, redaction::Redactor};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::Executor;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration as StdDuration, Instant},
};
use tokio::sync::broadcast;
use tokio::sync::{RwLock, mpsc};
use tracing_subscriber::prelude::*;

#[derive(Serialize)]
struct HealthResponse {
    name: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        name: "relay-server",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[derive(Clone)]
struct AppState {
    cfg: config::Config,
    db: db::Db,
    app_tx: broadcast::Sender<WsEnvelope>,
    jwt_encoding: EncodingKey,
    jwt_decoding: DecodingKey,
    redactor: Arc<Redactor>,
    hosts_tx: Arc<RwLock<HashMap<String, mpsc::Sender<Message>>>>,
    run_to_host: Arc<RwLock<HashMap<String, String>>>,
    web_dist_dir: Option<std::path::PathBuf>,
    server_log_path: Option<std::path::PathBuf>,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    access_token: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    if req.username != state.cfg.admin_username {
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".into()));
    }

    let parsed_hash = argon2::PasswordHash::new(&state.cfg.admin_password_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "bad password hash".into(),
        )
    })?;
    argon2::Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid credentials".into()))?;

    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: "admin".into(),
        exp,
    };
    let token =
        jsonwebtoken::encode(&Header::default(), &claims, &state.jwt_encoding).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "token encode failed".into(),
            )
        })?;

    Ok(Json(LoginResponse {
        access_token: token,
    }))
}

#[derive(Deserialize)]
struct WsAuthQuery {
    token: Option<String>,
    host_token: Option<String>,
    host_id: Option<String>,
}

#[derive(Deserialize)]
struct SendInputBody {
    input_id: String,
    text: String,
    #[serde(default)]
    actor: Option<String>,
}

async fn http_send_input(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(run_id): axum::extract::Path<String>,
    Json(body): Json<SendInputBody>,
) -> impl IntoResponse {
    let Some(token) = bearer_token(&headers) else {
        return (StatusCode::UNAUTHORIZED, "missing bearer token").into_response();
    };
    if validate_jwt(&state, &token).is_err() {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }

    let host_id = {
        let map = state.run_to_host.read().await;
        map.get(&run_id).cloned()
    };
    let Some(host_id) = host_id else {
        return (StatusCode::NOT_FOUND, "unknown run_id").into_response();
    };

    let actor = body.actor.as_deref().unwrap_or("web");
    let data = serde_json::json!({
        "input_id": body.input_id,
        "actor": actor,
        "text": body.text
    });
    let mut cmd = WsEnvelope::new("run.send_input", data);
    cmd.host_id = Some(host_id.clone());
    cmd.run_id = Some(run_id);

    let payload = match serde_json::to_string(&cmd) {
        Ok(p) => p,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "encode failed").into_response(),
    };

    let tx = {
        let hosts = state.hosts_tx.read().await;
        hosts.get(&host_id).cloned()
    };
    if let Some(tx) = tx {
        let _ = tx.send(Message::Text(payload)).await;
        return StatusCode::NO_CONTENT.into_response();
    }

    (StatusCode::BAD_GATEWAY, "host offline").into_response()
}

async fn http_list_runs(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(token) = bearer_token(&headers) else {
        return (StatusCode::UNAUTHORIZED, "missing bearer token").into_response();
    };
    if validate_jwt(&state, &token).is_err() {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }

    match db::list_runs(&state.db).await {
        Ok(rows) => Json(rows).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

async fn http_list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // MVP: sessions are backed by the runs table. Expose a stable semantic alias to align with
    // session-based clients (e.g. hapi/happy) while keeping existing /runs compatibility.
    http_list_runs(State(state), headers).await
}

#[derive(Deserialize)]
struct SessionsQuery {
    #[serde(default)]
    limit: Option<i64>,
}

async fn http_list_recent_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SessionsQuery>,
) -> impl IntoResponse {
    let Some(token) = bearer_token(&headers) else {
        return (StatusCode::UNAUTHORIZED, "missing bearer token").into_response();
    };
    if validate_jwt(&state, &token).is_err() {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }
    let limit = q.limit.unwrap_or(50);
    match db::list_recent_runs(&state.db, limit).await {
        Ok(rows) => Json(rows).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

async fn http_get_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let Some(token) = bearer_token(&headers) else {
        return (StatusCode::UNAUTHORIZED, "missing bearer token").into_response();
    };
    if validate_jwt(&state, &token).is_err() {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }
    match db::get_run(&state.db, &session_id).await {
        Ok(Some(row)) => Json(row).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "unknown session_id").into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

#[derive(Serialize)]
struct HostInfo {
    id: String,
    name: Option<String>,
    last_seen_at: Option<String>,
    online: bool,
}

async fn http_list_hosts(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(token) = bearer_token(&headers) else {
        return (StatusCode::UNAUTHORIZED, "missing bearer token").into_response();
    };
    if validate_jwt(&state, &token).is_err() {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }

    let online = {
        let hosts = state.hosts_tx.read().await;
        hosts
            .keys()
            .cloned()
            .collect::<std::collections::HashSet<_>>()
    };

    match db::list_hosts(&state.db).await {
        Ok(rows) => {
            let out = rows
                .into_iter()
                .map(|h| HostInfo {
                    online: online.contains(&h.id),
                    id: h.id,
                    name: h.name,
                    last_seen_at: h.last_seen_at,
                })
                .collect::<Vec<_>>();
            Json(out).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct LogsTailQuery {
    #[serde(default)]
    lines: Option<i64>,
    #[serde(default)]
    max_bytes: Option<i64>,
}

#[derive(Serialize)]
struct LogsTailResponse {
    path: String,
    text: String,
    truncated: bool,
}

async fn http_server_logs_tail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<LogsTailQuery>,
) -> impl IntoResponse {
    let Some(token) = bearer_token(&headers) else {
        return (StatusCode::UNAUTHORIZED, "missing bearer token").into_response();
    };
    if validate_jwt(&state, &token).is_err() {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }

    let Some(path) = state.server_log_path.clone() else {
        return (
            StatusCode::NOT_FOUND,
            "server log file is not enabled (set SERVER_LOG_PATH or run with DATABASE_URL under /data)",
        )
            .into_response();
    };

    let lines = q.lines.unwrap_or(200).clamp(1, 2000) as usize;
    let max_bytes = q.max_bytes.unwrap_or(200_000).clamp(1, 2_000_000) as usize;

    let read = tokio::task::spawn_blocking(move || tail_log_file(&path, lines, max_bytes)).await;
    let Ok(read) = read else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "failed to read log file").into_response();
    };
    match read {
        Ok(out) => Json(out).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct MessagesQuery {
    before_id: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
struct ChatMessage {
    id: i64,
    seq: Option<i64>,
    ts: String,
    role: &'static str,
    kind: String,
    actor: Option<String>,
    request_id: Option<String>,
    text: String,
}

fn truncate_text(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let mut out = String::with_capacity(std::cmp::min(s.len(), max_chars));
    for (i, ch) in s.chars().enumerate() {
        if i >= max_chars {
            out.push_str("…");
            break;
        }
        out.push(ch);
    }
    out
}

fn json_compact(v: &JsonValue) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "null".to_string())
}

async fn http_list_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(run_id): axum::extract::Path<String>,
    Query(q): Query<MessagesQuery>,
) -> impl IntoResponse {
    let Some(token) = bearer_token(&headers) else {
        return (StatusCode::UNAUTHORIZED, "missing bearer token").into_response();
    };
    if validate_jwt(&state, &token).is_err() {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }

    let limit = q.limit.unwrap_or(200);
    let rows = match db::list_message_events(&state.db, &run_id, q.before_id, limit).await {
        Ok(r) => r,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    };

    let mut out = Vec::with_capacity(rows.len());
    for row in rows.into_iter().rev() {
        let (role, text, request_id) = match row.r#type.as_str() {
            "run.output" => ("assistant", row.text.unwrap_or_default(), None),
            "run.input" => (
                "user",
                row.text_redacted.or(row.text).unwrap_or_default(),
                row.input_id.clone(),
            ),
            "run.permission_requested" => {
                let parsed = row
                    .data_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<JsonValue>(s).ok())
                    .unwrap_or(JsonValue::Null);
                let prompt = parsed
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let req = parsed
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                ("system", prompt, req)
            }
            "tool.call" => {
                let parsed = row
                    .data_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<JsonValue>(s).ok())
                    .unwrap_or(JsonValue::Null);
                let tool = parsed
                    .get("tool")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let req = parsed
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let args = parsed.get("args").unwrap_or(&JsonValue::Null);
                let args = truncate_text(&json_compact(args), 2000);
                ("system", format!("tool.call {tool} {args}"), req)
            }
            "tool.result" => {
                let parsed = row
                    .data_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<JsonValue>(s).ok())
                    .unwrap_or(JsonValue::Null);
                let tool = parsed
                    .get("tool")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let req = parsed
                    .get("request_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let ok = parsed.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                let duration_ms = parsed
                    .get("duration_ms")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let mut text = format!("tool.result {tool} ok={ok} duration_ms={duration_ms}");
                if ok {
                    if let Some(result) = parsed.get("result") {
                        let res = truncate_text(&json_compact(result), 2000);
                        text.push(' ');
                        text.push_str(&res);
                    }
                } else {
                    let err = parsed
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error");
                    text.push(' ');
                    text.push_str(&truncate_text(err, 2000));
                }
                ("system", text, req)
            }
            "run.started" => ("system", "run started".to_string(), None),
            "run.exited" => ("system", "run exited".to_string(), None),
            _ => continue,
        };

        out.push(ChatMessage {
            id: row.id,
            seq: row.seq,
            ts: row.ts,
            role,
            kind: row.r#type,
            actor: row.actor,
            request_id,
            text,
        });
    }

    Json(out).into_response()
}

async fn http_list_session_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(session_id): axum::extract::Path<String>,
    Query(q): Query<MessagesQuery>,
) -> impl IntoResponse {
    http_list_messages(
        State(state),
        headers,
        axum::extract::Path(session_id),
        Query(q),
    )
    .await
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let v = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let v = v.strip_prefix("Bearer ")?;
    Some(v.to_string())
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn tofu_register_or_verify_host(
    db: &db::Db,
    host_id: &str,
    host_token: &str,
) -> anyhow::Result<bool> {
    let host_id = host_id.trim();
    let host_token = host_token.trim();
    anyhow::ensure!(!host_id.is_empty(), "empty host_id");
    anyhow::ensure!(!host_token.is_empty(), "empty host_token");

    let token_hash = sha256_hex(host_token);
    let now = Utc::now().to_rfc3339();

    // TOFU: first connection "claims" host_id by inserting token_hash.
    // Subsequent connections must match the stored token_hash.
    let inserted = sqlx::query(
        r#"
INSERT INTO hosts (id, token_hash, last_seen_at) VALUES (?1, ?2, ?3)
ON CONFLICT(id) DO NOTHING
"#,
    )
    .bind(host_id)
    .bind(&token_hash)
    .bind(&now)
    .execute(db)
    .await?
    .rows_affected()
        == 1;

    let stored_hash = sqlx::query_scalar::<_, String>("SELECT token_hash FROM hosts WHERE id=?1")
        .bind(host_id)
        .fetch_optional(db)
        .await?;
    let Some(stored_hash) = stored_hash else {
        anyhow::bail!("missing host record after TOFU insert");
    };

    anyhow::ensure!(stored_hash == token_hash, "host_token mismatch");
    Ok(inserted)
}

async fn ws_app(
    State(state): State<AppState>,
    Query(q): Query<WsAuthQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if q.token
        .as_deref()
        .and_then(|t| validate_jwt(&state, t).ok())
        .is_none()
    {
        return (StatusCode::UNAUTHORIZED, "missing/invalid token").into_response();
    }

    ws.on_upgrade(move |socket| handle_app_socket(state, socket))
}

async fn ws_host(
    State(state): State<AppState>,
    Query(q): Query<WsAuthQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let (Some(host_id), Some(host_token)) = (q.host_id.clone(), q.host_token.clone()) else {
        return (
            StatusCode::UNAUTHORIZED,
            "missing host_id/host_token query params",
        )
            .into_response();
    };

    let inserted = match tofu_register_or_verify_host(&state.db, &host_id, &host_token).await {
        Ok(inserted) => inserted,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("mismatch") {
                tracing::warn!(%host_id, "host auth failed: token mismatch");
                return (StatusCode::UNAUTHORIZED, "invalid host_id/host_token").into_response();
            }
            tracing::error!(%host_id, error=?e, "host auth failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "host auth failed".to_string(),
            )
                .into_response();
        }
    };

    if inserted {
        tracing::info!(%host_id, "host registered (TOFU)");
    } else {
        tracing::info!(%host_id, "host authenticated");
    }

    ws.on_upgrade(move |socket| handle_host_socket(state, socket, host_id, host_token))
}

fn validate_jwt(state: &AppState, token: &str) -> anyhow::Result<Claims> {
    let claims =
        jsonwebtoken::decode::<Claims>(token, &state.jwt_decoding, &Validation::default())?.claims;
    Ok(claims)
}

async fn handle_app_socket(state: AppState, mut socket: WebSocket) {
    let mut rx = state.app_tx.subscribe();

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(env) => {
                        let Ok(text) = serde_json::to_string(&env) else { continue; };
                        if socket.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            incoming = socket.recv() => {
                let Some(Ok(incoming)) = incoming else { break; };
                match incoming {
                    Message::Text(text) => {
                        let Ok(env) = serde_json::from_str::<WsEnvelope>(&text) else { continue; };
                        let is_rpc = env.r#type.starts_with("rpc.");
                        if env.r#type != "run.send_input"
                            && env.r#type != "run.send_stdin"
                            && env.r#type != "run.stop"
                            && env.r#type != "run.resize"
                            && env.r#type != "run.permission.approve"
                            && env.r#type != "run.permission.deny"
                            && !is_rpc
                        {
                            continue;
                        }
                        let (host_id, run_id) = if env.r#type == "rpc.run.start"
                            || env.r#type == "rpc.host.info"
                            || env.r#type == "rpc.host.doctor"
                            || env.r#type == "rpc.host.capabilities"
                            || env.r#type == "rpc.host.logs.tail"
                        {
                            let host_id =
                                env.data.get("host_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let Some(host_id) = host_id else { continue; };
                            (host_id, None)
                        } else {
                            let Some(run_id) = env.run_id.clone() else { continue; };
                            // Prefer in-memory map, but fall back to the DB (useful after server restarts,
                            // because the in-memory map is not persisted).
                            let mut host_id = {
                                let map = state.run_to_host.read().await;
                                map.get(&run_id).cloned()
                            };
                            if host_id.is_none() {
                                host_id = sqlx::query_scalar::<_, String>(
                                    "SELECT host_id FROM runs WHERE id=?1",
                                )
                                .bind(&run_id)
                                .fetch_optional(&state.db)
                                .await
                                .ok()
                                .flatten();
                            }
                            let Some(host_id) = host_id else { continue; };
                            {
                                // Cache for subsequent inputs.
                                let mut map = state.run_to_host.write().await;
                                map.insert(run_id.clone(), host_id.clone());
                            }
                            (host_id, Some(run_id))
                        };

                        let mut cmd = WsEnvelope::new(env.r#type.clone(), env.data.clone());
                        cmd.host_id = Some(host_id.clone());
                        cmd.run_id = run_id;

                        let payload = match serde_json::to_string(&cmd) {
                            Ok(p) => p,
                            Err(_) => continue,
                        };

                        let tx = {
                            let hosts = state.hosts_tx.read().await;
                            hosts.get(&host_id).cloned()
                        };
                        if let Some(tx) = tx {
                            let _ = tx.send(Message::Text(payload)).await;
                        }
                    }
                    Message::Close(_) => break,
                    Message::Ping(p) => {
                        let _ = socket.send(Message::Pong(p)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn handle_host_socket(
    state: AppState,
    socket: WebSocket,
    host_id: String,
    host_token: String,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Message>(256);
    let tx_for_internal = tx.clone();

    {
        let mut hosts = state.hosts_tx.write().await;
        hosts.insert(host_id.clone(), tx);
    }

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Basic last_seen update loop.
    let update_seen = async {
        let now = Utc::now().to_rfc3339();
        let token_hash = { sha256_hex(&host_token) };
        let _ = sqlx::query(
            r#"
INSERT INTO hosts (id, token_hash, last_seen_at) VALUES (?1, ?2, ?3)
ON CONFLICT(id) DO UPDATE SET last_seen_at=excluded.last_seen_at
"#,
        )
        .bind(&host_id)
        .bind(&token_hash)
        .bind(&now)
        .execute(&state.db)
        .await;
    };
    update_seen.await;

    let seen_update_interval = StdDuration::from_secs(5);
    let run_touch_interval = StdDuration::from_secs(1);
    let mut last_seen_written_at = Instant::now();
    let mut last_active_written_by_run: HashMap<String, Instant> = HashMap::new();

    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                let Ok(env) = serde_json::from_str::<WsEnvelope>(&text) else {
                    continue;
                };
                let now_inst = Instant::now();
                if now_inst.duration_since(last_seen_written_at) >= seen_update_interval {
                    let _ = sqlx::query("UPDATE hosts SET last_seen_at=?2 WHERE id=?1")
                        .bind(&host_id)
                        .bind(Utc::now().to_rfc3339())
                        .execute(&state.db)
                        .await;
                    last_seen_written_at = now_inst;
                }
                let run_id = env.run_id.clone().unwrap_or_else(|| "unknown".into());
                let seq = env.seq;

                // Keep an always-fresh run_id -> host_id mapping so app-side inputs can be routed
                // even after a relay-server restart (mapping is in-memory only).
                if run_id != "unknown" {
                    let mut map = state.run_to_host.write().await;
                    map.insert(run_id.clone(), host_id.clone());
                }

                if env.r#type == "run.started" {
                    let tool = env
                        .data
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let cwd = env.data.get("cwd").and_then(|v| v.as_str()).unwrap_or(".");
                    let _ = db::upsert_run_started(&state.db, &run_id, &host_id, tool, cwd, env.ts)
                        .await;
                } else if env.r#type == "run.awaiting_input" {
                    let has_request_id = env
                        .data
                        .get("request_id")
                        .and_then(|v| v.as_str())
                        .is_some();
                    if has_request_id {
                        let _ = db::mark_run_awaiting_approval(&state.db, &run_id, env.ts).await;
                    } else {
                        let _ = db::mark_run_awaiting_input(&state.db, &run_id, env.ts).await;
                    }
                } else if env.r#type == "run.permission_requested" {
                    let request_id = env.data.get("request_id").and_then(|v| v.as_str());
                    if let Some(request_id) = request_id {
                        let reason = env.data.get("reason").and_then(|v| v.as_str());
                        let prompt = env.data.get("prompt").and_then(|v| v.as_str());
                        let op_tool = env.data.get("op_tool").and_then(|v| v.as_str());
                        let mut op_args_summary = env
                            .data
                            .get("op_args_summary")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        if op_args_summary.is_none() {
                            if let Some(op_args) = env.data.get("op_args") {
                                if !op_args.is_null() {
                                    let s = truncate_text(&json_compact(op_args), 80);
                                    if s != "null" && !s.is_empty() {
                                        op_args_summary = Some(s);
                                    }
                                }
                            }
                        }
                        let _ = db::set_run_pending_permission(
                            &state.db,
                            &run_id,
                            env.ts,
                            request_id,
                            reason,
                            prompt,
                            op_tool,
                            op_args_summary.as_deref(),
                        )
                        .await;
                    }
                } else if env.r#type == "run.exited" {
                    let exit_code = env
                        .data
                        .get("exit_code")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(-1);
                    let _ = db::finish_run(&state.db, &run_id, env.ts, exit_code).await;
                } else if env.r#type == "run.input" {
                    let _ = db::mark_run_running(&state.db, &run_id, env.ts).await;
                } else if env.r#type == "tool.result" {
                    if let Some(request_id) = env.data.get("request_id").and_then(|v| v.as_str()) {
                        let _ = db::clear_run_pending_permission_by_request_id(
                            &state.db, &run_id, env.ts, request_id,
                        )
                        .await;
                    }
                } else if env.r#type == "rpc.response" && run_id != "unknown" {
                    let mut map = state.run_to_host.write().await;
                    map.insert(run_id.clone(), host_id.clone());
                }

                if run_id != "unknown" {
                    let should_touch = match last_active_written_by_run.get(&run_id) {
                        Some(last) => now_inst.duration_since(*last) >= run_touch_interval,
                        None => true,
                    };
                    if should_touch {
                        let _ = db::touch_run_last_active(&state.db, &run_id, env.ts).await;
                        last_active_written_by_run.insert(run_id.clone(), now_inst);
                    }
                }

                // Persist minimal event. Skip RPC responses that don't belong to any run to avoid polluting
                // the DB with an "unknown" run_id.
                let should_persist = !(env.run_id.is_none() && env.r#type == "rpc.response");
                if should_persist {
                    let data_json = serde_json::to_string(&env.data).ok();
                    let _ = db::insert_event(
                        &state.db,
                        &run_id,
                        seq,
                        env.ts,
                        &env.r#type,
                        env.data.get("stream").and_then(|v| v.as_str()),
                        env.data.get("actor").and_then(|v| v.as_str()),
                        env.data.get("input_id").and_then(|v| v.as_str()),
                        env.data.get("text").and_then(|v| v.as_str()),
                        env.data.get("text_redacted").and_then(|v| v.as_str()),
                        env.data.get("text_sha256").and_then(|v| v.as_str()),
                        data_json.as_deref(),
                    )
                    .await;
                }

                // Ack to host for spool replay.
                if should_persist {
                    if let Some(last_seq) = seq {
                        let ack = WsEnvelope::new(
                            "run.ack",
                            serde_json::json!({
                                "run_id": run_id,
                                "last_seq": last_seq
                            }),
                        );
                        if let Ok(payload) = serde_json::to_string(&ack) {
                            let _ = tx_for_internal.send(Message::Text(payload)).await;
                        }
                    }
                }

                // Fan-out to apps.
                let mut broadcast_env = env;
                broadcast_env.host_id = Some(host_id.clone());
                let _ = state.app_tx.send(broadcast_env);
            }
            Message::Binary(_) => {}
            Message::Ping(p) => {
                let _ = tx_for_internal.send(Message::Pong(p)).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    {
        let mut hosts = state.hosts_tx.write().await;
        hosts.remove(&host_id);
    }
    send_task.abort();
}

async fn http_static_fallback(
    State(state): State<AppState>,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
) -> impl IntoResponse {
    let Some(root) = state.web_dist_dir.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    // Basic traversal prevention: reject any parent-dir component.
    if path.split('/').any(|c| c == "..") {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let full = root.join(&path);
    let mut served_path = path.clone();
    let data = match tokio::fs::read(&full).await {
        Ok(d) => d,
        Err(_) => {
            if path != "index.html" {
                let index = root.join("index.html");
                match tokio::fs::read(&index).await {
                    Ok(d) => {
                        served_path = "index.html".to_string();
                        d
                    }
                    Err(_) => return StatusCode::NOT_FOUND.into_response(),
                }
            } else {
                return StatusCode::NOT_FOUND.into_response();
            }
        }
    };

    let mime = match std::path::Path::new(&served_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        "map" => "application/json; charset=utf-8",
        "webmanifest" => "application/manifest+json; charset=utf-8",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    };

    let mut resp = axum::response::Response::new(axum::body::Body::from(data));
    resp.headers_mut()
        .insert(axum::http::header::CONTENT_TYPE, mime.parse().unwrap());
    // Avoid caching index.html so that deploys pick up new hashed asset URLs.
    // Cache hashed assets aggressively for performance.
    let cache_control = if served_path == "index.html" {
        "no-store"
    } else if served_path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    };
    resp.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        cache_control.parse().unwrap(),
    );
    resp
}

fn detect_server_log_path() -> Option<std::path::PathBuf> {
    let env = std::env::var("SERVER_LOG_PATH")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(v) = env {
        return Some(std::path::PathBuf::from(v));
    }

    // Default: in docker we store SQLite under /data, so write logs alongside it.
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
    if database_url.contains("/data/") && std::path::Path::new("/data").is_dir() {
        return Some(std::path::PathBuf::from("/data/relay-server.log"));
    }
    None
}

fn init_tracing(server_log_path: Option<std::path::PathBuf>) -> Option<std::path::PathBuf> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("relay_server=info"));

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(std::io::stdout);

    if let Some(path) = server_log_path.clone() {
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(file) => {
                let path2 = path.clone();
                let file_writer = move || {
                    file.try_clone()
                        .or_else(|_| {
                            std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&path2)
                        })
                        .unwrap_or_else(|_| std::fs::File::open("/dev/null").unwrap())
                };
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(file_writer);
                tracing_subscriber::registry()
                    .with(filter)
                    .with(stdout_layer)
                    .with(file_layer)
                    .init();
                return Some(path);
            }
            Err(err) => {
                eprintln!(
                    "failed to open SERVER_LOG_PATH {}: {err}; logs will go to stdout only",
                    path.display()
                );
            }
        }
    }

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .init();
    None
}

fn tail_log_file(
    path: &std::path::Path,
    lines: usize,
    max_bytes: usize,
) -> anyhow::Result<LogsTailResponse> {
    use std::io::{Read, Seek, SeekFrom};

    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    let start = len.saturating_sub(max_bytes as u64);
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let truncated = len > max_bytes as u64;

    let text = String::from_utf8_lossy(&buf).to_string();
    let mut parts = text.lines().collect::<Vec<_>>();
    if parts.len() > lines {
        parts = parts[parts.len() - lines..].to_vec();
    }
    let out_text = parts.join("\n");

    Ok(LogsTailResponse {
        path: path.display().to_string(),
        text: out_text,
        truncated,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() == 2 && args[0] == "--hash-password" {
        let password = args.remove(1);
        let salt = SaltString::generate(&mut OsRng);
        let hash = argon2::Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .to_string();
        println!("{hash}");
        return Ok(());
    }

    let server_log_path = init_tracing(detect_server_log_path());

    let cfg = config::Config::from_env()?;
    let bind_addr = cfg.bind_addr.clone();

    let db = db::connect(&cfg.database_url).await?;
    db::init(&db).await?;

    // Best-effort: enable WAL for better concurrency.
    let _ = db.execute("PRAGMA journal_mode = WAL;").await;

    let (app_tx, _) = broadcast::channel::<WsEnvelope>(1024);
    let redactor = Arc::new(Redactor::new(&cfg.redaction_extra_regex)?);

    let state = AppState {
        jwt_encoding: EncodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        jwt_decoding: DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        cfg,
        db,
        app_tx,
        redactor,
        hosts_tx: Arc::new(RwLock::new(HashMap::new())),
        run_to_host: Arc::new(RwLock::new(HashMap::new())),
        web_dist_dir: None,
        server_log_path,
    };

    let mut state = state;
    let web_dist_dir = std::env::var("WEB_DIST_DIR").unwrap_or_else(|_| "web/dist".into());
    if std::path::Path::new(&web_dist_dir).is_dir() {
        state.web_dist_dir = Some(std::path::PathBuf::from(web_dist_dir));
    }

    tracing::info!(
        bind_addr = %state.cfg.bind_addr,
        admin_username = %state.cfg.admin_username,
        web_dist_dir = ?state.web_dist_dir,
        server_log_path = ?state.server_log_path,
        "relay-server starting"
    );

    // Background cleanup: keep 3 days of finished runs/events (MVP: only events table).
    let cleanup_db = state.db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            let cutoff = (Utc::now() - Duration::days(3)).to_rfc3339();
            let _ = sqlx::query("DELETE FROM events WHERE ts < ?1")
                .bind(&cutoff)
                .execute(&cleanup_db)
                .await;
        }
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/login", post(login))
        .route("/runs", get(http_list_runs))
        .route("/sessions", get(http_list_sessions))
        .route("/sessions/recent", get(http_list_recent_sessions))
        .route("/hosts", get(http_list_hosts))
        .route("/server/logs/tail", get(http_server_logs_tail))
        .route("/runs/:run_id/messages", get(http_list_messages))
        .route("/sessions/:session_id", get(http_get_session))
        .route(
            "/sessions/:session_id/messages",
            get(http_list_session_messages),
        )
        .route("/runs/:run_id/input", post(http_send_input))
        .route("/ws/app", get(ws_app))
        .route("/ws/host", get(ws_host))
        .fallback(http_static_fallback)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tofu_registers_then_verifies() {
        let db = db::connect("sqlite::memory:").await.unwrap();
        db::init(&db).await.unwrap();

        let inserted = tofu_register_or_verify_host(&db, "host-1", "token-1")
            .await
            .unwrap();
        assert!(inserted);

        let inserted_again = tofu_register_or_verify_host(&db, "host-1", "token-1")
            .await
            .unwrap();
        assert!(!inserted_again);

        let stored_hash =
            sqlx::query_scalar::<_, String>("SELECT token_hash FROM hosts WHERE id=?1")
                .bind("host-1")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(stored_hash, sha256_hex("token-1"));

        let err = tofu_register_or_verify_host(&db, "host-1", "token-2")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("mismatch"));
    }
}
