---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: in-progress
priority: normal
epic: v1-0-agent-journey-planner
lane: journey-experience
lane_seq: 20
blocked_by: ['@resolve-journey-locations']
---

# Plan and compare ranked journey alternatives

## Objective

Make `reitti journey list` useful as an evidence source for a travel-planning agent.

## Acceptance criteria

- Supports explicit departure time or arrival deadline, timezone-aware defaults, mode filters, walking and accessibility preferences supported by the API, and a bounded alternative count.
- Returns multiple alternatives with duration, start/end, legs, transfers, walking, waiting, realtime state, cancellations, and relevant notices.
- Alternatives include deterministic comparison facts and recommendation labels such as fastest, fewest transfers, or least walking when justified by returned data.
- Text output remains concise; JSON retains enough provenance and detail for an agent to explain and compare routes.
- Golden and live integration tests cover ordinary, ambiguous, no-route, delayed, and cancelled journeys.

## Agent Runs

### 2026-09-08T12:44:54Z · @codex

Implemented the complete ranked-journey handler/schema and deterministic integration coverage. The handler consumes the landed endpoint resolver and typed provider client, enforces one-to-three operations and local output/walking bounds, preserves source evidence and explicit offsets, emits honest comparisons/navigation/realtime/accessibility/warnings, and renders timezone-consistent actionable text. Integration seams, deterministic coverage, local results, and primary pre-final-draft live evidence are recorded in validation.md.
