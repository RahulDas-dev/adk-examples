//! Example 06.2: Loading an Agent Skill (`SKILL.md`) into an agent -
//! model-driven tool path.
//!
//! The same weather skill as `06_1_skill.rs`, wired through the *other*
//! independent loading mechanism `adk-skill` provides: `SkillToolset`. Where
//! 06.1's `.with_skills(index)` automatically text-matches and injects a
//! skill's body into the user's message with no tool call involved,
//! `SkillToolset` instead contributes three real `Tool`s -
//! `list_skills`, `load_skill`, `load_skill_resource` - and the model has to
//! explicitly decide to call them.
//!
//! `SkillToolset`'s own doc comment confirms these two mechanisms don't
//! interact at all: *"This is opt-in; it does not alter the behavior of
//! [`SkillInjector`]"* (`SkillInjector` is what `.with_skills()` wires up
//! internally). So this file uses `SkillToolset` only, not both.
//!
//! Everything else is identical to 06.1 - same skill, same `AGENT_SKILL_DIR`
//! env var set once at startup for the skill's literal bash `${AGENT_SKILL_DIR}`
//! syntax to expand against. The only difference is the loading mechanism and
//! the extra tool-call round trips that come with it - compare the two files
//! and their runtime transcripts directly.
//!
//! One gap worth knowing: `SkillToolset::process_llm_request` is meant to
//! auto-inject instructions explaining its tools to the model, but nothing in
//! `adk-agent` or `adk-runner` ever calls `Toolset::process_llm_request` at
//! all (checked the whole dependency tree - zero call sites). So that method
//! is dead code via this path, and this example adds the equivalent guidance
//! manually through `.instruction()` instead.
//!
//! Setup:
//! ```text
//! ANTHROPIC_API_KEY=sk-ant-...
//! ANTHROPIC_MODEL=claude-sonnet-5   # optional
//! ```
//! Requires `curl` and `jq` on PATH (the weather skill's script uses them).
//! Run from the project root, so the relative skill path resolves.
//!
//! Run:
//! ```sh
//! cargo run --example 06_2_skill
//! ```
//!
//! Try: "What's the weather in Tokyo?" - watch for the extra list_skills /
//! load_skill tool calls in the transcript that 06.1 never makes.

use adk_devtools::{BashTool, Workspace};
use adk_rust::Launcher;
use adk_rust::prelude::*;
use adk_rust::skill::{SkillIndex, SkillToolset, SkillToolsetConfig, load_skill_index_with_extras};
use adk_rust::ToolRegistry;
use std::path::PathBuf;
use std::sync::Arc;

/// No skill here declares `allowed-tools`, so `SkillToolset` never needs to
/// resolve a business tool by name - this registry can stay empty.
struct EmptyToolRegistry;

impl ToolRegistry for EmptyToolRegistry {
    fn resolve(&self, _tool_name: &str) -> Option<Arc<dyn Tool>> {
        None
    }
}

/// `BashTool` spawns `"sh"` resolved via this process's own `PATH` -
/// `Workspace::bash_env()` reads it fresh on every call (verified against the
/// real `adk-devtools` source), so fixing it here, once, at startup covers
/// every subsequent `bash` call regardless of which terminal launched us.
/// Without this, `sh.exe` only resolves when launched from Git Bash itself -
/// a plain cmd.exe/PowerShell session's `PATH` usually has `Git\cmd` (enough
/// for `git.exe`) but not `Git\usr\bin`, where `sh.exe` actually lives, and
/// every `bash` tool call then fails with an empty, not-obviously-an-error
/// result (confirmed: reproduced this exact failure from PowerShell).
#[cfg(windows)]
fn ensure_git_bash_on_path() {
    let git_usr_bin = std::env::var("PROGRAMFILES")
        .map(|pf| format!("{pf}\\Git\\usr\\bin"))
        .unwrap_or_else(|_| "C:\\Program Files\\Git\\usr\\bin".to_string());

    if !std::path::Path::new(&git_usr_bin).join("sh.exe").exists() {
        return; // Git not installed there - nothing we can fix.
    }

    let path = std::env::var("PATH").unwrap_or_default();
    let already_present = path.split(';').any(|p| p.eq_ignore_ascii_case(&git_usr_bin));
    if !already_present {
        let new_path = format!("{git_usr_bin};{path}");
        // SAFETY: single-threaded example; called once at startup, before
        // any subprocess is spawned and before any other code reads PATH.
        unsafe {
            std::env::set_var("PATH", new_path);
        }
    }
}

#[cfg(not(windows))]
fn ensure_git_bash_on_path() {}

#[tokio::main]
async fn main() -> AnyhowResult<()> {
    ensure_git_bash_on_path();
    dotenvy::dotenv().ok();

    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    let model_name = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-5".to_string());
    let model = AnthropicClient::new(AnthropicConfig::new(&api_key, &model_name))?;

    let index = load_skill_index_with_extras(".", &[PathBuf::from("contrib/skills")])?;
    let weather_skill = index
        .find_by_name("weather")
        .cloned()
        .ok_or_else(|| adk_rust::anyhow::anyhow!("weather skill not found - run this from the project root"))?;
    let skill_dir = weather_skill.path.parent().expect("SKILL.md has a parent directory").to_path_buf();

    println!("Loaded skill '{}' from {}", weather_skill.name, skill_dir.display());

    // Same working-directory scoping as 06.1 - still needed so the skill's
    // relative paths (e.g. `scripts/weather.sh`) resolve correctly.
    let workspace = Workspace::new(skill_dir).allow_bash(true);
    let bash_tool = BashTool::new(workspace);

    // `index` holds every skill under contrib/skills/ (weather and
    // customer-verification, as of this writing) - filter to just the one
    // this example is actually about, so list_skills/load_skill only ever
    // see the weather skill, matching what this file's doc comment claims.
    let weather_only_index = SkillIndex::new(vec![weather_skill]);
    let skill_toolset = SkillToolset::new(
        Arc::new(weather_only_index),
        Arc::new(EmptyToolRegistry),
        SkillToolsetConfig::default(),
    );

    let agent = LlmAgentBuilder::new("weather_agent")
        .description("Weather assistant driven by a filesystem skill")
        .instruction(
            "You have list_skills and load_skill tools to discover and load skill instructions, \
             plus a bash tool to run commands. Only call them when the user's request actually \
             needs a skill's capability, such as checking the weather - not for greetings or \
             small talk. When a request does need one, always call these tools to fulfill it; \
             never claim you lack the ability to do something a loaded skill covers.\n\n\
             The bash tool's working directory is already the loaded skill's own folder - use \
             the exact relative path the skill's instructions give (e.g. `bash scripts/weather.sh \
             \"City\"`) directly. Do not cd elsewhere, guess a different path, or run `find` to \
             locate the script first.\n\n\
             When replying to the user, don't mention tool names, skill names, or any other \
             internal implementation detail - just report the result in plain language.",
        )
        .model(Arc::new(model))
        .toolset(Arc::new(skill_toolset))
        .tool(Arc::new(bash_tool))
        .build()?;

    Launcher::new(Arc::new(agent)).run().await?;
    Ok(())
}
