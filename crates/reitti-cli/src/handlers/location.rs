mod schema;

use std::time::Duration;

use serde::Serialize;
use serde_json::json;

use super::{await_provider, HandlerContext};
use crate::{
    command::{LocationKind as RequestedKind, LocationListArgs},
    config,
    digitransit::{DigitransitGeocoder, DigitransitRouter},
    error::AppError,
    output::CommandOutput,
};
use reitti_core::{
    Geocoder, Language, LocationCandidate, LocationKind, LocationRef, LocationSearchRequest,
    ProviderResult, ProviderSource, Router, ServiceArea, Stop,
};

/// A free-text endpoint is accepted automatically only when the provider supplied
/// an explicit confidence at or above this conservative threshold. Confidence
/// never breaks a tie: two or more candidates are always ambiguous.
pub(crate) const UNIQUE_QUERY_CONFIDENCE: f64 = 0.9;
const RESOLUTION_CANDIDATE_LIMIT: u8 = 5;

#[derive(Debug, Serialize)]
struct RequestMetadata {
    request_id: String,
    language: &'static str,
    timezone: String,
}

#[derive(Debug, Serialize)]
struct LocationListData {
    query: String,
    kind: &'static str,
    limit: u8,
    count: usize,
    complete: bool,
    candidates: Vec<LocationCandidate>,
    request: RequestMetadata,
    source: ProviderSource,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ResolvedLocation {
    pub input_ref: String,
    pub resolution: Resolution,
    #[serde(flatten)]
    pub candidate: LocationCandidate,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Resolution {
    ExactCoordinate,
    StablePlace,
    StableStop,
    UniqueQuery,
}

#[derive(Debug)]
pub(crate) struct ResolutionResult {
    pub selected: ResolvedLocation,
    pub source: Option<ProviderSource>,
}

pub fn execute(
    context: HandlerContext<'_>,
    args: LocationListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    let geocoder = geocoder(&context)?;
    // A kind filter is local because the typed provider port deliberately has
    // one stable search shape. Ask for the provider maximum so filtering does
    // not need hidden retries, then retain provider order and cap the output.
    let provider_limit = if matches!(args.kind, RequestedKind::Any) {
        args.limit
    } else {
        10
    };
    let result = await_provider(geocoder.search(LocationSearchRequest {
        query: args.query.clone(),
        language,
        limit: provider_limit,
    }))?;
    let candidates = result
        .value
        .into_iter()
        .filter(|candidate| requested_kind_matches(args.kind, candidate.kind))
        .take(args.limit as usize)
        .collect::<Vec<_>>();
    let count = candidates.len();
    let kind = requested_kind_name(args.kind);
    let text = location_text(&args.query, count, &candidates, &result.source);
    CommandOutput::success(
        text,
        LocationListData {
            query: args.query,
            kind,
            limit: args.limit,
            count,
            // Pelias does not expose a total or an exhaustion marker in this
            // contract. A short page therefore cannot prove completeness.
            complete: false,
            candidates,
            request: RequestMetadata {
                request_id: context.request_ids.next(),
                language: language.code(),
                timezone: context.config.timezone.0.clone(),
            },
            source: result.source,
        },
    )
}

/// Resolve one already-parsed journey endpoint with at most one provider call.
/// The returned candidate includes exact coordinates and the caller's original
/// reference, so journey planning can reuse it without repeating lookups.
pub(crate) fn resolve(
    context: &HandlerContext<'_>,
    input_ref: &str,
    reference: LocationRef,
    language: Language,
) -> Result<ResolutionResult, AppError> {
    match reference {
        LocationRef::Coordinate(coordinates) => {
            let normalized = format!("{},{}", coordinates.latitude, coordinates.longitude);
            Ok(ResolutionResult {
                selected: ResolvedLocation {
                    input_ref: input_ref.to_owned(),
                    resolution: Resolution::ExactCoordinate,
                    candidate: LocationCandidate {
                        reference: format!("coord:{normalized}"),
                        kind: LocationKind::Other,
                        id: None,
                        label: normalized.clone(),
                        name: normalized,
                        locality: None,
                        neighbourhood: None,
                        postal_code: None,
                        coordinates,
                        source: "caller".to_owned(),
                        source_layer: "coordinate".to_owned(),
                        confidence: None,
                        service_area: ServiceArea::Unknown,
                        modes: Vec::new(),
                    },
                },
                source: None,
            })
        }
        LocationRef::Query(query) => {
            let geocoder = geocoder(context)?;
            let result = await_provider(geocoder.search(LocationSearchRequest {
                query,
                language,
                limit: RESOLUTION_CANDIDATE_LIMIT,
            }))?;
            resolve_query(input_ref, result)
        }
        LocationRef::Place(id) => {
            let geocoder = geocoder(context)?;
            let result = await_provider(geocoder.place(&id, language))?;
            let source = result.source;
            let candidate = result.value.ok_or_else(|| {
                location_not_found(
                    input_ref,
                    "The selected place ref is stale or no longer exists. Run location list again and retry with a returned ref.",
                )
                .with_detail("source", json!(&source))
            })?;
            reject_outside(input_ref, &candidate)
                .map_err(|error| error.with_detail("source", json!(&source)))?;
            Ok(ResolutionResult {
                selected: ResolvedLocation {
                    input_ref: input_ref.to_owned(),
                    resolution: Resolution::StablePlace,
                    candidate,
                },
                source: Some(source),
            })
        }
        LocationRef::Stop(id) => {
            let router = router(context)?;
            let result = await_provider(router.stop(&id, language))?;
            let source = result.source;
            let stop = result.value.ok_or_else(|| {
                location_not_found(
                    input_ref,
                    "The selected HSL stop ref is stale or no longer exists. Search stops again and retry with a current stop:HSL: ref.",
                )
                .with_detail("source", json!(&source))
            })?;
            let candidate = stop_candidate(stop, input_ref)
                .map_err(|error| error.with_detail("source", json!(&source)))?;
            reject_outside(input_ref, &candidate)
                .map_err(|error| error.with_detail("source", json!(&source)))?;
            Ok(ResolutionResult {
                selected: ResolvedLocation {
                    input_ref: input_ref.to_owned(),
                    resolution: Resolution::StableStop,
                    candidate,
                },
                source: Some(source),
            })
        }
    }
}

fn resolve_query(
    input_ref: &str,
    result: ProviderResult<Vec<LocationCandidate>>,
) -> Result<ResolutionResult, AppError> {
    let source = result.source;
    let mut candidates = result.value;
    if candidates.is_empty() {
        return Err(location_not_found(
            input_ref,
            "No location candidates matched the query. Check the spelling or run location list with a broader query.",
        )
        .with_detail("source", json!(&source)));
    }
    if candidates.len() != 1 {
        return Err(location_ambiguous(
            input_ref,
            "The query returned multiple candidates; confidence cannot select a winner.",
            &candidates,
        )
        .with_detail("source", json!(&source)));
    }
    let candidate = candidates.pop().expect("singleton checked");
    let reason = match (candidate.kind, candidate.confidence) {
        (LocationKind::Address | LocationKind::Venue | LocationKind::Stop, Some(confidence))
            if confidence >= UNIQUE_QUERY_CONFIDENCE => None,
        (LocationKind::Locality | LocationKind::Other, _) => Some(
            "The only candidate is a broad locality or other feature, not a concrete address, venue, or stop.",
        ),
        (_, None) => Some("The only candidate has no explicit provider confidence."),
        (_, Some(_)) => Some("The only candidate is below the 0.9 confidence threshold."),
    };
    if let Some(reason) = reason {
        return Err(location_ambiguous(input_ref, reason, &[candidate])
            .with_detail("source", json!(&source)));
    }
    reject_outside(input_ref, &candidate)
        .map_err(|error| error.with_detail("source", json!(&source)))?;
    Ok(ResolutionResult {
        selected: ResolvedLocation {
            input_ref: input_ref.to_owned(),
            resolution: Resolution::UniqueQuery,
            candidate,
        },
        source: Some(source),
    })
}

fn reject_outside(input_ref: &str, candidate: &LocationCandidate) -> Result<(), AppError> {
    if candidate.service_area == ServiceArea::Outside {
        return Err(AppError::caller(
            "location_outside_service_area",
            format!(
                "Location '{}' is proven outside the nine-municipality HSL service area.",
                crate::command::escape_text(&candidate.label)
            ),
        )
        .with_detail("input_ref", input_ref)
        .with_detail("candidate", json!(candidate))
        .with_detail(
            "expected",
            "Helsinki, Espoo, Vantaa, Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio, or Tuusula",
        ));
    }
    Ok(())
}

fn location_not_found(input_ref: &str, reason: &str) -> AppError {
    AppError::caller("location_not_found", reason).with_detail("input_ref", input_ref)
}

fn location_ambiguous(input_ref: &str, reason: &str, candidates: &[LocationCandidate]) -> AppError {
    let retry_refs = candidates
        .iter()
        .map(|candidate| candidate.reference.as_str())
        .collect::<Vec<_>>();
    let mut error = AppError::caller("location_ambiguous", reason)
        .with_detail("input_ref", input_ref)
        .with_detail("reason", reason)
        .with_detail("candidates", json!(candidates))
        .with_detail("candidate_limit", RESOLUTION_CANDIDATE_LIMIT)
        .with_detail("complete", false)
        .with_detail("retry_refs", json!(retry_refs));
    if let [candidate] = candidates {
        error = error.with_detail("selected_ref_retry", candidate.reference.clone());
    }
    error
}

fn stop_candidate(stop: Stop, input_ref: &str) -> Result<LocationCandidate, AppError> {
    let coordinates = stop.coordinates.ok_or_else(|| {
        AppError::caller(
            "location_coordinates_unavailable",
            "The selected HSL stop exists but has no provider coordinates and cannot be used as a journey endpoint.",
        )
        .with_detail("input_ref", input_ref)
        .with_detail("stop_ref", stop.reference.clone())
        .with_detail("coordinates", json!(null))
    })?;
    Ok(LocationCandidate {
        reference: stop.reference,
        kind: LocationKind::Stop,
        id: Some(stop.id),
        label: stop.name.clone(),
        name: stop.name,
        locality: None,
        neighbourhood: None,
        postal_code: None,
        coordinates,
        source: "gtfshsl".to_owned(),
        source_layer: "stop".to_owned(),
        confidence: None,
        service_area: stop.service_area,
        modes: stop.modes,
    })
}

fn geocoder<'a>(context: &'a HandlerContext<'a>) -> Result<DigitransitGeocoder<'a>, AppError> {
    DigitransitGeocoder::new(
        &context.config.geocoding_url.0,
        subscription_key(context),
        context.transport,
        context.clock,
        timeout(
            context,
            "connect_timeout",
            &context.config.connect_timeout.0,
        )?,
        timeout(
            context,
            "request_timeout",
            &context.config.request_timeout.0,
        )?,
    )
    .map_err(super::provider_error)
}

fn router<'a>(context: &'a HandlerContext<'a>) -> Result<DigitransitRouter<'a>, AppError> {
    DigitransitRouter::new(
        &context.config.routing_url.0,
        subscription_key(context),
        context.transport,
        context.clock,
        timeout(
            context,
            "connect_timeout",
            &context.config.connect_timeout.0,
        )?,
        timeout(
            context,
            "request_timeout",
            &context.config.request_timeout.0,
        )?,
    )
    .map_err(super::provider_error)
}

fn subscription_key(context: &HandlerContext<'_>) -> String {
    context
        .config
        .subscription_key
        .as_ref()
        .expect("dispatcher requires credential before provider handlers")
        .0
        .expose()
        .to_owned()
}

fn timeout(_context: &HandlerContext<'_>, name: &str, value: &str) -> Result<Duration, AppError> {
    Ok(Duration::from_millis(config::validate_duration(
        name, value, None,
    )?))
}

fn requested_kind_matches(requested: RequestedKind, actual: LocationKind) -> bool {
    match requested {
        RequestedKind::Any => true,
        RequestedKind::Address => actual == LocationKind::Address,
        RequestedKind::Venue => actual == LocationKind::Venue,
        RequestedKind::Stop => actual == LocationKind::Stop,
    }
}

fn requested_kind_name(kind: RequestedKind) -> &'static str {
    match kind {
        RequestedKind::Any => "any",
        RequestedKind::Address => "address",
        RequestedKind::Venue => "venue",
        RequestedKind::Stop => "stop",
    }
}

fn location_text(
    query: &str,
    count: usize,
    candidates: &[LocationCandidate],
    source: &ProviderSource,
) -> String {
    let query = crate::command::escape_text(query);
    let mut lines = vec![format!(
        "{count} location candidate{} for “{query}” (Digitransit, retrieved {})",
        if count == 1 { "" } else { "s" },
        source.retrieved_at.format("%Y-%m-%d %H:%M UTC")
    )];
    for (index, candidate) in candidates.iter().enumerate() {
        let modes = if candidate.modes.is_empty() {
            "—".to_owned()
        } else {
            candidate
                .modes
                .iter()
                .map(|mode| format!("{mode:?}").to_lowercase())
                .collect::<Vec<_>>()
                .join(",")
        };
        lines.push(format!(
            "{}  {}  {}  {}",
            index + 1,
            crate::command::escape_text(&candidate.label),
            modes,
            crate::command::escape_text(&candidate.reference)
        ));
    }
    lines.push("Use a returned ref in reitti journey list --from <ref> …".to_owned());
    lines.push(crate::command::escape_text(&source.attribution));
    lines.join("\n")
}

pub(crate) use schema::schema;
