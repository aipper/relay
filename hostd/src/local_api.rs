use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::run_manager::RunManager;

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

async fn start_run(
    State(state): State<Arc<LocalState>>,
    Json(req): Json<StartRunRequest>,
) -> Result<Json<StartRunResponse>, (StatusCode, String)> {
    let run_id = state
        .rm
        .start_run(req.tool, req.cmd, req.cwd)
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

async fn list_runs(State(_state): State<Arc<LocalState>>) -> Json<Vec<String>> {
    Json(Vec::new())
}

pub fn router(state: Arc<LocalState>) -> Router {
    Router::new()
        .route("/runs", post(start_run).get(list_runs))
        .route("/runs/:run_id/input", post(send_input))
        .route("/runs/:run_id/stop", post(stop_run))
        .with_state(state)
}
