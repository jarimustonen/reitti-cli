# Release preflight evidence

Verified 2026-09-08; these checks establish tooling availability, not a completed release.

- The installed shipshape 0.12.1 generator pins cargo-dist 0.28.2. An isolated temporary generation from the approved contract established the pin without modifying product distribution files.
- The official cargo-dist v0.28.2 macOS arm64 archive was downloaded and its published SHA-256 verified; the executable reports cargo-dist 0.28.2. It is available as ignored local tooling, with no global installation.
- The pinned cargo-dist workflow template adds `actions/attest-build-provenance@v2` whenever `github-attestations` is enabled; it does not automatically bypass that step for private repositories. [Pinned template](https://github.com/axodotdev/cargo-dist/blob/v0.28.2/cargo-dist/templates/ci/github/release.yml.j2).
- Standard Linux arm64 GitHub-hosted runners are available for private repositories as of 2026-01-29, including `ubuntu-24.04-arm`; old 2025 public-only guidance is obsolete. This supports testing the requested Linux arm64 artifact without personal runners or changing visibility. [GitHub announcement](https://github.blog/changelog/2026-01-29-arm64-standard-runners-are-now-available-in-private-repositories/).

Final release work must still generate the distribution files through shipshape/cargo-dist, build all three targets, run installation and behavior checks, inspect actual workflow results, and verify repository privacy. Attestation capability is separate from runner availability and remains to be tested as recorded in the issue.

## Maintainer-selected macOS runner

The maintainer subsequently required the shared self-hosted macOS ARM64 build machine. A dedicated runner registration for this repository was created and its launch service started successfully. GitHub reports one runner, online and idle, with labels `self-hosted`, `macOS`, `ARM64`, and `reitti-macos-arm64`. Existing sibling-project services remained running. The official Actions runner 2.337.0 archive matched its published SHA-256; the machine provides Homebrew Rust/Cargo 1.97.1 and its runner PATH includes the Homebrew binary directory. Repository visibility remains PRIVATE after registration.

Use the established cargo-dist override `[dist.github-custom-runners]` with `aarch64-apple-darwin = "self-hosted"`, then regenerate the workflow through cargo-dist. This repository currently has exactly one self-hosted runner. Revalidate selection if further runners are registered. A completed macOS release job and artifact smoke test are still required; online status alone is not build evidence.

## First hosted CI attempt

On 2026-09-08, the foundation commit `fd534fe678e033c008774dc4372dcaa938fcc798` passed all five local gates, including 30 Rust tests. It was pushed to the private repository. [CI run 34215159729](https://github.com/jarimustonen/reitti-cli/actions/runs/34215159729) then failed before any job steps started: GitHub reported an account billing/spending restriction. This is not a failed Rust test or secret-scan finding; none of the three jobs executed. Account settings were not changed. Hosted CI and Linux artifact verification remain pending until runner execution is available. Keep local validation and implementation moving, retain the required self-hosted macOS release runner, and do not claim remote CI or release checks have passed.
