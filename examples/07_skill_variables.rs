//! Example 07: `{variable}` substitution in skill text, at scale.
//!
//! Example 06 established that the `agentskills.io` spec defines no variable
//! substitution at all — `adk-skill` injects a skill's body completely
//! verbatim. That's fine for file paths (use relative paths; see example 06),
//! but falls short for something like 200 banking skills whose instructions
//! need per-officer, per-branch runtime values (`{officer_id}`, `{branch_name}`,
//! ...) filled in before the model ever sees them.
//!
//! `adk-skill` has two *independent* loading mechanisms - `.with_skills(index)`
//! (automatic text-matching injection, no tool call involved at all) and
//! `SkillToolset` (model-driven: the model explicitly calls a tool to load a
//! skill). They don't interact; `SkillToolset`'s own doc comment says so
//! directly: *"This is opt-in; it does not alter the behavior of
//! [`SkillInjector`]"* - `SkillInjector` is exactly what `.with_skills()` wires
//! up internally. This example uses `SkillToolset` only, so there's exactly
//! one skill-delivery path instead of two overlapping ones.
//!
//! `SkillToolset` (the `progressive-disclosure` feature) contributes three
//! real `Tool`s: `list_skills`, `load_skill`, `load_skill_resource`. When the
//! model calls `load_skill("customer-verification")`, it gets the body back
//! as a tool result - and `EnhancedPlugin::after_tool_call`, scoped to
//! exactly those two tool names, rewrites that JSON result's `instructions`
//! (`load_skill`) or `content` (`load_skill_resource`) field before it reaches
//! the model. One plugin, one `substitute_vars` helper, no per-skill wiring -
//! it works identically no matter which of 200 skills got loaded.
//!
//! ## `AGENT_SKILL_DIR`, resolved per loaded skill
//!
//! Example 06.1 could only set `AGENT_SKILL_DIR` once at startup, because it
//! loads exactly one skill. That doesn't generalize to 200 skills in
//! different directories. This is the mechanism that does: `after_tool_call`
//! fires with the exact `skill_name` argument the model passed to `load_skill`/
//! `load_skill_resource`, so the plugin can look *that specific skill* up in
//! the index, take its directory, and set `AGENT_SKILL_DIR` to it right then -
//! dynamically, per call, correct for whichever skill was just loaded.
//!
//! `after_tool_call` (on `load_skill`) replaces `${AGENT_SKILL_DIR}`/
//! `$AGENT_SKILL_DIR` directly in the `instructions` text, so the model sees
//! the real path already baked in - same text-substitution mechanism as
//! `{var}` below. `AGENT_SKILL_DIR` is deliberately never mentioned to the
//! model anywhere (not in `.instruction()`, nowhere) - it's purely an
//! internal detail of how this plugin resolves paths before the model ever
//! sees them. An earlier version of this example's `.instruction()` *did*
//! name `$AGENT_SKILL_DIR` and showed a worked `bash` example using it as a
//! bare shell variable - which predictably taught the model to write that
//! same bare-variable form itself, and since nothing sets a real shell env
//! var for it, every one of those commands failed. The fix wasn't adding a
//! mechanism to make `$AGENT_SKILL_DIR` work as a shell variable - it was
//! never telling the model that name exists, so it has nothing to copy but
//! the real path already substituted into the text.
//!
//! One gap in `SkillToolset` itself, worth knowing: its `Toolset::process_llm_request`
//! is meant to auto-inject instructions explaining `list_skills`/`load_skill`/
//! `load_skill_resource` to the model, but nothing in `adk-agent` or `adk-runner`
//! ever calls `Toolset::process_llm_request` at all (checked the whole dependency
//! tree — zero call sites). So that method is dead code via this path, and this
//! example adds the equivalent guidance manually through `.instruction()` instead.
//!
//! ## Using the real `EnhancedPlugin` system (corrections to the docs site)
//!
//! Checking `https://adk-rust.com/en/docs/core/plugins` against the actual
//! `adk-plugin` source turned up several inaccuracies worth recording, since
//! that site has been unreliable elsewhere in this project too:
//! - The docs show registering via `.plugin(Arc::new(...))`. The real method
//!   is `.enhanced_plugin(Arc::new(...))` (or `.enhanced_plugins(vec![...])`)
//!   — `.plugin()` doesn't exist on `LlmAgentBuilder`.
//! - The docs list `before_run`/`after_run` and `on_event` as `EnhancedPlugin`
//!   hooks. The real trait has exactly four: `before_tool_call`, `after_tool_call`,
//!   `before_model_call`, `after_model_call` (plus `name()`, `priority()`, `close()`).
//!   The hooks the docs listed belong to a different, older `PluginConfig`
//!   struct (what `SkillInjector::build_plugin()` returns internally) - a
//!   separate system, not `EnhancedPlugin`.
//! - The docs say `PluginContext` carries `user_id`/`session_id`/`agent_name`.
//!   The real `PluginContext` is a generic typed key-value store (`insert::<T>`/
//!   `get::<T>`) for state shared *across* plugins - it has no such accessors.
//!   `user_id()`/`session_id()`/`state()` come from the separate `CallbackContext`
//!   parameter every hook also receives.
//!
//! There's also a real gap in the *published* `adk-rust` v2.2.0 crate itself:
//! its `plugin` feature only enables the `adk-plugin` dependency - it never
//! forwards to `adk-agent`'s `enhanced-plugins` feature (disabled via that
//! facade's `default-features = false`), so `.enhanced_plugin()` is otherwise
//! unreachable at any `adk-rust` feature combination. `Cargo.toml` here works
//! around it by depending on `adk-agent` directly with `features =
//! ["enhanced-plugins"]` - Cargo unifies that into the same build adk-rust uses.
//!
//! Values come from session state, seeded the same way as example 05's
//! `user:`-prefixed state (so it persists into whatever session `Launcher`
//! creates). Any `{name}` not found in state is left untouched rather than
//! silently dropped, so a missing/misspelled variable stays visible instead
//! of disappearing into blank text.
//!
//! Runs from any Windows terminal (cmd.exe, PowerShell, or Git Bash) -
//! `ensure_git_bash_on_path()` below fixes the one thing that otherwise only
//! works from Git Bash itself. See its own doc comment for why.
//!
//! Setup:
//! ```text
//! ANTHROPIC_API_KEY=sk-ant-...
//! ANTHROPIC_MODEL=claude-sonnet-5   # optional
//! ```
//! Requires `curl` and `jq` on PATH (the weather skill's script uses them).
//!
//! Run:
//! ```sh
//! cargo run --example 07_skill_variables
//! ```
//!
//! Try: "What's the weather in Tokyo?" (forces load_skill("weather"), setting
//! AGENT_SKILL_DIR to that skill's directory) then "I need to verify a
//! customer for a large withdrawal" (forces load_skill("customer-verification"),
//! re-pointing AGENT_SKILL_DIR at a *different* directory in the same run).

