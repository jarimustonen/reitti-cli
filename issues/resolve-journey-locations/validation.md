# Location resolution validation

## Implemented contract

- `location list` performs one typed Geocoding v1 request, preserves provider order, applies `any|address|venue|stop` filtering locally, caps output at the requested limit, and reports `complete: false` because Pelias supplies no verified exhaustion evidence.
- Candidate output preserves stable `place:` refs, provider names and Unicode, locality/neighbourhood/postcode, exact coordinates, layer/source/confidence/modes, municipality classification, request ID, retrieval time, licensing, and attribution.
- Journey endpoints resolve through `handlers::location::resolve`. It returns `ResolvedLocation` with `input_ref`, `resolution`, the selected candidate, and exact coordinates. The next journey worker can pass those coordinates directly to `Router::plan` without repeating a lookup.
- `coord:` performs no lookup. `query:`, `place:`, and `stop:HSL:` perform at most one lookup each. Explicit place/stop refs bypass free-text confidence checks; stale refs fail actionably.
- Free text resolves only for exactly one concrete address/venue/stop with explicit confidence at least `0.9`. Multiple results never use confidence as a tie-breaker. Missing/low confidence and locality/other singletons return `location_ambiguous` with provider evidence and copyable retry refs.
- Search may show outside candidates. Journey resolution rejects only `service_area: outside`; unknown remains eligible. Both Finnish and Swedish names of all nine supported municipalities are recognized by the landed provider normalizer. Exact caller coordinates remain `unknown` without reverse geocoding.
- Until @plan-ranked-journeys lands, two successfully resolved endpoints return the explicit `feature_incomplete` planning error with `stage: locations_resolved`, resolved endpoint details, sources, and `plan_requests_sent: 0`.

## Deterministic validation

`crates/reitti-cli/tests/location.rs` exercises injected transport, clock, and request IDs for empty search; kind filtering and provider order; Finnish/Swedish Unicode; known inside/outside and unknown area; no match; multiple and untrusted singleton ambiguity; selected-ref retry; stale place and stop refs; exact coordinates; per-endpoint request counts; no plan request; structured error exits/envelopes; source attribution; credential redaction; text control escaping; and validation against the bundled exact `location-list` JSON schema.

The primary agent independently ran a bounded credential-env-only smoke test against production using a temporary HOME. `location list --query Kamppi --limit 5 --json` exited 0 with empty stderr, five stable refs, readable Unicode, and truthful `complete: false`. `journey list --from query:Kamppi --to coord:60.175294,24.684855 --json` exited 1 with no stdout, five candidate retry refs, source attribution, and no credential echo. No additional live request was made by this worker.
