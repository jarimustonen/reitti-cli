# Agent skill installation validation

## Codex path compatibility

Verified 2026-09-08 with installed Codex CLI 0.153.4. The [current official skill documentation](https://learn.chatgpt.com/docs/build-skills) recommends `.agents/skills` for repository and user scopes. The family canon still requires the Codex installer layout `.codex/skills/<name>/...`.

A bounded, credential-free local App Server `skills/list` probe resolved this apparent discrepancy. In a disposable HOME/CODEX_HOME, distinct synthetic skills were placed in `.codex/skills` and `.agents/skills`. Both were reported as enabled user-scope skills with no discovery errors. No model turn or external API call was made, and no real user installation was changed. The temporary server and files were removed afterward. The [official App Server protocol](https://learn.chatgpt.com/docs/app-server) documents the read-only `skills/list` operation used.

Preserve the canon's `.codex/skills` layout and test the actual bundled skill through this native discovery path before release. Do not install duplicate copies into both directories: the runtime already recognizes the required layout, and duplicated skill names can appear independently in selectors. This verifies compatibility for the tested version rather than promising that a deprecated path will exist forever.

The actual reitti resource tree, version pinning, no-clobber/dry-run/all-agent behavior and Claude/pi native discovery still need their implementation-stage checks.

## Pi path compatibility and install base

Pi 0.84.4's installed `docs/skills.md` names `~/.pi/agent/skills` as a global skill directory. A separate disposable HOME/PI_CODING_AGENT_DIR probe ran Pi in offline RPC mode, with extensions/context files/prompt templates disabled and no session persistence. `get_commands` returned the synthetic skill placed at `.pi/agent/skills/<name>/SKILL.md` with source `skill`. No prompt/model call was sent; the temporary process and files were removed.

Use HOME as the default install base so all three canon layouts resolve to their actual user-level destinations. Treat `--target` as an explicit replacement install base, useful for staging and alternate home roots, not an automatic promise of repository-local discovery. In particular Pi's project-local layout is `.pi/skills`, whereas its required user-level layout is `.pi/agent/skills`. Do not silently mix these semantics or add duplicate installs.

## Claude path compatibility

Claude Code 2.1.236 recognized a synthetic user skill under `.claude/skills/<name>/SKILL.md` in a disposable HOME/CLAUDE_CONFIG_DIR. The bounded probe used the SDK control-protocol initialize exchange in streaming mode; its supported-command response contained the skill. No user prompt or model request was sent. MCP configuration was empty, session persistence and automatic updates were disabled, and the process/files were removed afterward. The [upstream SDK implementation](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/query.py) establishes that initialization is separate from sending a prompt.

All three required user-level path layouts now have native discovery evidence. The implementation must repeat discovery with the actual bundled reitti resource tree before release.

## Installer consistency correction

The primary agent corrected the design draft's contradictory sentence that allowed an older installed skill to be overwritten without force. No-clobber applies to every differing existing tree, regardless of version: require `--force`. An exactly identical tree may be reported unchanged without writing it. Preflight all selected destinations, prepare complete trees, replace each destination atomically, and report actual applied destinations if a later destination fails. Do not claim one atomic transaction across three independent runtime directories.
