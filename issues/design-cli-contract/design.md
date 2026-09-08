# `reitti` v1 CLI and JSON contract

**Status:** accepted implementation contract  
**CLI data schema:** `1`  
**Help schema:** `1`  
**Companion skill schema:** `1`

This document is the implementation boundary for the v1 issues under
@v1-0-agent-journey-planner. Provider details are grounded in
@validate-data-contracts and its credential-free fixtures, plus the primary
agent's [production-schema validation](../implement-digitransit-client/validation.md). The public surface is
small on purpose: location resolution, journey planning, stop discovery and
live context compose through stable references rather than hidden session state.

## 1. Product and trust boundary

`reitti` is an independent client of Digitransit. It returns provider facts and
small deterministic comparisons; it does not make autonomous travel decisions.
Every provider-backed result says when it was retrieved and identifies its
source. It never claims that a service, fare, vehicle, accessibility feature,
or realtime update exists when the source did not return evidence for it.

v1 supports the verified HSL router and these municipalities: Helsinki, Espoo,
Vantaa, Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio, and Tuusula. A result
outside that boundary may be shown by search with `service_area: "outside"`.
Journey planning rejects only a location proven outside; `unknown` is passed
unchanged to the HSL router with honest scope metadata. This is a product
boundary, not a claim that the provider has no wider data.

Runtime provider traffic is limited to:

- Geocoding v1 for free-text candidates and stable Pelias-id lookup;
- Routing v2 HSL GTFS GraphQL at `/routing/v2/hsl/gtfs/v1` for journeys,
  stops, departures, alerts, and integrated cancellation/realtime state.

There are no v1 calls to removed Routing v1, Routing Data v3, Map v3, or raw
HSL GTFS-RT. Routing v2's verified GraphQL-over-HTTP behavior is preserved:
POST JSON with an operation name and variables, inspect `errors` even on HTTP
200, and treat a nullable result as a domain result rather than an HTTP error.

### Bounded-call invariant

One CLI invocation performs at most the following provider requests:

| Command | Maximum | Shape |
|---|---:|---|
| `location list` | 1 | one geocoding search |
| `stop list` | 1 | one bounded Routing v2 stop query |
| `journey list` | 3 | at most one lookup for each `query:`/`place:`/`stop:` endpoint, then one plan query; `coord:` adds no lookup |
| `departure list` | 1 | one bounded Routing v2 stop query |
| `alert list` | 1 | one Routing v2 alert query, filtered and capped locally if needed |
| `doctor` | 0 offline / 2 online | local checks by default; `--online` adds one minimal geocoding and one Routing v2 probe |
| all other commands | 0 | local only |

Every provider operation gets one bounded attempt in v1. There are no
transparent retries or hidden waits. On a retryable network failure or HTTP
403/429, the error reports `retryable` and preserves `Retry-After` when present
without guessing the quota or reason. The client issue may implement only this
single-attempt policy; a future explicit retry policy can be added without
changing domain models.

## 2. Command grammar

All resource queries use the canon's `list` verb. Journeys are computed
alternatives, not saved resources; `journey list` performs planning and does not
create persistent state. Place and stop searches use explicit `--query` filters.
Departure boards are the `departure` resource with a required `--stop` filter.
The unreleased draft spellings `search`, `plan`, and `stop departures` are not
aliases or supported commands.

Canonical synopsis (brackets mean optional syntax, not literal characters):

```text
reitti [GLOBAL] location list --query <QUERY> [--kind <KIND>] [--language <LANG>] [--limit <N>]
reitti [GLOBAL] journey list --from <LOCATION_REF> --to <LOCATION_REF>
       [--depart-at <RFC3339> | --arrive-by <RFC3339>]
       [--mode <MODE>]... [--max-walk-m <M>] [--wheelchair]
       [--include-geometry] [--language <LANG>] [--limit <N>]
reitti [GLOBAL] stop list [--query <QUERY> | --near <COORDINATES>]
       [--radius-m <M>] [--language <LANG>] [--limit <N>]
reitti [GLOBAL] departure list --stop <STOP_ID> [--at <RFC3339>]
       [--window <DURATION>] [--mode <MODE>]... [--language <LANG>] [--limit <N>]
reitti [GLOBAL] alert list [--route <ROUTE_ID>]... [--stop <STOP_ID>]...
       [--active-at <RFC3339>] [--language <LANG>] [--limit <N>]
reitti [GLOBAL] config path
reitti [GLOBAL] config show [--show-secrets]
reitti [GLOBAL] config update [--language <LANG>] [--timezone <IANA_TZ>]
       [--routing-url <HTTPS_URL>] [--geocoding-url <HTTPS_URL>]
       [--connect-timeout <DURATION>] [--request-timeout <DURATION>]
       [--private-marker <TEXT>]... [--subscription-key-stdin] [--dry-run]
reitti [GLOBAL] schema list
reitti [GLOBAL] schema show <SCHEMA_NAME>
reitti [GLOBAL] doctor [--online]
reitti [GLOBAL] version
reitti [GLOBAL] skill list
reitti [GLOBAL] skill print <NAME> [--resource <PATH>]
reitti [GLOBAL] skill install [<NAME>] [--agent <AGENT>] [--target <DIR>]
       [--dry-run] [--force]
```

Global options are accepted before or after a subcommand unless `--` has
transferred ownership to a positional parser:

```text
--json                       schema-versioned JSON instead of text
--output <PATH>              atomically write the command's full data output to PATH
--verbose                    diagnostic JSONL on stderr; off by default
--routing-url <HTTPS_URL>    per-invocation endpoint override
--geocoding-url <HTTPS_URL>  per-invocation endpoint override
--connect-timeout <DURATION> per-invocation timeout override
--request-timeout <DURATION> per-invocation timeout override
--frozen-time <RFC3339>      hidden in text help; visible in structured help; tests only
--version                    full alias of `version`
--help                       help for the current command path
```

`--json` and `--output` may be combined. In that case the complete JSON payload
is written to the file and stdout receives a JSON file-result envelope with
`path`, `bytes`, `content_type`, and `schema_version_written`. Without
`--output`, data is written to stdout. Files are created atomically and are
never partially replaced. Provider commands are small and strictly capped, so
JSONL/pagination is not part of v1.

There are no aliases (`ls`, `get`, `disruption`, `depart`, etc.), prompts,
pagers, spinners, color/ANSI output, TTY-sensitive formatting, or interactive
selection.

### Accepted scalar grammars

- `LANG`: exactly `en`, `fi`, or `sv`.
- `MODE`: exactly `bus`, `tram`, `rail`, `subway`, or `ferry`; repeated values
  are rejected. Walking is an access/transfer mode and cannot be disabled.
- `--wheelchair` is a boolean request for wheelchair-aware routing (off by
  default), mapped to `preferences.accessibility.wheelchair.enabled`. The
  provider explicitly does not guarantee accessibility because data can be
  incomplete or wrong. Preserve unknown evidence; enabling this flag alone
  never establishes that a journey is accessible.
