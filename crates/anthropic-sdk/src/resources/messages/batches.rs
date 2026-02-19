use crate::client::{ApiResponse, Inner, RequestOptions};
use crate::error::Error;
use crate::jsonl::jsonl_stream_from_response;
use crate::pagination::Page;
use crate::streaming::RawStream;
use crate::types::batches::{
    BatchCreateParams, BatchListParams, DeletedMessageBatch, MessageBatch,
    MessageBatchIndividualResponse,
};
use reqwest::header::{HeaderValue, ACCEPT};
use reqwest::Method;
use std::sync::Arc;

const HEADER_ACCEPT_BINARY: HeaderValue = HeaderValue::from_static("application/binary");
#[derive(Clone)]
pub struct Batches {
    inner: Arc<Inner>,
}

impl Batches {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn create(
        &self,
        body: BatchCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<MessageBatch, Error> {
        Ok(self.create_with_response(body, options).await?.data)
    }

    pub async fn create_with_response(
        &self,
        body: BatchCreateParams,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<MessageBatch>, Error> {
        self.inner
            .request_json(
                Method::POST,
                "/v1/messages/batches",
                None,
                Some(&body),
                options.unwrap_or_default(),
            )
            .await
    }

    pub async fn retrieve(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<MessageBatch, Error> {
        Ok(self.retrieve_with_response(batch_id, options).await?.data)
    }

    pub async fn retrieve_with_response(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<MessageBatch>, Error> {
        self.inner
            .request_json(
                Method::GET,
                &format!("/v1/messages/batches/{batch_id}"),
                None,
                Option::<&()>::None,
                options.unwrap_or_default(),
            )
            .await
    }

    pub async fn list(
        &self,
        params: Option<BatchListParams>,
        options: Option<RequestOptions>,
    ) -> Result<Page<MessageBatch>, Error> {
        Ok(self.list_with_response(params, options).await?.data)
    }

    pub async fn list_with_response(
        &self,
        params: Option<BatchListParams>,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<Page<MessageBatch>>, Error> {
        let params = params.unwrap_or_default();
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(("limit".to_string(), limit.to_string()));
        }
        if let Some(before_id) = params.before_id {
            query.push(("before_id".to_string(), before_id));
        }
        if let Some(after_id) = params.after_id {
            query.push(("after_id".to_string(), after_id));
        }

        self.inner
            .request_json(
                Method::GET,
                "/v1/messages/batches",
                Some(query),
                Option::<&()>::None,
                options.unwrap_or_default(),
            )
            .await
    }

    pub async fn delete(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<DeletedMessageBatch, Error> {
        Ok(self.delete_with_response(batch_id, options).await?.data)
    }

    pub async fn delete_with_response(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<DeletedMessageBatch>, Error> {
        self.inner
            .request_json(
                Method::DELETE,
                &format!("/v1/messages/batches/{batch_id}"),
                None,
                Option::<&()>::None,
                options.unwrap_or_default(),
            )
            .await
    }

    pub async fn cancel(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<MessageBatch, Error> {
        Ok(self.cancel_with_response(batch_id, options).await?.data)
    }

    pub async fn cancel_with_response(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<MessageBatch>, Error> {
        self.inner
            .request_json(
                Method::POST,
                &format!("/v1/messages/batches/{batch_id}/cancel"),
                None,
                Option::<&()>::None,
                options.unwrap_or_default(),
            )
            .await
    }

    pub async fn results(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<RawStream<MessageBatchIndividualResponse>, Error> {
        let batch = self.retrieve(batch_id, None).await?;
        let results_url = batch.results_url.clone().ok_or_else(|| {
            Error::Internal("batch has no results_url; has it finished?".to_string())
        })?;

        let mut options = options.unwrap_or_default();
        options.headers.insert(ACCEPT, HEADER_ACCEPT_BINARY);
        let response = self
            .inner
            .request_raw(
                Method::GET,
                &results_url,
                None,
                Option::<&()>::None,
                options,
            )
            .await?;

        let request_id = response
            .headers()
            .get("request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let cancel = tokio_util::sync::CancellationToken::new();
        Ok(jsonl_stream_from_response(response, cancel, request_id))
    }
}
