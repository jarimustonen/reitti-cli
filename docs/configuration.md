# Configuration and operation

`reitti` is non-interactive. Every input comes from arguments, environment variables, the configuration file, or stdin where a command explicitly supports it.

## Configuration location

Inspect the path instead of guessing:

```sh
reitti --json config path
```

Resolution order for the file path is:

1. absolute `$REITTI_CONFIG_FILE`;
2. `$XDG_CONFIG_HOME/reitti/config.toml`;
3. `$HOME/.config/reitti/config.toml`.

The file and newly created configuration directories use private permissions. Symlinks and insecure file modes are rejected.

## Value precedence

Each value resolves independently using `flag > environment > file > built-in default`. A flag for one key does not suppress environment or file values for other keys.

| Value | Invocation flag | Environment | Built-in default |
| --- | --- | --- | --- |
| Subscription key | stdin update only | `DIGITRANSIT_SUBSCRIPTION_KEY` | none |
| Language | command `--language` where supported | `REITTI_LANGUAGE` | `en` |
| Timezone | none | `REITTI_TIMEZONE` | `Europe/Helsinki` |
| Routing URL | `--routing-url` | `REITTI_ROUTING_URL` | HSL routing v2 endpoint |
| Geocoding URL | `--geocoding-url` | `REITTI_GEOCODING_URL` | Digitransit geocoding v1 endpoint |
| Connect timeout | `--connect-timeout` | `REITTI_CONNECT_TIMEOUT` | `5s` |
| Request timeout | `--request-timeout` | `REITTI_REQUEST_TIMEOUT` | `20s` |
| Private markers | none | `REITTI_PRIVATE_MARKERS` (JSON array) | empty |

Run `reitti --json config show` to inspect effective values and their `flag`, `env`, `file`, or `default` source. Secrets are redacted unless `--show-secrets` is explicitly supplied; that option emits a warning.

## Persistent updates

`config update` changes only named values and writes atomically. Preview without writing:

```sh
reitti --json config update \
  --language fi \
  --set-connect-timeout 5s \
  --set-request-timeout 20s \
  --dry-run
```

The global `--routing-url`, `--geocoding-url`, `--connect-timeout`, and `--request-timeout` options affect one invocation. Use the corresponding `--set-*` option under `config update` to persist a value. Durations accept `ms`, `s`, `m`, and `h` suffixes.

### Subscription key through stdin

Do not place a real key in argv, a URL, shell history, a repository file, or a pasted transcript. A secret manager can emit one line directly to `reitti config update --subscription-key-stdin`; no particular or commercial tool is required. Without one, use this Bash-compatible hidden-input wrapper:

```bash
read -rsp 'Digitransit subscription key: ' key; printf '\n'
printf '%s\n' "$key" | reitti config update --subscription-key-stdin
unset key
```

The CLI reads exactly one nonblank line and rejects trailing input. It does not prompt.

## Diagnostics and output

Start with offline diagnostics:

```sh
reitti doctor
```

Offline `doctor` does not contact Digitransit. It checks local configuration, credential presence, bundled skill synchronization, build provenance, and whether configured private markers appear in bundled public text. Release operators separately scan complete artifact bytes for private build-path markers.

Use `reitti doctor --online` only for deliberate, bounded provider probes. The command may fail when credentials or connectivity are unavailable.

`--json` writes one schema-versioned document to stdout. `--verbose` adds one-line JSON diagnostic events to stderr without revealing the subscription key. `--output PATH` atomically writes the complete result to a private file and prints file metadata to stdout.

## Network behavior

The CLI performs bounded HTTPS requests with redirects disabled. It sends the subscription key in the `digitransit-subscription-key` header. Provider failures are classified in structured errors; rate-limit responses can include retry timing. The CLI does not retry silently or start background monitoring.
