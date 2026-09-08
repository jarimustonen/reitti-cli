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

## First-run help review

The primary agent exercised the merged foundation binary with a temporary empty HOME and no product environment variables. `doctor` correctly reports the missing subscription key without reading real user configuration. Final release polish must also improve the CLI's own help: root runtime URL/timeout options and most `config update` options currently have empty descriptions; subcommand text renders `Usage: update [OPTIONS]` rather than the full invocation. Explain runtime overrides versus persistent `--set-*` options, duration units and defaults, key input through stdin, dry-run behavior and configuration location/precedence. Make `--version` discoverable and provide useful first-run examples. Validate the final help against real invocations rather than only checking that example arrays exist. These are bounded usability refinements for release integration, not a reason to redesign the provider client.

## Independent Linux source-package validation

The primary agent exported only tracked files from commit `6d50821` into a disposable source tree, without `.git`, `.env` or ignored files. In an existing Linux ARM64 Podman VM, official image `docker.io/library/rust:1.88-bookworm` (digest `sha256:8aa70d1416cf5b1cff4b95ec6c57f1c5e4e649a3b53d616a26695cda6fbb46bc`) ran `cargo test --locked --workspace` successfully: all 30 Rust tests passed. The container was removed afterward; no host toolchain was installed or changed. This validates the declared MSRV on Linux ARM64 and source-archive behavior. It does not claim musl release artifacts, Linux x86_64 execution, or GitHub CI success. The existing VM also exposes x86_64 binfmt support, which may support later explicit emulated artifact smoke tests if needed.

## Landed provider-client verification

The primary agent independently ran all five repository gates on main commit `36165ca`: formatting, strict Clippy, all 44 Rust tests, 21 credential-free provider fixtures, and issue metadata validation passed. The same tracked source snapshot, exported without Git metadata or ignored files, passed all 44 tests in the existing Linux ARM64 container with Rust 1.88. The container was removed afterward. This extends MSRV/source-archive evidence through the real HTTP provider layer; it is still not a musl release artifact or hosted CI claim. Main was pushed to the private GitHub repository.

A local redacted Gitleaks scan of the 39 commits preceding the client landing found no leaks. The final release must repeat the scan after remaining implementation lands.

## Runner toolchain path follow-up

A later preflight found an existing rustup installation and stable ARM64 toolchain on the macOS machine, with Cargo 1.98.1 available. The dedicated repository runner initially used only the Homebrew/system PATH. Its PATH was updated to include the existing rustup-managed tools first, and only this repository's idle runner service was restarted successfully. No toolchain was installed by this change. This removes a tool-discovery problem before the generated release workflow runs; actual artifact builds remain pending.

## Shared invocation correlation follow-up

Primary source inspection found that `run_with_transport` allocates an ID for the verbose `invocation_started` log and handlers independently allocate another ID for `data.request.request_id`. With a nonconstant generator, these cannot be correlated. Final serialized integration must allocate one invocation ID for both, with an incrementing-generator regression test. This is a small common-dispatch correction; parallel handler workers should not edit the shared file.

