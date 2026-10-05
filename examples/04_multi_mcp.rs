//! Example 04: Multiple MCP servers merged into one agent.
//!
//! Combines two MCP servers of *different transports* into a single toolset:
//! - The official filesystem server (`@modelcontextprotocol/server-filesystem`),
//!   declared via a JSON config and run by `McpServerManager` as a stdio
//!   subprocess, scoped to `./fs-sandbox`.
//! - The fruits pricing server from `contrib/mcps/fruits_mcp.py`, over
//!   Streamable HTTP (same server as example 03).
//!
//! `McpServerManager` manages any number of **stdio** servers from one JSON
//! config (the same shape as Kiro's `mcp.json`) — add more entries under
//! `mcpServers` to run more stdio servers through it. It already implements
//! `Toolset` itself and auto-prefixes tool names that collide *between its own
//! managed servers*, so adding another stdio server needs no extra wiring.
//!
//! It cannot hold the HTTP fruits server too, though — `McpServerConfig` only
//! spawns subprocesses (`command`/`args`/`env`), there's no URL field. So the
//! manager (one Toolset covering all stdio servers) and the fruits `McpToolset`
//! are combined with a single `MergedToolset` call.
//!
//! Setup:
//! 1. Start the fruits MCP server:
//! ```sh
//! cd contrib/mcps && uv run fruits_mcp.py
//! ```
//! 2. Node.js/npx must be installed (for `npx -y @modelcontextprotocol/server-filesystem`).
//! 3. `.env` at the project root (gitignored):
//! ```text
//! ANTHROPIC_API_KEY=sk-ant-...
//! ANTHROPIC_MODEL=claude-sonnet-5           # optional
//! MCP_SERVER_URL=http://127.0.0.1:3000/mcp  # optional, fruits server
//! FS_MCP_ALLOWED_DIR=./fs-sandbox           # optional, directory the filesystem server may access
//! ```
//!
//! Run:
//! ```sh
//! cargo run --example 04_multi_mcp
//! ```
//!
//! Try: "List the files in the sandbox directory and read sample.txt" or
//! "What's the price of a kg of apples?"

use adk_rust::Launcher;
use adk_rust::ReadonlyContext;
use adk_rust::prelude::*;
use adk_rust::tool::{McpHttpClientBuilder, McpServerManager, MergedToolset, SimpleToolContext};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> AnyhowResult<()> {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    let model_name = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-5".to_string());
    let mcp_server_url =
        std::env::var("MCP_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/mcp".to_string());
    let fs_allowed_dir =
        std::env::var("FS_MCP_ALLOWED_DIR").unwrap_or_else(|_| "./fs-sandbox".to_string());

    let model = AnthropicClient::new(AnthropicConfig::new(&api_key, &model_name))?;

    // --- Stdio servers, declared once as JSON, run by McpServerManager ---
    // On Windows, `npx` is a `.cmd` shim and can't be spawned directly — it
    // must run through `cmd.exe /C`. Other platforms spawn it directly.
    // Add more entries under "mcpServers" here to manage more stdio servers.
    #[cfg(windows)]
    let filesystem_server = serde_json::json!({
        "command": "cmd",
        "args": ["/C", "npx", "-y", "@modelcontextprotocol/server-filesystem", fs_allowed_dir],
    });
    #[cfg(not(windows))]
    let filesystem_server = serde_json::json!({
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", fs_allowed_dir],
    });

    let stdio_config = serde_json::json!({
        "mcpServers": { "filesystem": filesystem_server }
    })
    .to_string();

    println!("Starting stdio MCP server(s), filesystem scoped to: {fs_allowed_dir}");
    let manager = Arc::new(McpServerManager::from_json(&stdio_config)?);
    for (id, result) in manager.start_all().await {
        result.map_err(|e| adk_rust::anyhow::anyhow!("failed to start MCP server '{id}': {e}"))?;
    }

    // --- Fruits server (Streamable HTTP) — outside the manager's reach ---
    println!("Connecting to fruits MCP server: {mcp_server_url}");
    let fruits_toolset = Arc::new(
        McpHttpClientBuilder::new(&mcp_server_url).timeout(Duration::from_secs(30)).connect().await?,
    );

    // The manager is itself a Toolset covering every stdio server it runs;
    // merge it with the HTTP toolset to get one combined tool source.
    let merged = MergedToolset::new(
        "mcp_tools",
        vec![manager as Arc<dyn Toolset>, fruits_toolset as Arc<dyn Toolset>],
    );

    // Discover tools across both servers, just to show what's available.
    let ctx: Arc<dyn ReadonlyContext> = Arc::new(SimpleToolContext::new("mcp-discovery"));
    let tools = merged.tools(ctx).await?;
    println!("Discovered {} tool(s) across both MCP servers:", tools.len());
    for tool in &tools {
        println!("  - {}", tool.name());
    }
    println!();

    let agent = LlmAgentBuilder::new("multi_mcp_agent")
        .description("Assistant with fruit pricing and sandboxed filesystem access")
        .instruction(
            "You have fruit pricing tools and filesystem tools scoped to a sandbox \
             directory. Use them when relevant. Be concise.",
        )
        .model(Arc::new(model))
        .toolset(Arc::new(merged))
        .build()?;

    Launcher::new(Arc::new(agent)).run().await?;
    Ok(())
}
