mod batches;

use crate::client::{ApiResponse, Inner, RequestOptions};
use crate::error::Error;
use crate::streaming::MessageStream;
use crate::streaming::RawStream;
use crate::types::messages::{
    Message, MessageCountTokensParams, MessageCreateParams, MessageTokensCount,
};
use reqwest::Method;
use std::sync::Arc;
use std::time::Duration;

pub use crate::resources::messages::batches::Batches;

#[derive(Clone)]
pub struct Messages {
    inner: Arc<Inner>,
    pub batches: Batches,
}

impl Messages {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self {
            inner: inner.clone(),
            batches: Batches::new(inner),
        }
    }

    pub async fn create(
        &self,
        params: MessageCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<Message, Error> {
        Ok(self.create_with_response(params, options).await?.data)
    }

    pub async fn create_with_response(
        &self,
        params: MessageCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<Message>, Error> {
        if params.stream == Some(true) {
            return Err(Error::Internal(
                "use messages.create_stream() for stream=true".to_string(),
            ));
        }

        if self.inner.timeout_is_default() {
            let max_limit = max_nonstreaming_tokens(&params.model);
            calculate_nonstreaming_timeout(params.max_tokens, max_limit)?;
        }

        self.inner
            .request_json(
                Method::POST,
                "/v1/messages",
                None,
                Some(&params),
                options.unwrap_or_default(),
            )
            .await
    }

    pub async fn create_stream(
        &self,
        mut params: MessageCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<RawStream<crate::types::messages::RawMessageStreamEvent>, Error> {
        params.stream = Some(true);
        self.inner
            .request_sse_json_stream(
                Method::POST,
                "/v1/messages",
                None,
                Some(&params),
                options.unwrap_or_default(),
                &[
                    "message_start",
                    "message_delta",
                    "message_stop",
                    "content_block_start",
                    "content_block_delta",
                    "content_block_stop",
                ],
            )
            .await
    }

    pub async fn stream(
        &self,
        params: MessageCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<MessageStream, Error> {
        let raw = self.create_stream(params, options).await?;
        Ok(MessageStream::new(raw))
    }

    pub async fn count_tokens(
        &self,
        params: MessageCountTokensParams,
        options: Option<RequestOptions>,
    ) -> Result<MessageTokensCount, Error> {
        Ok(self.count_tokens_with_response(params, options).await?.data)
    }

    pub async fn count_tokens_with_response(
        &self,
        params: MessageCountTokensParams,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<MessageTokensCount>, Error> {
        self.inner
            .request_json(
                Method::POST,
                "/v1/messages/count_tokens",
                None,
                Some(&params),
                options.unwrap_or_default(),
            )
            .await
    }
}

fn max_nonstreaming_tokens(model: &str) -> Option<u64> {
    match model {
        "claude-opus-4-20250514" => Some(8192),
        "claude-opus-4-0" => Some(8192),
        "claude-4-opus-20250514" => Some(8192),
        "anthropic.claude-opus-4-20250514-v1:0" => Some(8192),
        "claude-opus-4@20250514" => Some(8192),
        "claude-opus-4-1-20250805" => Some(8192),
        "anthropic.claude-opus-4-1-20250805-v1:0" => Some(8192),
        "claude-opus-4-1@20250805" => Some(8192),
        _ => None,
    }
}

fn calculate_nonstreaming_timeout(
    max_tokens: u64,
    max_nonstreaming_tokens: Option<u64>,
) -> Result<Duration, Error> {
    let max_time_ms: u64 = 60 * 60 * 1000;
    let default_time_ms: u64 = 10 * 60 * 1000;

    let expected_ms = (max_time_ms.saturating_mul(max_tokens)) / 128_000;
    if expected_ms > default_time_ms
        || max_nonstreaming_tokens
            .map(|limit| max_tokens > limit)
            .unwrap_or(false)
    {
        return Err(Error::Internal(
            "streaming is required for operations that may take longer than 10 minutes".to_string(),
        ));
    }
    Ok(Duration::from_millis(default_time_ms))
}
