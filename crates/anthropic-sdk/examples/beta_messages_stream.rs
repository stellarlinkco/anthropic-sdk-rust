use anthropic_sdk::resources::beta::messages::BetaMessageCreateParams;
use anthropic_sdk::types::messages::{
    MessageCreateParams, MessageParam, RawContentBlockDelta, RawMessageStreamEvent,
};
use anthropic_sdk::{Anthropic, ClientOptions};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), anthropic_sdk::Error> {
    let client = Anthropic::new(ClientOptions::default())?;

    let mut stream = client
        .beta
        .messages
        .stream(
            BetaMessageCreateParams {
                betas: None,
                body: MessageCreateParams {
                    model: "claude-sonnet-4-5-20250929".to_string(),
                    max_tokens: 256,
                    messages: vec![MessageParam::user(
                        "Stream a short greeting (beta endpoint).",
                    )],
                    ..Default::default()
                },
            },
            None,
        )
        .await?;

    while let Some(ev) = stream.next().await {
        let ev = ev?;
        if let RawMessageStreamEvent::ContentBlockDelta {
            delta: RawContentBlockDelta::TextDelta { text },
            ..
        } = ev
        {
            print!("{text}");
        }
    }

    if let Some(final_msg) = stream.final_message() {
        eprintln!("\n\nfinal stop_reason={:?}", final_msg.stop_reason);
    }

    Ok(())
}
