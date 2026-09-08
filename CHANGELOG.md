# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- oss-changelog:unreleased-start -->
## [Unreleased]

The entries below describe the intended first stable release, v1.0.0. They remain unreleased until the maintainer completes acceptance and creates the tag and GitHub Release.

### Added

- Place discovery and conservative free-text resolution for ambiguous journey endpoints.
- Named and nearby stop search for the HSL Journey Planner service area.
- Ranked departure-time and arrival-deadline journey alternatives with transit modes, walking and transfer facts, realtime evidence, accessibility uncertainty, alerts, navigation steps, and optional bounded GeoJSON geometry.
- Bounded departure boards and active service-alert queries with explicit scheduled, updated, cancelled, and unknown evidence.
- Schema-versioned JSON output, deterministic text output, structured errors and warnings, machine-readable command help, bundled JSON schemas, build provenance, and correlated request IDs.
- Secure layered configuration, stdin subscription-key setup, redacted inspection, atomic selective updates, and offline or bounded online diagnostics.
- A version-matched Agent Skill with no-clobber installation for Claude Code, Pi, and Codex.
- Cargo-dist release automation for macOS arm64 and static musl Linux arm64/x86_64 archives, a shell installer, checksums, source archives, exact build identity, and dynamic private-path remapping.

### Changed

- Contribution CI now checks formatting, Clippy, Rust tests on stable and Rust 1.88, credential-free Digitransit fixtures, and committed-secret history.

### Fixed

- Verbose invocation events and command results now share one request ID, allowing direct log-to-result correlation.
<!-- oss-changelog:unreleased-end -->
