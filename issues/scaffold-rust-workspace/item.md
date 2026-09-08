---
created: 2026-09-08
updated: 2026-09-08
type: task
reporter: jarimustonen
status: open
priority: normal
epic: v1-0-agent-journey-planner
lane: foundation
lane_seq: 30
blocked_by: ['@design-cli-contract']
---

# Scaffold the Rust core and CLI workspace

## Objective

Create the tested library-first foundation for `reitti` without prematurely implementing product behavior.

## Acceptance criteria

- Cargo workspace separates `reitti-core` domain logic from `reitti-cli` parsing, rendering, and I/O.
- Central error-to-exit mapping, schema envelopes, real build provenance, injected clock, HTTP abstraction, and secret-safe diagnostics are established.
- `version`, `config path`, `config show`, and a read-only `doctor` have text and JSON tests.
- CI runs formatting, linting, tests, and secret scanning on supported toolchains.
