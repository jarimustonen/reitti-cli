# OSS foundations plan

## Delegated decisions

The maintainer expressly delegated routine release-design and documentation choices for this issue while retaining these boundaries: keep `github.com/jarimustonen/reitti-cli` private, publish nothing, create no tag or release, and make no public-registry commitment. The approved contract records that delegation; it does not claim a separate human review.

- Treat the intended v1.0 project as MVP maturity for release-policy purposes while explicitly preserving `shipshape facts`' current `spike` inference and the reason for the override.
- Publish the future `reitti` executable through gated GitHub Releases using cargo-dist, not crates.io.
- Support `aarch64-apple-darwin`, `aarch64-unknown-linux-musl`, and `x86_64-unknown-linux-musl`; produce a shell installer.
- Use SemVer with a future stable v1.0, MIT licensing, curated release notes sourced from issuectl-linked commits, and future keyless provenance.
- Keep dependency automation, CI, changelog generation, cargo-dist files, release verification, and repository publicization in later release work.
- Do not assume GitHub Private Vulnerability Reporting is available while the repository is private. Verify it before publicization and use it for vulnerabilities only. Use GitHub's report-abuse and interaction-moderation mechanisms for conduct incidents until a separate private maintainer contact is established; no email address is invented.

## Execution

1. Capture deterministic repository facts and the no-contract audit baseline.
2. Stage, validate, and install `OSS-RELEASE.md`; approve it under the maintainer's explicit delegated authority.
3. Generate the MIT license, contributor onboarding, Contributor Covenant, and a threat-proportionate security policy.
4. Re-run contract validation and readiness audit, validate Rust green-gate commands and document/link integrity, and record remaining release-work gaps without claiming readiness.
5. Run `issuectl doctor`, commit the deliverables, record the commit, and close this issue.
