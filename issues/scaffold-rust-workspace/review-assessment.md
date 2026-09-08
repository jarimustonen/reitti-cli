# Existing review assessment

This continuation reused the review evidence collected before the cancelled run `01m2015e5pq98215wdvw8j0k5e`; it did not make new external LLM calls. The consolidated source was `/tmp/reitti-review-round2.txt` (Gemini, OpenAI, Claude, and DeepSeek cross-review), checked against the adopted source and focused reproductions. The full working assessment is gitignored at `history/assessment-foundation-existing.{json,md}`.

## Accepted and verified

The implementation now covers the material foundation findings: stdout write/flush failures, output reservation before config mutation and honest post-mutation failure metadata, non-Unicode environment rejection, secure doctor path inspection and text escaping, concrete config update values, distinct persistent `--set-*` arguments and ArgIds, complete support-command schemas, positional/example help, help file output, tarball-compatible provenance tests, one request-ID trait with injected deterministic clock/IDs, CRLF credential bounds, and generated-help handling. Focused regressions exercise these behaviors.

## Rejected or deferred

- Rust 1.88 is an intentional, available MSRV in 2026; the contrary claim is stale.
- Windows-only rename/test concerns are outside the supported macOS/Linux targets; file handles are already dropped before rename.
- Provider clients, exact domain schemas/models, fixtures, online probes, and the bundled skill belong to later issues and remain explicit `feature_incomplete` seams here.
- Scientific-notation rejection was not justified by an explicit contract prohibition, so the speculative grammar restriction was removed.
- Consumer-repository provenance under unsupported crate relocation, generalized public-coordinate exemption logic before public coordinates exist, help-overrides-invalid-value parsing, semantic-flag edge cases, and structured signal cancellation for currently short synchronous commands did not clear the likelihood/complexity bar.
- Directory fsync is durability hardening rather than a prerequisite for atomic rename; the adopted Unix implementation already includes it.
- No residual finding met the bar for a new issue. None was filed.
