use anthropic_sdk::types::batches::MessageBatchResult;
use anthropic_sdk::{Anthropic, ClientOptions};
use futures_util::StreamExt;

fn usage() -> ! {
    eprintln!("usage: cargo run -p anthropic-sdk-rs --example batches_results -- <batch_id>");
    std::process::exit(2);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let batch_id = std::env::args().nth(1).unwrap_or_else(|| usage());

    let client = Anthropic::new(ClientOptions::default())?;
    let mut stream = client.messages.batches.results(&batch_id, None).await?;

    while let Some(item) = stream.next().await {
        let item = item?;
        match item.result {
            MessageBatchResult::Succeeded { message } => {
                println!("{}: succeeded message_id={}", item.custom_id, message.id);
            }
            MessageBatchResult::Errored { error } => {
                println!("{}: errored error={:?}", item.custom_id, error);
            }
            MessageBatchResult::Canceled => {
                println!("{}: canceled", item.custom_id);
            }
            MessageBatchResult::Expired => {
                println!("{}: expired", item.custom_id);
            }
        }
    }

    Ok(())
}
