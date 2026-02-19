use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message {
    pub id: String,

    #[serde(default)]
    pub content: Vec<Value>,

    #[serde(default)]
    pub model: String,

    #[serde(default)]
    pub role: String,

    #[serde(default)]
    pub stop_reason: Option<String>,

    #[serde(default)]
    pub stop_sequence: Option<String>,

    #[serde(rename = "type", default)]
    pub kind: String,

    #[serde(default)]
    pub usage: Value,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTokensCount {
    pub input_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageParam {
    pub role: String,
    pub content: MessageContent,
}

impl MessageParam {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: MessageContent::Text(text.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageCreateParams {
    pub model: String,
    pub max_tokens: u64,
    pub messages: Vec<MessageParam>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageCountTokensParams {
    pub model: String,
    pub messages: Vec<MessageParam>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RawMessageStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: Message },

    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDelta,
        usage: MessageDeltaUsage,
    },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: usize, content_block: Value },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: usize,
        delta: RawContentBlockDelta,
    },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    pub container: Option<Value>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaUsage {
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: u64,

    #[serde(default)]
    pub server_tool_use: Option<Value>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RawContentBlockDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },

    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },

    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },

    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },

    #[serde(rename = "citations_delta")]
    CitationsDelta { citation: Value },

    #[serde(other)]
    Unknown,
}
