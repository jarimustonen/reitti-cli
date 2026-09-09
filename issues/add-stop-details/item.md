---
created: 2026-09-09
updated: 2026-09-09
type: feature
reporter: jarimustonen
status: open
priority: normal
---

# Add stop details and Reittiopas links

## Objective

Give agents a trustworthy way to inspect one HSL stop and provide the user with a human-facing HSL Reittiopas link for further information.

## Context

`reitti stop list` and `departure list` already return a compact stop object backed by Digitransit's Routing API, and the provider layer already has a single-stop lookup seam. The CLI does not expose that lookup as `stop show`, does not include a Reittiopas stop-page URL, and intentionally requests only a minimal set of stop fields.

The implementation must verify the current official URL and API contracts before deciding which additional fields are safe to promise. It must not scrape Reittiopas pages or introduce GTFS downloads when the bounded Routing API can provide the required facts.

## Acceptance criteria

- The current canonical HSL Reittiopas deep-link format for a raw `HSL:` stop ID is verified against official behavior and documented with its stability limitations.
- `reitti stop show <STOP_ID>` accepts one strict raw HSL GTFS stop ID and returns a schema-versioned stop-detail response from one bounded Digitransit request.
- The stop result includes a clearly named Reittiopas URL when a verified deterministic link can be formed; list and departure stop objects expose the same link consistently where applicable.
- The detailed result includes only useful fields confirmed by the deployed Digitransit schema and fixtures, preserving null or unknown values instead of inferring them.
- Unknown stops, malformed IDs, missing credentials, provider failures, attribution, and text output remain agent-actionable and conform to the existing CLI contracts.
- Help, JSON schemas, bundled skill guidance, deterministic provider fixtures, and integration tests cover the new surface.
- A bounded live acceptance check validates one ordinary stop and one station/platform case without retaining credentials or unstable private data.

## Out of scope

- Scraping HSL or Reittiopas web pages.
- Mirroring the complete GTFS stop dataset locally.
- Assuming that a Reittiopas page supplies machine-authoritative facts beyond the API response.
- Maps, background tracking, or unbounded departure history.

## Approval

The proposed implementation sequence and contract decisions are in `plan.md`. Do not lane or implement this issue until the maintainer approves that plan.
