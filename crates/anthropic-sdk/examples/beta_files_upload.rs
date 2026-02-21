use anthropic_sdk::resources::beta::files::FileUploadParams;
use anthropic_sdk::{Anthropic, ClientOptions};
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!("usage: cargo run -p anthropic-sdk-rs --example beta_files_upload -- <path> [mime_type] [filename]");
    std::process::exit(2);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| usage());
    let mime_type = args.next();
    let filename = args.next();

    let client = Anthropic::new(ClientOptions::default())?;
    let file = client
        .beta
        .files
        .upload(
            FileUploadParams {
                path: PathBuf::from(path),
                filename,
                mime_type,
                betas: None,
            },
            None,
        )
        .await?;

    println!("uploaded file id={}", file.id);
    Ok(())
}
