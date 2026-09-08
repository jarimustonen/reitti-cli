---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: open
priority: normal
epic: v1-0-agent-journey-planner
lane: foundation
lane_seq: 20
blocked_by: ['@validate-data-contracts']
---

# Design the agent-first CLI and result schema

## Objective

Define a small, stable `reitti` command surface and versioned output schema before implementation.

## Proposed surface

- `reitti location search`
- `reitti journey plan`
- `reitti stop search`
- `reitti stop departures`
- `reitti disruption list`
- Canon-required `config`, `version`, `doctor`, and `skill` surfaces

## Acceptance criteria

- Design covers strict validation, time zones, departure-vs-arrival searches, transport and accessibility preferences, ambiguous locations, deterministic text, global `--json`, errors, exits, and configuration precedence.
- Journey JSON preserves all returned alternatives and explains each trade-off without presenting a model-generated claim as source data.
- Structured help and examples are specified for every v1.0 command.
- The design is checked against `AGENTS-AI-FIRST-CLI.md` and recorded under this issue.
