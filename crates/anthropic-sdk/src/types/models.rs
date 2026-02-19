use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,

    pub created_at: String,

    pub display_name: String,

    #[serde(rename = "type", default)]
    pub kind: String,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelRetrieveParams {
    #[serde(default, skip_serializing)]
    pub betas: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelListParams {
    #[serde(default, skip_serializing)]
    pub betas: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_id: Option<String>,
}
