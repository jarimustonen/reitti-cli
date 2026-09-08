# Digitransit and HSL data-contract validation

**Retrieved and probed:** 2026-09-08, 06:47–06:57 UTC. Documentation source was also checked at `HSLdevcom/digitransit-site` commit [`39c7c1b`](https://github.com/HSLdevcom/digitransit-site/commit/39c7c1be11831a6d10fcbd92a2f83e2060fae5ce). No credential value or credential-bearing URL was retained.

## Decision

`reitti` v1 should target the **Routing v2 HSL GTFS** endpoint:

```text
POST https://api.digitransit.fi/routing/v2/hsl/gtfs/v1
Content-Type: application/json
Accept-Language: en | fi | sv
digitransit-subscription-key: <value from DIGITRANSIT_SUBSCRIPTION_KEY>
```

The header name is `digitransit-subscription-key`; its value must never be sent in a URL or logged. `application/graphql` is also accepted, but `application/json` supports operation names and variables and is the v1 client choice. The endpoint is POST-only; current documentation says another method returns 404 and a missing content type returns 415.

The names in the portal describe different layers:

- **Routing v2 HSL GTFS** is the current online journey-planning/timetable product. `v2` identifies the newer Digitransit/OTP routing deployment. The trailing `gtfs/v1` identifies its exposed GTFS GraphQL API, which current architecture documentation explicitly says is newer than the GraphQL API formerly served at `/routing/v1` despite the inner API version being `v1`.
- **Routing v1** is the removed predecessor. The old HSL GraphQL URL returned HTTP 404 with the explicit body “This endpoint has been removed. v2 Routing endpoint has replaced this endpoint.” A portal definition named “Routing v1 - v1” therefore does not establish production suitability.
- **Routing Data v3** is a file/artifact service, not a GraphQL journey API. It superseded **Routing Data v2**, not Routing v1. Its HSL index exposes GTFS archives, OSM PBF, OTP graph/configuration files, and build artifacts used to build or run OTP. It is useful for a future local/offline router, but is not part of `reitti` v1 runtime requests.

This distinction is supported independently by the [current Routing API page](https://digitransit.fi/en/developers/apis/1-routing-api/), the [Routing API architecture page](https://digitransit.fi/en/developers/architecture/x-apis/1-routing-api/), the [Routing Data API page](https://digitransit.fi/en/developers/apis/2-routing-data-api/), and the [deprecation list](https://digitransit.fi/en/developers/deprecations/).

## Source map for v1

| Product need | Source | Current endpoint | v1 use |
|---|---|---|---|
| Free-text place/address candidates | Digitransit Geocoding v1 (Pelias) | `GET https://api.digitransit.fi/geocoding/v1/search` | Yes; retain all plausible candidates rather than silently selecting the first. |
| Type-ahead candidates | Digitransit Geocoding v1 | `GET …/autocomplete` | Contract verified; optional UI behavior, not required by the non-interactive v1 CLI. |
| Stable candidate lookup | Digitransit Geocoding v1 | `GET …/place?ids=<Pelias gid>` | Yes for retrying a selected candidate. |
| Coordinate lookup | Digitransit Geocoding v1 | `GET …/reverse?point.lat=<lat>&point.lon=<lon>` | Yes when resolving coordinates to source labels. |
| Journeys, routes, stops and departures | Routing v2 HSL GTFS GraphQL | `POST https://api.digitransit.fi/routing/v2/hsl/gtfs/v1` | Yes; primary online source. |
| Alerts and cancellation state | Routing v2 HSL GTFS GraphQL | Same endpoint (`alerts`, `canceledTrips`, realtime fields) | Yes; the router already incorporates HSL GTFS-RT. |
| OTP/GTFS/OSM/build artifacts | Routing Data v3 | `GET https://api.digitransit.fi/routing-data/v3/hsl/` | No runtime use in v1; explicitly deferred with the epic's post-v1 local-router direction. |
| Raw alerts, trip updates, vehicle positions | HSL GTFS-RT 2.0 | `GET https://realtime.hsl.fi/realtime/{service-alerts,trip-updates,vehicle-positions}/v2/hsl` | No direct v1 call. Alerts and trip updates would duplicate Routing API data; vehicle positions are outside v1. |
| Map tiles | Map v3 | `https://cdn.digitransit.fi/map/v3/…` | Excluded: a headless journey CLI does not need map tiles. |

The Geocoding API's base `/geocoding/v1` URL itself returned 404; an operation suffix is required. The old prose URL `https://digitransit.fi/en/developers/apis/2-geocoding-api/` also returned 404. Its current authoritative replacement is [Geocoding API (`3-geocoding-api`)](https://digitransit.fi/en/developers/apis/3-geocoding-api/), backed by the maintained [`digitransit-site` source](https://github.com/HSLdevcom/digitransit-site/tree/master/src/pages/en/developers/apis/3-geocoding-api). The `3` is the documentation navigation order, while `/geocoding/v1` is the API version.

## Live probe results

The committed fixtures under `tests/fixtures/digitransit/` are selected, credential-free responses from the probe window above.

| Case | Result and contract evidence |
|---|---|
| Route lookup | HTTP 200; tram route query returned `gtfsId`, short/long names and mode. |
| Journey planning | HTTP 200; `planConnection` returned two alternatives with ISO-8601 start/end values, scheduled/estimated leg times, durations, modes, route IDs and realtime states. |
| Stop lookup | HTTP 200; returned stable HSL GTFS ID, name, coordinates and wheelchair-boarding value. |
| Departures | HTTP 200; returned scheduled and realtime seconds, service day, realtime boolean/state, headsign, trip and route IDs. |
| Alerts | HTTP 200; returned current HSL feed alerts with severity/effect, validity times and route/stop entities. The fixture retains two representative alerts. |
| Cancellation | HTTP 200; `canceledTrips(first: 5)` returned current realtime cancellations and `hasNextPage: true`. This proves pagination is mandatory; availability remains time-dependent. |
| Routing no-result | HTTP 200 with `data.stop: null`. HTTP success is not domain success. |
| Invalid GraphQL | HTTP 200 with an `errors` array and `InvalidSyntax` classification. The client must inspect the GraphQL envelope even on HTTP 200. |
| Ambiguous geocoding | HTTP 200; “Kamppi” produced several confidence-1 station candidates with distinct stable `gid` values. Equal confidence confirms that first-result auto-selection is unsafe. |
| Geocoding no-result | HTTP 200 with an empty `features` array. |
| Invalid `/place` ID | HTTP 400 with a Pelias error and empty features. |
| Removed Routing v1 | HTTP 404 with an explicit Routing v2 replacement notice. |
| Routing Data v3 | HTTP 200 HTML index; selected names include GTFS ZIPs, `hsl.pbf`, `graph.obj`, graph bundle, and OTP configuration. This is artifact delivery, not journey planning. |
| HSL GTFS-RT | Range GETs returned HTTP 206, `application/x-protobuf`, and fresh `Last-Modified` values for all three feeds. No Digitransit key was sent. |

GraphQL schema deprecations remain discoverable through [the HSL GraphiQL documentation explorer](https://api.digitransit.fi/graphiql/hsl/v2/gtfs/v1). The implementation must avoid fields marked deprecated there and treat additive schema evolution as normal.

## Time and realtime semantics

- `planConnection` accepts an ISO-8601 datetime with an explicit offset, for example `dateTime: {earliestDeparture: "2026-09-08T09:30+03:00"}`. Journey and leg time strings observed in the response retained the Helsinki offset.
- Stop-time `serviceDay` is a Unix epoch value for the service-day start; scheduled/realtime arrival and departure values are seconds after that service day. They must be combined rather than interpreted as standalone epoch timestamps.
- A stop time's realtime arrival/departure fields fall back to scheduled values when no update exists. `realtime` says whether an update was applied, and `realtimeState` distinguishes states such as `UPDATED` and `SCHEDULED`.
- Journey legs expose scheduled and estimated values separately. Realtime is enabled by default, but is not guaranteed and can be inaccurate. Absence must not be turned into a claimed delay or live status.
- Alert `effectiveStartDate` and `effectiveEndDate` are Unix seconds. Geocoding's response `timestamp` is Unix milliseconds. Fixtures record a separate RFC-3339 UTC retrieval time.
- `canceledTrips` reports realtime whole-trip cancellations. Current documentation says planned cancellations may instead be absent from the static feed and cancellation reasons exist only when an alert supplies one. The result is potentially very large and must be cursor-paginated.
- HSL's raw GTFS-RT feeds use GTFS-RT 2.0. Documented publication intervals are five minutes/on publication for service alerts, 15 seconds for trip updates, and one second for vehicle positions. Trip IDs are intentionally absent; `(route_id, start_date, start_time, direction_id)` identifies a trip. Those raw semantics matter for provenance, even though v1 consumes their integration through Routing API.

Authoritative references: [Routing realtime](https://digitransit.fi/en/developers/apis/1-routing-api/3-realtime-information/), [canceled trips](https://digitransit.fi/en/developers/apis/1-routing-api/canceled-trips/), [disruptions](https://digitransit.fi/en/developers/apis/1-routing-api/disruption-info/), and [HSL GTFS-RT](https://hsldevcom.github.io/gtfs_rt/).

## Quotas and failure handling

The current [registration guide](https://digitransit.fi/en/developers/api-registration/) says quota and rate limiting have been enforced since 2024-01-31, that normal use should not hit the limits, and that bulk jobs with thousands of consecutive calls should pause **0.5–1 second** between calls. It names HTTP 403 as a possible limit symptom and directs users to `digitransit-api@hsl.fi` if normal use is affected.

No numeric allowance is published on that page, and the successful routing/geocoding probes exposed no `RateLimit-*`, `X-RateLimit-*`, or `Retry-After` headers. It would be unsafe to invent a number or deliberately exhaust a shared production subscription. Therefore v1 must bound requests, avoid duplicate raw realtime fetches, cache only where freshness semantics allow, honor `Retry-After` if one is ever supplied, and report 403/429 as provider failures without claiming which quota was exceeded. Numeric quota display is excluded from v1 because the provider exposes no verified runtime value.

Authentication failures are distinct: an unauthenticated geocoding request returned HTTP 401 with `SubscriptionKeyNotFound` and a `WWW-Authenticate` challenge naming the header. The probe script does not persist that response because its contract is already safe to reproduce without a key.

## Attribution and licensing boundary

The current [Digitransit terms of use](https://digitransit.fi/en/developers/apis/7-terms-of-use/) apply **CC BY 4.0** to Digitransit data and require attribution to Digitransit together with the date and time the data was received. `reitti` output should therefore carry source, retrieval timestamp, and an attribution such as `© Digitransit; retrieved <RFC3339 timestamp>` rather than relying on Pelias's observed internal attribution URL.

The same terms state that OpenStreetMap-derived geographical and address data in routing, geocoding and map APIs is governed by **ODbL**. This is a data-license boundary, not a replacement software license: transformed or redistributed geographic databases need separate ODbL review. City-bike station open data is owned by City Bike Finland. v1 does not redistribute bulk provider databases or city-bike data, and every fixture remains a small contract example. The `reitti` source-code license does not relicense upstream data.

## Repeatable probe and fixture safety

Run the offline check without credentials:

```sh
python3 scripts/probe-digitransit.py check
```

For a deliberate live refresh, export the key through a secret-aware environment and run:

```sh
export DIGITRANSIT_SUBSCRIPTION_KEY='fictional-key-from-a-secret-store'
python3 scripts/probe-digitransit.py live
```

The script refuses a live run without the environment variable, sends the value only as a header, rejects any credential-bearing URL, checks every raw response and written fixture against the active value, stores only selected safe response headers, and never enables shell tracing. It does not use the secondary key. Fixture changes are expected when realtime state changes and must be reviewed before commit.

## Resolved uncertainties and v1 exclusions

- Current journey endpoint/version, Geocoding documentation replacement, Routing Data role, cancellation query, and raw HSL GTFS-RT endpoints are resolved by current documentation plus live evidence.
- Direct raw GTFS-RT, Map v3, Routing Data/local OTP, vehicle positions, autocomplete-specific UI, and numeric quota reporting are explicitly excluded from v1 for the reasons above. These align with the epic's existing post-v1 local-router/shared-mobility directions and do not block the planned CLI.
- A cancellation fixture was available during this probe window, so no synthetic cancellation or follow-up issue is needed.
