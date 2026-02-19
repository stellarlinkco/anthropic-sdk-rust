# anthropic-sdk-rust

Rust SDK for the Anthropic API.

## Status

This is an early version focusing on the core Anthropic API surface:

- Messages (non-streaming + SSE streaming)
- Models
- Message Batches (including JSONL results streaming)
- Legacy Completions

## Quickstart

```rust
use anthropic_sdk::{Anthropic, ClientOptions};
use anthropic_sdk::types::messages::{MessageCreateParams, MessageParam};

#[tokio::main]
async fn main() -> Result<(), anthropic_sdk::Error> {
    let client = Anthropic::new(ClientOptions::default())?;

    let message = client.messages.create(
        MessageCreateParams {
            model: "claude-sonnet-4-5-20250929".to_string(),
            max_tokens: 128,
            messages: vec![MessageParam::user("Hello, Claude")],
            ..Default::default()
        },
        None,
    ).await?;

    println!("{:?}", message.content);
    Ok(())
}
```

