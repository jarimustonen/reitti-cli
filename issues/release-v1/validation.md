# Release preflight evidence

Verified 2026-09-08; these checks establish tooling availability, not a completed release.

- The installed shipshape 0.12.1 generator pins cargo-dist 0.28.2. An isolated temporary generation from the approved contract established the pin without modifying product distribution files.
- The official cargo-dist v0.28.2 macOS arm64 archive was downloaded and its published SHA-256 verified; the executable reports cargo-dist 0.28.2. It is available as ignored local tooling, with no global installation.
- The pinned cargo-dist workflow template adds `actions/attest-build-provenance@v2` whenever `github-attestations` is enabled; it does not automatically bypass that step for private repositories. [Pinned template](https://github.com/axodotdev/cargo-dist/blob/v0.28.2/cargo-dist/templates/ci/github/release.yml.j2).
- Standard Linux arm64 GitHub-hosted runners are available for private repositories as of 2026-01-29, including `ubuntu-24.04-arm`; old 2025 public-only guidance is obsolete. This supports testing the requested Linux arm64 artifact without personal runners or changing visibility. [GitHub announcement](https://github.blog/changelog/2026-01-29-arm64-standard-runners-are-now-available-in-private-repositories/).

Final release work must still generate the distribution files through shipshape/cargo-dist, build all three targets, run installation and behavior checks, inspect actual workflow results, and verify repository privacy. Attestation capability is separate from runner availability and remains to be tested as recorded in the issue.
