use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedFile {
    pub id: String,

    #[serde(rename = "type", default)]
    pub kind: String,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub created_at: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,

    #[serde(rename = "type", default)]
    pub kind: String,

    #[serde(default)]
    pub downloadable: Option<bool>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