- Journey `--limit`: integer 1–6, default 3.
- Search `--limit`: integer 1–10, default 5.
- Departure `--limit`: integer 1–50, default 10.
- Alert `--limit`: integer 1–100, default 25.
- `--max-walk-m`: integer 0–20,000.
- `--radius-m`: integer 1–5,000; accepted only with `--near`, default 1,000.
- `DURATION`: positive ASCII integer followed by `ms`, `s`, `m`, or `h`; no spaces,
  fractions, or mixed units. Departure `--window` is 1m–24h, default 2h.
- All public datetimes are RFC 3339 with an explicit numeric offset or `Z`.
  Naive local timestamps are rejected. Output timestamps are RFC 3339 with the
  source offset where meaningful; retrieval timestamps are UTC `Z`.
- Coordinates are `LAT,LON` in WGS84 decimal degrees, latitude first, with no
  whitespace; latitude -90…90 and longitude -180…180.
- `STOP_ID` and `ROUTE_ID` are canonical raw HSL GTFS IDs matching
  `^HSL:[A-Za-z0-9_.-]+$`. IDs and queries are non-empty and must contain a
  non-whitespace character. Inputs are not silently trimmed or case-corrected.

## 3. Stable references and explicit disambiguation

A `LOCATION_REF` is one of these tagged, single-argument forms:

```text
query:<free text>                         query:Kamppi
place:<Pelias gid>                       place:gtfshsl:station:GTFS:HSL:1000102
stop:<GTFS id>                           stop:HSL:1020453
coord:<LAT,LON>                          coord:60.1699,24.9384
```

Within polymorphic journey endpoints, a stop uses `stop:<GTFS id>` and a bare
ID, bare coordinate, or bare place name is rejected with
`invalid_location_ref`; explicit tagging prevents an agent from accidentally
changing interpretation. Stop-specific positions instead accept a canonical
raw `STOP_ID`, for example `HSL:1020453`, because the argument type is already
unambiguous. A prefixed `stop:HSL:…` there is rejected with `invalid_stop_id`.

`location list` returns `candidate.ref` values byte-for-byte suitable for
`journey list`. A `query:` endpoint in `journey list` performs a bounded search:
zero candidates is `location_not_found`; exactly one accepted in-area candidate
resolves; more than one is `location_ambiguous`. The ambiguity error includes
the same bounded candidate objects as `location list` and an exact retry
example using `place:` or `stop:`. Provider confidence may be reported but is
never used alone to silently select a winner. A `place:` reference is looked up
by stable Pelias gid; a missing/stale gid is `location_not_found`.

A `coord:` is accepted as the exact endpoint supplied by the caller and passed
directly to Routing v2. It never triggers reverse geocoding. Because no
municipality polygon contract has been verified, a bare coordinate has
`service_area: "unknown"`; the CLI neither claims it is inside nor rejects it.
`stop:` is resolved by Routing v2 and retains the HSL GTFS id.

`stop list` has two mutually exclusive forms. `--query` searches named stops. `--near` finds stops around an exact coordinate and requires no
query. Omitting both is a usage error.

## 4. Shared JSON protocol

Every JSON document is UTF-8, ends with one newline, uses snake_case keys, and
uses deterministic key and array ordering. Optional values are represented by
explicit `null` when the schema declares them nullable; fields are not omitted
because a provider happened not to return data. Additive fields may appear
without a schema bump. Removing/renaming fields, changing types/enums/meaning,
or changing nullability/order guarantees increments `schema_version`.

Successful stdout has this envelope:

```json
{
  "schema_version": 1,
  "data": {},
  "warnings": []
}
```

`warnings` is always present. Each warning is:

```json
{
  "code": "results_truncated",
  "message": "Provider returned more than 25 alerts; only 25 are included.",
  "details": {"returned": 25, "complete": false}
}
```

Failures produce no stdout. With `--json`, stderr contains exactly one error
object (unless `--verbose` was explicitly requested, in which case preceding
lines are diagnostic JSONL):

```json
{
  "schema_version": 1,
  "error": {
    "code": "invalid_datetime",
    "message": "Invalid --depart-at value '2026-09-08 09:30'; expected RFC 3339 with an explicit offset.",
    "invalid_value": "2026-09-08 09:30",
    "expected": "RFC3339 with explicit offset, for example 2026-09-08T09:30:00+03:00",
    "retryable": false,
    "details": {}
  }
}
```

In text mode the same message is one line on stderr prefixed `error: `.
Non-fatal text warnings go to stderr prefixed `warning: `. Provider body text,
GraphQL query text, HTTP headers, config secret values, and backtraces are not
printed by default.

### Exit classes

| Exit | Meaning | Representative codes |
|---:|---|---|
| 0 | success, including empty lists and doctor warnings | — |
| 1 | caller/domain-actionable | `usage_error`, `invalid_*`, `credential_missing`, `provider_authentication`, `location_ambiguous`, `location_not_found`, `stop_not_found`, `no_journeys`, `no_matching_journeys`, `unsupported_preference`, `config_conflict`, `skill_exists`, any completed doctor run containing FAIL |
| 2 | system/provider/internal | `io_error`, `network_error`, `provider_rate_limited`, `provider_http`, `provider_graphql`, `provider_contract`, `internal_error`; doctor infrastructure unable to execute/report checks |
| 130 | cancelled by SIGINT | `cancelled` |
| 143 | cancelled by SIGTERM | `cancelled` |

Clap usage failures are centrally remapped to 1. Help and version displays exit
0. All errors pass through one typed error-to-code/exit mapper.

### Diagnostics

`--verbose` writes one JSON object per line to stderr, never plain text. Stable
fields are `timestamp`, `level`, `event`, `component`, `request_id`, and
`message`; provider events may add `provider`, `operation`, `http_status`, and
`elapsed_ms`. The subscription key and all secret-shaped header values are
always `"<redacted>"`. Diagnostics do not change stdout.

### Shared provider metadata

Every provider-backed `data` object contains:

```json
{
  "request": {
    "request_id": "req_01J00000000000000000000000",
    "language": "en",
    "timezone": "Europe/Helsinki"
  },
  "source": {
    "provider": "digitransit",
    "dataset": "hsl",
    "product": "routing-v2-hsl-gtfs",
    "retrieved_at": "2026-09-08T06:56:40Z",
    "attribution": "© Digitransit; retrieved 2026-09-08T06:56:40Z",
    "licenses": ["CC-BY-4.0", "ODbL-1.0"],
    "realtime_included": true
  }
}
```

`request_id` is locally generated and injectable in tests; it is correlation,
not provider provenance. Geocoding uses product `geocoding-v1` and
`realtime_included: false`. Mixed journey resolution uses a top-level `sources`
array in first-use order. Endpoint URLs and credential presence are config and
diagnostic facts, not data provenance, and are not emitted in ordinary results.

## 5. Domain objects

These definitions are normative. JSON examples below may abbreviate arrays but
implementations and golden fixtures must include every required field.

### Location candidate / resolved location

