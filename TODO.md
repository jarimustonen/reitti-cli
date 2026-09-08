# Work-session handoff

## 🔄 Continue here

The first stint completed and published @validate-data-contracts. Current Digitransit production behavior was verified with live, secret-safe probes. `reitti` v1 should use Routing v2 HSL GTFS at `POST https://api.digitransit.fi/routing/v2/hsl/gtfs/v1`; the old Routing v1 endpoint is removed, while Routing Data v3 is a separate artifact service rather than a journey-planning successor. Geocoding v1 endpoints, HSL GTFS-RT boundaries, realtime/time semantics, attribution, and the provider's qualitative quota guidance are recorded in `issues/validate-data-contracts/validation.md`.

User-facing key setup and rotation instructions are in `docs/digitransit-api-access.md`. The canonical local worktree has the primary and secondary subscription keys in ignored `.env` with mode 600; an isolated worker does not inherit that file. Do not print, commit, copy into a worker, or place credentials in command arguments. Resolve the local secret path out of band and source it only for deliberate live probes.

The round also added `scripts/probe-digitransit.py` and 19 redacted fixtures under `tests/fixtures/digitransit/`. The offline fixture check and the Rust formatting, lint, test, and issuectl health gates passed. There is no deploy step for this early library/CLI repository. No worktree run remains live or resumable.

For the next stint, continue the planned product direction with @design-cli-contract: settle command grammar, JSON schemas, error/exit behavior, help, ambiguity retries, journey comparison semantics, and configuration/credential precedence against the verified API contracts and `AGENTS-AI-FIRST-CLI.md`. In particular, build the contract around Routing v2 HSL GTFS rather than the stale preliminary Routing v1 assumption. After that work lands, re-read the live issuectl DAG rather than inferring later sequencing from this narrative.

The only unresolved provider detail is that Digitransit publishes no numeric subscription quota. Keep calls bounded, avoid duplicate realtime requests, honor `Retry-After` if supplied, and treat 403/429 as provider failures; numeric quota reporting is outside v1 unless the provider later exposes reliable data.
