# Journey implementation validation

## Integration seams

- `handlers/journey.rs` consumes the landed `location::resolve` seam. It does not repeat geocoding or stop lookup and appends one routing-plan source after endpoint lookup sources in provider-request order.
- Coordinates perform no lookup. The handler performs one `Router::plan` call using the typed `PlanRequest`; provider operations are therefore bounded at one to three with no handler retries.
- The typed client remains responsible for GraphQL variables, deployed nullable-field normalization, step and geometry caps, realtime source evidence, and provider transport errors. The handler owns local walking-distance filtering, local output limiting, comparisons, response shape, warnings, and human rendering.
- Explicit RFC 3339 argument text is retained byte-for-byte in request metadata. Parsed instants are sent to the provider. Injected defaults and all human-rendered times use the configured IANA timezone.

## Deterministic evidence

`crates/reitti-cli/tests/journey.rs` uses injected transport, clock, request IDs, and credential environment isolation. It covers:

- ordinary and arrive-by planning, default injected time, explicit-offset retention, typed mode/wheelchair/geometry/`first` variables;
- exact journey-schema validation, provider/source transfer counts, provider order, stable within-response IDs, canonical comparisons, ties, and missing-metric warnings;
- local walking caps (over cap, unknown distance, all excluded), oversized provider output, completeness and scoped warnings;
- transit-only realtime summaries while retaining walking-leg times, early/delayed/cancelled evidence, interlining, headsigns, platforms, alerts, walking steps, raw generated street facts, and geometry;
- normalization incompleteness surviving as null geometry, false navigation completeness, and a structured warning;
- empty routes with and without routing domain errors, routing errors alongside usable alternatives, and transport failures;
- non-Helsinki human timezone consistency and invalid grammar before provider access.

The adjusted resolver integration assertions in `crates/reitti-cli/tests/location.rs` verify exact request/source budgets: two query endpoints plus plan = 3, selected place or stop plus coordinate plus plan = 2, and coordinate pair plus plan = 1. Equal timestamps from the frozen clock do not collapse performed source records.

## Validation

The implementation checkpoint and final local suite contain 61 tests (21 CLI library unit, 20 CLI contract, 9 journey integration, 1 live-context integration, 7 location integration, 3 core unit) plus doc tests. Focused journey regressions include cancelled/interlined alert rendering, normalization-cap warnings, and provider over-return bounding.

Primary exported tracked checkpoint `c962e28` without `.git` or ignored files and ran `cargo test --locked --workspace` in a disposable Linux ARM64 Podman container using official `rust:1.88-bookworm`. The whole suite passed; the container was removed and no host toolchain changed. This independently validates the checkpoint on the declared MSRV/Linux target from a source archive. It does not establish GitHub Actions success; hosted CI remains blocked by the separately reported account billing restriction.

Primary supplied pre-final-draft live evidence without exposing credentials to this worktree:

- coordinate Kamppi to Espoo, geometry, limit 2: two alternatives, one source, proper steps/LineStrings/headsigns, authoritative transfer counts, and honest transit realtime summaries;
- stop `HSL:1020453` to Espoo, arrive-by `2026-09-08T18:00:00+03:00`, wheelchair-aware: two alternatives before the deadline, two sources, unknown accessibility, and latest-departure comparison;
- UTC text mode: heading and leg times used one timezone, estimates showed scheduled/delay evidence, platform/headsign and a real service alert were visible.

These are recorded as primary pre-final-draft smoke evidence, not this worker's credentialed validation or final release acceptance. Final credentialed smoke remains primary-owned.

## Review evidence

The required advisory request targeted `claude-fable-5` and `deepseek-v4-pro` for one round and included every new test artifact. It emitted no response and was cancelled by primary after more than 20 minutes; neither model is claimed to have passed or failed the code, and there was no partial output to assess. `history/review-ranked-journeys.md` accurately records the unavailable advisory result and the five source-backed primary findings. `history/assessment-ranked-journeys.{json,md}` classifies all five as confirmed fixes and verifies them against current source/tests. No speculative residual issue was filed.