use adk_devtools::{BashTool, Workspace};
use adk_rust::plugin::{AfterToolCallResult, BeforeModelCallResult, EnhancedPlugin, PluginContext};
use adk_rust::prelude::*;
use adk_rust::session::{CreateRequest, InMemorySessionService, SessionService};
use adk_rust::skill::{SkillIndex, SkillToolset, SkillToolsetConfig, load_skill_index_with_extras};
use adk_rust::{CallbackContext, Launcher, ToolRegistry};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

const APP_NAME: &str = "banking_skills_agent";
const USER_ID: &str = "user";

/// Replaces every well-formed `{identifier}` in `text` with a matching value
/// from `state` (checked as `user:{identifier}` first, then `{identifier}`
/// plain). An identifier with no match is left as `{identifier}` rather than
/// removed, so a typo'd or unset variable stays visible instead of vanishing.
fn substitute_vars(text: &str, state: &dyn State) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '{' {
            result.push(c);
            continue;
        }

        let mut ident = String::new();
        let mut closed = false;
        while let Some(&next) = chars.peek() {
            if next == '}' {
                chars.next();
                closed = true;
                break;
            }
            if !(next.is_ascii_alphanumeric() || next == '_') {
                break;
            }
            ident.push(next);
            chars.next();
        }

        if !closed || ident.is_empty() {
            result.push('{');
            result.push_str(&ident);
            if closed {
                result.push('}');
            }
            continue;
        }

        let value = state.get(&format!("user:{ident}")).or_else(|| state.get(&ident));
        match value.as_ref().and_then(JsonValue::as_str) {
            Some(s) => result.push_str(s),
            None => {
                result.push('{');
                result.push_str(&ident);
                result.push('}');
            }
        }
    }

    result
}

