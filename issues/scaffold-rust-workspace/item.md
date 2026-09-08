---
created: 2026-09-08
updated: 2026-09-08
type: task
reporter: jarimustonen
status: done
priority: normal
epic: v1-0-agent-journey-planner
lane: foundation
lane_seq: 30
blocked_by: ['@design-cli-contract']
commits:
- hash: 8a1fdd6030c3933bdd28ff770c07c64821f868bb
  summary: 'chore: track Rust workspace lockfile'
- hash: b0c42c9c6291fe586522f58930cc96c237730528
  summary: 'chore: add project-canon Rust scaffold'
- hash: 28030fe20e1a3e71b12bee02f55c0e5b6f0490ff
  summary: finish Rust workspace foundation
closed: 2026-09-08
---

# Scaffold the Rust core and CLI workspace

## Objective

Create the tested library-first foundation for `reitti` without prematurely implementing product behavior.

## Acceptance criteria

- Cargo workspace separates `reitti-core` domain logic from `reitti-cli` parsing, rendering, and I/O.
- Central error-to-exit mapping, schema envelopes, real build provenance, injected clock, HTTP abstraction, and secret-safe diagnostics are established.
- `version`, `config path`, `config show`, and a read-only `doctor` have text and JSON tests.
- CI runs formatting, linting, tests, and secret scanning on supported toolchains.

## Resolution

### 2026-09-08T10:20:06Z · @issuectl

Recovered the cancelled worker's uncommitted patch, completed material CLI/config/output/help/schema regressions, assessed the existing reviews, and passed all repository gates.
