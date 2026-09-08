---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: open
priority: normal
epic: v1-0-agent-journey-planner
lane: data-access
lane_seq: 10
blocked_by: ['@scaffold-rust-workspace']
---

# Implement the Digitransit data client

## Objective

Provide typed, resilient access to the verified Digitransit services needed by v1.0.

## Acceptance criteria

- Subscription key and API base URLs follow flag > environment > config > default precedence and are never logged.
- Typed clients support geocoding, routing, stops/departures, and disruptions selected in the validation issue.
- Timeouts, actionable HTTP/GraphQL errors, quota responses, retries for safe transient failures, and attribution metadata are covered by tests.
- Network-free contract tests use committed fixtures; opt-in live tests are clearly separated.
