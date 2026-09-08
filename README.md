# reitti-cli

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`reitti` is a non-interactive journey-planning CLI for people and AI agents. It resolves places, compares public-transport journeys, lists nearby stops and departures, and reports service alerts using [Digitransit](https://digitransit.fi/) open data.

It covers the HSL Journey Planner service area: Helsinki, Espoo, Vantaa, Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio, and Tuusula.

- [Installation](#installation)
- [API key and first run](#api-key-and-first-run)
- [Examples](#examples)
- [Configuration](#configuration)
- [Agent skill](#agent-skill)
- [Limits and interpretation](#limits-and-interpretation)
- [Privacy and data sources](#privacy-and-data-sources)

## Installation

### Shell installer

After the v1.0.0 assets are published on GitHub Releases, the generated installer selects the supported archive for the current machine:

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/jarimustonen/reitti-cli/releases/latest/download/reitti-cli-installer.sh | sh
```

Review a downloaded installer before executing it if that is your normal security policy. The command requires published, accessible release assets.

### Prebuilt archives

The release plan produces checksum-protected archives for:

- macOS arm64: `reitti-cli-aarch64-apple-darwin.tar.xz`
- Linux arm64 (static musl): `reitti-cli-aarch64-unknown-linux-musl.tar.xz`
- Linux x86_64 (static musl): `reitti-cli-x86_64-unknown-linux-musl.tar.xz`

Each archive has a matching `.sha256` file, and the release includes `sha256.sum`. Verify the checksum before installing `reitti` from an archive. Intel macOS and Windows do not have prebuilt v1 binaries.

### Build from source

Rust 1.88 or newer is required. Once the v1.0.0 tag exists, install that exact source revision with Cargo:

```sh
cargo install --locked --git https://github.com/jarimustonen/reitti-cli \
  --tag v1.0.0 reitti-cli
```

Before a tag exists, install from an authenticated checkout:

```sh
git clone https://github.com/jarimustonen/reitti-cli
cd reitti-cli
cargo install --locked --path crates/reitti-cli
```

The project is not published to crates.io or Homebrew.

## API key and first run

Digitransit requires a free subscription key. Follow the [registration and key-rotation guide](docs/digitransit-api-access.md), then save the key without placing it in command arguments or shell history. This Bash-compatible hidden-input wrapper works on supported macOS and Linux systems:

```bash
read -rsp 'Digitransit subscription key: ' key; printf '\n'
printf '%s\n' "$key" | reitti config update --subscription-key-stdin
unset key
```

The CLI itself remains non-interactive: `--subscription-key-stdin` reads exactly one line supplied by a caller. A secret manager can pipe the key to the same option instead.

Check the local setup first, then make deliberate network checks:

```sh
reitti doctor
reitti doctor --online
```

`doctor` is read-only. Offline mode validates configuration, credential presence, bundled-skill synchronization, build provenance, and configured private markers without contacting Digitransit. `--online` adds bounded geocoding and routing probes.

## Examples

Use `--json` for schema-versioned output. Times must be RFC 3339 with `Z` or an explicit offset; coordinates use `LAT,LON` without whitespace.

Find candidates before choosing an ambiguous place:

```sh
reitti --json location list --query "Kamppi" --kind stop --limit 5
reitti --json stop list --near 60.1699,24.9384 --radius-m 500 --limit 5
```

Plan from coordinates at the current time:

```sh
reitti --json journey list \
  --from coord:60.1699,24.9384 \
  --to coord:60.1776,24.6529 --limit 3
```

For a deadline, use the actual travel date and its UTC offset. The timestamp below is illustrative, not a current timetable query:

```sh
reitti --json journey list \
  --from query:Kamppi --to stop:HSL:1020453 \
  --arrive-by 2027-02-15T10:00:00+02:00 \
  --wheelchair --max-walk-m 800 --limit 6
```

List departures and active alerts. These commands take raw HSL identifiers, without a `stop:` or `route:` reference prefix:

```sh
reitti --json departure list --stop HSL:1020453 --window 2h --limit 10
reitti --json alert list --route HSL:31M1 --stop HSL:1020453 --limit 25
```

Add `--include-geometry` to a journey request when GeoJSON route lines are needed. Discover exact flags and response schemas without credentials:

```sh
reitti journey list --help
reitti --json schema list
reitti --json schema show journey-list
```

## Configuration

Persistent values follow this precedence independently for each key:

1. a matching invocation flag;
2. an environment variable;
3. the configuration file;
4. the built-in default.

`--routing-url`, `--geocoding-url`, `--connect-timeout`, and `--request-timeout` are temporary invocation overrides. Their persistent counterparts are the `config update --set-*` options. Supported duration suffixes are `ms`, `s`, `m`, and `h`; connection and whole-request defaults are 5 seconds and 20 seconds.

Inspect the selected file and every effective value's source:

```sh
reitti --json config path
reitti --json config show
reitti --json config update --language fi --timezone Europe/Helsinki --dry-run
```

The subscription key and private-marker values are redacted by default. Configuration files are created with private permissions. See [Configuration and operation](docs/configuration.md) for paths, environment variables, safe key setup, and runtime behavior.

## Agent skill

The binary bundles a version-matched Agent Skill with supporting workflow guidance. Inspect it without writing files:

```sh
reitti --json skill list
reitti skill print reitti
```

Install the native skill tree for Claude Code, Pi, and Codex into an isolated or real home only when intended:

```sh
reitti --json skill install reitti --agent all --dry-run
reitti --json skill install reitti --agent all
```

Installation is non-interactive and does not clobber a differing existing tree. Use `--force` only after reviewing the destination. Select one runtime with `--agent claude`, `--agent pi`, or `--agent codex`; use `--target DIR` to change the install base.

## Limits and interpretation

- A `query:` journey endpoint is accepted only when it resolves uniquely with strong provider confidence. Otherwise `location_ambiguous` returns candidates; select a returned `place:` or `stop:` reference rather than guessing.
- Results are bounded and may be incomplete. Comparison labels such as `fastest` apply only to alternatives in that response.
- `--wheelchair` asks the router for wheelchair-aware results but cannot prove every part of a trip is accessible. Preserve `unknown` accessibility evidence.
- `--max-walk-m` filters only the bounded alternatives already returned by Digitransit. No local match does not prove that no suitable route exists.
- Realtime status is explicit evidence. When realtime data is absent, scheduled times remain available and are labelled accordingly; equal scheduled and actual times alone do not prove an update.
- Navigation can be incomplete or truncated. GeoJSON coordinates are `[longitude, latitude]`, unlike CLI input coordinates.
- An empty alert list does not guarantee normal service.

The bundled Agent Skill contains the detailed interpretation and corrective-retry workflow.

## Privacy and data sources

`reitti` sends requested locations, stops, routes, times, language, and planning options to the configured Digitransit HTTPS endpoints. The subscription key is sent in the `digitransit-subscription-key` request header, never in the URL. The CLI has no analytics, account system, or background tracking. Persistent configuration stays on the local machine; command output goes to stdout or an explicitly selected file.

Journey, stop, departure, alert, and map-related facts come from Digitransit and its upstream sources. Preserve the attribution included in command responses when presenting results. Upstream data keeps its own licensing and attribution terms; the MIT License covers this software only.

This project is independent and is not affiliated with or endorsed by HSL or Digitransit.

See [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), and [distribution operations](docs/DISTRIBUTION.md).

## License

The software is available under the [MIT License](LICENSE). Transit, map, and other upstream data are not relicensed by this repository.
