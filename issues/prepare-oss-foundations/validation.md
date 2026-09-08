# OSS foundations validation

Date: 2026-09-08

## Baseline

`shipshape facts --json` detected a two-package Rust workspace (`reitti-core`, `reitti-cli`), one committer, no tags, no CI, no dependency bot, and no cargo-dist files. Its deterministic maturity inference was `spike`. `shipshape contract show` and `shipshape audit` both stopped with `contract_not_found`, confirming that no release contract existed.

The generated contract deliberately selects `mvp` as an authorized override because the repository is being prepared for a distributable v1.0. `OSS-RELEASE.md` keeps the current evidence visible and says explicitly that the placeholder CLI is not release-ready.

## Contract checks

The proposal was written first to `/tmp/shipshape-init/reitti-cli-staging/OSS-RELEASE.md`, then checked with:

```sh
shipshape contract validate --repo-root /tmp/shipshape-init/reitti-cli-staging --json
shipshape contract show --repo-root /tmp/shipshape-init/reitti-cli-staging --json
```

An initial proposal combined `targets: []` with a cargo-dist distribution. The validator correctly rejected that contradiction. The corrected contract declares `reitti-cli` as a `gh-releases` target using `cargo-dist` while declaring no crates.io target. Staged and installed validation then returned `valid: true`, `maturity: mvp`, and one target. The approved-state gate also passed:

```sh
shipshape contract show --repo-root . --require-approved --json
```

Approval records the maintainer's express delegation in the issue and contract text; it is not represented as a separate human review.

## Security assessment

The checked-in Rust implementation is currently a placeholder and contains no network, subprocess, credential-read, deserialization, authentication, or user-data handling code. The intended v1.0 scope and release contract do cross the threat gate: the CLI will make authenticated HTTPS requests, parse external Digitransit responses, and ship prebuilt executables/installers. `SECURITY.md` therefore uses the full MVP policy while stating that no release exists today.

GitHub verification returned `visibility: PRIVATE` and administrator access for the authenticated maintainer. The Private Vulnerability Reporting API endpoint returned HTTP 404, so the policy does not claim that **Report a vulnerability** is currently available. It documents the restricted collaborator path honestly and requires PVR to be enabled and re-verified before publicization. It does not invent an email address.

## Contributor and license checks

- `LICENSE` is the standard MIT text and agrees with `license = "MIT"` in the workspace manifest and the normalized contract.
- `CONTRIBUTING.md` points to the actual future public coordinates, `https://github.com/jarimustonen/reitti-cli`, targets `main`, and mirrors the current `AGENTS.md` merge gate: locked Rust checks, fixture validation, and `issuectl doctor`. It makes clear that ordinary contributors do not need issuectl to build or contribute.
- `CODE_OF_CONDUCT.md` is a concise project policy. It routes actual GitHub abuse through GitHub's report-abuse and interaction-moderation tools rather than misusing the vulnerability channel. It states honestly that no separate private conduct contact exists yet and makes establishing one a publicization prerequisite.
- Security, conduct, contribution, and license links are relative and resolve to files in the repository root.
- The documents contain no machine-local project paths. The temporary staging path appears only in this validation record as reproducibility evidence, not in public-facing root documentation.

## Readiness result

`readiness-final.json` records the post-foundation `shipshape audit --json` result. The local audit still reports the expected release-integration work:

- **Blocking:** CI is absent.
- **Recommended:** CHANGELOG and dependency automation are absent.
- **Recommended:** GitHub topics are absent.
- **Recommended/unknown:** GitHub Private Vulnerability Reporting is not verified enabled.

The GitHub community-profile projection still reports the new root files as absent because they are uncommitted and not yet merged/pushed to the private remote when the audit runs. This is not treated as evidence that the local files are missing. No visibility setting, tag, release, registry publication, or GitHub metadata was changed.

The CLI implementation remains incomplete. These foundations are valid inputs to `release-v1`, not a claim that v1.0 can be cut now. Release integration must replace the temporary private/placeholder wording in `OSS-RELEASE.md`, `CONTRIBUTING.md`, and `SECURITY.md` once the product, installation path, reporting settings, and release artifacts exist.
