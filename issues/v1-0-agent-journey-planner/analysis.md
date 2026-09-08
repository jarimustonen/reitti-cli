# HSL open-data and prior-art analysis

_Initial survey: 2026-09-08. Endpoint behavior still has to be verified with a registered key under @validate-data-contracts._

## Product finding

There is room for `reitti-cli`, but not because no agent integration exists. GitHub already contains two young HSL-specific MCP servers and an older human-oriented route CLI:

- [`devusvulgaris/mcp-hsl`](https://github.com/devusvulgaris/mcp-hsl) exposes geocoding, journey planning, and stop departures as three MCP tools. It is published as npm package `mcp-hsl` 0.1.1 but has no tests or CI in the inspected repository.
- [`nevaluoto/hsl-mcp`](https://github.com/nevaluoto/hsl-mcp) exposes six MCP tools, adding stop search, nearby stops, and disruptions. It is a Docker-oriented, single-file Python implementation.
- [`anttikon/routahe`](https://github.com/anttikon/routahe) is a traditional Reittiopas CLI. Its current design predates the agent-first output contract and modern Digitransit API-key model.
- [`lassihi/HSL-departure`](https://github.com/lassihi/HSL-departure) is a narrow command-line departure display rather than a journey planner.

`reitti-cli` should therefore differentiate itself as a protocol-neutral, scriptable source of structured travel evidence. Agents can invoke the executable directly in any shell. An MCP adapter can be added later without making MCP the core product boundary.

## Verified HSL-area coverage

The HSL-maintained [`digitransit-ui` HSL configuration](https://github.com/HSLdevcom/digitransit-ui/blob/master/app/configurations/config.hsl.js) describes Journey Planner coverage as:

> Helsinki, Espoo, Vantaa, Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio and Tuusula.

Version 1.0 adopts these nine municipalities as its promised service area. Digitransit also operates Finland, Southwest Finland, and Waltti routers, but `reitti-cli` must not imply those are supported until they have their own validation and product work.

## What the open-data stack offers

The starting point requested for this project is [HSL Open data](https://www.hsl.fi/hsl/avoin-data). The implementation-facing documentation is maintained at [Digitransit for developers](https://digitransit.fi/en/developers/) and in [`HSLdevcom/digitransit-site`](https://github.com/HSLdevcom/digitransit-site).

### Routing API

- OpenTripPlanner-backed GraphQL API.
- Plans itineraries and queries routes, trips, stops, timetables, realtime state, cancellations, and disruption information.
- This should be the primary v1.0 source because one response can preserve a coherent journey and much of its live context.
- Relevant docs: [Routing API](https://digitransit.fi/en/developers/apis/1-routing-api/), especially itinerary planning, stops, realtime information, disruption information, and canceled trips.

### Geocoding API

- Pelias-based REST API returning GeoJSON.
- Supports free-text address/place search, autocomplete, and coordinate lookup.
- It is needed to turn human descriptions into coordinates, but top-match auto-selection is unsafe when results are ambiguous.
- Relevant docs: [Geocoding API](https://digitransit.fi/en/developers/apis/3-geocoding-api/).

### Routing Data API

- Supplies the data and configuration used to build or run OpenTripPlanner.
- HSL artifacts include GTFS (`HSL.zip`, `HSL-lautta.zip`), OpenStreetMap PBF, graph bundles, elevation data, and OTP configuration/build reports.
- It is valuable for reproducible fixtures, diagnostics, bulk/offline analysis, and a possible future local-router mode. It is not the shortest path to an online v1.0 journey command.
- Relevant docs: [Routing Data API](https://digitransit.fi/en/developers/apis/2-routing-data-api/).

### Realtime APIs

- HSL service alerts, trip updates, and vehicle positions are available in GTFS Realtime; high-frequency vehicle-position events are also available over MQTT/HFP.
- Digitransit's former hosted GTFS-RT alert and trip-update endpoints are documented as deprecated for HSL. Current HSL GTFS-RT endpoints are documented separately at [HSL GTFS-RT](https://hsldevcom.github.io/gtfs_rt/).
- The Routing API already incorporates alerts and trip updates. A CLI should not make duplicate realtime calls unless it needs fields or latency that routing does not expose.
- Raw vehicle-position streams are likely outside v1.0: they are high-volume, consume agent context, and are not necessary for recommending ordinary routes.

### Map API

- Provides raster background tiles and vector tiles for stops, rental stations, and park-and-ride points.
- A headless CLI does not need map tiles for core route planning. Stable links or coordinates may be useful, but fetching or rendering maps is outside v1.0.
- Relevant docs: [Map API](https://digitransit.fi/en/developers/apis/4-map-api/).

## Access and licensing constraints

- Calls to `api.digitransit.fi` require registration and a subscription key. The key must be handled as a secret and never appear in logs, fixtures, command history examples, or JSON diagnostics.
- Digitransit advises clients to avoid excessive request rates and documents quotas through its API portal. Exact current limits must be captured during contract validation instead of hard-coded from stale prose.
- Digitransit data is generally CC BY 4.0 and requires attribution including the retrieval date/time.
- Journey Planner geographic, address, and map data derived from OpenStreetMap is governed by ODbL. City-bike data can have a different owner.
- The CLI must preserve source attribution and distinguish source facts from its own deterministic comparison labels.
- Relevant docs: [API registration](https://digitransit.fi/en/developers/api-registration/) and [terms of use](https://digitransit.fi/en/developers/apis/7-terms-of-use/).

## Risks to resolve before implementation

1. Current GraphQL schema names and realtime fields must be probed against the production HSL router.
2. Address ambiguity must have an explicit retry protocol suitable for agents.
3. “Recommended” must be explainable from returned facts and caller preferences, not an opaque local score.
4. Arrival deadlines and daylight-saving transitions need deterministic Europe/Helsinki handling.
5. Accessibility, fares, shared mobility, and bike/scooter availability should only enter v1.0 where source completeness can be stated honestly.
6. Attribution must survive machine-readable output and downstream agent summaries.
