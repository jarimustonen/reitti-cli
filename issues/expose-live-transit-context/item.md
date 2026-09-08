---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: open
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
