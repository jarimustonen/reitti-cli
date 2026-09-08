---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: done
priority: normal
epic: v1-0-agent-journey-planner
lane: foundation
lane_seq: 20
blocked_by: ['@validate-data-contracts']
commits:
- hash: a3a539e
  summary: define reitti v1 CLI contract
- hash: a1ceb98
  summary: refine reitti CLI contract after review
closed: 2026-09-08
---

# Design the agent-first CLI and result schema

## Objective

Define a small, stable `reitti` command surface and versioned output schema before implementation.

## Accepted surface

- `reitti location search`
- `reitti journey plan`
- `reitti stop search`
- `reitti stop departures`
- `reitti alert list`
- Canon-required `config`, `schema`, `version`, `doctor`, and `skill` surfaces

The normative implementation contract is in [`design.md`](design.md).

## Acceptance Criteria

- [x] Exact grammar and bounds cover strict validation, tagged location references, explicit ambiguity retries, time zones, departure-vs-arrival searches, transport/accessibility preferences, and bounded provider calls.
- [x] Schema-v1 envelopes cover every command, warnings, errors/exits, provider attribution, realtime evidence, null/unknown data, schema discovery, and deterministic human output.
- [x] Journey JSON preserves every bounded returned alternative and derives only transparent, evidence-based comparison labels; it never fabricates recommendation, availability, fare, accessibility, or realtime confidence.
- [x] Secure XDG configuration specifies per-key precedence and credential onboarding without secrets in argv, output, logs, fixtures, or examples.
- [x] Structured help and copy-pasteable examples are specified for every v1 command, together with doctor and the synchronized Claude/pi/Codex companion-skill surface.
- [x] Core/client/CLI seams and deterministic unit, fixture-contract, golden, mock-server, skill, and end-to-end tests are implementable without redesign.
- [x] The design preserves the verified Routing v2 HSL GTFS behavior and records applicability decisions for all sections of `AGENTS-AI-FIRST-CLI.md`.

## Resolution

### 2026-09-08T08:01:25Z · @issuectl

Accepted design delivered in design.md; all required repository gates and offline Digitransit fixture validation passed.

### 2026-09-08T08:08:20Z · @issuectl

Primary-agent design review addressed: offline-by-default doctor and exit mapping, honest coordinate scope, raw typed stop IDs, required navigation fields with optional bounded geometry, alert relevance union, corrected journey arithmetic, explicit single-attempt client policy, URN schema IDs, and removal of color/doctor mutation.


## Reopen Notes — 2026-09-08

Reopened for primary-agent review corrections before implementation: offline diagnostics, honest coordinate scope, raw stop IDs in typed positions, navigation detail, alert relevance, arithmetic, retry semantics, schema IDs, and incidental-surface reduction.
