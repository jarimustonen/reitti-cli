---
created: 2026-09-08
updated: 2026-09-08
type: chore
reporter: jarimustonen
status: open
priority: normal
epic: v1-0-agent-journey-planner
lane: release
lane_seq: 10
blocked_by: ['@ship-agent-skill']
---

# Package and verify the v1.0 release

## Objective

Produce a reproducible and documented first release after all user-facing capabilities are complete.

## Acceptance criteria

- README contains installation, key registration, quickstart, supported municipalities, privacy, attribution, and independence statements.
- Release artifacts cover macOS arm64 and Linux arm64/x86_64 with checksums and provenance.
- Clean-machine smoke tests exercise help, doctor, configuration, location search, journey planning, departures, disruptions, and skill installation.
- The version is tagged `v1.0.0` only after all epic success criteria and release checks pass.
