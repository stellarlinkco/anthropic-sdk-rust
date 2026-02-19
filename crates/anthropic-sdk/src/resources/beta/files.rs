use crate::client::{ApiResponse, Inner, RequestOptions};
use crate::error::Error;
use crate::pagination::Page;
use crate::types::files::{DeletedFile, FileMetadata};
use bytes::Bytes;
use reqwest::header::{HeaderName, HeaderValue, ACCEPT};
use reqwest::multipart::{Form, Part};
use reqwest::Method;
use std::path::PathBuf;
use std::sync::Arc;

const HEADER_ANTHROPIC_BETA: HeaderName = HeaderName::from_static("anthropic-beta");
const BETA_FILES_API: &str = "files-api-2025-04-14";

#[derive(Clone)]
pub struct Files {
    inner: Arc<Inner>,
}

impl Files {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, options: Option<RequestOptions>) -> Result<Page<FileMetadata>, Error> {
        Ok(self.list_with_response(options).await?.data)
    }

    pub async fn list_with_response(
        &self,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<Page<FileMetadata>>, Error> {
        let mut options = options.unwrap_or_default();
        options.headers.insert(
            HEADER_ANTHROPIC_BETA,
            HeaderValue::from_static(BETA_FILES_API),
        );

        self.inner
            .request_json(Method::GET, "/v1/files", None, Option::<&()>::None, options)
            .await
    }

    pub async fn delete(
        &self,
        file_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<DeletedFile, Error> {
        Ok(self.delete_with_response(file_id, options).await?.data)
    }

    pub async fn delete_with_response(
        &self,
        file_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<DeletedFile>, Error> {
        let mut options = options.unwrap_or_default();
        options.headers.insert(
            HEADER_ANTHROPIC_BETA,
            HeaderValue::from_static(BETA_FILES_API),
        );

        self.inner
            .request_json(
                Method::DELETE,
                &format!("/v1/files/{file_id}"),
                None,
                Option::<&()>::None,
                options,
            )
            .await
    }

    pub async fn retrieve_metadata(
        &self,
        file_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<FileMetadata, Error> {
        Ok(self
            .retrieve_metadata_with_response(file_id, options)
            .await?
            .data)
    }

    pub async fn retrieve_metadata_with_response(
        &self,
        file_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<FileMetadata>, Error> {
        let mut options = options.unwrap_or_default();
        options.headers.insert(
            HEADER_ANTHROPIC_BETA,
            HeaderValue::from_static(BETA_FILES_API),
        );

        self.inner
            .request_json(
                Method::GET,
                &format!("/v1/files/{file_id}"),
                None,
                Option::<&()>::None,
                options,
            )
            .await
    }

    pub async fn download(
        &self,
        file_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Bytes, Error> {
        Ok(self.download_with_response(file_id, options).await?.data)
    }

    pub async fn download_with_response(
        &self,
        file_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<Bytes>, Error> {
        let mut options = options.unwrap_or_default();
        options.headers.insert(
            HEADER_ANTHROPIC_BETA,
            HeaderValue::from_static(BETA_FILES_API),
        );
        options
            .headers
            .insert(ACCEPT, HeaderValue::from_static("application/binary"));

        let resp = self
            .inner
            .request_raw(
                Method::GET,
                &format!("/v1/files/{file_id}/content"),
                None,
                Option::<&()>::None,
                options,
            )
            .await?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let request_id = headers
            .get("request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let bytes = resp.bytes().await?;
        Ok(ApiResponse {
            data: bytes,
            request_id,
            status,
            headers,
        })
    }

    pub async fn upload(
        &self,
        params: FileUploadParams,
        options: Option<RequestOptions>,
    ) -> Result<FileMetadata, Error> {
        Ok(self.upload_with_response(params, options).await?.data)
    }

    pub async fn upload_with_response(
        &self,
        params: FileUploadParams,
        options: Option<RequestOptions>,
    ) -> Result<ApiResponse<FileMetadata>, Error> {
        let mut options = options.unwrap_or_default();
        let mut betas = params.betas.unwrap_or_default();
        if !betas.iter().any(|b| b == BETA_FILES_API) {
            betas.push(BETA_FILES_API.to_string());
        }
        options.headers.insert(
            HEADER_ANTHROPIC_BETA,
            HeaderValue::from_str(&betas.join(","))?,
        );

        let file_bytes = tokio::fs::read(&params.path).await.map_err(|e| {
            Error::Internal(format!(
                "failed to read file '{}': {e}",
                params.path.display()
            ))
        })?;

        let filename = params.filename.clone().unwrap_or_else(|| {
            params
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });
        let mime_type = params.mime_type.clone();

        let build_form = move || {
            let mut part = Part::bytes(file_bytes.clone()).file_name(filename.clone());
            if let Some(mime) = &mime_type {
                part = part
                    .mime_str(mime)
                    .map_err(|e| Error::Internal(e.to_string()))?;
            }
            Ok(Form::new().part("file", part))
        };

        self.inner
            .request_multipart_json(Method::POST, "/v1/files", None, build_form, options)
            .await
    }
}

#[derive(Debug, Clone)]
pub struct FileUploadParams {
    pub path: PathBuf,
    pub filename: Option<String>,
    pub mime_type: Option<String>,
    pub betas: Option<Vec<String>>,
}
