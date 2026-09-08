---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: testing
priority: normal
epic: v1-0-agent-journey-planner
lane: live-context
lane_seq: 10
blocked_by: ['@implement-digitransit-client']
---

# Expose live departures and disruptions

## Objective

Give route-planning agents the immediate operational context around a proposed trip.

## Acceptance criteria

- Stop search and nearby-stop discovery return stable identifiers suitable for follow-up calls.
- Stop departures distinguish scheduled, predicted, delayed, and cancelled service with source timestamps.
- Disruption listing supports useful route/stop filtering and preserves authoritative descriptions and validity windows.
- The implementation avoids duplicating standalone GTFS-RT calls when the Routing API already supplies equivalent data.
- Large or streaming vehicle-position feeds remain outside v1.0 unless validation proves a bounded agent use case.

## Agent Runs

### 2026-09-08T12:10:52Z · @pi

Implemented stop, departure, and alert handlers plus exact schemas and deterministic injected-transport tests. All five local gates pass (45 Rust tests total, 21 provider fixtures). Anthropic/DeepSeek review was assessed; eight localized source-backed corrections were applied, four proposals were rejected as incorrect or low-value. Primary live smoke passed all three commands without credential echo; see validation.md and review-assessment.md.
