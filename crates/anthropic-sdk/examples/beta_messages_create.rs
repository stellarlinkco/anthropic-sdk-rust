use anthropic_sdk::resources::beta::messages::BetaMessageCreateParams;
use anthropic_sdk::types::messages::{MessageCreateParams, MessageParam};
use anthropic_sdk::{Anthropic, ClientOptions};

#[tokio::main]
async fn main() -> Result<(), anthropic_sdk::Error> {
    let client = Anthropic::new(ClientOptions::default())?;

    let message = client
        .beta
        .messages
        .create(
            BetaMessageCreateParams {
                betas: None,
                body: MessageCreateParams {
                    model: "claude-sonnet-4-5-20250929".to_string(),
                    max_tokens: 128,
                    messages: vec![MessageParam::user("Hello (beta endpoint).")],
                    ..Default::default()
                },
            },
            None,
        )
        .await?;

    println!("{:#?}", message.content);
    Ok(())
}