```json
{
  "ref": "place:gtfshsl:station:GTFS:HSL:1000102",
  "kind": "stop",
  "id": "gtfshsl:station:GTFS:HSL:1000102",
  "label": "Kampin metroasema (Kamppi), Kamppi, Helsinki",
  "name": "Kampin metroasema",
  "locality": "Helsinki",
  "neighbourhood": "Kamppi",
  "postal_code": "00100",
  "coordinates": {"latitude": 60.168842, "longitude": 24.931199},
  "source": "gtfshsl",
  "source_layer": "station",
  "confidence": 1.0,
  "service_area": "inside",
  "modes": ["subway"]
}
```

Required keys are all keys shown. Nullable: `locality`, `neighbourhood`,
`postal_code`, `confidence`, and `id` for exact caller coordinates. `kind` is `address|venue|stop|locality|other`;
`service_area` is `inside|outside|unknown`. A verified HSL stop or a provider
locality among the nine supported municipalities is `inside`; an explicit
provider Finnish locality outside that set is `outside`; absent/conflicting
scope evidence is `unknown`. Bare coordinates are always `unknown` in v1.
`modes` is sorted in canonical mode order and may be empty. A resolved location
adds `input_ref` and `resolution`
(`exact_coordinate|stable_place|stable_stop|unique_query`) while retaining the
same fields. For `coord:`, `id` is null and `ref` is the normalized coordinate
reference.

### Alert

```json
{
  "id": "opaque-provider-alert-id",
  "header": "I and P trains will run less frequently than normal",
  "description": "…authoritative provider text…",
  "severity": "info",
  "effect": "detour",
  "valid_from": "2026-08-10T00:00:00+03:00",
  "valid_until": "2026-10-05T00:00:00+03:00",
  "entities": [
    {"kind": "route", "id": "HSL:3001I"}
  ],
  "source_feed": "HSL"
}
```

`id` preserves Routing v2's nondeprecated `Alert.id: ID!` as an opaque provider
identifier; do not invent a local hash. The id above is illustrative. Nullable:
`description`, `valid_from`, `valid_until`. Unknown provider enum values map to
`unknown`, with the original strings retained as nullable `source_severity` and
`source_effect` fields on the alert, never guessed into a known class.

### Realtime evidence

```json
{
  "state": "updated",
  "scheduled_time": "2026-09-08T09:52:00+03:00",
  "estimated_time": "2026-09-08T10:03:38+03:00",
  "delay_seconds": 698,
  "observed_realtime": true
}
```

`state` is `scheduled|updated|cancelled|added|unknown`. `estimated_time` and
`delay_seconds` are nullable. `observed_realtime` is true only when the provider
explicitly says an update applies. Since departure realtime values can fall back
to scheduled values, equality alone never proves realtime. `confidence` is not
a field in v1 because the provider supplies no verified confidence measure.

## 6. Data command contracts

### `location list`

`--kind` accepts `any|address|venue|stop`, default `any`. Results preserve
provider order and are capped by the requested limit. No candidates is a
successful empty result, not an error.

```console
$ reitti location list --query Kamppi --kind stop --limit 3
3 location candidates for “Kamppi” (Digitransit, retrieved 2026-09-08 06:56 UTC)
1  Kampin metroasema (Kamppi), Helsinki  subway  place:gtfshsl:station:GTFS:HSL:1000102
2  Kamppi, Kampinkuja 1, Helsinki         —       place:openstreetmap:station:node:1378007268
3  Kamppi (kaukoliikenneterminaali)       bus     place:gtfshsl:station:GTFS:HSL:1000015
Use a returned ref in `reitti journey list --from <ref> …`.
```

```json
{
  "schema_version": 1,
  "data": {
    "query": "Kamppi",
    "kind": "stop",
    "limit": 3,
    "count": 3,
    "complete": false,
    "candidates": [{"ref": "place:gtfshsl:station:GTFS:HSL:1000102", "kind": "stop", "id": "gtfshsl:station:GTFS:HSL:1000102", "label": "Kampin metroasema (Kamppi), Kamppi, Helsinki", "name": "Kampin metroasema", "locality": "Helsinki", "neighbourhood": "Kamppi", "postal_code": "00100", "coordinates": {"latitude": 60.168842, "longitude": 24.931199}, "source": "gtfshsl", "source_layer": "station", "confidence": 1.0, "service_area": "inside", "modes": ["subway"]}],
    "request": {"request_id": "req_01J00000000000000000000000", "language": "en", "timezone": "Europe/Helsinki"},
    "source": {"provider": "digitransit", "dataset": "hsl", "product": "geocoding-v1", "retrieved_at": "2026-09-08T06:56:40Z", "attribution": "© Digitransit; retrieved 2026-09-08T06:56:40Z", "licenses": ["CC-BY-4.0", "ODbL-1.0"], "realtime_included": false}
  },
  "warnings": []
}
```

`complete` is false whenever the provider indicates more candidates may exist or
reitti cannot prove completeness; it is never inferred solely from `count < limit`.

### `journey list`

Exactly zero or one of `--depart-at` and `--arrive-by` is accepted. With neither,
the request means depart at the injected current time and records
`time_source: "clock"`; with a flag it records `time_source: "argument"`.
There is no silent timezone conversion of argument timestamps. The default
configuration timezone is used for the no-time case and human presentation.

Alternative order is deterministic: Routing v2 edge order is the primary order;
if local deduplication removes byte-equivalent itineraries, survivors retain
source order. `alternative_id` is `alt-1`, `alt-2`, etc. and is stable only
within that response. The CLI returns every provider alternative up to `limit`;
it does not choose a single winner.

Comparison labels are derived only across returned alternatives:
`fastest`, `earliest_arrival`, `latest_departure` (arrive-by),
`fewest_transfers`, and `least_walking`. All tied alternatives receive a label.
Labels are sorted in that order and each fact includes the measured value and
tie count. There is no `recommended` label or opaque score.

```console
$ reitti journey list --from place:gtfshsl:station:GTFS:HSL:1000102 \
    --to coord:60.1776,24.6529 --depart-at 2026-09-08T09:30:00+03:00 --limit 2
2 alternatives · Kamppi → 60.1776,24.6529 · depart 09:30 EEST
1  10:02–10:45  42m55s  1 transfer   walk 10m56s  M1 → 531  earliest arrival
2  10:17–10:59  41m28s  1 transfer   walk 11m34s  M1 → 530  fastest

Alternative 1
  walk 1m24s to Kamppi
  M1 Kamppi 10:03 est (09:52 sched) → Matinkylä 10:21 est
  walk 5m23s to Matinkylä (M)
  531 Matinkylä (M) 10:30 sched → Puolarmäki 10:41 sched
  walk 4m09s to destination
Realtime is mixed: M1 updated; bus 531 scheduled only. No fare data available.
© Digitransit; retrieved 2026-09-08T06:56:40Z. Independent client; not affiliated with HSL or Digitransit.
```

Normative abbreviated JSON (all arrays are complete in actual output):