/// No skill here declares `allowed-tools`, so `SkillToolset` never needs to
/// resolve a business tool by name - this registry can stay empty.
struct EmptyToolRegistry;

impl ToolRegistry for EmptyToolRegistry {
    fn resolve(&self, _tool_name: &str) -> Option<Arc<dyn Tool>> {
        None
    }
}

/// Marker stored in `PluginContext` once the skill catalog has been
/// prepended for the current turn, so a second/third model call within that
/// same turn (one per tool round-trip) doesn't prepend it again.
#[derive(Clone)]
struct CatalogInjected;

/// Substitutes `{var}` placeholders in `load_skill`/`load_skill_resource`
/// tool results, and - for those same two tools - resolves `AGENT_SKILL_DIR`
/// to the specific skill that was just loaded, from its `skill_name` argument.
/// Also prepends the skill catalog to the current user message each turn, so
/// the model never has to call list_skills just to see what's available.
struct SkillVariablePlugin {
    index: Arc<SkillIndex>,
}

#[async_trait]
impl EnhancedPlugin for SkillVariablePlugin {
    fn name(&self) -> &str {
        "skill-variable-substitution"
    }

    async fn before_model_call(
        &self,
        mut request: LlmRequest,
        _ctx: Arc<dyn CallbackContext>,
        plugin_ctx: &PluginContext,
    ) -> Result<BeforeModelCallResult> {
        // One turn triggers several model calls when tools are involved
        // (initial call, then one more per tool round-trip) - this hook fires
        // on every one of them. Without this guard, the catalog would get
        // prepended again on top of itself each time within the same turn.
        // PluginContext resets per invocation, so this allows exactly one
        // prepend per user turn, not one per model call.
        if plugin_ctx.get::<CatalogInjected>().await.is_none()
            && let Some(content) = request.contents.iter_mut().rev().find(|c| c.role == "user")
            && let Some(Part::Text { text }) = content.parts.iter_mut().find(|p| matches!(p, Part::Text { .. }))
        {
            let mut skill_catalog = String::from(
                "<skills_system>\n\
                 To run a skill, call the 'load_skill' tool with the skill name and any arguments.\n\
                 Do NOT call skill names as functions or generate skill content yourself.\n\
                 When a user request clearly matches a skill, call load_skill BEFORE generating a response.\n\
                 Available skills:\n",
            );
            for skill in self.index.skills() {
                skill_catalog.push_str(&format!("- **{}**: {}\n", skill.name, skill.description));
                if let Some(hint) = &skill.hint {
                    skill_catalog.push_str(&format!("  hint: {hint}\n"));
                }
            }
            skill_catalog.push_str("</skills_system>");

            *text = format!("{skill_catalog}\n\n{text}");
            plugin_ctx.insert(CatalogInjected).await;
        }
        Ok(BeforeModelCallResult::Continue(request))
    }

