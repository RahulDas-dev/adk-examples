//! Basic single-agent console chat using Anthropic Claude.
//!
//! Builds one `LlmAgent` backed by `AnthropicClient` and runs it through the
//! interactive console `Launcher` (type a message, type `exit` to quit).
//!
//! Setup: put these in a `.env` file at the project root (gitignored):
//! ```text
//! ANTHROPIC_API_KEY=sk-ant-...
//! ANTHROPIC_MODEL=claude-sonnet-5   # optional, defaults to claude-sonnet-5
//! ```
//!
//! Run:
//! ```sh
//! cargo run --example 01_basic_agent
//! ```

use adk_rust::Launcher;
use adk_rust::prelude::*;

#[tokio::main]
async fn main() -> AnyhowResult<()> {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    let model_name = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-5".to_string());

    let model = AnthropicClient::new(AnthropicConfig::new(&api_key, &model_name))?;

    let agent = LlmAgentBuilder::new("assistant")
        .instruction("You are a helpful assistant. Be concise and accurate.")
        .model(Arc::new(model))
        .build()?;

    Launcher::new(Arc::new(agent)).run().await?;
    Ok(())
}
