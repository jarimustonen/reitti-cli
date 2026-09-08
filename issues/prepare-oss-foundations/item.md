---
created: 2026-09-08
updated: 2026-09-08
type: chore
status: done
priority: normal
epic: v1-0-agent-journey-planner
lane: oss-foundation
closed: 2026-09-08
closed_by: pi
commits:
- hash: 11e7ad1
  summary: establish OSS release foundations
---

# Prepare open-source release foundations

## Description

User-authorized OSS preparation independent of CLI implementation. Use shipshape skills to establish and validate the release contract, MIT license, security policy and contributor guidance. Keep GitHub private. Do not publish packages or change visibility. Limit this slice to OSS-RELEASE.md, LICENSE, SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md if appropriate, and this issue directory. CLI README, CI and artifacts are integrated by release-v1 after implementation. Record readiness findings under this issue. Completion requires a valid release contract, accurate public project metadata, no personal environment assumptions, and issuectl doctor without critical findings.

## Acceptance Criteria

- [x] The approved shipshape contract validates and records the authorized GitHub Release distribution design without claiming current release readiness.
- [x] MIT licensing, vulnerability reporting, contributor guidance, and conduct expectations are documented without invented contact details.
- [x] The repository remains private and no tag, release, registry publication, or visibility change was made.
- [x] The locked Rust checks, fixture validation, and issuectl doctor pass.

## Resolution

### 2026-09-08T08:01:39Z · @pi

Accepted: the approved shipshape contract validates; MIT, security, contribution, and conduct documents are present; the locked Rust checks, fixture check, and issuectl doctor pass. The repository remains private and nothing was published. release-v1 must add CI/cargo-dist/changelog and installation verification, verify Private Vulnerability Reporting plus a private conduct contact before publicization, and remove temporary private/placeholder wording once it is no longer true.
