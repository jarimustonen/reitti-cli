# Plan: stop details and Reittiopas links

**Status:** Proposed for maintainer approval. Keep the issue unlaned until approved.

## Product decision

Add a focused `reitti stop show <STOP_ID>` command and a source-labelled Reittiopas deep link. Use Digitransit's existing bounded Routing API as the machine-authoritative source; treat the Reittiopas page as a human follow-up view, not as another API or a source to scrape.

Do not add a local GTFS catalogue in this slice. It would add download, freshness, storage, and reconciliation concerns without improving the ordinary one-stop lookup.

## Proposed CLI contract

```sh
reitti --json stop show HSL:1020453
reitti stop show HSL:1020453 --language fi
```

- `STOP_ID` is a required positional raw HSL GTFS identifier, matching `departure list --stop` semantics. Tagged `stop:HSL:...`, blank, and malformed values fail before provider I/O.
- The command performs one bounded `StopDetail` Routing API request.
- Success returns `stop`, `request`, and `source`. The stop detail reuses the existing compact stop fields and adds only fields verified in the deployed schema.
- `reittiopas_url` is an explicit field rather than a generic `url`, so an agent can identify its destination and authority. It is included in every common stop object only if the URL can be deterministically and correctly formed for all returned HSL stop IDs; otherwise it remains specific to `stop show` and nullable.
- A missing provider stop returns `stop_not_found` (exit 1). Provider, credential, attribution, redaction, and retry behavior remain unchanged.
- Text output presents the stop identity and useful verified facts in a compact block, ending with the Reittiopas URL and Digitransit attribution.

The likely detailed fields to investigate are zone, parent station, and serving routes. They are candidates, not promises: each must be useful to an agent, available in one bounded query, and have clear null/unknown semantics. Do not expose provider fields merely because they exist.

## Phase 1: validate current external contracts

1. Inspect the deployed Digitransit HSL GraphQL schema and official API documentation for stop-detail fields. Record exact field names, types, nullability, localization, and whether route collections are bounded or complete.
2. Verify the current Reittiopas stop-page URL behavior with representative ordinary stops and station/platform identifiers. Check encoding of `HSL:...`, redirects, locale behavior, and unknown IDs.
3. Prefer an official API-provided canonical URL if one exists. Otherwise document the evidence for deterministic URL construction and its stability boundary.
4. Save the findings in `validation.md`, including rejected candidate fields and the reason each was rejected. Do not store credentials or raw captures containing unstable/private data.

**Gate:** If no supported stable deep link can be established, stop for a product decision rather than shipping a guessed URL. `stop show` may still be worthwhile, but that would be a changed scope requiring approval.

## Phase 2: implement the smallest vertical slice

1. Add `StopCommand::Show` and a strict positional stop-ID argument in `command.rs`, with text and structured help examples.
2. Introduce a dedicated stop-detail domain model rather than bloating search-only provider responses. Reuse the existing compact `Stop` model for shared identity fields.
3. Extend only the provider's `StopDetail` query with the approved fields. Keep stop search and departure queries compact unless the shared Reittiopas URL is computed locally after normalization.
4. Normalize nullable and enum values conservatively. Unknown source values remain unknown; collections are explicitly bounded and completeness is reported if the provider contract cannot prove completeness.
5. Generate the Reittiopas link in one tested helper from the validated raw ID. Never accept a provider-supplied arbitrary host without validation.
6. Add the handler, human renderer, request metadata, source attribution, and central error mapping using existing seams.
7. Add a strict `stop-show` JSON schema and register it in schema discovery. Any additive common-stop field must be reflected consistently in stop-list and departure schemas.

## Phase 3: agent guidance and documentation

1. Update the bundled `reitti` skill with when to use `stop show`, how to distinguish API facts from the linked human page, and the raw-ID requirement.
2. Update README examples only if the command improves first-run discovery; keep exact field documentation in command help/schema and workflow reference.
3. Keep skill CLI-version synchronization and schema checks green for the release that ships the new surface.

## Verification

### Deterministic tests

- CLI grammar accepts a raw ID and rejects tagged, malformed, and missing IDs before transport access.
- Mock transport verifies one request, the `StopDetail` operation name, exact variables, and bounded query shape.
- Normalization fixtures cover an ordinary stop, a station/platform relationship, null optional fields, unknown enum values, and an unknown stop.
- URL tests cover required percent-encoding/path rules and prove that untrusted IDs cannot alter scheme, host, query, or path structure.
- JSON output validates against `stop-show`; any changed stop-list/departure outputs continue to validate against their schemas.
- Text output contains no escaped-control regression and preserves Unicode names.
- Credential, provider-error, output-file, request-ID, and attribution contracts remain intact.

### Repository gates

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
python3 scripts/probe-digitransit.py check
issuectl doctor --json
```

### Bounded live acceptance

With a credential supplied only through the protected environment, query one ordinary stop and one station/platform case. Confirm the returned API facts, link destinations, localization, request count, attribution, and absence of credential leakage. Retain only sanitized evidence in `validation.md`.

## Risks and controls

- **Reittiopas URL changes:** validate before implementation, name the destination explicitly, and centralize construction in one helper/test.
- **Overpromising incomplete route data:** expose completeness or omit the collection when the API cannot establish it.
- **Schema bloat across list results:** use a dedicated detail model and extend the shared stop object only for cheap deterministic link metadata.
- **Source confusion:** label Digitransit as the fact source and Reittiopas as a human-facing follow-up link.
- **Extra provider traffic:** preserve one request per `stop show`; do not enrich every list row with follow-up calls.

## Approval requested

Approve the following decisions before laning:

1. Add `reitti stop show <STOP_ID>` as the public surface.
2. Prefer the existing Digitransit Routing API over GTFS download or web scraping.
3. Add an explicit `reittiopas_url` only after Phase 1 verifies a supported stable construction.
4. Limit initial enrichment to useful, one-query fields with honest null and completeness semantics.