```json
{
  "schema_version": 1,
  "data": {
    "request": {
      "request_id": "req_01J00000000000000000000000",
      "language": "en",
      "timezone": "Europe/Helsinki",
      "from": "place:gtfshsl:station:GTFS:HSL:1000102",
      "to": "coord:60.1776,24.6529",
      "time": {"kind": "depart_at", "value": "2026-09-08T09:30:00+03:00", "time_source": "argument"},
      "modes": ["bus", "tram", "rail", "subway", "ferry"],
      "max_walk_m": null,
      "wheelchair": false,
      "include_geometry": false,
      "limit": 2
    },
    "resolved": {
      "from": {"input_ref": "place:gtfshsl:station:GTFS:HSL:1000102", "resolution": "stable_place", "ref": "place:gtfshsl:station:GTFS:HSL:1000102", "kind": "stop", "id": "gtfshsl:station:GTFS:HSL:1000102", "label": "Kampin metroasema", "name": "Kampin metroasema", "locality": "Helsinki", "neighbourhood": "Kamppi", "postal_code": "00100", "coordinates": {"latitude": 60.168842, "longitude": 24.931199}, "source": "gtfshsl", "source_layer": "station", "confidence": 1.0, "service_area": "inside", "modes": ["subway"]},
      "to": {"input_ref": "coord:60.1776,24.6529", "resolution": "exact_coordinate", "ref": "coord:60.1776,24.6529", "kind": "other", "id": null, "label": "60.1776,24.6529", "name": "60.1776,24.6529", "locality": null, "neighbourhood": null, "postal_code": null, "coordinates": {"latitude": 60.1776, "longitude": 24.6529}, "source": "caller", "source_layer": "coordinate", "confidence": null, "service_area": "unknown", "modes": []}
    },
    "count": 2,
    "complete": false,
    "alternatives": [
      {
        "id": "alt-1",
        "source_index": 0,
        "start_time": "2026-09-08T10:02:14+03:00",
        "end_time": "2026-09-08T10:45:09+03:00",
        "duration_seconds": 2575,
        "transfers": 1,
        "walk_seconds": 656,
        "wait_seconds": 174,
        "transit_seconds": 1745,
        "walk_distance_m": null,
        "accessibility": {"wheelchair_requested": false, "status": "unknown", "evidence": []},
        "realtime": {"status": "mixed", "updated_legs": 1, "scheduled_only_legs": 1, "cancelled_legs": 0},
        "comparison": [
          {"label": "earliest_arrival", "metric": "end_time", "value": "2026-09-08T10:45:09+03:00", "tied": 1}
        ],
        "alerts": [],
        "fare": null,
        "legs": [
          {
            "index": 0,
            "mode": "walk",
            "from": {"name": "Origin", "stop_ref": null, "platform": null, "coordinates": null},
            "to": {"name": "Kamppi", "stop_ref": null, "platform": null, "coordinates": null},
            "route": null,
            "trip_id": null,
            "headsign": null,
            "duration_seconds": 84,
            "distance_m": null,
            "start": {"state": "scheduled", "scheduled_time": "2026-09-08T10:02:14+03:00", "estimated_time": null, "delay_seconds": null, "observed_realtime": false},
            "end": {"state": "scheduled", "scheduled_time": "2026-09-08T10:03:38+03:00", "estimated_time": null, "delay_seconds": null, "observed_realtime": false},
            "intermediate_stops": [],
            "steps": [],
            "navigation_complete": true,
            "geometry": null,
            "alerts": [],
            "cancelled": false
          },
          {
            "index": 1,
            "mode": "subway",
            "from": {"name": "Kamppi", "stop_ref": null, "platform": null, "coordinates": null},
            "to": {"name": "Matinkylä", "stop_ref": null, "platform": null, "coordinates": null},
            "route": {"id": "HSL:31M1", "short_name": "M1", "long_name": null},
            "trip_id": null,
            "headsign": null,
            "duration_seconds": 1085,
            "distance_m": null,
            "start": {"state": "updated", "scheduled_time": "2026-09-08T09:52:00+03:00", "estimated_time": "2026-09-08T10:03:38+03:00", "delay_seconds": 698, "observed_realtime": true},
            "end": {"state": "updated", "scheduled_time": "2026-09-08T10:11:00+03:00", "estimated_time": "2026-09-08T10:21:43+03:00", "delay_seconds": 643, "observed_realtime": true},
            "intermediate_stops": [],
            "steps": [],
            "navigation_complete": true,
            "geometry": null,
            "alerts": [],
            "cancelled": false
          }
        ]
      }
    ],
    "sources": [{"provider": "digitransit", "dataset": "hsl", "product": "routing-v2-hsl-gtfs", "retrieved_at": "2026-09-08T06:56:40Z", "attribution": "© Digitransit; retrieved 2026-09-08T06:56:40Z", "licenses": ["CC-BY-4.0", "ODbL-1.0"], "realtime_included": true}]
  },
  "warnings": [{"code": "fare_unavailable", "message": "Routing v2 returned no verified fare data; fare is null.", "details": {}}]
}
```

Request and preserve the itinerary's authoritative `numberOfTransfers`,
`duration`, `waitingTime`, `walkTime`, and `walkDistance` as normalized summary
metrics. In particular, do not count transit legs minus one: the provider
excludes stay-seated/interlined continuations from `numberOfTransfers`. Request
`Leg.interlineWithPreviousLeg` and expose `continues_previous_vehicle` for
navigation. For legacy fixtures lacking optional duration metrics, derive them
only from complete consistent legs and label that derivation in metadata;
missing evidence remains null. Comparisons use normalized seconds and metres. The Routing v2 query must request all
schema-supported navigation facts: endpoint stop IDs and coordinates, route and
trip IDs, headsign, platform, distance, intermediate stops, walking steps,
accessibility evidence, alert links, and scheduled/estimated times. Null/empty
means upstream absence or a schema-confirmed unsupported field, never that the
client chose not to query a useful small field. Read intermediate transit calls
from nondeprecated `Leg.stopCalls`; do not query deprecated `intermediateStops`
or `intermediatePlaces`. Preserve raw walking directions and `bogusName` so a
generated street name is not presented as an authoritative one.

`--max-walk-m` is an explicit **local filter** on total walking distance among
the bounded provider alternatives, not an upstream search parameter. The current
`WalkPreferencesInput` has no maximum-distance field. Do not synthesize one.
Missing total walking distance cannot prove the cap, so that candidate is
excluded with an explanation. Return the filtering counts and `complete: false`
when filtering discards candidates. If none remain, return
`no_matching_journeys` (exit 1), explaining that none of the returned candidates
met the cap; other routes may exist. Do not describe this as proof that no route
exists. Apply comparison labels after filtering.

Geometry is the bounded exception: `--include-geometry` asks Routing v2 for leg
geometry and emits GeoJSON LineString coordinates `[longitude, latitude]`.
Without it `geometry` is null. Walking legs contain `steps`, always requested,
with rows `{instruction,distance_m,street_name,relative_direction,coordinates}`;
each nullable field reflects source absence. The adapter caps walking steps at 200 per leg. Geometry is all-or-nothing and
capped at 10,000 decoded points across the response: if the source exceeds that
cap, geometry is omitted and `navigation_complete: false` plus
`navigation_truncated` are emitted rather than serving a misleading truncated
line. A walking-step overflow retains the first 200 in source order, marks the
same incomplete state, and warns explicitly.

