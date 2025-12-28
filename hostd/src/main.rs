mod config;
mod local_api;
mod run_manager;
mod spool;

use futures_util::{SinkExt, StreamExt};
use relay_protocol::WsEnvelope;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::config::Config;
use crate::run_manager::RunManager;
use crate::spool::Spool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cfg = Config::from_env();
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
    let rm = RunManager::new(cfg.host_id.clone(), redactor, events_tx.clone());

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

    // Local unix API server.
    let local = Arc::new(local_api::LocalState { rm: rm.clone() });
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
            rm.clone(),
            events_tx.subscribe(),
            spool.clone(),
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

    let listener = tokio::net::UnixListener::bind(sock_path)?;
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
    rm: RunManager,
    mut events_rx: broadcast::Receiver<WsEnvelope>,
    spool: Spool,
) -> anyhow::Result<()> {
    let (ws, _) = tokio_tungstenite::connect_async(ws_url.to_string()).await?;
    tracing::info!("connected to server ws");

    let (mut ws_sender, mut ws_receiver) = ws.split();
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(10));

    async fn flush_spool<S>(ws_sender: &mut S, spool: &Spool, limit: usize) -> anyhow::Result<()>
    where
        S: futures_util::Sink<tokio_tungstenite::tungstenite::Message> + Unpin,
        S::Error: std::fmt::Display,
    {
        let pending = tokio::task::spawn_blocking({
            let spool = spool.clone();
            move || spool.pending_events(limit)
        })
        .await??;
        for env in pending {
            let text = serde_json::to_string(&env)?;
            ws_sender
                .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }
        Ok(())
    }

    // Replay pending events first (best-effort).
    let _ = flush_spool(&mut ws_sender, &spool, 10_000).await;

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                let msg = serde_json::to_string(&WsEnvelope::new("host.heartbeat", serde_json::json!({})))?;
                ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(msg.into())).await?;
                // Periodically try to drain any remaining spool backlog.
                let _ = flush_spool(&mut ws_sender, &spool, 500).await;
            }
            ev = events_rx.recv() => {
                match ev {
                    Ok(env) => {
                        let text = serde_json::to_string(&env)?;
                        ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(text.into())).await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            incoming = ws_receiver.next() => {
                let Some(incoming) = incoming else { break; };
                let msg = incoming?;
                match msg {
                    tokio_tungstenite::tungstenite::Message::Text(text) => {
                        let Ok(env) = serde_json::from_str::<WsEnvelope>(&text) else { continue; };
                        if env.r#type == "run.ack" {
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
                            let _ = flush_spool(&mut ws_sender, &spool, 500).await;
                        } else if env.r#type == "run.send_input" {
                            let Some(run_id) = env.run_id.as_deref() else { continue; };
                            let actor = env.data.get("actor").and_then(|v| v.as_str()).unwrap_or("web");
                            let input_id = env.data.get("input_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let text = env.data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            let _ = rm.send_input(run_id, actor, input_id, text).await;
                        } else if env.r#type == "run.stop" {
                            let Some(run_id) = env.run_id.as_deref() else { continue; };
                            let signal = env.data.get("signal").and_then(|v| v.as_str()).unwrap_or("term");
                            let _ = rm.stop_run(run_id, signal).await;
                        }
                    }
                    tokio_tungstenite::tungstenite::Message::Ping(p) => {
                        ws_sender.send(tokio_tungstenite::tungstenite::Message::Pong(p)).await?;
                    }
                    tokio_tungstenite::tungstenite::Message::Close(_) => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
