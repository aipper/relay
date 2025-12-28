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
use sqlx::Executor;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::broadcast;
use tokio::sync::{RwLock, mpsc};

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

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let v = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let v = v.strip_prefix("Bearer ")?;
    Some(v.to_string())
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

    // MVP: accept any host_id/host_token without registration flow; store token hash later.
    tracing::info!(%host_id, "host connected");

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
                        if env.r#type != "run.send_input" && env.r#type != "run.stop" { continue; }
                        let Some(run_id) = env.run_id.clone() else { continue; };

                        let host_id = {
                            let map = state.run_to_host.read().await;
                            map.get(&run_id).cloned()
                        };
                        let Some(host_id) = host_id else { continue; };

                        let mut cmd = WsEnvelope::new(env.r#type.clone(), env.data.clone());
                        cmd.host_id = Some(host_id.clone());
                        cmd.run_id = Some(run_id);

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
        let token_hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(host_token.as_bytes());
            format!("{:x}", hasher.finalize())
        };
        let _ = sqlx::query(
            r#"
INSERT INTO hosts (id, token_hash, last_seen_at) VALUES (?1, ?2, ?3)
ON CONFLICT(id) DO UPDATE SET token_hash=excluded.token_hash, last_seen_at=excluded.last_seen_at
"#,
        )
        .bind(&host_id)
        .bind(&token_hash)
        .bind(&now)
        .execute(&state.db)
        .await;
    };
    update_seen.await;

    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                let Ok(env) = serde_json::from_str::<WsEnvelope>(&text) else {
                    continue;
                };
                let run_id = env.run_id.clone().unwrap_or_else(|| "unknown".into());
                let seq = env.seq;

                if env.r#type == "run.started" {
                    let mut map = state.run_to_host.write().await;
                    map.insert(run_id.clone(), host_id.clone());

                    let tool = env
                        .data
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let cwd = env.data.get("cwd").and_then(|v| v.as_str()).unwrap_or(".");
                    let _ = db::upsert_run_started(&state.db, &run_id, &host_id, tool, cwd, env.ts)
                        .await;
                } else if env.r#type == "run.awaiting_input" {
                    let _ = db::mark_run_awaiting_input(&state.db, &run_id).await;
                } else if env.r#type == "run.exited" {
                    let exit_code = env
                        .data
                        .get("exit_code")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(-1);
                    let _ = db::finish_run(&state.db, &run_id, env.ts, exit_code).await;
                }

                // Persist minimal event.
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
                )
                .await;

                // Ack to host for spool replay.
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

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
    };

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
        .route("/runs/:run_id/input", post(http_send_input))
        .route("/ws/app", get(ws_app))
        .route("/ws/host", get(ws_host))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
