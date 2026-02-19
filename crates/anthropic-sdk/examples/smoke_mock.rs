use anthropic_sdk::types::models::ModelInfo;
use anthropic_sdk::{Anthropic, ClientOptions};
use reqwest::header::HeaderMap;
use serde_json::json;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
          "data": [{
            "id": "model_1",
            "created_at": "2025-01-01T00:00:00Z",
            "display_name": "Model 1",
            "type": "model"
          }],
          "has_more": false,
          "first_id": "model_1",
          "last_id": "model_1"
        })))
        .mount(&server)
        .await;

    let client = Anthropic::new(ClientOptions {
        api_key: Some("test-key".to_string()),
        auth_token: None,
        base_url: Some(server.uri()),
        timeout: Some(Duration::from_millis(250)),
        max_retries: Some(0),
        default_headers: HeaderMap::new(),
    })?;

    let page = client.models.list(None, None).await?;
    let models: Vec<ModelInfo> = page.data;
    println!("models={}", models.len());

    Ok(())
}
