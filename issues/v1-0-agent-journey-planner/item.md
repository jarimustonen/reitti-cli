---
created: 2026-09-08
updated: 2026-09-08
type: epic
owner: jarimustonen
status: open
priority: high
---

# v1.0 agent journey planner

## Goal

Deliver `reitti` v1.0: a dependable, non-interactive CLI that lets a person or AI agent resolve places in the HSL service area, request journeys for a departure or arrival time, compare meaningful alternatives, and retrieve the live context needed to explain a recommendation.

The supported HSL area is Helsinki, Espoo, Vantaa, Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio, and Tuusula. The software must identify itself as independent from HSL and Digitransit.

## Success criteria

- A caller can plan journeys using place names, addresses, stops, or coordinates.
- Results expose several alternatives with explicit trade-offs such as duration, transfers, walking, realtime state, and disruptions.
- Every operational command has stable schema-versioned JSON output suitable for agents; text output is deterministic and useful to people.
- Ambiguous locations are surfaced as candidates instead of silently selecting an unsafe match.
- Authentication, attribution, errors, rate limits, and timestamps follow Digitransit requirements.
- The CLI ships a synchronized companion Agent Skill and passes unit, contract, integration, and end-to-end tests.
- A reproducible v1.0 release can be installed and run on macOS and Linux.

## Scope

Included: HSL router journey planning, geocoding, stop discovery and departures, relevant realtime status and disruptions, configuration/authentication, ranked alternatives, agent documentation, packaging, and release verification.

Excluded from v1.0: ticket purchase, account-specific HSL data, a graphical map UI, persistent user profiling, autonomous travel decisions without user-visible evidence, and guaranteed coverage outside the nine-municipality HSL area.

## Issues

- [ ] @validate-data-contracts — validate current endpoints, licensing, and fixtures
- [x] @design-cli-contract — define the command and JSON contracts
- [ ] @scaffold-rust-workspace — establish the testable Rust foundation
- [ ] @implement-digitransit-client — implement typed data access
- [ ] @resolve-journey-locations — make endpoint resolution safe and retryable
- [ ] @plan-ranked-journeys — compare and explain route alternatives
- [ ] @expose-live-transit-context — expose departures and disruptions
- [ ] @ship-agent-skill — package the agent operating workflow
- [ ] @release-v1 — verify and publish v1.0

## Phases

1. Verify source contracts and settle the public CLI/schema.
2. Establish the Rust workspace and Digitransit integration.
3. Deliver location resolution, journey alternatives, and live context.
4. Package the agent workflow and cut v1.0.

## Comments

The epic analysis and initial product plan are in `analysis.md` and `plan.md` beside this file.

## Decisions

### 2026-09-08T07:51:41Z · @codex

The user authorizes autonomous CLI design, full implementation, onboarding/configuration, documentation and shipshape OSS preparation across successive work rounds. Preserve GitHub repository privacy. The CLI provides composable facts for personal journey planning and travel assistance; agents own complex workflows. Prepare a polished usable open-source project, including clean-machine installation and configuration, rather than stopping after the CLI design round.

### 2026-09-08T07:54:24Z · @codex

The user explicitly requires primary-agent ownership of quality: independently review and revise delegated work, especially complex design. Worker reports and passing checks are evidence, not automatic acceptance. The primary agent reviews the CLI contract before implementation and inspects implementation behavior before release readiness is claimed.

