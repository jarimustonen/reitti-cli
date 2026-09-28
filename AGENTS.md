# reitti-cli

Agent-first command-line journey planner for Helsinki, Espoo, Vantaa,
Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio, and Tuusula, powered by
Digitransit open data. The workspace is a library crate (`reitti-core`) and a
binary crate (`reitti-cli`) that ships the `reitti` command and a bundled agent
skill. There is no service to deploy: in this repository "deploy" means
publishing a versioned release to crates.io, GitHub Releases, and the project
Homebrew tap.

## CLI surface

The CLI follows the AI-first CLI canon: strict input validation, `--json`
output, JSONL logs, no interactive prompts, informative errors, composable
commands. The canon is the `/ai-first-cli-canon` skill that `project-canon
skill install` puts into the harness. Read it there when designing or changing
the surface. It is versioned and grows over time, so a copy kept in this
repository would only fall behind it.

## Where things live

- `docs/` is published documentation with its own `AGENTS.md`; read that
  before editing anything there. `docs/DISTRIBUTION.md` is the release
  runbook and `OSS-RELEASE.md` is the approved release contract.
- `history/` is gitignored. It holds agent scratch, planning drafts, and the
  logs and source snapshots left by release preflights. Nothing durable belongs
  there; durable knowledge goes into an `AGENTS.md` or an issue.
- `.env` at the root is gitignored and is where the maintainer keeps the
  Digitransit subscription key for local work.
- Every directory that needs agent guidance has `AGENTS.md`, with `CLAUDE.md`
  as a symlink to it, and optional `AGENTS-<TOPIC>.md` files for large topics.

## Issues and planning

Issues are managed with `issuectl` through the `/issue` skill. `issues/AGENTS.md`
and `.issuectl/AGENTS.md` are owned by that tool and describe the schema,
transitions, and mutation rules; they are the authority on how to touch an
issue. Each issue lives at `issues/<slug>/item.md` with its status in
frontmatter, not in the path.

Planning documents belong under the issue they serve, so that a plan has a
status and an owner and can be found from the work it describes. The names
other skills expect are `plan.md`, `analysis.md`, `validation.md` (design
assumptions checked against current reality, noting what differed from the
first analysis), `design.md`, `breakdown.md` (epic to child issues, with
dependencies and critical path), and `todo.md`. If work needs such a document,
it also needs an issue.

## Validation

There is no push or pull-request CI, and this is a maintainer decision rather
than a gap. The one GitHub workflow is the tag-triggered cargo-dist release
build, and its macOS job runs on the maintainer's self-hosted machine, which is
reserved for trusted release builds. Readiness audits will report the missing
CI; the answer is to explain the arrangement, not to regenerate a workflow.

The checks that CI would have run are these, run locally before merging:

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
python3 scripts/probe-digitransit.py check
issuectl doctor --json
```

The probe script's `check` mode is offline. It verifies that the recorded
Digitransit fixtures under `tests/fixtures/digitransit` are complete, carry the
expected schema version, and contain neither the secret header name in a URL
nor the currently exported key. Its `live` mode re-records the fixtures against
the real API and needs the key in the environment. The provider tests in
`crates/reitti-cli/tests` run against those fixtures, so provider failures and
CLI contracts can be tested deterministically without credentials; add such a
test when you change either.

Before a release, scan the whole tracked history for leaked secrets:

```sh
gitleaks git --redact --no-banner .
```

Redaction matters because the output ends up in transcripts and logs. A real
finding means rotating the credential first and cleaning history second.

## The subscription key

The Digitransit key is a real credential tied to the maintainer's account. It
is passed only in the request header and read from the environment or the
configuration file. Putting it in a command-line argument would leave it in
shell history and process listings; putting it in a fixture, test, or example
would publish it, since fixtures and docs ship with the repository. Live smoke
tests are deliberate and bounded for the same reason: they spend the
maintainer's quota against a real service.

## Reviews

Review findings are proposals. The maintainer has asked explicitly that the
primary agent reject false positives and low-value churn instead of working
through a list to zero. Fix a finding when reading the source or a focused
reproduction shows a material correctness, usability, security, or maintenance
problem, and prefer the smallest correction that does it. Speculative
machinery, work for unsupported platforms, cosmetic refactors, and repeated
review rounds cost more in a single-maintainer project than they return.
Record briefly why a suggestion was rejected; not every residual needs an
issue.

## Releases

Publication is the one thing here that cannot be taken back. A crates.io
version can be yanked but never replaced, a pushed tag and its GitHub Release
assets are what installers download, and the Homebrew formula follows them.
`docs/DISTRIBUTION.md` describes the channel, the artifact set, the
`reitti-core` before `reitti-cli` publication order, and how to verify a
release actually ran; a local build proves nothing about the published
release, and readiness is shown by working installation and smoke tests, not
by the presence of release documents.

On 2026-09-14 the maintainer gave standing authorization for agents to decide
and execute releases without a further confirmation. The condition is that an
explicit stint has finished with `/stint-handoff` and that the reservation-aware
issue DAG from `issuectl dag --json --reservations ...` is completely empty:
`lanes: []`, `unscheduled: []`, and `spawnable_heads: 0`. That empty result is
the signal that nothing is in flight or waiting on a decision. It authorizes
the decision only. Release from a clean, pushed `main` after the local checks,
the Gitleaks scan, SemVer and changelog preparation, the distribution checks in
the runbook, bounded live smoke tests, and inspection of the actual publication
have all passed. A non-empty or unverifiable DAG, a failed check, an open
decision, or an ambiguous publication state means stop and report rather than
ask; the maintainer prefers a clear report to a confirmation round.
`/stint-handoff` never deploys; a release is the separate action after it.

Two facts about the release configuration are easy to lose. The macOS artifact
is built on the maintainer's self-hosted ARM64 runner, the same machine the
other family projects use, and `dist-workspace.toml` carries that as a
`[dist.github-custom-runners]` override for `aarch64-apple-darwin`.
`shipshape dist generate` deliberately omits personal runner overrides, so
regenerating the distribution configuration drops it; put it back and rerun
cargo-dist's `generate` so the release workflow reflects it. Replacing the
runner with a hosted macOS runner would change a decision the maintainer has
made explicitly. The release workflow itself is generated output, not a file to
edit by hand.
