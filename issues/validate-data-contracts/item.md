---
created: 2026-09-08
updated: 2026-09-08
type: task
reporter: jarimustonen
status: done
priority: normal
epic: v1-0-agent-journey-planner
lane: foundation
lane_seq: 10
closed: 2026-09-08
commits:
- hash: ebdc7cc
  summary: validate Digitransit data contracts
---

# Validate HSL and Digitransit data contracts

## Objective

Turn the initial open-data survey into verified, executable API knowledge before implementation begins.

## Work

- Register or document acquisition of a Digitransit subscription key without committing secrets.
- Verify current HSL routing GraphQL, Pelias geocoding, Routing Data, and HSL GTFS-RT endpoints.
- Capture representative redacted fixtures for routes, stops, departures, alerts, ambiguity, cancellation, and no-result cases.
- Confirm rate/quota behavior, attribution text, ODbL boundaries, timestamps, realtime semantics, and deprecation notices.
- Record which data comes through the Routing API and which requires a separate realtime feed.

## Acceptance criteria

- `validation.md` identifies every endpoint and header used by v1.0, with retrieval dates and authoritative links.
- Fixtures and a repeatable probe script cover successful and failing requests without exposing an API key.
- Open uncertainties are converted into explicit issues or excluded from v1.0.
