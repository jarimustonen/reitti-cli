---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: done
priority: normal
epic: v1-0-agent-journey-planner
lane: agent-integration
lane_seq: 10
blocked_by: ['@plan-ranked-journeys', '@expose-live-transit-context']
closed: 2026-09-08
commits:
- hash: 9a77267c1fc3a502b8dd24b0052d9946f2da3274
  summary: bundle the reitti agent skill
- hash: 164c851
  summary: record agent skill validation
---

# Ship the reitti companion Agent Skill

## Objective

Teach supported AI agents when and how to use `reitti` safely and efficiently.

## Acceptance Criteria

- [x] `reitti skill list`, `skill print`, and `skill install` expose a version-pinned Agent Skill for Claude, pi, and Codex according to the project canon.
- [x] The skill demonstrates location disambiguation, comparing alternatives, arrival deadlines, realtime caveats, and user-facing attribution.
- [x] Examples use JSON output, bounded result sizes, explicit times/timezones, and corrective retries after actionable errors.
- [x] Skill/version drift is mechanically blocked in CI.

## Resolution

### 2026-09-08T13:40:41Z · @issuectl

Bundled skill, three-runtime installer, strict skill schemas, doctor/version integration, and build-time drift checks completed. Local gates, native runtime discovery, 35-check artifact smoke, and Linux ARM64/Rust 1.88 source-archive tests passed. Release-v1 retains the documented version-bump and packaging handoff.
