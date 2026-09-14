# Validation: stop details and Reittiopas links

Validated on 2026-09-14 before implementation. No credential values or raw response captures are retained here.

## Digitransit Routing API

Sources checked:

- Official Digitransit stop documentation: <https://digitransit.fi/en/developers/apis/1-routing-api/stops/>
- The deployed HSL Routing API v2 GraphQL schema at `https://api.digitransit.fi/routing/v2/hsl/gtfs/v1`, queried through introspection with the protected local subscription key.

The deployed `Stop` fields selected for the bounded detail query are:

| Field | Deployed type | Detail meaning |
|---|---|---|
| `gtfsId` | `String!` | Raw feed-qualified stop ID. The official docs specify `FeedId:StopId`, with `HSL` as the HSL feed ID. |
| `name` | `String!` (also accepts a `language: String` argument) | Localized by the request language used by the existing client. |
| `code` | `String` | Nullable public stop code. |
| `platformCode` | `String` | Nullable platform designation. |
| `lat`, `lon` | `Float` | Nullable coordinates; both remain absent if the pair is unavailable. |
| `vehicleMode` | `Mode` | Nullable/forward-compatible mode; an unknown value remains an empty mode list in the existing compact model. |
| `wheelchairBoarding` | `WheelchairBoarding` | Nullable/forward-compatible accessibility evidence; absent or unknown remains `unknown`. |
| `zoneId` | `String` | Nullable HSL fare-zone label. The official stop documentation identifies this as the stop's zone. |
| `parentStation` | `Stop` | Nullable parent station. Detail requests select only its non-null `gtfsId` and `name`. |

A current station/platform check found a platform stop whose `parentStation` identifies its station, while an independently queried ordinary stop had no parent. This confirms that absence is meaningful and must remain `null`.

### Rejected candidates

- `patterns: [Pattern]` is documented for routes through a stop, but the deployed field has no pagination or limit argument. Its completeness and size cannot be bounded by this command, so serving routes are omitted.
- Station child-stop lists are similarly omitted. `stop show` describes one raw stop ID and does not turn into a station catalogue.
- No facts are taken from the Reittiopas web page. Digitransit remains machine-authoritative.

## Reittiopas deep link

The deterministic stop route is:

```text
https://reittiopas.hsl.fi/pysakit/<percent-encoded raw HSL GTFS stop ID>
```

For example, `HSL:1020453` becomes:

```text
https://reittiopas.hsl.fi/pysakit/HSL%3A1020453
```

Evidence:

- The official HSL-maintained `HSLdevcom/digitransit-ui` source at commit `cc020fdf4757354693c715f1d7fd032dded4399c` defines the HSL stop prefix as `pysakit` and `stopPagePath` as `/${PREFIX_STOPS}/${encodeURIComponent(gtfsId)}`.
- The same application registers `/:stopId` below `/pysakit`, requests that stop, and redirects a missing stop to the stop-list route.
- HTTPS checks against representative current ordinary and platform stop paths returned the route-planner application without a server redirect. An unknown-ID path also returns the single-page application shell; existence must therefore come from Digitransit's stop response, not the HTTP status.
- The route is language-independent: localization is application state rather than an `en`/`fi`/`sv` path segment.

This is a verified link into HSL's official route-planner implementation, but it is a web-UI route rather than a versioned public API. HSL may change it independently. Construction is therefore centralized and tested, the field is explicitly named `reittiopas_url`, and consumers must treat it as a human follow-up link rather than a machine data source.

## Bounded live acceptance

A protected local credential was present in a mode-`0600` config file, remained redacted in `config show`, and passed the existing bounded online doctor probes. The worktree-local debug binary was then used for exactly two `stop show` invocations; no global binary or installed skill was changed.

- An ordinary current stop (`HSL:1040129`, Finnish request) returned a zone and no parent station.
- A current Pasila platform (`HSL:1174504`, Swedish request) returned a zone and its parent station.
- Both responses reported the requested language in request metadata, carried Digitransit source attribution, and formed an HTTPS URL under the fixed `reittiopas.hsl.fi/pysakit/HSL%3A…` route. Both linked routes returned the official route-planner application with HTTP 200.
- Both commands had empty stderr and neither captured output contained a subscription-key header name or credential value. The implementation's transport test separately proves exactly one `StopDetail` request per invocation, the exact ID variable, and the bounded field selection.

Only these sanitized conclusions were retained; live response bodies and credentials were not saved.
