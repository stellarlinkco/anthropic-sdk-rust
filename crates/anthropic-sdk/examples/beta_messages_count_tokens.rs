use anthropic_sdk::resources::beta::messages::BetaMessageCountTokensParams;
use anthropic_sdk::types::messages::{MessageCountTokensParams, MessageParam};
use anthropic_sdk::{Anthropic, ClientOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Anthropic::new(ClientOptions::default())?;

    let out = client
        .beta
        .messages
        .count_tokens(
            BetaMessageCountTokensParams {
                betas: None,
                body: MessageCountTokensParams {
                    model: "claude-sonnet-4-5-20250929".to_string(),
                    messages: vec![MessageParam::user("Count the tokens in this prompt.")],
                    ..Default::default()
                },
            },
            None,
        )
        .await?;

    println!("input_tokens={}", out.input_tokens);
    Ok(())
}
