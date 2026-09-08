---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: in-progress
priority: normal
epic: v1-0-agent-journey-planner
lane: agent-integration
lane_seq: 10
blocked_by: ['@plan-ranked-journeys', '@expose-live-transit-context']
---

# Ship the reitti companion Agent Skill

## Objective

Teach supported AI agents when and how to use `reitti` safely and efficiently.

## Acceptance criteria

- `reitti skill list`, `skill print`, and `skill install` expose a version-pinned Agent Skill for Claude, pi, and Codex according to the project canon.
- The skill demonstrates location disambiguation, comparing alternatives, arrival deadlines, realtime caveats, and user-facing attribution.
- Examples use JSON output, bounded result sizes, explicit times/timezones, and corrective retries after actionable errors.
- Skill/version drift is mechanically blocked in CI.
