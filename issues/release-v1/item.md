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
blocked_by: ['@ship-agent-skill', '@prepare-oss-foundations']
---

# Package and verify the v1.0 release

## Objective

Produce a reproducible and documented first release after all user-facing capabilities are complete.

## Acceptance criteria

- README contains installation, key registration, quickstart, supported municipalities, privacy, attribution, and independence statements.
- Release artifacts cover macOS arm64 and Linux arm64/x86_64 with checksums and provenance.
- Clean-machine smoke tests exercise help, doctor, configuration, location search, journey planning, departures, disruptions, and skill installation.
- The version is tagged `v1.0.0` only after all epic success criteria and release checks pass.

## Decisions

### 2026-09-08T08:04:12Z · @codex

Primary review of landed OSS foundations: final integration must remove stale placeholder/private-development wording. SECURITY.md currently says enable GitHub PVR before visibility change although the private-repo endpoint returns 404; verify actual platform sequencing and describe it correctly, without pretending it is enabled. Do not invent or mandate a separate conduct email as an additional release blocker: GitHub reporting/moderation and the documented existing maintainer channels are a proportionate policy unless the maintainer chooses another contact. No public crates.io release is intended. Verify three cargo-dist targets, shell installer, checksums/provenance and source install; keep GitHub private.
