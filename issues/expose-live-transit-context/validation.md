# Live transit context implementation validation

## Implemented seams

The stop, departure, and alert handlers use the existing `HandlerContext` and construct the landed typed `DigitransitRouter` over its injected transport, clock, request-ID generator, effective configuration, and credential. Each command performs exactly one Routing v2 request.

- `stop list` maps named and nearby searches to `StopSearchRequest`, preserves typed stop/platform/accessibility/source evidence, and reports bounded output as non-exhaustive.
- `departure list` requests one bounded 50-row board, then applies the requested time window, local mode filter, operational-time/trip ordering, and caller limit in that order. Unknown stops return `stop_not_found`; a known empty board succeeds. Scheduled-only, explicit realtime, delay, cancellation, service date, and source timestamps remain separate evidence. Human rows are rendered in the configured display timezone while JSON retains provider-normalized instants and Helsinki-derived service dates.
- `alert list` obtains the HSL alert feed once and applies route/stop relevance as a union. Feed-wide and unknown-scope notices are retained. Known validity boundaries independently exclude future or expired alerts; otherwise missing boundaries are retained with a warning. Severity, start time, and provider ID sorting precede the caller limit. Pattern, StopOnRoute, and StopOnTrip route/stop relationships participate in relevance checks.

The three owned schema documents now recursively constrain the actual data objects, including provider metadata, nullable source facts, raw unknown alert enums, alert entity relationships, realtime evidence, stable references, and all output bounds.

## Deterministic coverage

`crates/reitti-cli/tests/live_context.rs` invokes the public `run_with_transport` seam with a fixed clock and request IDs. It validates actual stop, departure, and alert outputs against their bundled JSON schemas and covers:

- named and nearby stops, stable IDs/refs, platform, accessibility, distance, and source evidence;
- the default two-hour departure window, injected-clock default, scheduled-only and explicitly cancelled rows, Helsinki service date, local mode filtering, successful bounded emptiness, and non-Helsinki human display;
- route-only and stop-only alert relevance in one request, Pattern and StopOnRoute relationships, unknown scope, absent headers/validity, unknown source effect, severity-first sorting, and a severe source row beyond earlier rows;
- open-ended alerts whose known future start or known past end proves inactivity;
- exact one-request counts, unknown-stop classification, malformed optional provider data, provider HTTP errors, and credential-safe text/JSON errors.

The landed adapter's existing deterministic tests separately cover Helsinki service-date recovery across both 2026 DST transitions and source service-day arithmetic. This worker used no live credentials.

## Primary-agent live smoke evidence

The primary agent subsequently ran three bounded invocations of this worker's built CLI in a temporary HOME with a root-only environment credential. Nearby stop search returned five stable HSL stops with real platforms; departure lookup returned five updated departures with early estimates and `service_date: 2026-09-08`; the combined route/stop alert query returned two conservatively retained unknown-scope alerts with `alert_scope_unknown`. Every invocation exited 0 with empty stderr, `complete: false`, and no credential echo. The primary retained the raw results in its private run scratch as `history/{stops,departures,alerts}-live-check.json`; no live call was repeated here.
