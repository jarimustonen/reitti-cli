# Routing v2 implementation validation

The primary agent fetched the production HSL GTFS GraphQL introspection schema
on **2026-09-08T08:07:11Z** in one authenticated, redirect-disabled, size-bounded
request. The key was supplied through the environment and was neither printed
nor retained. The result contained 243 types. The ephemeral JSON capture has
SHA-256 `f92cc76129eae4f44eef9ba6a7953207c5a564a28770226b9b6fd3cbbc7131dc`.

Source: `POST https://api.digitransit.fi/routing/v2/hsl/gtfs/v1`, GraphQL
`__schema`. The [provider's Routing API documentation](https://digitransit.fi/en/developers/apis/1-routing-api/)
provides context; the production schema is the evidence for the exact fields below.
Do not substitute OTP development documentation for deployed-field verification.

## Corrections to the first design draft

- `Alert.id: ID!` exists and is not deprecated. Preserve this provider identifier;
  a locally invented hash is unnecessary.
- `PlanPreferencesInput.accessibility.wheelchair.enabled: Boolean` requests
  wheelchair-aware routing. Its description explicitly states that this does not
  guarantee accessible itineraries because data can be wrong. The CLI should
  expose a boolean request, preserve unknown accessibility, and avoid a `required`
  spelling that implies a verified hard guarantee.
- `PlanPreferencesInput.street.walk` exposes `speed`, `reluctance`, `safetyFactor`,
  and `boardCost`. It has **no maximum walking distance field**. A CLI
  `--max-walk-m` therefore means a client-side total-walking-distance cap over
  the bounded returned alternatives. It must not invent a GraphQL argument or
  claim that filtering these alternatives proves no other route exists. Missing
  distances cannot establish that the cap is satisfied.
- `Leg.steps`, `distance`, `headsign`, `stopCalls`, `legGeometry`, `from`, and
  `to` are nondeprecated. Request navigation-relevant fields; null should reflect
  missing source data, not an implementation that did not query it.
- `Leg.intermediateStops` and `intermediatePlaces` are deprecated. Use
  `stopCalls` for intermediate transit call information.
- `Place` exposes `lat`, `lon`, `name`, `stop`, and `stopPosition`.
  `Stop` exposes `gtfsId`, `name`, `platformCode`, `vehicleMode`, `code`, and
  `wheelchairBoarding`. These are available for practical stop/platform guidance.
- `Geometry.points` is a Google encoded polyline. Decode it with explicit bounds
  when optional geometry is requested; a string is not already GeoJSON.
- `step` exposes `relativeDirection`, `absoluteDirection`, `distance`,
  `streetName`, `lat`, `lon`, `area`, `bogusName`, `stayOn`, and `exit`.
  Preserve the source facts for walking instructions; generated street names
  (`bogusName`) must not be represented as verified street names.
- `PlanModesInput` contains `direct`, `directOnly`, `transit`, and `transitOnly`.
  `PlanTransitModesInput.transit` is a list of objects with a required `mode`,
  not a list of raw mode strings. Validate the exact provider enum values.
- `PlanDateTimeInput` contains mutually exclusive `earliestDeparture` and
  `latestArrival`, both `OffsetDateTime`.
- `QueryType.stops(name:, ids:)` has no server-side limit argument.
  `stopsByRadius(lat:, lon:, radius:, first:, feeds:)` is a connection.
  Bound response bytes and local output even where server-side limiting is absent.
- `Stop.stoptimesWithoutPatterns` supports `numberOfDepartures`, `startTime`,
  `timeRange`, `omitCanceled`, and `omitNonPickups`. Explicitly request cancellation
  inclusion where the product promises cancellation evidence.

- `Itinerary.numberOfTransfers: Int!` explicitly excludes stay-seated/interlined
  continuations. Use it instead of transit-leg-count minus one. The itinerary
  also provides nondeprecated `duration`, `waitingTime`, `walkTime` (seconds)
  and `walkDistance` (metres). `Leg.interlineWithPreviousLeg` provides the
  navigation fact that the passenger stays in the same vehicle.

## Client policy

One attempt per provider operation, bounded request count and response size,
no credential-bearing redirects, explicit timeouts, and safe error envelopes.
Return retryability and a validated `Retry-After` when available so the agent
can decide whether to retry. This intentionally refines the earlier issue's
"retries" criterion: no transparent retry loop or hidden sleep is required.
Missing/invalid credentials are caller-actionable; transient network/provider
failures are system failures. Never retain the raw provider error body in normal
or verbose output when it could echo request headers or secrets.

## Rich navigation request

A follow-up production request at **2026-09-08T08:13:39Z** used named operation
`NavigationPlan`, typed variables, explicit `earliestDeparture`, five transit
mode objects, `wheelchair.enabled: true`, and optional geometry. It requested
itinerary summary metrics, endpoint stops/platforms/coordinates, route/trip IDs,
headsign, interlining, scheduled/estimated times, `stopCalls`, walking steps,
encoded geometry, and full alert IDs/text/entities.

The provider returned HTTP success, no GraphQL errors, no `routingErrors`, and
three alternatives in 38,377 response bytes. Their durations were 3,065, 3,103,
and 3,304 seconds; source transfer counts were 1, 1, and 2. Each alternative
included walking steps (22, 18, 21), intermediate stop calls (22, 19, 17), and
geometry (390, 408, 392 points). Actual platform codes were present on some
endpoints and null on others. This validates the query shape and fields, not
an accessibility guarantee or a permanent timetable.

A separate schema-union lookup established:

- `CallStopLocation` includes `Stop`, `Location`, and `LocationGroup`.
- `CallScheduledTime` includes `ArrivalDepartureTime` and `TimeWindow`.
- `AlertEntity` includes `Agency`, `Pattern`, `Route`, `RouteType`, `Stop`,
  `StopOnRoute`, `StopOnTrip`, `Trip`, and `Unknown`.
- `StopOnRoute` contains `route` and `stop`; `StopOnTrip` contains `trip` and
  `stop`; a pattern exposes `route`. Preserve these relationships when filtering
  alerts rather than discarding every non-Route/non-Stop entity.

The ephemeral captures contain only public journey data, schema and query
variables for public example locations. They may be transformed into the
existing credential-free fixture format by the client implementation; they
must not be presented as permanently current live transit data.

## Client implementation refinements

The typed adapter follows deployed nullability rather than the abbreviated
examples in the first design draft. `Alert.alertHeaderText` is nullable (the
handler can display the description when available), and a null alert entity
list is retained as explicit unknown scope for conservative relevance handling.
Nullable leg steps produce an empty step list with `navigation_complete: false`;
nullable interlining remains unknown. Nullable itinerary duration, waiting,
walking time/distance, and leg duration remain null rather than becoming zero.
Ordinary planning can therefore retain an itinerary whose walking distance is
unknown, while the later `--max-walk-m` handler must exclude it because it cannot
prove the cap. Required IDs, stop/route names needed to identify a result,
scheduled times, and source transfer count remain strict contract boundaries.

Departure normalization treats nullable `realtime` and `realtimeDeparture` as
unknown/scheduled-only evidence instead of rejecting the board. Service-day
seconds are combined using the Europe/Helsinki service date independently of
output timezone. Journey delay seconds are derived from the provider's explicit
estimated and scheduled timestamps, which preserves negative (early) values
without depending on a separately formatted duration string.

A separate bounded production verification used `dateTime.latestArrival` with
`first: 2` for the public Kamppi-to-Espoo example. The deployed endpoint accepted
the request and returned two alternatives arriving before the requested instant,
with no GraphQL or routing errors. This confirms that arrive-by planning should
continue to use `first`; switching to `last` is not warranted.

The transformed credential-free fixture `routing-navigation-rich.json` records
the named query, typed variables, attribution timestamp, and one selected rich
alternative from the production navigation capture. It validates stop calls,
platform/route/trip/headsign/coordinate facts, walking steps, realtime evidence,
alerts and bounded polyline decoding; it is historical contract evidence, not a
current timetable claim. `routing-alert-scopes.json` is explicitly an offline
edge-contract fixture covering route, stop, feed-wide and unknown scopes.
