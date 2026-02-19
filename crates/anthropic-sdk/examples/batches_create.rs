use anthropic_sdk::types::batches::{BatchCreateParams, BatchRequest};
use anthropic_sdk::types::messages::{MessageCreateParams, MessageParam};
use anthropic_sdk::{Anthropic, ClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Anthropic::new(ClientOptions::default())?;

    let batch = client
        .messages
        .batches
        .create(
            BatchCreateParams {
                requests: vec![BatchRequest {
                    custom_id: "req_1".to_string(),
                    params: MessageCreateParams {
                        model: "claude-sonnet-4-5-20250929".to_string(),
                        max_tokens: 64,
                        messages: vec![MessageParam::user("Say hello from a batch request.")],
                        ..Default::default()
                    },
                }],
            },
            None,
        )
        .await?;

    println!("batch id={}", batch.id);
    Ok(())
}
