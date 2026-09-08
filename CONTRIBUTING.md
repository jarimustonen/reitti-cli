# Contributing to reitti-cli

Thank you for helping improve `reitti`, an agent-first command-line journey planner powered by Digitransit open data.

## Before you start

For a bug or feature proposal, first open a [GitHub issue](https://github.com/jarimustonen/reitti-cli/issues) so the intended behavior and scope can be agreed before substantial work begins. Do not disclose vulnerabilities publicly; follow [SECURITY.md](SECURITY.md) instead.

## Development setup

Install the stable Rust toolchain, including the `rustfmt` and `clippy` components, then clone the repository. From the workspace root, run:

```sh
cargo build --workspace
```

Do not commit credentials. Live Digitransit access requires a subscription key; follow [the API-access guide](docs/digitransit-api-access.md) and provide the key through a protected environment or `config update --subscription-key-stdin`.

## Make a change

1. Branch from `main`.
2. Keep the change focused and add or update tests for behavior changes.
3. Preserve deterministic text output and schema-versioned JSON output for CLI changes.
4. Use a clear, imperative commit summary.
5. Open a pull request against `main` and explain the motivation, behavior change, and validation performed.

The maintainers curate release notes from issue-linked commits; contributors do not need to edit the changelog unless requested.

## Required checks

Run the Rust checks before opening a pull request:

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

The full maintainer merge gate also checks the committed Digitransit fixtures and issue repository:

```sh
python3 scripts/probe-digitransit.py check
issuectl doctor --json
```

Contributors do not need `issuectl` to build, use, or make an ordinary code contribution. Maintainers run the issue-repository check before merge; contributors who already use `issuectl` may run it locally. A pull request should not merge until the full gate passes. If a check cannot run in your environment, state that clearly in the pull request.

## Pull requests

Keep unrelated changes in separate pull requests. Update user-facing documentation when behavior changes, avoid drive-by formatting, and call out compatibility or release implications explicitly. Maintainers may ask for a smaller change or additional evidence before merging.

## Licensing

By contributing, you agree that your contribution is licensed under the project's [MIT License](LICENSE).

## Code of conduct

All contributors must follow the [Code of Conduct](CODE_OF_CONDUCT.md).
