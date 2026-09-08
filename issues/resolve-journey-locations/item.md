---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: open
priority: normal
epic: v1-0-agent-journey-planner
lane: journey-experience
lane_seq: 10
blocked_by: ['@implement-digitransit-client']
---

# Resolve and disambiguate journey locations

## Objective

Let agents safely turn human place descriptions into journey endpoints.

## Acceptance criteria

- Locations can be supplied as an address/place query, HSL stop identifier, or WGS84 coordinates.
- Search returns stable candidate identifiers, labels, locality, coordinates, source layer, and confidence where available.
- `journey plan` refuses unsafe ambiguity and returns actionable candidate data for a corrected retry.
- Queries are constrained and labelled for the supported HSL area without silently fabricating out-of-area support.
- Finnish and Swedish names are preserved where provided by the source.
