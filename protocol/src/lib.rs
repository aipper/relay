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
    /// Which structured action the user picked (see `PermissionRequestedData.actions`).
    /// Additive: old clients omit it; hosts fall back to the envelope type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_action_id: Option<String>,
}

/// How a structured approval action resolves when chosen.
/// Additive: unknown variants must be treated as `Custom` by old clients
/// (serde `other` fallback is intentionally NOT used so new behaviors
/// fail closed to Custom only in new clients; old clients ignore the field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionActionBehavior {
    Approve,
    Deny,
    Abort,
    Custom,
}

/// One structured approval action (paseo `AgentPermissionRequest.actions` envelope,
/// adopted additively onto `run.permission_requested.data`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAction {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior: Option<PermissionActionBehavior>,
}

/// Optional suggested quick replies accompanying a permission request
/// (paseo `AgentPermissionRequest.suggestions`, additive).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSuggestion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
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
    /// Structured approval actions (paseo envelope, additive).
    /// Old clients ignore unknown fields, so absence means "render approve/deny".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<PermissionAction>>,
    /// Optional suggested quick replies (paseo envelope, additive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<PermissionSuggestion>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_shape_permission_requested_parses_without_new_fields() {
        let old = serde_json::json!({
            "request_id": "req-1",
            "reason": "permission",
            "approve_text": "y\n",
            "deny_text": "n\n"
        });
        let parsed: PermissionRequestedData = serde_json::from_value(old).unwrap();
        assert!(parsed.actions.is_none());
        assert!(parsed.suggestions.is_none());
        let v = serde_json::to_value(&parsed).unwrap();
        assert!(v.get("actions").is_none());
        assert!(v.get("suggestions").is_none());
    }

    #[test]
    fn old_shape_permission_approve_parses_without_selected_action() {
        let old = serde_json::json!({ "request_id": "req-1" });
        let parsed: PermissionApproveData = serde_json::from_value(old).unwrap();
        assert!(parsed.selected_action_id.is_none());
        let v = serde_json::to_value(&parsed).unwrap();
        assert!(v.get("selected_action_id").is_none());
    }
}
