# Reitti workflow reference

## Inputs and bounds

- Languages: `en`, `fi`, `sv`.
- Transit modes: `bus`, `tram`, `rail`, `subway`, `ferry`.
- Journey limit: 1–6; location and stop limit: 1–10; departure limit: 1–50; alert limit: 1–100.
- Datetimes must be RFC 3339 with `Z` or an explicit numeric offset.
- Coordinates are latitude first: `60.1699,24.9384`, with no whitespace.
- Raw stop and route IDs begin with `HSL:`. Journey endpoint refs add a type tag, for example `stop:HSL:1020453`.

Use `reitti <resource> <verb> --help --json` when exact flags are uncertain.

## Corrective retries

If a free-text endpoint is ambiguous, inspect `error.details` for candidates and retry with a returned stable ref:

```sh
reitti --json journey list --from query:Kamppi --to coord:60.1776,24.6529 --limit 3
# Retry after user/context evidence selects a displayed candidate:
reitti --json journey list \
  --from place:gtfshsl:station:GTFS:HSL:1000102 \
  --to coord:60.1776,24.6529 --limit 3
```

For `invalid_datetime`, add the applicable offset; do not assume the user's timezone. For `invalid_stop_id`, remove `stop:` only when supplying the raw ID to `departure list` or a stop/route alert filter. For `provider_rate_limited` or another retryable provider error, respect a returned retry-after value and stop/report persistent errors or a deadline that makes retry pointless.

## Reading journey evidence

- `comparison` labels apply only across alternatives in that response. Ties can label multiple alternatives.
- `complete: false` means other candidates or routes may exist.
- `fare: null`, unknown accessibility, and missing walking distance are absence of verified evidence.
- `realtime.status: mixed` means some transit legs have updates and others are scheduled-only.
- A cancelled leg or departure is explicit provider evidence; do not infer cancellation from a missing trip.
- `--max-walk-m` is a local filter over bounded returned alternatives; unknown distance fails the cap, and no match does not prove no route exists.
- Add `--include-geometry` for a GeoJSON route line. It uses `[longitude, latitude]`; CLI coordinate inputs use latitude first.
- Walking steps are already requested. If navigation is incomplete or truncated, do not present it as complete turn-by-turn guidance.

When the user has an arrival deadline, reject alternatives ending after it and explain any tight transfer or scheduled-only segment visible in the returned facts. When several options remain, weigh the user's stated preference rather than treating a comparison label as a recommendation.

## Attribution and uncertainty

Name Digitransit as the source and include the returned retrieval time or attribution. Distinguish the planning snapshot from current conditions. Recheck departures or alerts close to travel when useful, but do not start background tracking or promise notifications.
