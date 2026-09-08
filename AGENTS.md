# reitti-cli

Agent-first command-line journey planner for Helsinki, Espoo, Vantaa, Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio, and Tuusula, powered by Digitransit open data.

## CLI Design Principles

This project follows the AI-first CLI conventions in [`AGENTS-AI-FIRST-CLI.md`](AGENTS-AI-FIRST-CLI.md) — strict input validation, `--json` output, JSONL logs, no interactive prompts, informative errors, composable commands. Read that file before designing or changing CLI surface. The file is a verbatim copy from `project-canon`; treat it as shared canon, not a project-local doc to edit.

## Gitignored directories

- `history/` — agent scratchpad and ephemeral planning docs (not tracked)

## Validation and release work

Run these gates before merging implementation changes:

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
python3 scripts/probe-digitransit.py check
issuectl doctor --json
```

Add relevant deterministic integration tests for CLI contracts and provider failures.
Live smoke tests are deliberate, bounded checks with credentials supplied through
the environment; never print credentials or store them in arguments or fixtures.

This is a library and CLI project with no service deployment step. The current
authorized work includes CLI design, implementation, onboarding, documentation,
and OSS release preparation. Keep the GitHub repository private; changing its
visibility is reserved for the maintainer. Release readiness must be supported by
working installation and smoke tests, not only the presence of release documents.

### Review judgment

Review findings are proposals, not an automatic work list. The maintainer
explicitly requires the primary agent to reject false positives and low-value
review churn. Fix a finding when source inspection or a focused reproduction
shows a material correctness, usability, security, or maintenance problem.
Prefer the smallest effective correction. Do not add speculative machinery,
unsupported-platform work, cosmetic refactors, or repeated review rounds merely
to reach zero findings. Record concise reasons for rejecting suggestions; do not
automatically file every residual as an issue.

### macOS release runner

The maintainer requires the existing self-hosted macOS ARM64 build machine for
this project’s macOS release artifacts. Preserve the cargo-dist
`[dist.github-custom-runners]` override for `aarch64-apple-darwin`; regenerate
the workflow with cargo-dist after applying it. Do not replace this choice with
a hosted macOS runner during shipshape generation. Untrusted pull-request jobs
run on hosted runners; the self-hosted machine runs trusted release builds.

## Documentation Pattern

Every directory follows this structure:

- `CLAUDE.md` — symlink to `AGENTS.md`
- `AGENTS.md` — all AI-relevant info (consolidated)
- `AGENTS-<TOPIC>.md` — complex topics split out (optional)

## Issues & Planning

Issue tracking is managed by [`issuectl`](https://github.com/jarimustonen/issuectl). Use the `/issue` skill (installed by `issuectl init`) to create, search, update, and close issues.

- `issues/<slug>/item.md` — every issue and epic (flat layout — no numeric prefix, no `open/closed/` split)
- Status lives in the `status:` frontmatter field, not in the path
- `issues/AGENTS.md` — issue schema, types, workflow (owned by issuectl)
- `.issuectl/AGENTS.md` — repo-local policy for AI agents (owned by issuectl)

All planning documents (plans, analyses, validations, designs, breakdowns, todos) belong under their parent issue directory — not as standalone files. If work needs a planning document, it also needs an issue.

- `issues/<slug>/plan.md` — architecture, implementation plans
- `issues/<slug>/analysis.md` — research and analysis
- `issues/<slug>/validation.md` — design assumptions checked against current reality, noting what differs from first-pass analysis
- `issues/<slug>/design.md` — design documents
- `issues/<slug>/breakdown.md` — epic → child-issue breakdown with dependencies and critical path
- `issues/<slug>/todo.md` — task checklists
