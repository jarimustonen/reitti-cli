---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: done
priority: normal
epic: v1-0-agent-journey-planner
lane: data-access
lane_seq: 10
blocked_by: ['@scaffold-rust-workspace']
closed: 2026-09-08
commits:
- hash: ffbc72a
  summary: typed Digitransit clients, transport, DTOs, fixtures and handler seams
- hash: a1f5276
  summary: align cancellation, DST, bounds and normalization with deployed semantics
---

# Implement the Digitransit data client

## Objective

Provide typed, resilient access to the verified Digitransit services needed by v1.0.

## Acceptance criteria

- Subscription key and API base URLs follow flag > environment > config > default precedence and are never logged.
- Typed clients support geocoding, routing, stops/departures, and disruptions selected in the validation issue.
- Timeouts, actionable HTTP/GraphQL errors, quota responses, retries for safe transient failures, and attribution metadata are covered by tests.
- Network-free contract tests use committed fixtures; opt-in live tests are clearly separated.

## Resolution

### 2026-09-08T11:26:37Z · @issuectl

Implemented typed geocoding and Routing v2 clients for location/place, plans, stop search/detail/nearby, departures, alerts, and online probes. Added redirect/retry denial, finite timeouts, streaming 8 MiB cap, redacted diagnostics, production-schema normalizers, rich offline fixtures, deterministic provider/transport regressions, domain-owned handler/schema seams, and successful full repository gates. Primary independently verified doctor --online against production with exactly two requests and no secret output.
