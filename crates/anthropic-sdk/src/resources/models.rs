use crate::client::{ApiResponse, Inner, RequestOptions};
use crate::error::Error;
use crate::pagination::Page;
use crate::types::models::{ModelInfo, ModelListParams, ModelRetrieveParams};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Method;
use std::sync::Arc;

const HEADER_ANTHROPIC_BETA: HeaderName = HeaderName::from_static("anthropic-beta");

#[derive(Clone)]
pub struct Models {
    inner: Arc<Inner>,
}

impl Models {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn retrieve(
        &self,
        model_id: &str,
        params: Option<ModelRetrieveParams>,
        options: Option<RequestOptions>,
    ) -> Result<ModelInfo, Error> {
        Ok(self
            .retrieve_with_response(model_id, params, options)
            .await?
            .data)
    }

    pub async fn retrieve_with_response(
        &self,
        model_id: &str,
        params: Option<ModelRetrieveParams>,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<ModelInfo>, Error> {
        let mut options = options.unwrap_or_default();
        if let Some(betas) = params.and_then(|p| p.betas) {
            if !betas.is_empty() {
                options.headers.insert(
                    HEADER_ANTHROPIC_BETA,
                    HeaderValue::from_str(&betas.join(","))?,
                );
            }
        }

        self.inner
            .request_json(
                Method::GET,
                &format!("/v1/models/{model_id}"),
                None,
                Option::<&()>::None,
                options,
            )
            .await
    }

    pub async fn list(
        &self,
        params: Option<ModelListParams>,
        options: Option<RequestOptions>,
    ) -> Result<Page<ModelInfo>, Error> {
        Ok(self.list_with_response(params, options).await?.data)
    }

    pub async fn list_with_response(
        &self,
        params: Option<ModelListParams>,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<Page<ModelInfo>>, Error> {
        let params = params.unwrap_or_default();
        let mut options = options.unwrap_or_default();

        if let Some(betas) = params.betas.clone() {
            if !betas.is_empty() {
                options.headers.insert(
                    HEADER_ANTHROPIC_BETA,
                    HeaderValue::from_str(&betas.join(","))?,
                );
            }
        }

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
                "/v1/models",
                Some(query),
                Option::<&()>::None,
                options,
            )
            .await
    }
}
