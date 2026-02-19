use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    pub id: String,
    pub completion: String,
    pub model: String,
    pub stop_reason: Option<String>,

    #[serde(rename = "type", default)]
    pub kind: String,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionCreateParams {
    #[serde(default, skip_serializing)]
    pub betas: Option<Vec<String>>,

    pub max_tokens_to_sample: u64,
    pub model: String,
    pub prompt: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl CompletionCreateParams {
    pub(crate) fn body(&self) -> CompletionCreateBody<'_> {
        CompletionCreateBody { params: self }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct CompletionCreateBody<'a> {
    #[serde(flatten)]
    params: &'a CompletionCreateParams,
}
