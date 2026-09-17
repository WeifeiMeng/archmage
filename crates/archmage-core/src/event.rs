use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub run_id: String,
    pub sequence: i64,
    pub event_type: String,
    pub node_id: Option<String>,
    pub attempt_id: Option<String>,
    pub schema_version: i64,
    pub payload: Value,
    pub created_at: String,
}
