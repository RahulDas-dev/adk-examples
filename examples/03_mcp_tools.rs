//! Example 03: Agent with MCP tools.
//!
//! Connects to a running MCP server over Streamable HTTP — here, the
//! fruit-pricing server at `contrib/mcps/fruits_mcp.py` — discovers its
//! tools, and wires them into an `LlmAgent` via `McpToolset`.
//!
//! Setup:
//! 1. Start the MCP server (listens on `http://127.0.0.1:3000/mcp` by default):
//! ```sh
//! cd contrib/mcps && uv run fruits_mcp.py
//! ```
//! 2. Put these in a `.env` file at the project root (gitignored):
//! ```text
//! ANTHROPIC_API_KEY=sk-ant-...
//! ANTHROPIC_MODEL=claude-sonnet-5          # optional, defaults to claude-sonnet-5
//! MCP_SERVER_URL=http://127.0.0.1:3000/mcp # optional, this is already the default
//! ```
//!
//! Run:
//! ```sh
//! cargo run --example 03_mcp_tools
//! ```
//!
//! Try asking it: "What's the price of a kg of mangoes?" or "What would 2kg of
//! apples and 5 bananas cost?"

use adk_rust::Launcher;
use adk_rust::ReadonlyContext;
use adk_rust::prelude::*;
use adk_rust::tool::{McpHttpClientBuilder, SimpleToolContext};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> AnyhowResult<()> {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    let model_name = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-5".to_string());
    let mcp_server_url =
        std::env::var("MCP_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/mcp".to_string());

    println!("Connecting to MCP server: {mcp_server_url}");

    let model = AnthropicClient::new(AnthropicConfig::new(&api_key, &model_name))?;

    let mcp_toolset = Arc::new(
        McpHttpClientBuilder::new(&mcp_server_url).timeout(Duration::from_secs(30)).connect().await?,
    );

    // Discover tools up front, just to show what the server offers.
    let ctx: Arc<dyn ReadonlyContext> = Arc::new(SimpleToolContext::new("mcp-discovery"));
    let tools = mcp_toolset.tools(ctx).await?;
    println!("Discovered {} tool(s) from the MCP server:", tools.len());
    for tool in &tools {
        println!("  - {}", tool.name());
    }
    println!();

    let agent = LlmAgentBuilder::new("fruit_agent")
        .description("Fruit pricing assistant backed by the fruits MCP server")
        .instruction(
            "You are a fruit pricing assistant. Use the available tools to look up \
             fruit prices and calculate order totals. Be concise.",
        )
        .model(Arc::new(model))
        .toolset(mcp_toolset)
        .build()?;

    Launcher::new(Arc::new(agent)).run().await?;
    Ok(())
}