Zero alternatives is `no_journeys` exit 1 and includes the resolved endpoints
and retry suggestions; it is not success with a fabricated fallback. A GraphQL
`errors` array is exit 2 even with HTTP 200. Partial GraphQL data plus errors is
not served in v1 because completeness cannot be established.

### `stop list`

```console
$ reitti stop list --near 60.1699,24.9384 --radius-m 500 --limit 2
2 stops within 500 m of 60.1699,24.9384
HSL:1020453  Päärautatieasema  tram  310 m  stop:HSL:1020453
HSL:1000102  Kamppi            subway 470 m  stop:HSL:1000102
```

```json
{
  "schema_version": 1,
  "data": {
    "search": {"kind": "near", "query": null, "coordinates": {"latitude": 60.1699, "longitude": 24.9384}, "radius_m": 500, "limit": 2},
    "count": 2,
    "complete": false,
    "stops": [{"ref": "stop:HSL:1020453", "id": "HSL:1020453", "name": "Päärautatieasema", "code": null, "coordinates": {"latitude": 60.1702, "longitude": 24.9391}, "distance_m": 310, "modes": ["tram"], "wheelchair_boarding": "unknown", "service_area": "inside"}],
    "request": {"request_id": "req_01J00000000000000000000000", "language": "en", "timezone": "Europe/Helsinki"},
    "source": {"provider": "digitransit", "dataset": "hsl", "product": "routing-v2-hsl-gtfs", "retrieved_at": "2026-09-08T06:56:40Z", "attribution": "© Digitransit; retrieved 2026-09-08T06:56:40Z", "licenses": ["CC-BY-4.0", "ODbL-1.0"], "realtime_included": true}
  },
  "warnings": []
}
```

For named search, `search.kind` is `query`, `query` is non-null, coordinates and
radius are null. `wheelchair_boarding` is
`accessible|not_accessible|unknown`; source unknown never becomes accessible.
No matches is successful with `stops: []`.

### `departure list`

`--at` defaults to the injected current time. Departures are in ascending
operational departure time, then trip id. Routing `serviceDay` is combined with
seconds-after-service-day before output. `scheduledDeparture` and
`realtimeDeparture` are never interpreted as Unix timestamps by themselves.

```console
$ reitti departure list --stop HSL:1020453 --at 2026-09-08T09:55:00+03:00 --limit 3
Päärautatieasema · next 3 departures from 09:55 EEST
09:57 est  3  Kuusitie via Kallio  37s early  updated
09:58 est  5  Katajanokan term.    44s early  updated
10:01 est  9  Ilmala via Kallio    23s early  updated
© Digitransit; retrieved 2026-09-08T06:56:40Z
```

```json
{
  "schema_version": 1,
  "data": {
    "stop": {"ref": "stop:HSL:1020453", "id": "HSL:1020453", "name": "Päärautatieasema", "code": null, "coordinates": null, "modes": ["tram"], "wheelchair_boarding": "unknown", "service_area": "inside"},
    "at": "2026-09-08T09:55:00+03:00",
    "time_source": "argument",
    "window_seconds": 7200,
    "count": 3,
    "complete": false,
    "departures": [{"trip_id": "HSL:1003_20260907_Ti_1_0943", "route": {"id": null, "short_name": "3", "long_name": null}, "headsign": "Kuusitie via Kallio", "platform": null, "service_date": "2026-09-08", "departure": {"state": "updated", "scheduled_time": "2026-09-08T09:58:00+03:00", "estimated_time": "2026-09-08T09:57:23+03:00", "delay_seconds": -37, "observed_realtime": true}, "cancelled": false, "alerts": []}],
    "request": {"request_id": "req_01J00000000000000000000000", "language": "en", "timezone": "Europe/Helsinki"},
    "source": {"provider": "digitransit", "dataset": "hsl", "product": "routing-v2-hsl-gtfs", "retrieved_at": "2026-09-08T06:56:40Z", "attribution": "© Digitransit; retrieved 2026-09-08T06:56:40Z", "licenses": ["CC-BY-4.0", "ODbL-1.0"], "realtime_included": true}
  },
  "warnings": []
}
```

A known stop with no departures is successful and empty. Unknown stop is
`stop_not_found`. Cancellation is true only from explicit Routing v2 evidence;
a missing trip is not called cancelled.

### `alert list`

Supplied route and stop references form one relevance union, and applicable
feed-wide alerts are also retained. For example, with both a route and stop
filter, an alert affecting only that route remains relevant; there is no
cross-type AND that hides it. Repeated identical filters are rejected.
An alert whose entity scope is absent or unknown is conservatively retained as
possibly feed-wide with an `alert_scope_unknown` warning, never silently
dropped. `--active-at` defaults to the injected current time. An alert with an
unknown validity boundary is retained and marked by a warning; the CLI does not
infer inactivity. Sort by severity
(`severe`, `warning`, `info`, `unknown`), then start time (null last), then id.

```console
$ reitti alert list --route HSL:31M1 --active-at 2026-09-08T09:00:00+03:00
1 alert for route HSL:31M1 at 09:00 EEST
WARNING  Lines Metro and Metro, possibly delayed  valid 08:18–10:00
  Technical failure. Estimated duration: 8:18–10:00.
© Digitransit; retrieved 2026-09-08T06:56:40Z
```

```json
{
  "schema_version": 1,
  "data": {
    "filters": {"routes": ["HSL:31M1"], "stops": [], "active_at": "2026-09-08T09:00:00+03:00", "time_source": "argument", "limit": 25},
    "count": 1,
    "complete": false,
    "alerts": [{"id": "opaque-provider-alert-id", "header": "Lines Metro and Metro, Possibly delayed, 8:18 - 10:00", "description": "Metros: Metro. Possibly delayed. Reason: Technical failure. Estimated duration: 8:18 - 10:00", "severity": "warning", "effect": "other_effect", "valid_from": "2026-09-08T08:18:00+03:00", "valid_until": "2026-09-08T10:00:00+03:00", "entities": [{"kind": "route", "id": "HSL:31M1"}], "source_feed": "HSL"}],
    "request": {"request_id": "req_01J00000000000000000000000", "language": "en", "timezone": "Europe/Helsinki"},
    "source": {"provider": "digitransit", "dataset": "hsl", "product": "routing-v2-hsl-gtfs", "retrieved_at": "2026-09-08T06:56:40Z", "attribution": "© Digitransit; retrieved 2026-09-08T06:56:40Z", "licenses": ["CC-BY-4.0", "ODbL-1.0"], "realtime_included": true}
  },
  "warnings": []
}
```

