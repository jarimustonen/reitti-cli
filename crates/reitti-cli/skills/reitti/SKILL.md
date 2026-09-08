---
name: reitti
description: Resolve HSL-area places and stops, compare public-transport journeys, check departure or arrival plans, and inspect live departures and service alerts with the reitti CLI. Use for travel in Helsinki, Espoo, Vantaa, Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio, or Tuusula.
cli_version: "1.0.0"
schema_version: 1
---

# Reitti

Use `reitti` to gather bounded Digitransit facts, then make the conversational travel decision yourself. Prefer `--json`, explicit limits, and RFC 3339 times with an offset.

## Resolve endpoints

A journey endpoint may use `query:` directly; its conservative resolver will not silently choose among ambiguous candidates. Use candidate discovery when it helps clarify the request:

```sh
reitti --json location list --query "Kamppi" --kind stop --limit 5
reitti --json stop list --near 60.1699,24.9384 --radius-m 500 --limit 5
```

Journey endpoints accept only `query:`, `place:`, `stop:`, or `coord:` references. If `query:` produces `location_ambiguous`, present plausible candidates and retry with a returned `place:` or `stop:` ref only after the user or existing context supplies enough evidence to select it.

## Plan and compare

Omit time flags to plan from now. Use `--depart-at` for a departure constraint or `--arrive-by` for a deadline; never pass both. Use the user's actual travel date and offset. This timestamp is illustrative:

```sh
reitti --json journey list \
  --from place:gtfshsl:station:GTFS:HSL:1000102 \
  --to stop:HSL:1020453 \
  --arrive-by 2027-02-15T10:00:00+02:00 --limit 3
```

Compare every returned alternative using its times, duration, transfers, walking, and `comparison` facts. Labels such as `fastest` and `fewest_transfers` are measured facts, not a recommendation. Account for the user's priorities and explain trade-offs. A response can be incomplete even when it returns the requested count.

`--wheelchair` requests wheelchair-aware routing but does not prove accessibility. State `accessibility.status` and missing evidence honestly. Do not infer fares, availability, or accessibility from null or unknown fields.

`--max-walk-m` filters only the bounded alternatives the provider returned; unknown walking distance cannot satisfy the cap, and `no_matching_journeys` does not prove that no route exists. A justified larger `--limit` (up to 6) may expose another provider alternative. Add `--include-geometry` when a map or route line is needed. Geometry is GeoJSON `[longitude, latitude]`, unlike CLI coordinate input `LAT,LON`; walking steps are returned without this flag. Respect `navigation_complete` and truncation warnings.

## Check live context

Use a raw HSL stop ID, without the `stop:` prefix, for departures:

```sh
reitti --json departure list --stop HSL:1020453 --window 2h --limit 10
reitti --json alert list --route HSL:31M1 --stop HSL:1020453 --limit 25
```

Describe realtime evidence as `updated`, `scheduled`, `cancelled`, or `unknown`; equality of times does not prove a live update. An empty alert list does not guarantee normal service. Mention retrieval time and preserve the returned Digitransit attribution in user-facing summaries.

## Correct failures

Read structured `error.code`, `expected`, `details`, and retry guidance. The CLI makes one bounded provider attempt; make another bounded query only for a clear purpose. Honor `Retry-After`, and stop or report the error when it persists or the travel deadline makes retry pointless. Register for API access at <https://digitransit.fi/en/developers/api-registration/>. Fix `credential_missing` by providing `DIGITRANSIT_SUBSCRIPTION_KEY` through a protected environment or piping one line to `reitti config update --subscription-key-stdin`. Never request a key in chat, put it in arguments, or display it.

Run `reitti --json version` to inspect CLI, schema, and bundled-skill versions. If it differs from this skill's `cli_version`, read `reitti skill print reitti` for the running binary's workflow; do not silently overwrite a locally modified installed skill. After an unexplained failure, run offline `reitti --json doctor`; use `--online` only when bounded provider probes are actually needed.

For exact accepted values, response interpretation, and retry examples, read [references/workflows.md](references/workflows.md).
