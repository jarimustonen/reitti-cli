---
created: 2026-09-18
updated: 2026-09-18
type: chore
status: done
priority: normal
provenance: other
provenance_detail: Taskfleet implementation brief
source_ref: taskfleet:01m2sm3jkrwc4m53sckesvnzmj/task:cargo-dist-0.33.0
originating_run: 01m2sm3jkrwc4m53sckesvnzmj
originating_run_kind: spinoff
closed: 2026-09-18
---

# Upgrade cargo-dist to 0.33.0

## Description

## Goal

Upgrade the active cargo-dist pin from 0.28.2 to 0.33.0 and regenerate the tag-triggered release workflow with the exact disposable cargo-dist binary.

## Acceptance criteria

- Preserve the self-hosted macOS ARM64 runner, dynamic build setup, three required targets, shell and Homebrew installers, attestations, Homebrew publication, and tag-only trigger.
- Update only active current-pin declarations; preserve release-v1 validation and acceptance history unchanged.
- `dist generate --mode ci --check`, JSON planning, YAML parsing, and all repository-local gates pass.
- Do not publish a release.

## Resolution

### 2026-09-18T06:46:39Z · @issuectl

Upgraded the active cargo-dist pin to 0.33.0, regenerated and verified the release workflow, preserved all required distribution invariants, and passed every required local gate. No release was published.
