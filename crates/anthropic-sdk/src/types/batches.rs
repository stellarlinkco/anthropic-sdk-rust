use crate::pagination::PageParams;
use crate::types::messages::{Message, MessageCreateParams};
use crate::types::shared::ErrorResponse;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedMessageBatch {
    pub id: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBatch {
    pub id: String,

    pub processing_status: String,

    pub results_url: Option<String>,

    pub request_counts: MessageBatchRequestCounts,

    #[serde(rename = "type", default)]
    pub kind: String,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageBatchRequestCounts {
    #[serde(default)]
    pub canceled: u64,
    #[serde(default)]
    pub errored: u64,
    #[serde(default)]
    pub expired: u64,
    #[serde(default)]
    pub processing: u64,
    #[serde(default)]
    pub succeeded: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCreateParams {
    pub requests: Vec<BatchRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub custom_id: String,
    pub params: MessageCreateParams,
}

pub type BatchListParams = PageParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBatchIndividualResponse {
    pub custom_id: String,
    pub result: MessageBatchResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageBatchResult {
    #[serde(rename = "succeeded")]
    Succeeded { message: Message },
    #[serde(rename = "errored")]
    Errored { error: ErrorResponse },
    #[serde(rename = "canceled")]
    Canceled,
    #[serde(rename = "expired")]
    Expired,
}
