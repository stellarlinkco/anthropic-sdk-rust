use crate::error::{ApiError, Error, HttpApiError};
use crate::resources::{beta::Beta, completions::Completions, messages::Messages, models::Models};
use crate::streaming::{RawStream, SseEvent, SseParser};
use bytes::Bytes;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use httpdate::parse_http_date;
use reqwest::header::{
    HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT,
};
use reqwest::multipart::Form;
use reqwest::{Client as HttpClient, Method, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const DEFAULT_MAX_RETRIES: u32 = 2;

const HEADER_ANTHROPIC_VERSION: HeaderName = HeaderName::from_static("anthropic-version");
const HEADER_REQUEST_ID: HeaderName = HeaderName::from_static("request-id");
const HEADER_RETRY_AFTER: HeaderName = HeaderName::from_static("retry-after");
const HEADER_RETRY_AFTER_MS: HeaderName = HeaderName::from_static("retry-after-ms");
const HEADER_X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");
const HEADER_X_SHOULD_RETRY: HeaderName = HeaderName::from_static("x-should-retry");
const HEADER_X_STAINLESS_RETRY_COUNT: HeaderName =
    HeaderName::from_static("x-stainless-retry-count");
const HEADER_X_STAINLESS_TIMEOUT: HeaderName = HeaderName::from_static("x-stainless-timeout");

#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub api_key: Option<String>,
    pub auth_token: Option<String>,
    pub base_url: Option<String>,
    pub timeout: Option<Duration>,
    pub max_retries: Option<u32>,
    pub default_headers: HeaderMap,
}

impl Default for ClientOptions {
    fn default() -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let auth_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
        let base_url = std::env::var("ANTHROPIC_BASE_URL").ok();
        Self {
            api_key,
            auth_token,
            base_url,
            timeout: None,
            max_retries: None,
            default_headers: HeaderMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    pub timeout: Option<Duration>,
    pub max_retries: Option<u32>,
    pub headers: HeaderMap,
    pub remove_headers: Vec<HeaderName>,
}

impl RequestOptions {
    pub fn remove_header(mut self, name: HeaderName) -> Self {
        self.remove_headers.push(name);
        self
    }

    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ApiResponse<T> {
    pub data: T,
    pub request_id: Option<String>,
    pub status: StatusCode,
    pub headers: HeaderMap,
}

#[derive(Clone)]
pub struct Anthropic {
    pub messages: Messages,
    pub models: Models,
    pub completions: Completions,
    pub beta: Beta,
}

impl Anthropic {
    pub fn new(options: ClientOptions) -> Result<Self, Error> {
        let inner = Arc::new(Inner::new(options)?);
        Ok(Self {
            messages: Messages::new(inner.clone()),
            models: Models::new(inner.clone()),
            completions: Completions::new(inner.clone()),
            beta: Beta::new(inner.clone()),
        })
    }

    pub fn with_options(&self, options: ClientOptions) -> Result<Self, Error> {
        Self::new(options)
    }
}

pub(crate) struct Inner {
    http: HttpClient,
    base_url: Url,
    timeout: Duration,
    timeout_is_default: bool,
    max_retries: u32,

    api_key: Option<String>,
    auth_token: Option<String>,

    default_headers: HeaderMap,
    user_agent: HeaderValue,
}

impl Inner {
    fn new(options: ClientOptions) -> Result<Self, Error> {
        let base_url_str = options.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL);
        let base_url = Url::parse(base_url_str)?;

        let timeout = options.timeout.unwrap_or(DEFAULT_TIMEOUT);
        let timeout_is_default = options.timeout.is_none();
        let max_retries = options.max_retries.unwrap_or(DEFAULT_MAX_RETRIES);

        let http = HttpClient::builder().build()?;
        let user_agent =
            HeaderValue::from_str(&format!("anthropic-sdk-rust/{}", env!("CARGO_PKG_VERSION")))?;

