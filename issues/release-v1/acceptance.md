# Final private-release acceptance

The primary agent owns this acceptance after all implementation work has landed. Preflight evidence in `validation.md` is useful but does not substitute for tests of the final source and packaged artifacts.

1. Run the five repository gates on integrated main. Inspect actual command help, exact schemas, configuration precedence and the bundled agent workflow. Assess material review findings against source and supported behavior.
2. Build the final version from a tracked source archive without Git metadata or ignored files. Inject the exact source commit, use the distribution profile and supported path-remapping setup, and retain checksums. Build macOS ARM64 on the maintainer-selected runner machine; build and execute both Linux musl architectures. Record native versus emulated execution and distinguish direct builds from GitHub Actions.
3. In a disposable empty home, execute each artifact's version aliases, command help and schema catalogue; configure a synthetic noncredential through stdin; verify configuration redaction and file mode; run offline doctor. Exercise skill installation, dry-run, identical rerun, no-clobber and force with actual packaged resource bytes. Never alter real installed skills.
4. Verify the generated shell installer's platform selection and artifact installation with final archives, using a disposable destination and an isolated asset source if private GitHub downloads are unavailable. Do not imply that anonymous installation works before public visibility and published assets exist.
5. Repeat native Claude, Pi and Codex discovery using the installed actual skill in isolated homes, without model prompts or real credentials. Verify the skill and CLI version agree in source archives and optimized builds.
6. Run bounded real-service acceptance with the private development key confined to the primary process environment: coordinate and selected-reference journeys, arrival deadlines, ambiguity, navigation geometry, nearby stops, departures, alerts and online doctor. Inspect timing, route labels, attribution, warnings and human readability. Save only sanitized public-data evidence.
7. Scan final tracked history locally with `gitleaks git --redact --no-banner .`, and scan distributed bytes for secrets and operator-supplied private path markers. Check generated release-workflow consistency, macOS runner selection, distribution plan, checksums and declared source identity. Review the final shipshape plan and repository privacy.
8. Ordinary push and pull-request CI is intentionally absent by maintainer decision; do not treat that absence as a release gap or regenerate the deleted workflow to satisfy an audit. Record external release-workflow failures exactly. The account billing/spending restriction has prevented hosted Actions jobs from starting; direct local tests do not turn that into a successful release run. Attestations require actual provider evidence. Preserve a concrete resumable release path and leave repository visibility private.

As of the bundled-skill landing, all 76 Rust tests, 21 provider fixtures, formatting, strict Clippy and issue metadata validation pass on main. The bundled skill is integrated; final release integration remains in progress. Native discovery of the draft real skill has passed in Claude Code 2.1.236, Pi 0.84.4 and Codex 0.153.4 using disposable homes and no model prompts; repeat against final packaged resources only if they change materially.

## Packaged checkpoint acceptance

The primary tested release-infrastructure checkpoint `682a3ce1e8f20ac491eaa1d1156f2f4f6cc959ad` (version 1.0.0), before final public-documentation integration. Actual cargo-dist 0.28.2 local archives were built from tracked source archives with the explicit source commit and dynamic path remapping. The macOS ARM64 archive was built on the required self-hosted machine; Linux ARM64 used official Rust 1.88/Alpine. Each extracted archive passed 35 offline CLI/configuration/skill-install checks in a disposable home. Published-format SHA-256 sidecars matched their archives. The macOS binary contained no private builder-home marker. Its actual bundled skill was discovered by Claude Code 2.1.236, Pi 0.84.4 and Codex 0.153.4 without model prompts or real user configuration.

These are checkpoint artifacts, not a claim that the final release or tag-triggered GitHub release workflow has completed. Linux x86_64 packaging, final-source rebuilds, generated-installer acceptance and final live checks remain. Cargo-dist global builds need a Git checkout to generate source.tar.gz; target-local builds succeeded without Git metadata. Actual artifact paths come from the manifest (currently target/distrib), not an assumed target/dist directory.

## Final local acceptance — 2026-09-08

Release candidate **1.0.0**, source **53c6ea56fd8d543a5e240445e1d60232af5cdf88**, passed final local acceptance. Later acceptance-record commits do not change the source identity embedded in these artifacts.

