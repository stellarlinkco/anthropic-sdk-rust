# anthropic-sdk-rust

Rust SDK for the Anthropic API.

## Status

This is an early version focusing on the core Anthropic API surface:

- Messages (non-streaming + SSE streaming)
- Models
- Message Batches (including JSONL results streaming)
- Legacy Completions

## Install

```toml
[dependencies]
anthropic-sdk-rs = "0.1.2"
```

Or:

```bash
cargo add anthropic-sdk-rs
```

## Quickstart

Package name on crates.io is `anthropic-sdk-rs`; Rust import path is `anthropic_sdk`.

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

## Examples

Build all examples:

```bash
cargo build --examples
```

Run the mock smoke test (no API key required):

```bash
cargo run -p anthropic-sdk-rs --example smoke_mock
```

Run real API examples (requires `ANTHROPIC_API_KEY`):

```bash
cargo run -p anthropic-sdk-rs --example messages_create
cargo run -p anthropic-sdk-rs --example messages_stream
cargo run -p anthropic-sdk-rs --example beta_messages_create
cargo run -p anthropic-sdk-rs --example beta_messages_stream
cargo run -p anthropic-sdk-rs --example beta_messages_count_tokens
cargo run -p anthropic-sdk-rs --example beta_files_upload -- ./path/to/file text/plain
```

## Publishing

Dry-run publish:

```bash
make publish-dry-run
```

If you intentionally want to include uncommitted changes:

```bash
make publish-dry-run-dirty
```
