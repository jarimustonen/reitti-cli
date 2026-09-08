# Distribution operations

Releases publish `reitti-core` and `reitti-cli` to crates.io, prebuilt binaries to GitHub Releases, and formula `reitti-cli` to `jarimustonen/homebrew-reitti`. The CLI crate pins its core dependency to the same version; publish core first and wait for registry visibility before publishing the CLI.

<!-- shipshape-dist:managed:start -->
## Generated release channel

`dist-workspace.toml` and `.github/workflows/release.yml` are generated from the approved `OSS-RELEASE.md` distribution contract. Refresh the configuration with `shipshape dist generate`, retain the Homebrew installer, tap and publish-job settings plus the maintainer-required macOS runner override if the shipshape generator omits them, and regenerate the workflow with the pinned cargo-dist. Never hand-edit `release.yml`.

The repository pins cargo-dist 0.28.2. Its release plan creates:

- `reitti-cli-aarch64-apple-darwin.tar.xz` and checksum;
- `reitti-cli-aarch64-unknown-linux-musl.tar.xz` and checksum;
- `reitti-cli-x86_64-unknown-linux-musl.tar.xz` and checksum;
- `reitti-cli-installer.sh`;
- `sha256.sum`; and
- `source.tar.gz` and checksum.

### Platform guarantee

The channel publishes prebuilt binaries for macOS arm64 and static musl Linux on arm64 and x86_64. There is no prebuilt or support guarantee outside those three declared targets. The macOS archive is built on the maintainer-selected self-hosted macOS ARM64 runner. That trusted runner is reserved for tag-triggered release builds. Ordinary push and pull-request CI is intentionally absent.

### Build identity and private-path removal

Release builds set `REITTI_BUILD_COMMIT` to the exact 40-character source commit. `reitti version --json` reports that identity as `build_provenance.kind = "ci-injected"`. A source build without Git metadata reports an explicit null commit and tarball or vendored provenance instead of inventing an identity.

`.github/build-setup.yml` dynamically appends compiler path remaps for the runner home and checked-out source tree while preserving existing `CARGO_ENCODED_RUSTFLAGS`, or safely converting existing `RUSTFLAGS`. The neutral prefixes are `/build/home` and `/build/source`; no maintainer path is tracked. The generated workflow inserts that setup before every cargo-dist build.

For an equivalent direct local release build, preserve existing flags and append remaps derived from the current environment before invoking Cargo:

```bash
set -euo pipefail
encoded="${CARGO_ENCODED_RUSTFLAGS:-}"
if [[ -z "$encoded" && -n "${RUSTFLAGS:-}" ]]; then
  encoded="$(python3 - <<'PY'
import os, shlex
print("\x1f".join(shlex.split(os.environ["RUSTFLAGS"])), end="")
PY
  )"
fi
append_flag() { [[ -z "$encoded" ]] || encoded+=$'\x1f'; encoded+="$1"; }
append_flag "--remap-path-prefix=${HOME}=/build/home"
append_flag "--remap-path-prefix=$(pwd -P)=/build/source"
export CARGO_ENCODED_RUSTFLAGS="$encoded"
export REITTI_BUILD_COMMIT="$(git rev-parse HEAD)"
cargo build --locked --profile dist --package reitti-cli
```

This is an operator build recipe, not a global Cargo configuration recommendation. Cargo-dist's global build, including `source.tar.gz`, must run from a Git checkout because source-archive generation invokes Git. Target-local archive builds can use tracked source exports when the commit is injected explicitly. Before publication, scan the complete archive and executable bytes for operator-supplied private path markers and confirm `reitti version --json` reports the intended commit.

### Refresh and verification

After changing `distribution` in `OSS-RELEASE.md`:

1. run `shipshape dist generate --require-approved` with the checksum-verified pinned cargo-dist on `PATH`;
2. retain `installers = ["shell", "homebrew"]`, `tap = "jarimustonen/homebrew-reitti"`, `publish-jobs = ["homebrew"]`, `github-build-setup = "../build-setup.yml"`, and `[dist.github-custom-runners]` with `aarch64-apple-darwin = "self-hosted"`;
3. run cargo-dist `generate` again and verify there is no generated diff on a second run;
4. inspect `dist plan --output-format=json` and read artifact paths from its JSON (`target/distrib` in cargo-dist 0.28.2);
5. verify all three target archives, the shell installer, aggregate and per-file checksums, source archive, and exact build commit;
6. smoke-test every executable and the shell installer's platform selection in disposable homes; and
7. inspect the actual tag-triggered GitHub release workflow before claiming release jobs or attestations passed.

The public v1.0.0 release produced GitHub artifact attestations for all three binary archives. Verify a downloaded archive against this repository before using it:

```sh
gh attestation verify reitti-cli-aarch64-apple-darwin.tar.xz \
  --repo jarimustonen/reitti-cli
```

Substitute the matching Linux archive when appropriate. Check that the verified source commit matches the release tag; retain checksum verification as well. The [v1.0.0 release run](https://github.com/jarimustonen/reitti-cli/actions/runs/34242521413) completed all build, attestation, asset-publication, and Homebrew steps successfully. Its macOS job used the required self-hosted runner.
<!-- shipshape-dist:managed:end -->

## Release boundary

The maintainer authorized public visibility and the first v1.0.0 publication. Future releases require an explicit maintainer instruction. Release preparation alone does not authorize publication or billing changes. Ordinary push and pull-request CI is intentionally absent by maintainer decision and must not be regenerated merely to satisfy an audit. The generated tag-triggered cargo-dist workflow remains the release path; local checks are not a substitute for a successful release run.