An empty list means no matching alerts were returned; it does **not** guarantee
normal service. `complete` is false unless the client can prove the provider
response was exhaustive. Global cancelled-trip enumeration is deliberately not
a command: validation proved it can be large and cursor-paginated, while journey
and departure commands can expose relevant explicit cancellation evidence.

## 7. Configuration and credential onboarding

There is no repository/data home. `reitti` is stateless except for user config,
so canon data-root discovery and `fmt` do not apply.

The config file is TOML at:

- `$XDG_CONFIG_HOME/reitti/config.toml` when `XDG_CONFIG_HOME` is set to an
  absolute path;
- otherwise `$HOME/.config/reitti/config.toml` on Unix (including macOS).

`REITTI_CONFIG_FILE` may select an explicit absolute config path. Relative paths,
an empty XDG variable, insecure file type, and symlinked credential files are
rejected rather than guessed through. Parent directory and created file modes
are 0700 and 0600 on Unix. Writes are lock-protected and atomic.

Persistent key precedence is resolved independently:

```text
explicit non-secret global flag > REITTI_* environment > config file > built-in default
```

| Key | Flag | Environment | Default | Secret |
|---|---|---|---|---|
| subscription key | none | `DIGITRANSIT_SUBSCRIPTION_KEY` | absent | yes |
| language | `--language` on data commands | `REITTI_LANGUAGE` | `en` | no |
| timezone | none | `REITTI_TIMEZONE` | `Europe/Helsinki` | no |
| routing URL | `--routing-url` | `REITTI_ROUTING_URL` | verified Routing v2 HSL URL | no |
| geocoding URL | `--geocoding-url` | `REITTI_GEOCODING_URL` | verified Geocoding v1 base URL | no |
| connect timeout | `--connect-timeout` | `REITTI_CONNECT_TIMEOUT` | `5s` | no |
| request timeout | `--request-timeout` | `REITTI_REQUEST_TIMEOUT` | `20s` | no |
| private markers | none | `REITTI_PRIVATE_MARKERS` (JSON array) | empty | sensitive |

The provider subscription key deliberately has **no argv flag**. Recommended
ephemeral onboarding is to inject `DIGITRANSIT_SUBSCRIPTION_KEY` through a
secret-aware process environment. Persistent onboarding is:

```console
$ secret-tool lookup service digitransit | reitti config update --subscription-key-stdin
Updated /home/alex/.config/reitti/config.toml
subscription_key: <redacted> (source: stdin)
```

`--subscription-key-stdin` reads exactly one UTF-8 line, strips only the final
line ending, rejects empty/whitespace, NUL, or additional lines, and never echoes
the value. It is mutually exclusive with stdin-consuming future options. It
stores the key in the mode-0600 config file. Shell history, process argv,
ordinary output, error details, `--verbose`, dry-run plans, tests, and fixtures
never contain the value. `config update --dry-run --subscription-key-stdin`
validates stdin and reports only `"value":"<redacted>"`.

`config update` is selective and idempotent. At least one update must be named.
Lists replace in full. Endpoint overrides must be HTTPS; HTTP is rejected. The
built-in endpoint defaults are public Digitransit coordinates, not a user's
private environment.

```console
$ reitti config path
/home/alex/.config/reitti/config.toml (not created)

$ reitti config show
subscription_key  <redacted>                                      env
language          en                                              default
timezone          Europe/Helsinki                                 default
routing_url       https://api.digitransit.fi/routing/v2/hsl/gtfs/v1 default
geocoding_url     https://api.digitransit.fi/geocoding/v1          default
connect_timeout   5s                                             default
request_timeout   20s                                            default
```

`config path --json` data is `{ "path": <absolute>, "exists": <bool>,
"source": "env|xdg|default" }`. `config show --json` contains `path` and
`values`, each value shaped as `{ "value": ..., "source":
"flag|env|file|default", "secret": <bool> }`. Secrets and sensitive marker
lists are `<redacted>` by default. `--show-secrets` is allowed only on
`config show`, writes a conspicuous text warning to stderr or a structured
warning in JSON stdout, and never alters verbose diagnostics. It exists for
canon compliance but companion-skill guidance must not use it.

Missing credential on provider commands is `credential_missing` exit 1, and a
provider HTTP 401 is `provider_authentication` exit 1. Both include the safe fix:
set `DIGITRANSIT_SUBSCRIPTION_KEY` or pipe the key to
`reitti config update --subscription-key-stdin`. It never includes the rejected
value. Config path/show/version/schema/skill work without credentials.

## 8. Schema, version, help, doctor, and skill

### Schema discovery

`schema list` returns the stable names:

```text
location-list
journey-list
stop-list
departure-list
alert-list
config-path
config-show
config-update
schema-list
schema-show
version
doctor
skill-list
skill-print
skill-install
error
help
```

`schema show <name>` prints the bundled JSON Schema 2020-12 document for the
**unwrapped `data` object**, except `error` and `help`, which describe their
complete documents. Under `--json`, data is `{name, dialect, schema}`. Unknown
name is `schema_not_found` exit 1 with accepted names. Schemas use stable, location-independent `$id` values
`urn:reitti:schema:v1:<name>`, for example
`urn:reitti:schema:v1:journey-list`. They are identifiers, not fetchable URLs.

### Version

`reitti version`, `reitti --version`, `reitti --version --json`,
`reitti --json --version`, and `reitti version --json` obey the canon's
byte-identity rules for equivalent modes.

```json
{
  "schema_version": 1,
  "data": {
    "version": "1.0.0",
    "commit": "0123456789abcdef0123456789abcdef01234567",
    "build_provenance": {"kind": "git", "note": "git source tree"},
    "schema_version": 1,
    "supported_schemas": [1],
    "skills": [{"name": "reitti", "cli_version": "1.0.0", "schema_version": 1}]
  },
  "warnings": []
}
```

A no-git release build uses `commit: null` only with a truthful
`build_provenance.kind` of `tarball`, `vendored`, or `ci-injected`; a git build
that fails to stamp a full 40-hex SHA fails its release gate.

### Structured help

Every command path supports both orderings of `--help --json` and emits:

```json
{
  "schema_version": 1,
  "data": {
    "path": ["journey", "list"],
    "summary": "Plan and compare bounded journey alternatives.",
    "usage": "reitti [GLOBAL] journey list --from <LOCATION_REF> --to <LOCATION_REF> …",
    "args": [],
    "flags": [{"name": "--from", "required": true, "repeatable": false, "value_name": "LOCATION_REF", "possible_values": [], "default": null, "env": null, "global": false, "hidden": false, "deprecated": null}],
    "subcommands": [],
    "exit_codes": [{"code": 0, "meaning": "success"}, {"code": 1, "meaning": "caller/domain-actionable error"}, {"code": 2, "meaning": "system/provider/internal error"}, {"code": 130, "meaning": "SIGINT cancellation"}, {"code": 143, "meaning": "SIGTERM cancellation"}],
    "examples": [{"description": "Plan from a selected place for an explicit departure time", "argv": ["reitti", "--json", "journey", "list", "--from", "place:gtfshsl:station:GTFS:HSL:1000102", "--to", "coord:60.1776,24.6529", "--depart-at", "2026-09-08T09:30:00+03:00"]}]
  },
  "warnings": []
}
```