- Integrated main passed formatting, strict Clippy, all **77 Rust tests**, **21 credential-free provider fixtures**, and issue metadata validation. A tracked source export independently passed all 77 tests on Linux ARM64 with Rust 1.88. Final redacted Gitleaks history scanning found no leaks.
- Pinned cargo-dist 0.28.2 produced all three optimized archives from tracked source exports with explicit build identity and dynamic path remapping. macOS ARM64 was built directly on the required self-hosted runner machine; Linux ARM64 ran natively in the existing container VM, and Linux x86_64 used explicit emulation. These were direct builds, not GitHub Actions jobs.
- The actual generated shell installer selected and installed the matching final archive on each of the three platforms using its local download override and disposable installation homes. Every installed binary passed **35 checks** covering exact version/commit, help, schemas, configuration stdin input/mode/redaction, offline doctor, and skill installation/dry-run/idempotence/conflict/force behavior. No real installed skills or user configuration were changed.
- All archive sidecars and aggregate SHA-256 entries matched. Binary archive contents contained neither the operator's home-path marker nor the worktree-path marker. Archive paths were safe; the generated source archive excluded `.env` and ignored scratch data and preserved the documentation symlink.
- Native Claude, Pi and Codex discovery passed against the packaged 1.0.0 skill at the earlier checkpoint recorded above; the final bundled skill resources did not materially change afterward.
- Eight bounded live-provider acceptance cases passed on the integrated 1.0.0 checkpoint preceding the CI/documentation-only change: online doctor, stop lookup, nearby stops, coordinate journeys with geometry, selected-stop arrival-deadline wheelchair-aware journeys, expected ambiguous-query rejection, departures and scoped alerts. Journey timing, transfer/walking facts, arrival deadlines and conservative unknown accessibility/fare data were inspected. One additional final-source journey verified that the real verbose log request ID equals the response request ID. Credentials remained confined to the primary process environment.
- Ordinary push/PR CI was removed at the maintainer's request. GitHub contains only the tag-triggered release workflow; the self-hosted macOS override remains. No run was created by this final implementation push. Repository visibility remains **PRIVATE**.

| Archive | SHA-256 |
| --- | --- |
| reitti-cli-aarch64-apple-darwin.tar.xz | fb98f376c5d821b4fdd224e5a4029c0a964b241b6e835f3e60f903e9738c1aa4 |
| reitti-cli-aarch64-unknown-linux-musl.tar.xz | 4b0c9d8925a6b1537943be8706722eac143bd93a8fe118a3a2610bc988d75a44 |
| reitti-cli-x86_64-unknown-linux-musl.tar.xz | dfdaf275ab8ca97e3d3fb22ea3c3f2929669a1859f95782e62035cba53e5630f |
| source.tar.gz | c7519c72589fd86317993921672a13a49772d0ce1e6cc94bf3813fa9a66614b8 |

Local artifacts and sanitized logs are retained in ignored `history/final-artifacts/`, `history/final-*-installer.log`, `history/final-artifact-integrity.json`, `history/final-live/`, and the final gate/build logs. These paths are workspace evidence, not published download links.

### Remaining publication boundary

No `v1.0.0` tag or GitHub Release has been created. The latest attempted ordinary CI run failed before any steps because GitHub reported an account payment/spending restriction; removing ordinary CI does not establish that the retained release workflow can run. Private-repository attestation capability also remains unverified. Local checksums and explicit source identity do not constitute generated provider attestations.

The sealed shipshape plan in ignored `history/release-plan-no-ci.json` targets source 53c6ea56fd8d543a5e240445e1d60232af5cdf88 and has not been cut. Re-plan against the intended release commit before execution if main advances. Once account execution is available, run the retained release process, inspect all three artifacts and actual provenance, and verify asset downloads. Keep repository privacy as instructed. This issue remains open specifically for that publication/provenance acceptance; ordinary CI absence is intentional and must not be treated as unfinished work.

## Published v1.0.0 — 2026-09-08

This section supersedes the earlier private-preparation and publication-pending boundaries. The maintainer explicitly authorized public visibility, crates.io and Homebrew publication, and the final README changes. Release source is **2aeb5e1be33a29de08d805f40fd266c6f73f731c**, tagged **v1.0.0**. Later acceptance/documentation commits are not the binary source identity.