        Ok(Self {
            http,
            base_url,
            timeout,
            timeout_is_default,
            max_retries,
            api_key: options.api_key,
            auth_token: options.auth_token,
            default_headers: options.default_headers,
            user_agent,
        })
    }

    pub fn timeout_is_default(&self) -> bool {
        self.timeout_is_default
    }

    pub fn build_url(&self, path_or_url: &str) -> Result<Url, Error> {
        if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
            return Ok(Url::parse(path_or_url)?);
        }
        let path = if path_or_url.starts_with('/') {
            path_or_url
        } else {
            return Ok(self.base_url.join(&format!("/{path_or_url}"))?);
        };
        Ok(self.base_url.join(path)?)
    }

    fn should_retry_status_headers(status: StatusCode, headers: &HeaderMap) -> bool {
        if let Some(value) = headers
            .get(HEADER_X_SHOULD_RETRY)
            .and_then(|v| v.to_str().ok())
        {
            if value == "true" {
                return true;
            }
            if value == "false" {
                return false;
            }
        }

        matches!(
            status,
            StatusCode::REQUEST_TIMEOUT | StatusCode::CONFLICT | StatusCode::TOO_MANY_REQUESTS
        ) || status.as_u16() >= 500
    }

    fn retry_delay_from_headers(headers: &HeaderMap) -> Option<Duration> {
        if let Some(ms) = headers
            .get(HEADER_RETRY_AFTER_MS)
            .and_then(|v| v.to_str().ok())
        {
            if let Ok(ms) = ms.parse::<f64>() {
                if ms.is_finite() && ms >= 0.0 {
                    return Some(Duration::from_millis(ms as u64));
                }
            }
        }

        let ra = headers.get(HEADER_RETRY_AFTER)?.to_str().ok()?;
        if let Ok(seconds) = ra.parse::<f64>() {
            if seconds.is_finite() && seconds >= 0.0 {
                return Some(Duration::from_millis((seconds * 1000.0) as u64));
            }
            return None;
        }

        // HTTP-date
        let at = parse_http_date(ra).ok()?;
        let now = SystemTime::now();
        let wait = at.duration_since(now).ok()?;
        Some(wait)
    }

    fn default_retry_delay(retries_remaining: u32, max_retries: u32) -> Duration {
        let initial = 0.5_f64;
        let max_delay = 8.0_f64;

        let num_retries = max_retries.saturating_sub(retries_remaining);
        let sleep_seconds = (initial * 2.0_f64.powi(num_retries as i32)).min(max_delay);
        let jitter = 1.0 - fastrand::f64() * 0.25;
        Duration::from_millis((sleep_seconds * jitter * 1000.0) as u64)
    }

