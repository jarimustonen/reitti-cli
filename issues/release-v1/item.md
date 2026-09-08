---
created: 2026-09-08
updated: 2026-09-08
type: chore
reporter: jarimustonen
status: done
priority: normal
epic: v1-0-agent-journey-planner
lane: release
lane_seq: 10
blocked_by: ['@ship-agent-skill', '@prepare-oss-foundations']
closed: 2026-09-08
closed_by: codex
---

# Package and verify the v1.0 release

## Objective

Produce a reproducible and documented first release after all user-facing capabilities are complete.

## Acceptance Criteria

- [x] README contains installation, key registration, quickstart, supported municipalities, privacy, attribution, and independence statements.
- [x] Release artifacts cover macOS arm64 and Linux arm64/x86_64 with checksums and provenance.
- [x] Clean-machine smoke tests exercise help, doctor, configuration, location search, journey planning, departures, disruptions, and skill installation.
- [x] The version is tagged `v1.0.0` only after all epic success criteria and release checks pass.

## Decisions

### 2026-09-08T08:04:12Z · @codex

Primary review of landed OSS foundations: final integration must remove stale placeholder/private-development wording. SECURITY.md currently says enable GitHub PVR before visibility change although the private-repo endpoint returns 404; verify actual platform sequencing and describe it correctly, without pretending it is enabled. Do not invent or mandate a separate conduct email as an additional release blocker: GitHub reporting/moderation and the documented existing maintainer channels are a proportionate policy unless the maintainer chooses another contact. No public crates.io release is intended. Verify three cargo-dist targets, shell installer, checksums/provenance and source install; keep GitHub private.

### 2026-09-08T08:19:55Z · @codex

Release preflight evidence: GitHub documents that artifact attestations in private/internal repositories require Enterprise Cloud (https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations). This target is currently private and User-owned; the available account API did not expose a plan, so capability is not verified. Check the actual private-repository build/sign path before promising keyless provenance or preserving an assumed blocker. Keep requested repository privacy. Do not silently claim unproduced attestations; prefer an honest, reviewed private-release path with verifiable checksums/build commit evidence and public-release-ready automation, documenting any platform limitation precisely.


### Maintainer steering: macOS build runner

Use the existing self-hosted macOS ARM64 machine used by the other family projects. This explicit maintainer instruction supersedes the initial hosted-only assumption. Configure and verify a dedicated repository runner; preserve the cargo-dist custom-runner override through workflow generation. Keep machine-specific connection details out of public documentation.

### Maintainer steering: ordinary GitHub CI

Ordinary push and pull-request GitHub CI is intentionally absent. Remove the broken contribution workflow, run required validation and secret scanning locally, and do not regenerate ordinary CI merely to satisfy an audit. Preserve the generated tag-triggered cargo-dist release workflow and its self-hosted macOS ARM64 runner override.

### Maintainer authorization: public v1.0.0 publication

On 2026-09-08 the maintainer explicitly requested README refinements, crates.io and Homebrew as the first installation channels, Code of Conduct removal, public GitHub visibility, and publication. This supersedes the earlier private-preparation boundary. Publish core then CLI to crates.io; cargo-dist owns GitHub assets and the Homebrew formula so the formula correctly installs binary `reitti` from `reitti-cli` archives. Preserve the three platforms, macOS self-hosted runner, local gates, and absence of ordinary CI. Use existing SOPS-managed release credentials only through process environment or GitHub secret input; never persist decrypted values.

## Resolution

### 2026-09-08T15:15:16Z · @codex

Published v1.0.0 to crates.io, GitHub Releases and Homebrew. All release jobs, three artifact attestations, Cargo/Homebrew installation and public shell installation on all three platforms verified. See acceptance.md for exact source and evidence.