    async fn after_tool_call(
        &self,
        tool: Arc<dyn Tool>,
        args: &JsonValue,
        mut result: JsonValue,
        ctx: Arc<dyn CallbackContext>,
        _plugin_ctx: &PluginContext,
    ) -> Result<AfterToolCallResult> {
        // Diagnostic only: the console otherwise never shows what `bash`
        // actually returned, so a failure is impossible to debug after the
        // fact. Print exit_code/stdout/stderr so a future failure leaves a
        // real error message instead of silence.
        if tool.name() == "bash" {
            let exit_code = result.get("exit_code").map(ToString::to_string).unwrap_or_default();
            let stdout = result.get("stdout").and_then(JsonValue::as_str).unwrap_or_default();
            let stderr = result.get("stderr").and_then(JsonValue::as_str).unwrap_or_default();
            println!(
                "[plugin:{}] bash result: exit_code={exit_code} stdout={stdout:?} stderr={stderr:?}",
                self.name()
            );
        }

        let field = match tool.name() {
            "load_skill" => Some("instructions"),
            "load_skill_resource" => Some("content"),
            _ => None,
        };

        let Some(field) = field else {
            return Ok(AfterToolCallResult::Continue(result));
        };

        // Extract the skill name the model asked for and look it up in the
        // index, to get *that* skill's own directory - correct for this call
        // regardless of what any previous call resolved.
        let skill_dir = args.get("skill_name").and_then(JsonValue::as_str).and_then(|skill_name| {
            let dir = self.index.find_by_name(skill_name).and_then(|s| s.path.parent())?;
            let cwd = std::env::current_dir().unwrap_or_default();
            let dir = if dir.is_absolute() { dir.to_path_buf() } else { cwd.join(dir) };
            let dir = dir.to_string_lossy().replace('\\', "/");
            Some(dir)
        });

        if let Some(text) = result.get(field).and_then(JsonValue::as_str) {
            let mut substituted = text.to_string();

            // Replace the skill-directory placeholder first - both brace
            // forms - directly with the resolved path, independent of the
            // env var set above.
            if let Some(dir) = &skill_dir {
                substituted = substituted.replace("${AGENT_SKILL_DIR}", dir).replace("$AGENT_SKILL_DIR", dir);
            }

            // Then session-state-backed {var} placeholders, e.g. {officer_name}.
            if let Some(state) = ctx.state() {
                substituted = substitute_vars(&substituted, state);
            }

            // Only load_skill gets the header - load_skill_resource returns a
            // bundled file's raw content, which shouldn't be prefixed with it.
            if tool.name() == "load_skill"
                && let Some(skill_name) = args.get("skill_name").and_then(JsonValue::as_str)
            {
                let dir_display = skill_dir.as_deref().unwrap_or("<unresolved>");
                substituted = format!(
                    "Skill - {skill_name}\nBase Directory for the skill - {dir_display}\n\n{substituted}"
                );
            }

            result[field] = JsonValue::String(substituted);
        }

        Ok(AfterToolCallResult::Continue(result))
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

    // Seed officer context as user-level state - see example 05 for why this
    // survives into the session Launcher creates next.
    println!("Seeding officer profile into session state...");
    let session_service = Arc::new(InMemorySessionService::new());
    let mut initial_state: HashMap<String, JsonValue> = HashMap::new();
    initial_state.insert("user:officer_name".to_string(), serde_json::json!("Rahul Das"));
    initial_state.insert("user:officer_id".to_string(), serde_json::json!("OFF-12345"));
    initial_state.insert("user:branch_name".to_string(), serde_json::json!("Mumbai Central"));

    session_service
        .create(CreateRequest {
            app_name: APP_NAME.to_string(),
            user_id: USER_ID.to_string(),
            session_id: None,
            state: initial_state,
        })
        .await?;

    let index = Arc::new(load_skill_index_with_extras(".", &[PathBuf::from("contrib/skills")])?);
    println!(
        "Loaded {} skill(s): {}",
        index.len(),
        index.skills().iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
    );

    // SkillToolset contributes list_skills / load_skill / load_skill_resource
    // as real tools - the only skill-delivery mechanism in this example.
    let skill_toolset =
        SkillToolset::new(index.clone(), Arc::new(EmptyToolRegistry), SkillToolsetConfig::default());

    // Rooted at the project directory, not any one skill's folder - the
    // *actual* skill directory is only ever text-substituted into the
    // instructions the model reads - nothing reads it from a real env var,
    // so no custom allowlist is needed.
    let workspace = Workspace::new(".").allow_bash(true);
    let bash_tool = BashTool::new(workspace);

    let agent = LlmAgentBuilder::new(APP_NAME)
        .description("Helpful weather assistant")
        .instruction(
            "You are a helpful weather assistant. Use your available tools to find and load the \
             relevant instructions before acting on a request that needs them, then follow \
             exactly what you're told, including any file paths given, rather than guessing or \
             substituting your own.",
        )
        .model(Arc::new(model))
        .toolset(Arc::new(skill_toolset))
        .tool(Arc::new(bash_tool))
        .enhanced_plugin(Arc::new(SkillVariablePlugin { index }))
        .build()?;

    Launcher::new(Arc::new(agent)).app_name(APP_NAME).with_session_service(session_service).run().await?;
    Ok(())
}
