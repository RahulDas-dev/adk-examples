//! Example 06.1: Loading an Agent Skill (`SKILL.md`) into an agent -
//! automatic injection path.
//!
//! Wires a filesystem-based skill — a `SKILL.md` file plus bundled
//! scripts/references, per the `agentskills.io` specification — into an
//! `LlmAgent` via adk-rust's skill system. Uses the weather skill at
//! `contrib/skills/weather/`.
//!
//! See `06_2_skill.rs` for the same skill wired through the other,
//! independent loading mechanism (`SkillToolset`) instead - compare the two
//! files directly to see what differs in both code and runtime behavior.
//!
//! How this works, verified against the real `adk-skill` source:
//! - `load_skill_index_with_extras(root, extra_dirs)` discovers and parses
//!   every `.md` file under `extra_dirs` (plus the conventional `.skills/`
//!   and `.claude/skills/` under `root`) into a `SkillIndex`.
//! - `.with_skills(index)` on `LlmAgentBuilder` wires the index to the agent.
//!   On every turn, the agent scores the user's message against each skill's
//!   name/description/tags/body and, if a skill clears `min_score`, prepends
//!   its instructions to the user's message (`LlmAgent` calls
//!   `apply_skill_injection` internally — no manual plugin wiring needed).
//! - The *default* `SelectionPolicy` (`min_score: 1.0`) is stricter than most
//!   real skill bodies can clear — even `adk-skill`'s own tests use `0.1`.
//!   We do the same here.
//!
//! `adk-skill` only discovers, scores, and injects skill *text* — it doesn't
//! execute anything, and the `adk-rust` facade crate has no bundled
//! shell-execution tool. The real one lives in a separate crate,
//! `adk-devtools` (not re-exported by `adk-rust` — added here as its own
//! dependency), whose `BashTool` runs commands inside a `Workspace` with
//! streaming output and a timeout.
//!
//! ## How a skill locates its own files
//!
//! The weather skill's instructions reference `${AGENT_SKILL_DIR}` — a Claude
//! Code convention, not part of the `agentskills.io` spec itself (the spec's
//! own words are *"use relative paths from the skill root"*). Since it's
//! literal bash `${VAR}` syntax inside the skill's code blocks, the right fix
//! is a real shell environment variable, not text substitution in the
//! prompt - the shell expands it itself when the model's `bash` command runs.
//!
//! This example loads exactly one skill, so setting `AGENT_SKILL_DIR` once at
//! startup and adding it to the `Workspace`'s env allowlist is enough. With
//! multiple skills in different directories, a single startup-time value
//! can't be right for all of them - see `07_skill_variables.rs`, which
//! resolves `AGENT_SKILL_DIR` dynamically, per skill, from the exact
//! `skill_name` the model passes to `load_skill`.
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
//! cargo run --example 06_1_skill
//! ```
//!
//! Try: "What's the weather in Tokyo?"

use adk_devtools::{BashTool, DEFAULT_ENV_ALLOWLIST, Workspace};
use adk_rust::Launcher;
use adk_rust::prelude::*;
use adk_rust::skill::{SelectionPolicy, load_skill_index_with_extras};
use std::path::PathBuf;
use std::sync::Arc;

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

    // The skill's instructions use literal bash `${AGENT_SKILL_DIR}` syntax,
    // so the shell itself expands it when `bash` runs - we just need the
    // real env var set and allowlisted through to the subprocess.
    let skill_dir_str = skill_dir.to_string_lossy().replace('\\', "/");
    // SAFETY: single-threaded example; no concurrent env access.
    unsafe {
        std::env::set_var("AGENT_SKILL_DIR", &skill_dir_str);
    }
    let mut allowlist: Vec<String> = DEFAULT_ENV_ALLOWLIST.iter().map(|s| s.to_string()).collect();
    allowlist.push("AGENT_SKILL_DIR".to_string());
    let workspace = Workspace::new(".").allow_bash(true).env_allowlist(allowlist);
    let bash_tool = BashTool::new(workspace);

    // adk-skill's own tests use 0.1, not the default 1.0 - see the module doc above.
    let skill_policy = SelectionPolicy { top_k: 1, min_score: 0.1, ..SelectionPolicy::default() };

    let agent = LlmAgentBuilder::new("weather_agent")
        .description("Weather assistant driven by a filesystem skill")
        .instruction(
            "When a skill's instructions are injected into the user's message, follow them \
             exactly - including running the exact bash command they specify via the bash tool.",
        )
        .model(Arc::new(model))
        .with_skills(index)
        .with_skill_policy(skill_policy)
        .tool(Arc::new(bash_tool))
        .build()?;

    Launcher::new(Arc::new(agent)).run().await?;
    Ok(())
}
