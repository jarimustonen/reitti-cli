# Agent skill installation validation

## Codex path compatibility

Verified 2026-09-08 with installed Codex CLI 0.153.4. The [current official skill documentation](https://learn.chatgpt.com/docs/build-skills) recommends `.agents/skills` for repository and user scopes. The family canon still requires the Codex installer layout `.codex/skills/<name>/...`.

A bounded, credential-free local App Server `skills/list` probe resolved this apparent discrepancy. In a disposable HOME/CODEX_HOME, distinct synthetic skills were placed in `.codex/skills` and `.agents/skills`. Both were reported as enabled user-scope skills with no discovery errors. No model turn or external API call was made, and no real user installation was changed. The temporary server and files were removed afterward. The [official App Server protocol](https://learn.chatgpt.com/docs/app-server) documents the read-only `skills/list` operation used.

Preserve the canon's `.codex/skills` layout and test the actual bundled skill through this native discovery path before release. Do not install duplicate copies into both directories: the runtime already recognizes the required layout, and duplicated skill names can appear independently in selectors. This verifies compatibility for the tested version rather than promising that a deprecated path will exist forever.

The actual reitti resource tree, version pinning, no-clobber/dry-run/all-agent behavior and Claude/pi native discovery still need their implementation-stage checks.