    fn make_headers(
        &self,
        retry_count: u32,
        timeout: Duration,
        options: &RequestOptions,
    ) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, self.user_agent.clone());
        headers.insert(
            HEADER_ANTHROPIC_VERSION,
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        headers.insert(
            HEADER_X_STAINLESS_RETRY_COUNT,
            HeaderValue::from_str(&retry_count.to_string())?,
        );
        headers.insert(
            HEADER_X_STAINLESS_TIMEOUT,
            HeaderValue::from_str(&timeout.as_secs().to_string())?,
        );

        if let Some(api_key) = &self.api_key {
            headers.insert(HEADER_X_API_KEY, HeaderValue::from_str(api_key)?);
        }
        if let Some(auth_token) = &self.auth_token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {auth_token}"))?,
            );
        }

        for (name, value) in self.default_headers.iter() {
            headers.insert(name, value.clone());
        }

        for name in &options.remove_headers {
            headers.remove(name);
        }
        for (name, value) in options.headers.iter() {
            headers.insert(name, value.clone());
        }

        if headers.get(HEADER_X_API_KEY).is_none() && headers.get(AUTHORIZATION).is_none() {
            return Err(Error::AuthMissing);
        }
        Ok(headers)
    }

    pub async fn request_json<T, B>(
        &self,
        method: Method,
        path_or_url: &str,
        query: Option<Vec<(String, String)>>,
        body: Option<&B>,
        options: RequestOptions,
    ) -> Result<ApiResponse<T>, Error>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let response = self
            .request_raw(method, path_or_url, query, body, options.clone())
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let request_id = headers
            .get(HEADER_REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let bytes = response.bytes().await?;
        let data = serde_json::from_slice::<T>(&bytes)?;
        Ok(ApiResponse {
            data,
            request_id,
            status,
            headers,
        })
    }

    pub async fn request_multipart_json<T, F>(
        &self,
        method: Method,
        path_or_url: &str,
        query: Option<Vec<(String, String)>>,
        build_form: F,
        options: RequestOptions,
    ) -> Result<ApiResponse<T>, Error>
    where
        T: DeserializeOwned,
        F: Fn() -> Result<Form, Error> + Send + Sync,
    {
        let response = self
            .request_multipart_raw(method, path_or_url, query, build_form, options.clone())
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let request_id = headers
            .get(HEADER_REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let bytes = response.bytes().await?;
        let data = serde_json::from_slice::<T>(&bytes)?;
        Ok(ApiResponse {
            data,
            request_id,
            status,
            headers,
        })
    }

    pub async fn request_multipart_raw<F>(
        &self,
        method: Method,
        path_or_url: &str,
        query: Option<Vec<(String, String)>>,
        build_form: F,
        options: RequestOptions,
    ) -> Result<Response, Error>
    where
        F: Fn() -> Result<Form, Error> + Send + Sync,
    {
        let timeout = options.timeout.unwrap_or(self.timeout);
        let max_retries = options.max_retries.unwrap_or(self.max_retries);

        let mut url = self.build_url(path_or_url)?;
        if let Some(pairs) = query {
            if !pairs.is_empty() {
                let mut qp = url.query_pairs_mut();
                for (k, v) in pairs {
                    qp.append_pair(&k, &v);
                }
            }
        }

        let mut retries_remaining = max_retries;
        loop {
            let retry_count = max_retries.saturating_sub(retries_remaining);
            let headers = self.make_headers(retry_count, timeout, &options)?;
            let form = build_form()?;

            let req = self
                .http
                .request(method.clone(), url.clone())
                .headers(headers)
                .multipart(form);

            let response = tokio::time::timeout(timeout, req.send()).await;
            match response {
                Err(_) => {
                    if retries_remaining > 0 {
                        let delay = Self::default_retry_delay(retries_remaining, max_retries);
                        tokio::time::sleep(delay).await;
                        retries_remaining -= 1;
                        continue;
                    }
                    return Err(Error::Timeout);
                }
                Ok(resp) => match resp {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            return Ok(resp);
                        }

                        let status = resp.status();
                        let headers = resp.headers().clone();
                        let should_retry = retries_remaining > 0
                            && Self::should_retry_status_headers(status, &headers);

                        let body_bytes = resp.bytes().await.unwrap_or_default();
                        let json = serde_json::from_slice::<Value>(&body_bytes).ok();
                        let text = String::from_utf8_lossy(&body_bytes).to_string();
                        let message = extract_error_message(json.as_ref(), &text);
                        let api_err = ApiError::new(Some(status), Some(&headers), json, message);
                        let http_err = HttpApiError::from_status(Some(status), api_err);

                        if should_retry {
                            let delay = Self::retry_delay_from_headers(&headers)
                                .filter(|d| *d < Duration::from_secs(60))
                                .unwrap_or_else(|| {
                                    Self::default_retry_delay(retries_remaining, max_retries)
                                });
                            tokio::time::sleep(delay).await;
                            retries_remaining -= 1;
                            continue;
                        }

                        return Err(Error::Http(http_err));
                    }
                    Err(err) => {
                        let is_timeout = err.is_timeout();
                        if retries_remaining > 0 {
                            let delay = Self::default_retry_delay(retries_remaining, max_retries);
                            tokio::time::sleep(delay).await;
                            retries_remaining -= 1;
                            continue;
                        }

                        if is_timeout {
                            return Err(Error::Timeout);
                        }
                        return Err(Error::Transport(err));
                    }
                },
            }
        }
    }

    pub async fn request_raw<B>(
        &self,
        method: Method,
        path_or_url: &str,
        query: Option<Vec<(String, String)>>,
        body: Option<&B>,
        options: RequestOptions,
    ) -> Result<Response, Error>
    where
        B: Serialize + ?Sized,
    {
        let body_bytes = match body {
            Some(b) => Some(serde_json::to_vec(b)?),
            None => None,
        };

        let timeout = options.timeout.unwrap_or(self.timeout);
        let max_retries = options.max_retries.unwrap_or(self.max_retries);

        let mut url = self.build_url(path_or_url)?;
        if let Some(pairs) = query {
            if !pairs.is_empty() {
                let mut qp = url.query_pairs_mut();
                for (k, v) in pairs {
                    qp.append_pair(&k, &v);
                }
            }
        }

        let mut retries_remaining = max_retries;
        loop {
            let retry_count = max_retries.saturating_sub(retries_remaining);
            let mut headers = self.make_headers(retry_count, timeout, &options)?;
            if body_bytes.is_some() {
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            }

            let mut req = self
                .http
                .request(method.clone(), url.clone())
                .headers(headers);
            if let Some(bytes) = &body_bytes {
                req = req.body(bytes.clone());
            }

            let response = tokio::time::timeout(timeout, req.send()).await;
            match response {
                Err(_) => {
                    if retries_remaining > 0 {
                        let delay = Self::default_retry_delay(retries_remaining, max_retries);
                        tokio::time::sleep(delay).await;
                        retries_remaining -= 1;
                        continue;
                    }
                    return Err(Error::Timeout);
                }
                Ok(resp) => match resp {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            return Ok(resp);
                        }

                        let status = resp.status();
                        let headers = resp.headers().clone();
                        let should_retry = retries_remaining > 0
                            && Self::should_retry_status_headers(status, &headers);

                        let body_bytes = resp.bytes().await.unwrap_or_default();
                        let json = serde_json::from_slice::<Value>(&body_bytes).ok();
                        let text = String::from_utf8_lossy(&body_bytes).to_string();
                        let message = extract_error_message(json.as_ref(), &text);
                        let api_err = ApiError::new(Some(status), Some(&headers), json, message);
                        let http_err = HttpApiError::from_status(Some(status), api_err);

                        if should_retry {
                            let delay = Self::retry_delay_from_headers(&headers)
                                .filter(|d| *d < Duration::from_secs(60))
                                .unwrap_or_else(|| {
                                    Self::default_retry_delay(retries_remaining, max_retries)
                                });
                            tokio::time::sleep(delay).await;
                            retries_remaining -= 1;
                            continue;
                        }

                        return Err(Error::Http(http_err));
                    }
                    Err(err) => {
                        let is_timeout = err.is_timeout();
                        if retries_remaining > 0 {
                            let delay = Self::default_retry_delay(retries_remaining, max_retries);
                            tokio::time::sleep(delay).await;
                            retries_remaining -= 1;
                            continue;
                        }

                        if is_timeout {
                            return Err(Error::Timeout);
                        }
                        return Err(Error::Transport(err));
                    }
                },
            }
        }
    }

    pub async fn request_sse_json_stream<T, B>(
        &self,
        method: Method,
        path_or_url: &str,
        query: Option<Vec<(String, String)>>,
        body: Option<&B>,
        options: RequestOptions,
        allowed_events: &'static [&'static str],
    ) -> Result<RawStream<T>, Error>
    where
        T: DeserializeOwned + Send + 'static,
        B: Serialize + ?Sized,
    {
        let response = self
            .request_raw(method, path_or_url, query, body, options)
            .await?;

        let headers = response.headers().clone();
        let request_id = headers
            .get(HEADER_REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let cancel = tokio_util::sync::CancellationToken::new();
        let cancel_for_stream = cancel.clone();
        let bytes_stream: BoxStream<'static, Result<Bytes, reqwest::Error>> =
            Box::pin(response.bytes_stream());

        let headers_for_errors = Arc::new(headers.clone());

        let stream = futures_util::stream::unfold(
            (
                bytes_stream,
                SseParser::new(),
                std::collections::VecDeque::<SseEvent>::new(),
                false,
                cancel_for_stream,
            ),
            move |(mut bytes_stream, mut parser, mut pending, mut done, cancel)| {
                let headers_for_errors = headers_for_errors.clone();
                async move {
                    if done {
                        return None;
                    }

                    loop {
                        if cancel.is_cancelled() {
                            return None;
                        }

                        if let Some(ev) = pending.pop_front() {
                            let allowed: &[&str] = allowed_events;
                            match ev.event.as_deref() {
                                Some("ping") => continue,
                                Some("error") => {
                                    let json = serde_json::from_str::<Value>(&ev.data).ok();
                                    let message = extract_error_message(json.as_ref(), &ev.data);
                                    let api_err = ApiError::new(
                                        None,
                                        Some(headers_for_errors.as_ref()),
                                        json,
                                        message,
                                    );
                                    let err = Error::Http(HttpApiError::Other(api_err));
                                    done = true;
                                    return Some((
                                        Err(err),
                                        (bytes_stream, parser, pending, done, cancel),
                                    ));
                                }
                                Some(name) if allowed.contains(&name) => {
                                    let item = match serde_json::from_str::<T>(&ev.data) {
                                        Ok(v) => v,
                                        Err(e) => {
                                            done = true;
                                            return Some((
                                                Err(Error::Json(e)),
                                                (bytes_stream, parser, pending, done, cancel),
                                            ));
                                        }
                                    };
                                    return Some((
                                        Ok(item),
                                        (bytes_stream, parser, pending, done, cancel),
                                    ));
                                }
                                _ => continue,
                            }
                        }

                        let next = tokio::select! {
                          _ = cancel.cancelled() => return None,
                          next = bytes_stream.next() => next,
                        };

                        match next {
                            Some(Ok(chunk)) => match parser.push(&chunk) {
                                Ok(events) => pending.extend(events),
                                Err(e) => {
                                    done = true;
                                    return Some((
                                        Err(e),
                                        (bytes_stream, parser, pending, done, cancel),
                                    ));
                                }
                            },
                            Some(Err(e)) => {
                                done = true;
                                return Some((
                                    Err(Error::Transport(e)),
                                    (bytes_stream, parser, pending, done, cancel),
                                ));
                            }
                            None => match parser.finish() {
                                Ok(Some(ev)) => {
                                    pending.push_back(ev);
                                    continue;
                                }
                                Ok(None) => return None,
                                Err(e) => {
                                    done = true;
                                    return Some((
                                        Err(e),
                                        (bytes_stream, parser, pending, done, cancel),
                                    ));
                                }
                            },
                        }
                    }
                }
            },
        );

        Ok(RawStream::new(Box::pin(stream), cancel, request_id))
    }
}

fn extract_error_message(json: Option<&Value>, fallback_text: &str) -> Option<String> {
    let json_msg = json
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("error"))
        .and_then(|e| e.as_object())
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            json.and_then(|v| v.as_object())
                .and_then(|obj| obj.get("message"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        });

    if json_msg.is_some() {
        return json_msg;
    }
    if fallback_text.is_empty() {
        return None;
    }
    Some(fallback_text.to_string())
}