The subsequent [CI run 34221042725](https://github.com/jarimustonen/reitti-cli/actions/runs/34221042725), on the landed provider client, was again blocked before steps by the same account billing/spending restriction. Local macOS and Linux/MSRV checks passed independently as recorded above.

## Native build on the required macOS machine

The primary agent exported tracked source at `36165ca7a8e1734a1cdb0abe8ca20d7a34ddb01a` into an isolated directory on the maintainer-selected macOS runner machine and ran `cargo build --release --locked --workspace` with the existing rustup toolchain. The optimized build succeeded. `file` identified a Mach-O ARM64 executable, and `reitti version --json` executed successfully, reporting version 0.0.0, schema version 1 and the explicitly supplied source commit. Its SHA-256 was `f16adbfb3bd41f5b18f8f2f3353be3a66bbe2ef3f7285dee582af22d73805ccd`.

This was a direct build on the runner machine, not a GitHub Actions job or the final release artifact. Source transfer excluded ignored files and Git metadata; no credentials were copied. Final v1 artifacts must be rebuilt after the remaining features and version bump, and the generated workflow still needs its own execution evidence.

## Distribution path remapping

A byte scan of the optimized macOS preflight executable found the builder's private home directory in dependency source-location strings under the Cargo registry. The project workspace path was absent, but stripping debug information alone does not remove all panic/source-location strings. Final distributed builds must remap compiler paths using values derived from the build environment and a supported cargo-dist setup/configuration mechanism. Do not hardcode a maintainer path into tracked build configuration or edit generated workflow files by hand. Verify the resulting binary bytes against the actual operator-supplied build-path markers before release. This follows canon section 23 and is separate from the distributed-text doctor check.

The primary agent verified the remapping remedy in a second native optimized build of the same source: append `--remap-path-prefix=<build-home>=/build` dynamically to `CARGO_ENCODED_RUSTFLAGS`, preserving existing encoded flags or parsing existing `RUSTFLAGS` first. The rebuilt executable no longer contains the builder home path, contains the neutral replacement, and still executes `version --json` successfully. Its SHA-256 is `98cfb41c24011479df414720d2cbd5e9cfb27aebab8632de0a81a43ca5942729`. The final release workflow must adopt equivalent supported configuration rather than relying on this one-off preflight environment.

A native Linux ARM64 musl preflight also succeeded from the tracked client source using official `rust:1.88-alpine`, selected as Linux ARM64 (image digest `sha256:eb5ce72a65a7b223c98892aa3cdabc97af7d73ff011ba0757dbf5a67c3809186`). The container-only `musl-dev` package supplied build headers; no host tool was installed. `cargo build --release --locked --workspace --target aarch64-unknown-linux-musl` produced a statically linked ELF ARM64 executable, and its version command ran inside Alpine. This verifies a local static-build route while hosted CI is unavailable; it is not the final v1 artifact.

The corresponding Linux x86_64 musl preflight also passed: official `rust:1.88-alpine` selected as Linux AMD64 (digest `sha256:64eba3726734dcfe89e0a62a0485007a3ab7c7372ce5b38c621d8812f70215f0`) built the same tracked source with `--target x86_64-unknown-linux-musl`. `file` confirmed a static PIE ELF x86-64 executable, and its version command ran successfully under the existing VM's x86_64 emulation. The first build took 15 minutes with two build jobs; the isolated dependency cache remains available for final-source rebuilds. Both Linux containers were removed after execution. All three requested target architectures now have concrete build/run preflight evidence, while final v1 artifacts and generated-workflow execution remain separate acceptance steps.


## Final packaging tooling recheck

The externally managed shipshape installation now reports 0.12.2; this task did not update it. Isolated configuration generation still pins cargo-dist 0.28.2 and preserves all three agreed targets. Its readiness audit reports the expected remaining changelog, dependency-update automation and unverified private vulnerability-reporting recommendations; none is evidence that the unfinished product is ready.

Official cargo-dist 0.28.2 static Linux archives were downloaded and matched their published SHA-256: ARM64 `77b8e045ee5da1892679842c142dd30d9e8275f9e4b211714999b2e235b86092`, x86_64 `6f11b15b1b9bd2938cb9de488825420f514218dc5f92b2871ca155538b493e46`. Both executables ran successfully in their corresponding Alpine containers. The previously verified macOS tool also ran on the required runner machine. These are isolated packaging tools, not global installations or final product artifacts.

The [pinned cargo-dist configuration reference](https://github.com/axodotdev/cargo-dist/blob/v0.28.2/book/src/reference/config.md#github-build-setup) documents the supported `github-build-setup` hook: a YAML step list inserted before the build, with its path relative to the workflow directory. Release integration should use this mechanism for dynamic compiler path remapping and then regenerate the workflow.
