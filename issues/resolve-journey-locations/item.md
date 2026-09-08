---
created: 2026-09-08
updated: 2026-09-08
type: feature
reporter: jarimustonen
status: done
priority: normal
epic: v1-0-agent-journey-planner
lane: journey-experience
lane_seq: 10
blocked_by: ['@implement-digitransit-client']
closed: 2026-09-08
closed_by: codex
commits:
- hash: 03a181b
  summary: implement safe location listing and journey endpoint resolution
- hash: e2e68ef
  summary: preserve bounded resolution evidence after review
---

# Resolve and disambiguate journey locations

## Objective

Let agents safely turn human place descriptions into journey endpoints.

## Acceptance criteria

- Locations can be supplied as an address/place query, HSL stop identifier, or WGS84 coordinates.
- Search returns stable candidate identifiers, labels, locality, coordinates, source layer, and confidence where available.
- `journey list` refuses unsafe ambiguity and returns actionable candidate data for a corrected retry.
- Queries are constrained and labelled for the supported HSL area without silently fabricating out-of-area support.
- Finnish and Swedish names are preserved where provided by the source.

## Decisions

### 2026-09-08T08:16:07Z · @codex

Primary review: a singleton geocoder result is not automatically a trustworthy address match. Define accepted singleton explicitly: reject low or absent confidence and broad locality/other matches unless caller selects their returned place ref. Prefer explicit selected refs; location_ambiguous may contain just one unconfirmed candidate with a reason. Preserve Finnish/Swedish names, recognize both language forms of supported municipalities when classifying area, and keep unknown coordinates unknown. Do not use a coarse bounding box as proof of municipality membership.

## Agent Runs

### 2026-09-08T11:48:21Z · @codex

Implemented typed location listing and reusable journey endpoint resolution. Deterministic injected-transport tests cover ambiguity, selected refs, stale IDs, area trust, Unicode, schemas, request caps, redaction, and the resolution-only journey seam. Primary independently verified bounded production location search and ambiguity behavior without credential echo; details are in validation.md.

## Resolution

### 2026-09-08T12:14:06Z · @codex

Delivered typed location search and reusable endpoint resolution with conservative singleton acceptance, explicit ambiguity retries, stable selected refs, exact coordinate handoff, truthful area/completeness/source evidence, and deterministic CLI tests. Journey planning remains explicitly unfinished for @plan-ranked-journeys and sends no plan request.
