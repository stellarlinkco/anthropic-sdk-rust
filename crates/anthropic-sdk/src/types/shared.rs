use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    pub message: String,

    #[serde(rename = "type")]
    pub kind: String,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorObject,

    pub request_id: Option<String>,

    #[serde(rename = "type")]
    pub kind: String,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