Top-level text help lists only command groups and global flags. Drill-down help
lists validation ranges, config environment mappings, exits, and at least one
copy-pasteable example. Required examples for the command paths are:

```text
location list: reitti --json location list --query Kamppi --kind stop --limit 5
journey list: reitti --json journey list --from place:… --to stop:HSL:1020453 --arrive-by 2026-09-08T10:00:00+03:00
stop list: reitti --json stop list --near 60.1699,24.9384 --radius-m 500 --limit 5
departure list: reitti --json departure list --stop HSL:1020453 --at 2026-09-08T09:55:00+03:00 --limit 10
alert list: reitti --json alert list --route HSL:31M1 --stop HSL:1020453 --active-at 2026-09-08T09:00:00+03:00
config path: reitti --json config path
config show: reitti --json config show
config update: printf '%s\n' "$DIGITRANSIT_KEY" | reitti --json config update --subscription-key-stdin
schema list: reitti --json schema list
schema show: reitti --json schema show journey-list
version: reitti --json version
doctor: reitti --json doctor --online
skill list: reitti --json skill list
skill print: reitti skill print reitti
skill install: reitti --json skill install reitti --agent all --dry-run
```

Documentation uses a fictional environment variable in the one stdin example,
never a literal secret.

### Doctor

`doctor` is diagnostic-only, read-only, offline by default, and runs stable
local checks in this order:

1. `config.path` — resolvable absolute XDG/config path and secure file type/mode;
2. `config.values` — every configured value parses and no secret appears in a
   non-secret key;
3. `credential.subscription_key` — key resolves, reported only as present/absent;
4. `skill.sync` — installed skills found under supported runtime layouts match;
5. `public_artifacts.private_markers` — exact case-insensitive scan of bundled
   text against configured `private_markers`, exempting this project's eventual
   derived public coordinates;
6. `build.provenance` — commit/provenance shape is valid.

Only `--online` adds, in this order, `provider.geocoding` (one minimal
authenticated request) and `provider.routing_v2` (one minimal authenticated
GraphQL request and envelope check). Troubleshooting never spends quota or
requires network unless the caller opts in.

No key is a local FAIL with a safe suggestion; offline doctor still completes
all other checks. Missing `private_markers` is WARN naming
`REITTI_PRIVATE_MARKERS`; it does not invent private identities. HTTP 401,
403, or 429 is an online FAIL without a quota claim. Any completed doctor run
with one or more FAIL checks exits 1 and still emits all checks. Exit 2 is
reserved for infrastructure failure that prevents doctor from executing or
serializing its check report. Each JSON check is
`{id,status,message,fix_suggestion,details}` where status is `ok|warn|fail`;
nullable `fix_suggestion` is always present. Summary is `{ok,warn,fail}`.

Text is one fixed `OK|WARN|FAIL  <id>  <message>` line per check followed by
`summary: N ok, M warn, K fail`. There is no `--fix`: configuration mutation
belongs to explicit `config update`, and skill replacement belongs to explicit
`skill install --force`. This diagnostic-only design avoids an incidental
mutation path.

### Support-command JSON payloads

The following are the exact `data` shapes omitted from the domain examples.
All still use the common `{schema_version,data,warnings}` envelope.

```json
{
  "config_path": {"path": "/home/alex/.config/reitti/config.toml", "exists": false, "source": "default"},
  "config_show": {
    "path": "/home/alex/.config/reitti/config.toml",
    "values": {
      "subscription_key": {"value": "<redacted>", "source": "env", "secret": true},
      "language": {"value": "en", "source": "default", "secret": false},
      "timezone": {"value": "Europe/Helsinki", "source": "default", "secret": false},
      "routing_url": {"value": "https://api.digitransit.fi/routing/v2/hsl/gtfs/v1", "source": "default", "secret": false},
      "geocoding_url": {"value": "https://api.digitransit.fi/geocoding/v1", "source": "default", "secret": false},
      "connect_timeout": {"value": "5s", "source": "default", "secret": false},
      "request_timeout": {"value": "20s", "source": "default", "secret": false},
      "private_markers": {"value": "<redacted>", "source": "default", "secret": true}
    }
  },
  "config_update": {
    "path": "/home/alex/.config/reitti/config.toml",
    "updated": ["subscription_key"],
    "unchanged": [],
    "values": {"subscription_key": {"value": "<redacted>", "source": "file", "secret": true}}
  },
  "schema_list": {"schemas": [{"name": "journey-list", "schema_version": 1}]},
  "schema_show": {"name": "journey-list", "dialect": "https://json-schema.org/draft/2020-12/schema", "schema": {}},
  "doctor": {
    "online": false,
    "checks": [{"id": "config.path", "status": "ok", "message": "Config path is secure and readable.", "fix_suggestion": null, "details": {}}],
    "summary": {"ok": 1, "warn": 0, "fail": 0}
  },
  "skill_list": {
    "skills": [{"name": "reitti", "description": "Plan and explain HSL-area journeys with reitti.", "cli_version": "1.0.0", "schema_version": 1}],
    "supported_agents": ["claude", "pi", "codex"],
    "install": {"selection_flag": "--agent", "default": "all", "accepted_values": ["claude", "pi", "codex", "all"], "target_flag": "--target", "dry_run_flag": "--dry-run", "force_flag": "--force", "interactive": false, "no_clobber_default": true, "overwrite_requires_force": true, "layouts": []}
  },
  "skill_print": {"name": "reitti", "cli_version": "1.0.0", "schema_version_skill": 1, "content": "---\nname: reitti\n…", "path_in_repo": "skills/reitti/SKILL.md", "resources": ["SKILL.md"]},
  "skill_install": {"name": "reitti", "agent": "all", "installed": [".claude/skills/reitti/SKILL.md", ".pi/agent/skills/reitti/SKILL.md", ".codex/skills/reitti/SKILL.md"], "existed": [], "skipped": []}
}
```

The object above is a compact shape catalogue, not one command's response.
`config_update` adds `dry_run: true` and `would` instead of `updated` on a dry
run. Skill install dry-run uses the shared top-level planning envelope and
performs no writes. `schema_show.schema` is the complete bundled schema rather than the
empty illustrative object above.

### Companion skill

Exactly one bundled Agent Skill is named `reitti`. Its trigger description says
it is for HSL-area place resolution, journey comparison, arrivals/departures,
and alerts. It teaches this safe workflow:

1. inspect `version --json` and run `doctor --json` after unexplained failures;
2. search and show candidates to the user when the endpoint is ambiguous;
3. retry with returned tagged references;
4. pass explicit RFC-3339 times for reproducible plans;
5. compare all returned alternatives using source facts and deterministic labels;
6. describe realtime as updated/scheduled/unknown, never as confidence;
7. state when fare/accessibility/availability is null or unknown;
8. preserve Digitransit attribution in downstream summaries.

`skill list --json` declares `supported_agents: ["claude","pi","codex"]` and:

```json
{
  "selection_flag": "--agent",
  "default": "all",
  "accepted_values": ["claude", "pi", "codex", "all"],
  "target_flag": "--target",
  "dry_run_flag": "--dry-run",
  "force_flag": "--force",
  "interactive": false,
  "no_clobber_default": true,
  "overwrite_requires_force": true,
  "layouts": [
    {"agent": "claude", "path": ".claude/skills/<name>/...", "form": "agent-skills-tree"},
    {"agent": "pi", "path": ".pi/agent/skills/<name>/...", "form": "agent-skills-tree"},
    {"agent": "codex", "path": ".codex/skills/<name>/...", "form": "agent-skills-tree"}
  ]
}
```

No selector and `--agent all` both install all three native trees. `--target`
changes only the base. Install is atomic, no-clobber by default, and supports a
truthful planning envelope. `skill print reitti` is byte-identical to bundled
`SKILL.md`; `--resource` prints another bundled support file. JSON print data is
`{name,cli_version,schema_version_skill,content,path_in_repo,resources}`. An
older installed skill is replaced with a warning; a newer one needs `--force`.
Frontmatter description is at most 1024 characters and carries matching
`cli_version` and `schema_version`.

## 9. Implementation seams

Use the existing workspace boundary rather than embedding clap or HTTP in domain
logic:

```text
reitti-core
  model/       Location, Stop, Journey, Leg, Departure, Alert, provenance
  reference/   strict tagged-reference and scalar parsing
  compare/     pure duration aggregation, tie labels, stable ordering
  resolve/     provider-neutral resolution decisions and ambiguity errors
  time/        Clock trait, service-day arithmetic, Helsinki display
  contract/    serde DTOs and schema-v1 projection

reitti-cli
  command/     clap command DTOs only
  config/      XDG resolution, secret-aware selective atomic update
  client/      Provider trait adapters; Digitransit HTTP implementation
  render/      deterministic text and JSON envelopes
  support/     help, schema, version, doctor, bundled skill
```

Core ports are async-capable traits but domain types do not depend on reqwest or
GraphQL generated types:

```rust
trait Geocoder {
    async fn search(&self, request: LocationSearchRequest) -> Result<Vec<LocationCandidate>, ProviderError>;
    async fn place(&self, id: &PlaceId, language: Language) -> Result<Option<LocationCandidate>, ProviderError>;
}
trait Router {
    async fn plan(&self, request: PlanRequest) -> Result<PlanResult, ProviderError>;
    async fn stops(&self, request: StopSearchRequest) -> Result<Vec<Stop>, ProviderError>;
    async fn departures(&self, request: DepartureRequest) -> Result<Option<DepartureBoard>, ProviderError>;
    async fn alerts(&self, request: AlertRequest) -> Result<Vec<Alert>, ProviderError>;
}
trait Clock { fn now(&self) -> DateTime<Utc>; }
trait RequestIdGenerator { fn next(&self) -> RequestId; }
```

The Digitransit adapter is the only layer that knows GraphQL field names,
HTTP headers, service-day seconds, or Pelias GeoJSON. The CLI orchestration owns
the three-call budget. Renderers consume schema DTOs, not wire responses. This
lets location, live, and journey sibling issues land independently without
rewriting the CLI or provider client.

## 10. Determinism and test contract

`--frozen-time` sets the injected clock for one invocation. Tests also inject a
request id generator and provider retrieval timestamp. Production never uses a
frozen clock to assert provider freshness. Stable serialization requires:

- structs or insertion-ordered maps, never hash iteration;
- explicit enum order and documented sorts above;
- provider-order preservation where specified;
- UTC-independent fixture locale and `Europe/Helsinki` timezone data;
- integer normalized durations and canonical coordinate formatting;
- final newline in all output and no ANSI/color output.

Required test layers:

1. parser table tests for every accepted/rejected scalar and mutual exclusion;
2. pure core tests for ambiguity, bounds, comparisons/ties, service-day
   arithmetic, DST boundaries, realtime fallback, and null preservation;
3. adapter contract tests replaying every credential-free fixture under
   `tests/fixtures/digitransit/`, including HTTP-200 GraphQL errors, nullable
   stop, removed Routing v1, pagination evidence, ambiguity, cancellations;
4. golden text and JSON tests for every v1 command, empty result, warning, and
   error class with frozen time/request ids;
5. config tests in disposable HOME/XDG roots for precedence per key, modes,
   symlink rejection, atomic writes, redaction, stdin handling, and no secret in
   captured argv/stdout/stderr;
6. mock-server integration tests asserting exact maximum request counts,
   operation names, variables, headers, timeout behavior, one attempt with no
   transparent retries, and no
   direct GTFS-RT/Routing Data/Map calls;
7. structured-help/schema snapshots and JSON Schema validation of every golden;
8. skill byte-identity/layout/drift/no-clobber tests for Claude, pi, and Codex;
9. end-to-end smoke tests for search→select→plan and search-stop→departures,
   including actionable ambiguity retry.

Repository gates for every implementation issue are:

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
python3 scripts/probe-digitransit.py check
issuectl doctor --json
```

Live tests are opt-in, credential-env-only, request-capped, and never update
goldens automatically.

## 11. Canon decisions and explicit non-applicability

| Canon | v1 decision |
|---|---|
| §1–§5 | strict parsing, structured channels, no prompts, actionable details, stdout/file composition |
| §6–§7 | noun-verb surface and canonical CRUD; no synonyms or declarative apply |
| §8 | per-key flag > env > file > neutral default; mandatory path/show; no data root because the CLI owns no records |
| §9–§10 | flag-selected deterministic format, schema 1 envelopes, real build provenance, warnings in stdout JSON |
| §11 | `config update` and `skill install` have dry-run and idempotent retry-safe behavior; provider commands are read-only |
| §12–§13 | no long-running or unbounded-inline v1 command; strict caps make streaming/pagination/export unnecessary |
| §14–§18 | structured drill-down help, native three-agent skill, version sync, diagnostic-only offline doctor with explicit online probes; repair stays on owning commands |
| §19 | injectable clock and hidden frozen-time; injected ids/timestamps complete deterministic tests |
| §20–§21 | no owned record tree to format or bootstrap; selective XDG config update replaces init |
| §22 | existing `reitti-core` / `reitti-cli` split retained |
| §23 | neutral public defaults/fixtures and configurable exact-marker doctor scan |
| §24 | Routing v1 and excluded APIs were reverified by validation; no inherited unresolved blocker remains in this design |

## 12. Implementation completion checklist

A sibling issue may call its slice contract-complete only when:

- its grammar and structured help exactly match this document;
- its output validates against the bundled schema-v1 document;
- source facts, local derivations, nulls, and warnings remain distinguishable;
- provider requests satisfy the per-command cap and Routing v2 behavior;
- human output has a frozen golden and enough detail to act;
- all errors use the central mapper and preserve clean stdout/stderr;
- no credential appears in argv, output, diagnostics, fixtures, or snapshots;
- all four repository gates pass.
