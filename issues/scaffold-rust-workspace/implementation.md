# Foundation implementation seams

This slice establishes the CLI and library boundaries without implementing Digitransit calls or journey behavior. Domain commands validate their complete draft grammar, require credentials only when provider access would begin, and then return `feature_incomplete` (exit 2). They never fabricate provider success.

## Stable integration points

- `reitti-core::provider::{Geocoder, Router, ProviderFuture}` are provider-neutral async ports. Extend their request/result types as the data-client and journey slices land; provider wire names must remain outside core.
- `reitti-cli::client::HttpTransport` is the injectable HTTP boundary. `HttpRequest` already separates method, URL, headers, body and both timeout classes; `SecretHeader` redacts `Debug`. The next client slice must enforce request/response caps, one attempt, redirect denial, operation names and `Retry-After` validation here.
- `reitti-core::{Clock, FixedClock, SystemClock}` and `reitti_cli::RequestIdGenerator` are the deterministic time/id seams. Runtime `--frozen-time` is hidden and test-only. Provider retrieval timestamps remain separate source provenance.
- `reitti-cli::error::AppError` is the central exact-code/exit mapping. `reitti-cli::output::{Envelope, CommandOutput, AtomicOutput}` owns schema wrapping and private (0600 on Unix), collision-safe output replacement. Output destinations are securely reserved before command execution; stdout write/flush failures are exit 2, and a rare report-write failure after mutation returns `output_write_failed` with `mutation_applied: true`.
- `reitti-cli::config` resolves each key independently (`flag > env > file > default`), opens credential files with no-follow semantics, and performs locked selective atomic updates. Provider adapters should consume `EffectiveConfig`, not reread environment variables.
- `reitti-core::reference` enforces tagged polymorphic journey endpoints and raw HSL IDs in stop-only/route-only positions. Exact coordinates deliberately carry no inferred service-area membership; consumers must retain `ServiceArea::Unknown`.
- `reitti-cli::support` owns version/build provenance, schema discovery, offline doctor shapes and the truthful skill catalogue. Foundation-owned support schemas require their documented fields and validate real command output; only domain schemas remain labelled placeholders. The catalogue and version payload remain empty until @ship-agent-skill actually bundles a resource.

## Deliberate limitations for the next workers

- No HTTP implementation is linked and `doctor --online` reports two failed, zero-request checks. Offline doctor never invokes a transport.
- Domain schemas are discoverable, stable-URN scaffold documents with permissive object bodies. The owning domain slices must replace each body with its accepted exact schema before claiming schema-complete behavior.
- No companion skill is bundled, printable or installable yet. `skill list` says so; print/install fail explicitly.
- This continuation recovered the complete uncommitted implementation from cancelled run `01m2015e5pq98215wdvw8j0k5e` using `history/foundation-recovery.patch`. It reused the already-collected external review evidence rather than claiming new review calls; the assessed decisions are in `review-assessment.md`.
- The package version remains the unreleased workspace version `0.0.0`. Build provenance is nevertheless a full local git SHA, or explicit null plus tarball/vendored provenance outside a git source tree.
- Text rendering in future provider adapters must pass every untrusted field through `command::escape_text`; JSON serialization supplies standard control escaping but does not protect terminal text by itself.

## Validation

The continuation completed these gates on 2026-09-08:

- `cargo fmt --all --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace`
- `python3 scripts/probe-digitransit.py check` (19 fixtures valid and URL-safe)
- `issuectl doctor --json` (no findings)

Focused regressions additionally cover output write and flush failure classification, post-mutation output failure metadata, output-path preflight before config mutation, non-Unicode environment rejection, global-versus-persistent config setters in both argv positions, actual update values, pristine-HOME onboarding, secure config files and locks, support-schema validation and required-field rejection, structured/text help examples and positionals, help file output, version alias behavior, deterministic diagnostic injection, option termination, and readable Unicode with escaped controls.
