//! Example 05: Dynamic instructions via `InstructionProvider`, reading
//! persisted user-level state.
//!
//! `.instruction()` sets a fixed system prompt once, at build time.
//! `.instruction_provider()` instead takes an async closure evaluated fresh on
//! *every* invocation, given that invocation's `ReadonlyContext` — letting the
//! system prompt vary per request without rebuilding the agent.
//!
//! This example seeds a banking officer's profile as **user-level** state
//! (the `user:` key prefix) before the agent ever runs, then has the
//! instruction provider read it back through `ctx.state()` on every turn.
//!
//! Key detail, verified against `adk-session`'s source: a `user:`-prefixed key
//! passed to `SessionService::create` is stored per `(app_name, user_id)`,
//! independent of the session ID, and gets merged into *any* session created
//! for that same user afterwards. `Launcher::run()` always creates its own
//! fresh session internally — so pre-seeding user-level state in a throwaway
//! session still surfaces correctly once the real conversation starts. A
//! plain (unprefixed) key would NOT carry over this way, since that's
//! session-scoped state tied to the specific session it was written in — that's
//! why the current time below is computed directly rather than read from state.
//!
//! Setup:
//! ```text
//! ANTHROPIC_API_KEY=sk-ant-...
//! ANTHROPIC_MODEL=claude-sonnet-5   # optional
//! ```
//!
//! Run:
//! ```sh
//! cargo run --example 05_instruction_provider
//! ```
//!
//! Try: "What's my name and branch?"

use adk_rust::InstructionProvider;
use adk_rust::Launcher;
use adk_rust::ReadonlyContext;
use adk_rust::prelude::*;
use adk_rust::session::{CreateRequest, InMemorySessionService, SessionService};
use chrono::Local;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

const APP_NAME: &str = "banking_agent";
const USER_ID: &str = "user";

#[tokio::main]
async fn main() -> AnyhowResult<()> {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    let model_name = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-5".to_string());

    let model = AnthropicClient::new(AnthropicConfig::new(&api_key, &model_name))?;

    // Seed a user profile as user-level state (the `user:` prefix), via a
    // throwaway session — only the user-level keys need to survive into the
    // real session Launcher creates next; the session itself is discarded.
    println!("Seeding user profile into session state...");
    let session_service = Arc::new(InMemorySessionService::new());
    let mut initial_state: HashMap<String, Value> = HashMap::new();
    initial_state.insert("user:user_name".to_string(), serde_json::json!("Rahul Das"));
    initial_state.insert("user:officer_id".to_string(), serde_json::json!("OFF-12345"));
    initial_state.insert("user:branch_name".to_string(), serde_json::json!("Mumbai Central"));
    initial_state.insert("user:role".to_string(), serde_json::json!("Senior Banking Officer"));

    session_service
        .create(CreateRequest {
            app_name: APP_NAME.to_string(),
            user_id: USER_ID.to_string(),
            session_id: None,
            state: initial_state,
        })
        .await?;

    // Evaluated on every invocation, given that invocation's ReadonlyContext.
    let provider: InstructionProvider = Box::new(|ctx: Arc<dyn ReadonlyContext>| {
        Box::pin(async move {
            let Some(state) = ctx.state() else {
                return Ok("You are a banking assistant.".to_string());
            };

            let get_str = |key: &str| -> String {
                state.get(key).and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "N/A".to_string())
            };

            let user_name = get_str("user:user_name");
            let officer_id = get_str("user:officer_id");
            let branch = get_str("user:branch_name");
            let now = Local::now().format("%Y-%m-%d %H:%M:%S %Z");

            Ok(format!(
                "You are assisting {user_name}.\n\
                 Officer ID: {officer_id}\n\
                 Branch: {branch}\n\
                 Current time: {now}\n\n\
                 When asked about 'my name', 'my branch', or 'my ID', use the info above directly \
                 — don't say you don't have access to it."
            ))
        })
    });

    let agent = LlmAgentBuilder::new(APP_NAME)
        .description("Banking assistant whose instruction is rebuilt from user profile state")
        .instruction_provider(provider)
        .model(Arc::new(model))
        .build()?;

    println!("Agent created. Try: \"What's my name and branch?\"\n");
    Launcher::new(Arc::new(agent)).app_name(APP_NAME).with_session_service(session_service).run().await?;
    Ok(())
}
