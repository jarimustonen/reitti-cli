# reitti-cli v1.0 product plan

## Product promise

`reitti` gives a person or AI agent a compact, current, source-attributed answer to: “How can I get from here to there, and what are the meaningful alternatives?”

The CLI is the stable product boundary. Human-readable output is useful for direct terminal use; schema-versioned JSON is complete enough for an agent to compare options and explain a choice without scraping prose. No MCP host is required.

## Design principles

- **Evidence before recommendation.** Preserve source times, realtime states, notices, resolved locations, and retrieval timestamps.
- **Alternatives, not one opaque answer.** Return a bounded set and label observable trade-offs: fastest, fewest transfers, least walking, earliest arrival, and realtime risk when the data supports them.
- **Correct ambiguity.** Never silently route from a weak place match. Return candidate identifiers that can be supplied in a retry.
- **Explicit time semantics.** Require RFC 3339 for machine calls, use Europe/Helsinki intentionally, and distinguish depart-at from arrive-by.
- **Protocol-neutral core.** Keep route planning and comparison in `reitti-core`; CLI rendering and HTTP live at the edges. MCP can later wrap the same core.
- **HSL scope stated honestly.** v1.0 promises the nine verified municipalities, while the provider/router abstraction may support later regions.

## Preliminary command surface

The detailed surface is owned by @design-cli-contract and may change after API validation.

```text
reitti location search <QUERY>
reitti journey plan --from <PLACE> --to <PLACE> [--depart-at <RFC3339> | --arrive-by <RFC3339>]
reitti stop search <QUERY>
reitti stop departures <STOP_ID>
reitti disruption list [--route <ID>] [--stop <ID>]
reitti config path
reitti config show
reitti doctor
reitti version
reitti skill list|print|install
```

Every data command supports the global `--json`; errors are structured on stderr, and caller-actionable failures are distinct from system/network failures. Commands remain non-interactive and never change output format by detecting a TTY.

## Journey response model

A machine response should include:

- request and retrieval metadata, schema version, provider/router, timezone, attribution, and warnings;
- resolved origin and destination, including stable identifiers and coordinates;
- zero or more alternatives with stable ordering and an explicit comparison summary;
- start/end, total duration, transfers, walking/waiting/transit durations, accessibility facts, and realtime confidence/state where available;
- ordered legs with mode, route, stops, platform/headsign, scheduled and estimated times, geometry/link references only when useful;
- cancellations and relevant service alerts with source validity windows;
- deterministic labels explaining why an alternative is notable;
- no fabricated fare, delay, accessibility, or recommendation facts when upstream data is absent.

Text mode should show a concise comparison first and enough leg detail to act. JSON mode should retain source detail without dumping the raw GraphQL response.

## Delivery sequence

1. **Validate contracts** — probe authoritative endpoints and freeze representative fixtures.
2. **Settle the contract** — command grammar, JSON schemas, errors, help, and comparison semantics.
3. **Build the foundation** — Rust core/CLI split and shared agent-first plumbing.
4. **Integrate data** — typed geocoding, routing, stop/departure, and disruption clients.
5. **Deliver journeys** — disambiguation followed by preference-aware alternatives.
6. **Add live context** — departures and disruptions, avoiding redundant realtime calls.
7. **Teach agents** — bundle a version-synchronized Agent Skill with safe workflows.
8. **Release** — package, smoke-test, document, and tag v1.0.0.

## Dependency graph

```text
validate contracts
  → design CLI contract
    → scaffold Rust workspace
      → Digitransit client
        ├→ location resolution → ranked journeys ─┐
        └→ departures and disruptions ────────────┤
                                                  → Agent Skill → v1.0 release
```

## Explicit post-v1 directions

- Other Digitransit routers and nationwide Finland coverage.
- Optional MCP adapter generated from the same schemas.
- Offline/local OTP operation using Routing Data artifacts.
- Shared mobility, park-and-ride, fares, and richer accessibility where verified.
- Bounded vehicle-location queries if they improve a concrete agent workflow.
