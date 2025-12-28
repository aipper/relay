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
