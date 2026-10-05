# Examples

Each example is a standalone binary. Run one with:

```sh
cargo run --example <name>
```

Requires a `.env` file at the project root (gitignored) with your credentials, e.g.:

```text
ANTHROPIC_API_KEY=sk-ant-...
ANTHROPIC_MODEL=claude-sonnet-5   # optional, per-example default noted below
```

| Example | Run | Description |
|---|---|---|
| [`01_basic_agent.rs`](./01_basic_agent.rs) | `cargo run --example 01_basic_agent` | Single `LlmAgent` backed by Anthropic Claude, run through the interactive console `Launcher`. |
| [`02_custom_tools.rs`](./02_custom_tools.rs) | `cargo run --example 02_custom_tools` | Agent with two custom `FunctionTool`s (`get_datetime`, `get_weather`) using `#[derive(JsonSchema)]` parameter structs. |
| [`03_mcp_tools.rs`](./03_mcp_tools.rs) | `cargo run --example 03_mcp_tools` | Agent wired to a remote MCP server (`McpToolset` over Streamable HTTP). Requires the fruits MCP server from `contrib/mcps/` running first. |
| [`04_multi_mcp.rs`](./04_multi_mcp.rs) | `cargo run --example 04_multi_mcp` | Declares stdio MCP servers via a JSON config run by `McpServerManager` (add more under `mcpServers` to manage more), then merges that with the HTTP fruits server via `MergedToolset`. Requires the fruits server running and Node/npx installed. |
| [`05_instruction_provider.rs`](./05_instruction_provider.rs) | `cargo run --example 05_instruction_provider` | Dynamic system prompt via `.instruction_provider()`, reading a user profile seeded as `user:`-prefixed session state back through `ctx.state()` on every turn. |
| [`06_1_skill.rs`](./06_1_skill.rs) | `cargo run --example 06_1_skill` | Loads the `agentskills.io`-spec `SKILL.md` weather skill from `contrib/skills/weather/` via `load_skill_index_with_extras` + `.with_skills()` (automatic text-matching injection), executed with `adk-devtools`' `BashTool` scoped to the skill's own directory. Requires `curl`/`jq` on PATH. |
| [`06_2_skill.rs`](./06_2_skill.rs) | `cargo run --example 06_2_skill` | The same weather skill as 06.1, but loaded via the other, independent `SkillToolset` mechanism instead - the model explicitly calls `list_skills`/`load_skill` itself. Compare the two files and transcripts directly to see the difference. Requires `curl`/`jq` on PATH. |
| [`07_skill_variables.rs`](./07_skill_variables.rs) | `cargo run --example 07_skill_variables` | Model-driven skill loading via `SkillToolset` (`list_skills`/`load_skill`/`load_skill_resource`, no automatic injection). An `EnhancedPlugin`'s `after_tool_call` hook substitutes `{var}` placeholders and `${AGENT_SKILL_DIR}` directly in the `load_skill` result text, resolved fresh from the `skill_name` argument on every call - correct for whichever of e.g. 200 skills was just loaded, with no per-skill wiring. `AGENT_SKILL_DIR` is deliberately never named in `.instruction()` - see the file's doc comment for why that matters. Needs `adk-agent` depended on directly with `features = ["enhanced-plugins"]`. |
