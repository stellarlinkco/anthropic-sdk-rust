use crate::client::{ApiResponse, Inner, RequestOptions};
use crate::error::Error;
use crate::streaming::RawStream;
use crate::types::completions::{Completion, CompletionCreateParams};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Method;
use std::sync::Arc;

const HEADER_ANTHROPIC_BETA: HeaderName = HeaderName::from_static("anthropic-beta");

#[derive(Clone)]
pub struct Completions {
    inner: Arc<Inner>,
}

impl Completions {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn create(
        &self,
        params: CompletionCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<Completion, Error> {
        Ok(self.create_with_response(params, options).await?.data)
    }

    pub async fn create_with_response(
        &self,
        params: CompletionCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<Completion>, Error> {
        let mut options = options.unwrap_or_default();
        if let Some(betas) = params.betas.clone() {
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
                "/v1/complete",
                None,
                Some(&params.body()),
                options,
            )
            .await
    }

    pub async fn create_stream(
        &self,
        mut params: CompletionCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<RawStream<Completion>, Error> {
        params.stream = Some(true);
        let mut options = options.unwrap_or_default();
        if let Some(betas) = params.betas.clone() {
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
                "/v1/complete",
                None,
                Some(&params.body()),
                options,
                &["completion"],
            )
            .await
    }
}
