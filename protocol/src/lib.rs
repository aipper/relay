use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod redaction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEnvelope {
    pub r#type: String,
    pub ts: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<i64>,
    #[serde(default)]
    pub data: Value,
}

impl WsEnvelope {
    pub fn new(r#type: impl Into<String>, data: Value) -> Self {
        Self {
            r#type: r#type.into(),
            ts: Utc::now(),
            host_id: None,
            run_id: None,
            seq: None,
            data,
        }
    }
}

// --- Permission payloads (typed helpers over WsEnvelope.data) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecision {
    Approve,
    Deny,
    ApproveForSession,
    Abort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionApproveData {
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<PermissionDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequestedData {
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op_tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op_args_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approve_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deny_text: Option<String>,
    // Optional structured questions to render as a form in the UI.
    // We keep this as `Value` to avoid locking in a schema prematurely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub questions: Option<Value>,
}
