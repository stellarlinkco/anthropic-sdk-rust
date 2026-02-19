use crate::client::{ApiResponse, Inner, RequestOptions};
use crate::error::Error;
use crate::streaming::{MessageStream, RawStream};
use crate::types::messages::{
    Message, MessageCountTokensParams, MessageCreateParams, MessageTokensCount,
    RawMessageStreamEvent,
};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Method;
use std::sync::Arc;
use std::time::Duration;

const HEADER_ANTHROPIC_BETA: HeaderName = HeaderName::from_static("anthropic-beta");
const BETA_TOKEN_COUNTING: &str = "token-counting-2024-11-01";

#[derive(Clone)]
pub struct Messages {
    inner: Arc<Inner>,
}

impl Messages {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn create(
        &self,
        params: BetaMessageCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<Message, Error> {
        Ok(self.create_with_response(params, options).await?.data)
    }

    pub async fn create_with_response(
        &self,
        params: BetaMessageCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<Message>, Error> {
        if params.body.stream == Some(true) {
            return Err(Error::Internal(
                "use beta.messages.create_stream() for stream=true".to_string(),
            ));
        }

        if self.inner.timeout_is_default() {
            calculate_nonstreaming_timeout(params.body.max_tokens, None)?;
        }

        let mut options = options.unwrap_or_default();
        if let Some(betas) = params.betas {
            if !betas.is_empty() {
                options.headers.insert(
                    HEADER_ANTHROPIC_BETA,
                    HeaderValue::from_str(&betas.join(","))?,
                );
            }
        }

        self.inner
            .request_json(
                Method::POST,
                "/v1/messages?beta=true",
                None,
                Some(&params.body),
                options,
            )
            .await
    }

    pub async fn create_stream(
        &self,
        mut params: BetaMessageCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<RawStream<RawMessageStreamEvent>, Error> {
        params.body.stream = Some(true);
        let mut options = options.unwrap_or_default();
        if let Some(betas) = params.betas {
            if !betas.is_empty() {
                options.headers.insert(
                    HEADER_ANTHROPIC_BETA,
                    HeaderValue::from_str(&betas.join(","))?,
                );
            }
        }

        self.inner
            .request_sse_json_stream(
                Method::POST,
                "/v1/messages?beta=true",
                None,
                Some(&params.body),
                options,
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
        params: BetaMessageCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<MessageStream, Error> {
        let raw = self.create_stream(params, options).await?;
        Ok(MessageStream::new(raw))
    }

    pub async fn count_tokens(
        &self,
        params: BetaMessageCountTokensParams,
        options: Option<RequestOptions>,
    ) -> Result<MessageTokensCount, Error> {
        Ok(self.count_tokens_with_response(params, options).await?.data)
    }

    pub async fn count_tokens_with_response(
        &self,
        params: BetaMessageCountTokensParams,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<MessageTokensCount>, Error> {
        let mut options = options.unwrap_or_default();

        let mut betas = params.betas.unwrap_or_default();
        if !betas.iter().any(|b| b == BETA_TOKEN_COUNTING) {
            betas.push(BETA_TOKEN_COUNTING.to_string());
        }
        options.headers.insert(
            HEADER_ANTHROPIC_BETA,
            HeaderValue::from_str(&betas.join(","))?,
        );

        self.inner
            .request_json(
                Method::POST,
                "/v1/messages/count_tokens?beta=true",
                None,
                Some(&params.body),
                options,
            )
            .await
    }
}

#[derive(Debug, Clone, Default)]
pub struct BetaMessageCreateParams {
    pub betas: Option<Vec<String>>,
    pub body: MessageCreateParams,
}

#[derive(Debug, Clone, Default)]
pub struct BetaMessageCountTokensParams {
    pub betas: Option<Vec<String>>,
    pub body: MessageCountTokensParams,
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
