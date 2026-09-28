---
name: reitti
description: Resolve HSL-area places and stops, compare public-transport journeys, check departure or arrival plans, and inspect live departures and service alerts with the reitti CLI. Use for travel in Helsinki, Espoo, Vantaa, Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio, or Tuusula.
cli_version: "1.1.0"
schema_version: 1
---

# Reitti

`reitti` fetches bounded facts from Digitransit, the open-data service behind HSL's journey planner: place and stop candidates, journey alternatives, one stop's details, departures, and service alerts. It deliberately decides nothing. The travel advice, the weighing of a faster route against a longer walk, and the honesty about what the data does not show are your work. Everything below exists so you can do that work well.

Use `--json`. It gives schema-versioned data with a `warnings` array instead of terminal prose. Each command's `--help` lists its flags, but the per-flag descriptions are thin, so exact accepted values, limits, ID grammar, and retry examples live in [references/workflows.md](references/workflows.md). `reitti --json schema show <command-name>` describes each response shape.

## Two kinds of identifier

Journey endpoints take a typed reference: `query:<free text>`, `place:<id>`, `stop:<id>`, or `coord:<LAT,LON>`. Commands that address one stop or route directly, which are `stop show`, `departure list`, and the alert filters, take the raw HSL ID without a tag, such as `HSL:1020453`, and reject the tagged form. Coordinates are latitude first on input, while returned GeoJSON geometry is `[longitude, latitude]` as GeoJSON requires. Both distinctions are easy to get backwards.

## Resolving places

A `query:` endpoint is resolved conservatively. The CLI accepts it only when the geocoder returns exactly one concrete address, venue, or stop with high confidence; anything else fails with `location_ambiguous`, and the error carries the candidates together with their stable `place:` or `stop:` references in `retry_refs`. This is by design: "Kamppi" is a metro station, a bus terminal, a district, and several venues, and silently picking one would produce a confident plan to the wrong place. When the conversation already makes clear which candidate is meant, choose it, say what you chose, and continue. When the candidates differ in a way the user would care about and nothing in the context settles it, that is worth a question.

You can also look before you plan:

```sh
reitti --json location list --query "Kamppi" --kind stop --limit 5
reitti --json stop list --near 60.1699,24.9384 --radius-m 500 --limit 5
reitti --json stop show HSL:1020453
```

`stop show` returns one stop's code, platform, zone, parent station, coordinates, modes, and wheelchair-boarding evidence, plus a `reittiopas_url`. That link is HSL's web page for humans and is a good thing to hand to the user. It is not a second data source: there is nothing to gain from fetching it and reading facts out of it, and the page is not built to be read that way.

## Planning journeys

```sh
reitti --json journey list \
  --from place:gtfshsl:station:GTFS:HSL:1000102 \
  --to stop:HSL:1020453 \
  --arrive-by 2027-02-15T10:00:00+02:00 --limit 3
```

Without a time flag, planning starts now. `--depart-at` and `--arrive-by` are alternatives; each needs an RFC 3339 time with an explicit offset. The service area is in Europe/Helsinki, so use that date's offset (+02:00 in winter, +03:00 in summer) unless the user is clearly speaking in another zone. The timestamp above is only an example.

What comes back is a bounded set of up to `--limit` alternatives, at most six, and the response describes its own limits. Read it that way:

- `comparison` labels such as `fastest` and `fewest_transfers` are measurements across the returned alternatives only. Ties can label several alternatives, and a label is not a recommendation. The recommendation is yours, built from the user's priorities and the trade-offs you can see: transfers, walking, tight connections, scheduled-only legs, and the arrival deadline.
- `complete: false` and warnings such as `alternatives_truncated` mean other routes may exist. A larger `--limit` may surface one when there is a reason to look.
- `fare` is always null in this version, `accessibility.status` is always `unknown`, and a missing walking distance is missing evidence. `--wheelchair` asks the router for wheelchair-aware results, and the per-stop `wheelchair_boarding` evidence in `accessibility.evidence` is real, but nothing in the response proves a whole trip accessible. Say what the evidence shows and no more.
- `--max-walk-m` filters the alternatives the provider already returned. An alternative with unknown walking distance cannot pass the cap, and `no_matching_journeys` means none of the bounded results could be shown to fit, not that no route exists.
- Walking steps are always included. `navigation_complete: false` or a `navigation_truncated` warning means the turn-by-turn part is partial and should not be presented as complete guidance. `--include-geometry` adds a GeoJSON route line for maps.

## Live context

```sh
reitti --json departure list --stop HSL:1020453 --window 2h --limit 10
reitti --json alert list --route HSL:31M1 --stop HSL:1020453 --limit 25
```

Every departure and journey leg carries a realtime `state` of `scheduled`, `updated`, `cancelled`, `added`, or `unknown`, and a journey summarises its legs in `realtime.status` as `updated`, `scheduled_only`, `mixed`, `cancelled`, or `unknown`. Use those words with the user. A scheduled time equal to the estimated time does not show that a live update arrived; only the state does. A cancellation is explicit provider evidence, and a trip that is merely absent is not one. An empty alert list is the absence of alerts in Digitransit, not proof of normal service, and the response warns about that too.

The data is a snapshot. Tell the user when it was retrieved (`retrieved_at`) and keep the `attribution` the response carries in what you present, because Digitransit's data licence asks for it. Rechecking departures or alerts close to travel time is often worth one more query. Watching and notifying is not something this CLI can do, so do not promise it.

## Correct failures

Errors are structured: `error.code`, `expected`, `details`, and a `retryable` flag. The message usually says what to change, and `details` often carries the material for a retry, such as `retry_refs` for an ambiguous location or `retry_after_seconds` when Digitransit rate-limited the request. The CLI makes one provider attempt per invocation and never retries on its own, so every retry is your decision. Each call spends the user's personal Digitransit quota, and a retry after the travel deadline has passed helps nobody. Retry when the error tells you what to fix, honour any retry-after value, and report a persisting error rather than looping on it.

`credential_missing` means no Digitransit subscription key is configured. Keys are free at <https://digitransit.fi/en/developers/api-registration/>. The key belongs in `DIGITRANSIT_SUBSCRIPTION_KEY` in a protected environment, or piped as a single line to `reitti config update --subscription-key-stdin`. Do not ask for it in chat, pass it as an argument, or echo it: transcripts, shell history, and logs persist, and the key is the user's identity and quota with the provider. Point the user at the README's setup steps instead.

`reitti --json doctor` checks configuration, credential presence, skill synchronisation, and build provenance without touching the network, which makes it the cheap first look after a failure you cannot explain. `--online` adds real provider probes and spends quota, so it is for when the question is specifically whether Digitransit is reachable.

## Skill and binary versions

The binary bundles its own copy of this skill, and `reitti --json version` reports both the CLI version and the bundled skill's `cli_version`. If the copy you are reading is older than the binary, flags or fields may have moved; `reitti skill print reitti` shows the guidance that matches the running binary, and `--resource references/workflows.md` prints the reference the same way. `skill install` refuses to overwrite an installed tree that differs from the bundle unless given `--force`, because the difference may be someone's deliberate local edit. Replacing such a tree is the user's call, not a routine update.
