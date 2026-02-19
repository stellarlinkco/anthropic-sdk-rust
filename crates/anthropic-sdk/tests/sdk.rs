use anthropic_sdk::types::batches::MessageBatchResult;
use anthropic_sdk::types::messages::{MessageCreateParams, MessageParam, RawMessageStreamEvent};
use anthropic_sdk::types::models::ModelListParams;
use anthropic_sdk::{Anthropic, ClientOptions, Error};
use futures_util::StreamExt;
use reqwest::header::HeaderMap;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

fn client_for(server: &MockServer) -> Anthropic {
    Anthropic::new(ClientOptions {
        api_key: Some("test-key".to_string()),
        auth_token: None,
        base_url: Some(server.uri()),
        timeout: Some(Duration::from_millis(200)),
        max_retries: Some(2),
        default_headers: HeaderMap::new(),
    })
    .unwrap()
}

#[tokio::test]
async fn auth_missing_errors() {
    let server = MockServer::start().await;
    let client = Anthropic::new(ClientOptions {
        api_key: None,
        auth_token: None,
        base_url: Some(server.uri()),
        timeout: Some(Duration::from_millis(200)),
        max_retries: Some(0),
        default_headers: HeaderMap::new(),
    })
    .unwrap();

    let err = client.models.list(None, None).await.unwrap_err();
    match err {
        Error::AuthMissing => {}
        other => panic!("expected AuthMissing, got {other:?}"),
    }
}

#[tokio::test]
async fn models_list_sends_headers_and_beta() {
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

    let client = client_for(&server);
    let page = client
        .models
        .list(
            Some(ModelListParams {
                betas: Some(vec!["foo".into(), "bar".into()]),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();

    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, "model_1");

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let h = &reqs[0].headers;
    assert_eq!(h.get("x-api-key").unwrap().to_str().unwrap(), "test-key");
    assert_eq!(
        h.get("anthropic-version").unwrap().to_str().unwrap(),
        "2023-06-01"
    );
    assert_eq!(
        h.get("anthropic-beta").unwrap().to_str().unwrap(),
        "foo,bar"
    );
    assert!(h.contains_key("x-stainless-retry-count"));
    assert!(h.contains_key("x-stainless-timeout"));
}

#[derive(Clone)]
struct SequenceResponder {
    calls: Arc<AtomicUsize>,
}

impl Respond for SequenceResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            ResponseTemplate::new(429)
                .insert_header("retry-after-ms", "1")
                .set_body_string("{\"error\":{\"message\":\"rate limited\"}}")
        } else {
            ResponseTemplate::new(200).set_body_json(json!({
              "data": [],
              "has_more": false,
              "first_id": null,
              "last_id": null
            }))
        }
    }
}

#[tokio::test]
async fn retries_on_429_and_increments_retry_count() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));

    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(SequenceResponder {
            calls: calls.clone(),
        })
        .mount(&server)
        .await;

    let client = client_for(&server);
    let _ = client.models.list(None, None).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0]
            .headers
            .get("x-stainless-retry-count")
            .unwrap()
            .to_str()
            .unwrap(),
        "0"
    );
    assert_eq!(
        requests[1]
            .headers
            .get("x-stainless-retry-count")
            .unwrap()
            .to_str()
            .unwrap(),
        "1"
    );
}

#[tokio::test]
async fn message_stream_aggregates_text_and_final_message() {
    let server = MockServer::start().await;

    let sse = [
    "event: message_start",
    "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"test-model\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":null,\"container\":null}}",
    "",
    "event: content_block_start",
    "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\",\"citations\":null}}",
    "",
    "event: content_block_delta",
    "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}",
    "",
    "event: content_block_stop",
    "data: {\"type\":\"content_block_stop\",\"index\":0}",
    "",
    "event: message_delta",
    "data: {\"type\":\"message_delta\",\"delta\":{\"container\":null,\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"input_tokens\":null,\"output_tokens\":5,\"server_tool_use\":null}}",
    "",
    "event: message_stop",
    "data: {\"type\":\"message_stop\"}",
    "",
  ]
  .join("\n");

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .mount(&server)
        .await;

    let client = client_for(&server);
    let mut stream = client
        .messages
        .stream(
            MessageCreateParams {
                model: "test-model".to_string(),
                max_tokens: 16,
                messages: vec![MessageParam::user("hi")],
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();

    let mut saw_text_delta = false;
    while let Some(item) = stream.next().await {
        let ev = item.unwrap();
        if let RawMessageStreamEvent::ContentBlockDelta { .. } = ev {
            saw_text_delta = true;
            let snap = stream.snapshot().unwrap();
            let block = snap.content[0].as_object().unwrap();
            assert_eq!(block.get("text").unwrap().as_str().unwrap(), "Hello");
        }
    }
    assert!(saw_text_delta);

    let final_msg = stream.final_message().unwrap();
    let block = final_msg.content[0].as_object().unwrap();
    assert_eq!(block.get("text").unwrap().as_str().unwrap(), "Hello");
    assert_eq!(final_msg.stop_reason.as_deref(), Some("end_turn"));
}

#[tokio::test]
async fn batch_results_jsonl_streams() {
    let server = MockServer::start().await;
    let results_url = format!("{}/results", server.uri());

    Mock::given(method("GET"))
        .and(path("/v1/messages/batches/batch1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
          "id": "batch1",
          "processing_status": "ended",
          "results_url": results_url,
          "request_counts": {"canceled":0,"errored":0,"expired":0,"processing":0,"succeeded":1},
          "type": "message_batch"
        })))
        .mount(&server)
        .await;

    let jsonl = [
        json!({
          "custom_id":"req1",
          "result":{
            "type":"succeeded",
            "message":{
              "id":"msg1",
              "type":"message",
              "role":"assistant",
              "model":"test-model",
              "content":[{"type":"text","text":"A","citations":null}],
              "stop_reason":"end_turn",
              "stop_sequence":null,
              "usage":null,
              "container":null
            }
          }
        })
        .to_string(),
        json!({"custom_id":"req2","result":{"type":"canceled"}}).to_string(),
    ]
    .join("\n");

    Mock::given(method("GET"))
        .and(path("/results"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/binary")
                .set_body_string(jsonl),
        )
        .mount(&server)
        .await;

    let client = client_for(&server);
    let mut stream = client
        .messages
        .batches
        .results("batch1", None)
        .await
        .unwrap();

    let mut out = Vec::new();
    while let Some(item) = stream.next().await {
        out.push(item.unwrap());
    }

    assert_eq!(out.len(), 2);
    assert_eq!(out[0].custom_id, "req1");
    match &out[0].result {
        MessageBatchResult::Succeeded { message } => {
            let block = message.content[0].as_object().unwrap();
            assert_eq!(block.get("text").unwrap().as_str().unwrap(), "A");
        }
        _ => panic!("expected succeeded"),
    }
}
