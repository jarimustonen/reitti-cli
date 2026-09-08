# Agent skill installation validation

## Codex path compatibility

Verified 2026-09-08 with installed Codex CLI 0.153.4. The [current official skill documentation](https://learn.chatgpt.com/docs/build-skills) recommends `.agents/skills` for repository and user scopes. The family canon still requires the Codex installer layout `.codex/skills/<name>/...`.

A bounded, credential-free local App Server `skills/list` probe resolved this apparent discrepancy. In a disposable HOME/CODEX_HOME, distinct synthetic skills were placed in `.codex/skills` and `.agents/skills`. Both were reported as enabled user-scope skills with no discovery errors. No model turn or external API call was made, and no real user installation was changed. The temporary server and files were removed afterward. The [official App Server protocol](https://learn.chatgpt.com/docs/app-server) documents the read-only `skills/list` operation used.

Preserve the canon's `.codex/skills` layout and test the actual bundled skill through this native discovery path before release. Do not install duplicate copies into both directories: the runtime already recognizes the required layout, and duplicated skill names can appear independently in selectors. This verifies compatibility for the tested version rather than promising that a deprecated path will exist forever.

The actual bundled reitti resource tree was subsequently verified through this same native discovery path; see **Actual-bundle acceptance** below.

## Pi path compatibility and install base

Pi 0.84.4's installed `docs/skills.md` names `~/.pi/agent/skills` as a global skill directory. A separate disposable HOME/PI_CODING_AGENT_DIR probe ran Pi in offline RPC mode, with extensions/context files/prompt templates disabled and no session persistence. `get_commands` returned the synthetic skill placed at `.pi/agent/skills/<name>/SKILL.md` with source `skill`. No prompt/model call was sent; the temporary process and files were removed.

Use HOME as the default install base so all three canon layouts resolve to their actual user-level destinations. Treat `--target` as an explicit replacement install base, useful for staging and alternate home roots, not an automatic promise of repository-local discovery. In particular Pi's project-local layout is `.pi/skills`, whereas its required user-level layout is `.pi/agent/skills`. Do not silently mix these semantics or add duplicate installs.

## Claude path compatibility

Claude Code 2.1.236 recognized a synthetic user skill under `.claude/skills/<name>/SKILL.md` in a disposable HOME/CLAUDE_CONFIG_DIR. The bounded probe used the SDK control-protocol initialize exchange in streaming mode; its supported-command response contained the skill. No user prompt or model request was sent. MCP configuration was empty, session persistence and automatic updates were disabled, and the process/files were removed afterward. The [upstream SDK implementation](https://github.com/anthropics/claude-agent-sdk-python/blob/main/src/claude_agent_sdk/_internal/query.py) establishes that initialization is separate from sending a prompt.

All three required user-level path layouts now have native discovery evidence. The implementation repeated discovery with the actual bundled reitti resource tree; see **Actual-bundle acceptance** below.

## Installer consistency correction

The primary agent corrected the design draft's contradictory sentence that allowed an older installed skill to be overwritten without force. No-clobber applies to every differing existing tree, regardless of version: require `--force`. An exactly identical tree may be reported unchanged without writing it. Preflight all selected destinations, prepare complete trees, replace each destination atomically, and report actual applied destinations if a later destination fails. Do not claim one atomic transaction across three independent runtime directories.

## Implementation and integration seams

- The tracked bundle is `crates/reitti-cli/skills/reitti/`, with `SKILL.md` and the focused `references/workflows.md` support resource. Both are compiled into the binary with byte-preserving `include_bytes!`; print never reads an installed copy.
- `src/skill.rs` owns the one-skill catalogue, declared-resource selection, HOME/`--target` layout resolution, complete-tree comparison, staging, and per-destination application. The implementation is deliberately local rather than a generalized transaction framework.
- Fresh trees use native no-replace directory rename and forced replacements use native directory exchange: `renameatx_np` with `RENAME_EXCL`/`RENAME_SWAP` on macOS, and `renameat2` with `RENAME_NOREPLACE`/`RENAME_EXCHANGE` on Linux. Unsupported filesystems/platforms fail instead of falling back to a rename sequence with a visibility gap.
- All selected destinations are preflighted and all changed trees are prepared before the first apply. Temporary trees are cleaned on preparation/apply failure. A failure after an earlier destination applies reports the actual `applied` list; cleanup failure after exchange also records the replaced destination.
- The default base is HOME. `--target` replaces that base and accepts absolute or cwd-resolved relative paths; it does not change `.claude/skills`, `.pi/agent/skills`, or `.codex/skills`, and does not promise project-local runtime discovery.
- `src/skill_manifest.rs` validates only the leading frontmatter. `build.rs` runs this check before the build-commit early return, so git, source-archive, vendored, and CI-injected builds all enforce matching package/skill versions, schema 1, name, description length, and a real body. A release version bump must update `crates/reitti-cli/skills/reitti/SKILL.md` in the same commit or the build fails.
- `version --json` and `skill list` expose the actual bundled metadata and all three layouts. Offline doctor checks leading installed frontmatter at each canonical HOME destination for both CLI and skill-schema version without changing it.

## Deterministic installer and schema evidence

`crates/reitti-cli/tests/skill.rs` uses fresh disposable HOME/base directories and covers:

- default and explicit `--agent all`, plus each single runtime and a relative `--target`;
- all declared resources and print/install byte identity, `show` alias behavior, real repository paths, and traversal/arbitrary-resource rejection;
- exact identical reruns, no-clobber for older/newer/different trees, forced replacement, extra empty directories, and symlink entries;
- dry-run zero mutation and explicit global `--output` plan writing;
- unsupported agent/name, catalogue/version/frontmatter agreement, strict skill schemas, and doctor mismatch behavior;
- staged-tree cleanup and truthful applied-state details after a post-exchange cleanup failure.

The generic Skill Creator `quick_validate.py` was not used as a compatibility gate because its allowed-properties whitelist rejects the family-required top-level `cli_version` and `schema_version`. Those fields remain required and are validated directly by the build check and tests. Prior native probes accepted them.

## Actual-bundle acceptance

Primary tested the draft `target/debug/reitti` in a disposable HOME with an allowlisted credential-free environment and no model prompt. All processes and files were removed afterward:

- Claude Code 2.1.236 SDK control-protocol initialize discovered `reitti`;
- Pi 0.84.4 offline RPC `get_commands` discovered `reitti`;
- Codex 0.153.4 App Server `skills/list` discovered `reitti` from the required `.codex/skills` tree.

Primary also ran `/tmp/reitti-artifact-smoke.py` against the draft binary. All 35 offline checks passed, including config stdin/redaction/mode 0600, schema/help surfaces, default three-agent installation, identical rerun, no-clobber, and force replacement. This evidence applies to checkpoint `9a77267`; it does not involve live credentials or provider requests.

The final local gate run passed `cargo fmt --all --check`, clippy with locked workspace/all targets and warnings denied, all 76 workspace tests, all 21 credential-free Digitransit fixture checks, and `issuectl doctor --json` without findings. No credentialed live request was made by this worker.

Primary exported committed checkpoint `9a77267c1fc3a502b8dd24b0052d9946f2da3274` from tracked files without `.git` or ignored content and ran the whole workspace in the official Rust 1.88 Linux ARM64 container. All 76 tests passed, including 11 skill CLI tests exercising Linux native rename/exchange, no-clobber, and force replacement. The container was removed; primary retained the log privately at `history/skill-linux-test.log`. Hosted GitHub CI did not run because the account spending/billing restriction remains in effect; no remote CI success is claimed.

## Release-v1 handoff

`@release-v1` owns the final README/release packaging and the version 1.0.0 bump. The workspace `version` and bundled `cli_version` must be changed together in that release commit; the build-time drift check intentionally rejects an intermediate mismatch. Release acceptance should rerun the actual-bundle native discovery against the release binary and retain `.codex/skills/reitti` as the single Codex install destination (do not duplicate into `.agents/skills`).
