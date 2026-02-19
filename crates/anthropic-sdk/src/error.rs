use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde_json::Value;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: Option<StatusCode>,
    pub request_id: Option<String>,
    pub message: Option<String>,
    pub body: Option<Value>,
}

impl ApiError {
    pub fn new(
        status: Option<StatusCode>,
        headers: Option<&HeaderMap>,
        body: Option<Value>,
        message: Option<String>,
    ) -> Self {
        let request_id = headers
            .and_then(|h| h.get("request-id"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Self {
            status,
            request_id,
            message,
            body,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(msg) = &self.message {
            return write!(f, "{msg}");
        }
        if let Some(body) = &self.body {
            return write!(f, "{body}");
        }
        write!(f, "unknown error")
    }
}

#[derive(Debug, Error)]
pub enum HttpApiError {
    #[error("400 Bad Request: {0}")]
    BadRequest(ApiError),
    #[error("401 Authentication Error: {0}")]
    Authentication(ApiError),
    #[error("403 Permission Denied: {0}")]
    PermissionDenied(ApiError),
    #[error("404 Not Found: {0}")]
    NotFound(ApiError),
    #[error("409 Conflict: {0}")]
    Conflict(ApiError),
    #[error("422 Unprocessable Entity: {0}")]
    UnprocessableEntity(ApiError),
    #[error("429 Rate Limit: {0}")]
    RateLimit(ApiError),
    #[error("5xx Internal Server Error: {0}")]
    InternalServer(ApiError),
    #[error("API Error: {0}")]
    Other(ApiError),
}

impl HttpApiError {
    pub fn from_status(status: Option<StatusCode>, err: ApiError) -> Self {
        match status.map(|s| s.as_u16()) {
            Some(400) => Self::BadRequest(err),
            Some(401) => Self::Authentication(err),
            Some(403) => Self::PermissionDenied(err),
            Some(404) => Self::NotFound(err),
            Some(409) => Self::Conflict(err),
            Some(422) => Self::UnprocessableEntity(err),
            Some(429) => Self::RateLimit(err),
            Some(s) if s >= 500 => Self::InternalServer(err),
            _ => Self::Other(err),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "authentication is missing; set api_key/auth_token or pass X-Api-Key/Authorization headers"
    )]
    AuthMissing,

    #[error(transparent)]
    Http(#[from] HttpApiError),

    #[error("request timed out")]
    Timeout,

    #[error(transparent)]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error(transparent)]
    Transport(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error("invalid server-sent event stream: {0}")]
    InvalidSse(String),

    #[error("invalid jsonl stream: {0}")]
    InvalidJsonl(String),

    #[error("stream aborted")]
    Aborted,

    #[error("internal error: {0}")]
    Internal(String),
}