- [reitti-core 1.0.0](https://crates.io/crates/reitti-core/1.0.0) and [reitti-cli 1.0.0](https://crates.io/crates/reitti-cli/1.0.0) were published in dependency order. The CLI installs command `reitti`. Downloaded registry archives contain the required documentation and bundled skill, without ignored scratch data or `.env`.
- [GitHub Release v1.0.0](https://github.com/jarimustonen/reitti-cli/releases/tag/v1.0.0) contains all three binary archives, shell installer, Homebrew formula, source archive, and checksums. [Release workflow 34242521413](https://github.com/jarimustonen/reitti-cli/actions/runs/34242521413) completed every job successfully; its macOS ARM64 job ran on the maintainer-selected self-hosted machine.
- [Homebrew formula](https://github.com/jarimustonen/homebrew-reitti/blob/main/Formula/reitti-cli.rb) was generated and published by cargo-dist, with verified archive checksums and the correct `reitti` executable. `brew install jarimustonen/reitti/reitti-cli` succeeded in a disposable Homebrew prefix and the installed program passed 35 offline checks. No global tool installation was changed.
- `cargo install --locked --version 1.0.0 --root <disposable-prefix> reitti-cli` downloaded and installed the actual registry package. It passed the same 35 checks. Its source-archive provenance correctly reports a null Git commit; this differs from explicitly stamped prebuilt release binaries.
- The public shell installer downloaded the actual published archive on native macOS ARM64, Linux ARM64 in the local VM, and explicitly emulated Linux x86_64. Each installed binary passed 35 checks, including exact release commit, version aliases, help, schemas, private/redacted configuration, offline doctor, and skill installation behavior. These used public download URLs, not a local asset override.
- Every published archive sidecar and aggregate checksum matched. Executable architecture headers matched the declared targets; private builder-path markers were absent. Source/archive paths and ignored-file exclusions passed inspection.
- `gh attestation verify <archive> --repo jarimustonen/reitti-cli` succeeded for all three binary archives. Verified certificates identify the release workflow at `refs/tags/v1.0.0`, source commit 2aeb5e1be33a29de08d805f40fd266c6f73f731c, a self-hosted macOS runner, and GitHub-hosted Linux runners.
- All 77 Rust tests, strict Clippy, formatting, 21 provider fixtures, and issue metadata checks passed after the packaging changes. The prior bounded live-provider acceptance remains applicable: no routing/provider behavior changed in this publication pass.
- GitHub visibility is PUBLIC; Private Vulnerability Reporting is enabled and verified. Ordinary push/PR CI remains intentionally absent. Code of Conduct was removed as requested, with no dangling contribution link.
- Shipshape run **01M20RDJQMVH8Y54AVZ02ZZH8B**, plan **4beb37cee0faae3726fceb3e0dd2bc8bfd1f7ef3b15986b57c40ece18df40332**, reached **completed** after verifying all destinations and the remote default branch.

| Published archive | SHA-256 |
| --- | --- |
| reitti-cli-aarch64-apple-darwin.tar.xz | 5aa03c484152ad70e9e711c9372d39d76d715b28d34701e481289793081bdb27 |
| reitti-cli-aarch64-unknown-linux-musl.tar.xz | 528c86fa641d3cabd76507fc2e922f016798eb692f3f290a35c83ef39f107c04 |
| reitti-cli-x86_64-unknown-linux-musl.tar.xz | 4c827db56debda07e3635e610ccbdb843e15a2bb7f770e43935bf33dde26e214 |
| source.tar.gz | 6d55d4212f4cc2f89b0d8a35d714e43401586f566300cb9b39b27fba9d5d617d |

Ignored local evidence: `history/public-release-completed.json`, `history/public-release-cut.log`, `history/public-artifact-integrity.json`, `history/public-*-attestation.json`, `history/public-*-installer.log`, `history/public-cargo-smoke.json`, `history/public-homebrew-smoke.json`, and downloaded assets in `history/published-v1.0.0/`. The Linux test VM initially stopped between tool invocations; keeping its startup and both test processes in one bounded invocation completed both tests successfully. This was a test-host lifecycle issue, not a release failure.
