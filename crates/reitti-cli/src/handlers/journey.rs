mod schema;

use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use reitti_core::{
    Alert, Geometry, Itinerary, Language, Leg, Mode, Place, PlanEndpoint, PlanRequest, PlanTime,
    ProviderSource, RealtimeEvidence, RealtimeState, Route, Router, StopCall, WalkStep,
    WheelchairBoarding,
};
use serde::Serialize;
use serde_json::{json, Value};

use super::{await_provider, location, HandlerContext};
use crate::{
    command::{JourneyListArgs, ModeArg},
    config,
    digitransit::DigitransitRouter,
    error::AppError,
    output::{CommandOutput, Warning},
};

const DEFAULT_MODES: [Mode; 5] = [Mode::Bus, Mode::Tram, Mode::Rail, Mode::Subway, Mode::Ferry];

#[derive(Serialize)]
struct JourneyData {
    request: RequestMetadata,
    resolved: ResolvedEndpoints,
    count: usize,
    complete: bool,
    filtering: Filtering,
    alternatives: Vec<Alternative>,
    sources: Vec<ProviderSource>,
}

#[derive(Serialize)]
struct RequestMetadata {
    request_id: String,
    language: &'static str,
    timezone: String,
    from: String,
    to: String,
    time: RequestTime,
    modes: Vec<Mode>,
    max_walk_m: Option<u32>,
    wheelchair: bool,
    include_geometry: bool,
    limit: u8,
}

#[derive(Serialize)]
struct RequestTime {
    kind: &'static str,
    value: String,
    time_source: &'static str,
}

#[derive(Serialize)]
struct ResolvedEndpoints {
    from: location::ResolvedLocation,
    to: location::ResolvedLocation,
}

#[derive(Serialize)]
struct Filtering {
    provider_count: usize,
    returned_count: usize,
    max_walk_m: Option<u32>,
    excluded_over_limit: usize,
    excluded_over_cap: usize,
    excluded_unknown_distance: usize,
}

#[derive(Serialize)]
struct Alternative {
    id: String,
    source_index: usize,
    start_time: DateTime<FixedOffset>,
    end_time: DateTime<FixedOffset>,
    duration_seconds: Option<i64>,
    transfers: i32,
    walk_seconds: Option<i64>,
    wait_seconds: Option<i64>,
    transit_seconds: Option<i64>,
    walk_distance_m: Option<f64>,
    accessibility: Accessibility,
    realtime: RealtimeSummary,
    comparison: Vec<Comparison>,
    alerts: Vec<Alert>,
    fare: Option<Value>,
    legs: Vec<JourneyLeg>,
}

#[derive(Serialize)]
struct Accessibility {
    wheelchair_requested: bool,
    status: &'static str,
    evidence: Vec<AccessibilityEvidence>,
}

#[derive(Serialize)]
struct AccessibilityEvidence {
    leg_index: usize,
    endpoint: &'static str,
    wheelchair_boarding: WheelchairBoarding,
}

#[derive(Serialize)]
struct RealtimeSummary {
    status: &'static str,
    updated_legs: usize,
    scheduled_only_legs: usize,
    cancelled_legs: usize,
    unknown_legs: usize,
}

#[derive(Serialize)]
struct Comparison {
    label: &'static str,
    metric: &'static str,
    value: Value,
    tied: usize,
}

#[derive(Serialize)]
struct JourneyLeg {
    index: usize,
    mode: String,
    from: Place,
    to: Place,
    route: Option<Route>,
    trip_id: Option<String>,
    headsign: Option<String>,
    duration_seconds: Option<f64>,
    distance_m: Option<f64>,
    start: RealtimeEvidence,
    end: RealtimeEvidence,
    continues_previous_vehicle: Option<bool>,
    intermediate_stops: Vec<StopCall>,
    steps: Vec<JourneyStep>,
    navigation_complete: bool,
    geometry: Option<Geometry>,
    alerts: Vec<Alert>,
    cancelled: bool,
}

#[derive(Serialize)]
struct JourneyStep {
    instruction: Option<String>,
    distance_m: Option<f64>,
    street_name: Option<String>,
    generated_name: Option<bool>,
    relative_direction: Option<String>,
    absolute_direction: Option<String>,
    coordinates: Option<reitti_core::Coordinates>,
    area: Option<bool>,
    stay_on: Option<bool>,
    exit: Option<String>,
}

pub fn execute(
    context: HandlerContext<'_>,
    args: JourneyListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    // The dispatcher validates grammar before credential loading or provider access.
    let from_ref = args.from.parse().expect("dispatcher validated --from");
    let to_ref = args.to.parse().expect("dispatcher validated --to");
    let from = location::resolve(&context, &args.from, from_ref, language)?;
    let to = location::resolve(&context, &args.to, to_ref, language)?;

    let (plan_time, request_time, arrive_by) = request_time(&context, &args)?;
    let modes = requested_modes(&args.mode);
    let router = router(&context)?;
    let planned = await_provider(router.plan(PlanRequest {
        from: PlanEndpoint {
            coordinates: from.selected.candidate.coordinates,
            label: Some(from.selected.candidate.label.clone()),
        },
        to: PlanEndpoint {
            coordinates: to.selected.candidate.coordinates,
            label: Some(to.selected.candidate.label.clone()),
        },
        language,
        limit: args.limit,
        time: plan_time,
        modes: modes.clone(),
        wheelchair: args.wheelchair,
        include_geometry: args.include_geometry,
    }))?;

    let mut sources = sources_in_request_order([from.source, to.source]);
    sources.push(planned.source);
    let provider_count = planned.value.itineraries.len();
    let excluded_over_limit = provider_count.saturating_sub(args.limit as usize);
    let routing_errors = planned.value.routing_errors.clone();
    let provider_complete = planned.value.complete;
    if provider_count == 0 {
        let message = if routing_errors.is_empty() {
            "Digitransit returned no journey alternatives. Try another time, mode, or endpoint."
        } else {
            "Digitransit returned routing domain errors and no journey alternatives. Correct the reported route inputs or try another search."
        };
        return Err(AppError::caller("no_journeys", message)
            .with_detail("from", json!(&from.selected))
            .with_detail("to", json!(&to.selected))
            .with_detail("routing_errors", json!(routing_errors))
            .with_detail(
                "suggestions",
                json!(["try another time", "change modes", "verify endpoint refs"]),
            )
            .with_detail("sources", json!(sources)));
    }

    let mut excluded_over_cap = 0;
    let mut excluded_unknown_distance = 0;
    let filtered = planned
        .value
        .itineraries
        .into_iter()
        .take(args.limit as usize)
        .filter(|itinerary| match args.max_walk_m {
            None => true,
            Some(cap) => match itinerary.walk_distance_m {
                Some(distance) if distance <= f64::from(cap) => true,
                Some(_) => {
                    excluded_over_cap += 1;
                    false
                }
                None => {
                    excluded_unknown_distance += 1;
                    false
                }
            },
        })
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        return Err(AppError::caller(
            "no_matching_journeys",
            "None of the bounded alternatives returned by Digitransit could be shown to meet --max-walk-m; other routes may exist.",
        )
        .with_detail("provider_count", provider_count)
        .with_detail("max_walk_m", args.max_walk_m.map_or(Value::Null, |v| json!(v)))
        .with_detail("excluded_over_limit", excluded_over_limit)
        .with_detail("excluded_over_cap", excluded_over_cap)
        .with_detail("excluded_unknown_distance", excluded_unknown_distance)
        .with_detail("global_route_absence_proven", false)
        .with_detail("sources", json!(sources)));
    }

    let mut alternatives = filtered
        .into_iter()
        .enumerate()
        .map(|(returned_index, itinerary)| alternative(returned_index, itinerary, args.wheelchair))
        .collect::<Vec<_>>();
    let mut warnings = vec![Warning {
        code: "fare_unavailable".into(),
        message: "Routing v2 returned no verified fare data; fare is null.".into(),
        details: json!({}),
    }];
    if !routing_errors.is_empty() {
        warnings.push(Warning {
            code: "routing_errors".into(),
            message: "Digitransit returned usable alternatives together with routing domain errors; the alternatives are preserved without treating those errors as a transport failure.".into(),
            details: json!({"routing_errors":routing_errors}),
        });
    }
    assign_comparisons(&mut alternatives, arrive_by, &mut warnings);
    if excluded_over_limit > 0 {
        warnings.push(Warning {
            code: "alternatives_truncated".into(),
            message: "Digitransit returned more alternatives than requested; extra alternatives were omitted in provider order before comparison.".into(),
            details: json!({"provider_count":provider_count,"limit":args.limit,"excluded_over_limit":excluded_over_limit}),
        });
    }
    if excluded_over_cap + excluded_unknown_distance > 0 {
        warnings.push(Warning {
            code: "alternatives_filtered".into(),
            message: "Some bounded provider alternatives were excluded by the local walking-distance cap; other routes may exist.".into(),
            details: json!({"excluded_over_cap":excluded_over_cap,"excluded_unknown_distance":excluded_unknown_distance}),
        });
    }
    let incomplete_legs = alternatives
        .iter()
        .flat_map(|alternative| &alternative.legs)
        .filter(|leg| !leg.navigation_complete)
        .count();
    if incomplete_legs > 0 {
        warnings.push(Warning {
            code: "navigation_truncated".into(),
            message: "Some navigation guidance is incomplete because source steps were absent or bounded geometry/step caps were reached.".into(),
            details: json!({"incomplete_legs":incomplete_legs}),
        });
    }

    let display_timezone: chrono_tz::Tz = context.config.timezone.0.parse().map_err(|_| {
        AppError::caller(
            "invalid_timezone",
            "The configured timezone is not a valid IANA timezone.",
        )
    })?;
    let complete = provider_complete
        && routing_errors.is_empty()
        && excluded_over_limit == 0
        && excluded_over_cap == 0
        && excluded_unknown_distance == 0;
    let text = journey_text(
        &alternatives,
        &from.selected.candidate.label,
        &to.selected.candidate.label,
        &request_time,
        display_timezone,
        sources.last().expect("plan source retained"),
    );
    let count = alternatives.len();
    CommandOutput::success(
        text,
        JourneyData {
            request: RequestMetadata {
                request_id: context.request_ids.next(),
                language: language.code(),
                timezone: context.config.timezone.0.clone(),
                from: args.from,
                to: args.to,
                time: request_time,
                modes,
                max_walk_m: args.max_walk_m,
                wheelchair: args.wheelchair,
                include_geometry: args.include_geometry,
                limit: args.limit,
            },
            resolved: ResolvedEndpoints {
                from: from.selected,
                to: to.selected,
            },
            count,
            complete,
            filtering: Filtering {
                provider_count,
                returned_count: count,
                max_walk_m: args.max_walk_m,
                excluded_over_limit,
                excluded_over_cap,
                excluded_unknown_distance,
            },
            alternatives,
            sources,
        },
    )?
    .with_warnings(warnings)
}

fn request_time(
    context: &HandlerContext<'_>,
    args: &JourneyListArgs,
) -> Result<(PlanTime, RequestTime, bool), AppError> {
    if let Some(literal) = &args.depart_at {
        let parsed = DateTime::parse_from_rfc3339(literal).expect("dispatcher validated datetime");
        return Ok((
            PlanTime::DepartAt(parsed),
            RequestTime {
                kind: "depart_at",
                value: literal.clone(),
                time_source: "argument",
            },
            false,
        ));
    }
    if let Some(literal) = &args.arrive_by {
        let parsed = DateTime::parse_from_rfc3339(literal).expect("dispatcher validated datetime");
        return Ok((
            PlanTime::ArriveBy(parsed),
            RequestTime {
                kind: "arrive_by",
                value: literal.clone(),
                time_source: "argument",
            },
            true,
        ));
    }
    let timezone: chrono_tz::Tz = context.config.timezone.0.parse().map_err(|_| {
        AppError::caller(
            "invalid_timezone",
            "The configured timezone is not a valid IANA timezone.",
        )
    })?;
    let value = context.clock.now().with_timezone(&timezone).fixed_offset();
    Ok((
        PlanTime::DepartAt(value),
        RequestTime {
            kind: "depart_at",
            value: value.to_rfc3339(),
            time_source: "clock",
        },
        false,
    ))
}

fn requested_modes(requested: &[ModeArg]) -> Vec<Mode> {
    if requested.is_empty() {
        return DEFAULT_MODES.to_vec();
    }
    requested
        .iter()
        .map(|mode| match mode {
            ModeArg::Bus => Mode::Bus,
            ModeArg::Tram => Mode::Tram,
            ModeArg::Rail => Mode::Rail,
            ModeArg::Subway => Mode::Subway,
            ModeArg::Ferry => Mode::Ferry,
        })
        .collect()
}

fn alternative(
    returned_index: usize,
    itinerary: Itinerary,
    wheelchair_requested: bool,
) -> Alternative {
    let transit_seconds = match (
        itinerary.duration_seconds,
        itinerary.walk_seconds,
        itinerary.wait_seconds,
    ) {
        (Some(duration), Some(walk), Some(wait)) => duration
            .checked_sub(walk)
            .and_then(|value| value.checked_sub(wait))
            .filter(|value| *value >= 0),
        _ => None,
    };
    let accessibility = accessibility(&itinerary, wheelchair_requested);
    let realtime = realtime(&itinerary.legs);
    let legs = itinerary.legs.into_iter().map(journey_leg).collect();
    Alternative {
        id: format!("alt-{}", returned_index + 1),
        source_index: itinerary.source_index,
        start_time: itinerary.start_time,
        end_time: itinerary.end_time,
        duration_seconds: itinerary.duration_seconds,
        transfers: itinerary.transfers,
        walk_seconds: itinerary.walk_seconds,
        wait_seconds: itinerary.wait_seconds,
        transit_seconds,
        walk_distance_m: itinerary.walk_distance_m,
        accessibility,
        realtime,
        comparison: Vec::new(),
        alerts: itinerary.alerts,
        fare: None,
        legs,
    }
}

fn accessibility(itinerary: &Itinerary, wheelchair_requested: bool) -> Accessibility {
    let mut evidence = Vec::new();
    for leg in &itinerary.legs {
        for (endpoint, value) in [
            ("from", leg.from.wheelchair_boarding),
            ("to", leg.to.wheelchair_boarding),
        ] {
            if value != WheelchairBoarding::Unknown {
                evidence.push(AccessibilityEvidence {
                    leg_index: leg.index,
                    endpoint,
                    wheelchair_boarding: value,
                });
            }
        }
    }
    // Stop boarding facts and a wheelchair-aware search are useful evidence,
    // but neither proves the complete journey accessible.
    Accessibility {
        wheelchair_requested,
        status: "unknown",
        evidence,
    }
}

fn realtime(legs: &[Leg]) -> RealtimeSummary {
    let mut summary = RealtimeSummary {
        status: "unknown",
        updated_legs: 0,
        scheduled_only_legs: 0,
        cancelled_legs: 0,
        unknown_legs: 0,
    };
    // Access/egress walking times remain on each leg, but do not downgrade
    // otherwise-updated transit evidence to a misleading mixed summary.
    for leg in legs.iter().filter(|leg| leg.mode != "walk") {
        match leg.start.state {
            RealtimeState::Cancelled => summary.cancelled_legs += 1,
            RealtimeState::Updated | RealtimeState::Added => summary.updated_legs += 1,
            RealtimeState::Scheduled => summary.scheduled_only_legs += 1,
            RealtimeState::Unknown => summary.unknown_legs += 1,
        }
    }
    let transit_count = summary.updated_legs
        + summary.scheduled_only_legs
        + summary.cancelled_legs
        + summary.unknown_legs;
    summary.status = if transit_count == 0 {
        "unknown"
    } else if summary.cancelled_legs > 0 {
        "cancelled"
    } else if summary.unknown_legs > 0 {
        "unknown"
    } else if summary.updated_legs > 0 && summary.scheduled_only_legs > 0 {
        "mixed"
    } else if summary.updated_legs > 0 {
        "updated"
    } else {
        "scheduled_only"
    };
    summary
}

fn journey_leg(leg: Leg) -> JourneyLeg {
    let steps = leg.steps.iter().map(journey_step).collect();
    JourneyLeg {
        index: leg.index,
        mode: leg.mode,
        from: leg.from,
        to: leg.to,
        route: leg.route,
        trip_id: leg.trip_id,
        headsign: leg.headsign,
        duration_seconds: leg.duration_seconds,
        distance_m: leg.distance_m,
        start: leg.start,
        end: leg.end,
        continues_previous_vehicle: leg.continues_previous_vehicle,
        intermediate_stops: leg.intermediate_stops,
        steps,
        navigation_complete: leg.navigation_complete,
        geometry: leg.geometry,
        alerts: leg.alerts,
        cancelled: leg.cancelled,
    }
}

fn journey_step(step: &WalkStep) -> JourneyStep {
    let trusted_street = step
        .street_name
        .as_ref()
        .filter(|_| step.generated_name != Some(true));
    let instruction = match (&step.relative_direction, trusted_street) {
        (Some(direction), Some(street)) => {
            Some(format!("{} on {street}", display_direction(direction)))
        }
        (Some(direction), None) => Some(display_direction(direction)),
        (None, Some(street)) => Some(format!("continue on {street}")),
        (None, None) => None,
    };
    JourneyStep {
        instruction,
        distance_m: step.distance_m,
        // Preserve the raw source fact even when bogusName marks it generated;
        // only derived/person-facing guidance suppresses that authority.
        street_name: step.street_name.clone(),
        generated_name: step.generated_name,
        relative_direction: step.relative_direction.clone(),
        absolute_direction: step.absolute_direction.clone(),
        coordinates: step.coordinates,
        area: step.area,
        stay_on: step.stay_on,
        exit: step.exit.clone(),
    }
}

fn display_direction(direction: &str) -> String {
    direction.replace('_', " ")
}

fn assign_comparisons(
    alternatives: &mut [Alternative],
    arrive_by: bool,
    warnings: &mut Vec<Warning>,
) {
    if alternatives
        .iter()
        .all(|alternative| alternative.duration_seconds.is_some())
    {
        let best = alternatives
            .iter()
            .filter_map(|a| a.duration_seconds)
            .min()
            .unwrap();
        label_i64(alternatives, "fastest", "duration_seconds", best, |a| {
            a.duration_seconds
        });
    } else {
        comparison_warning(warnings, "fastest", "duration_seconds");
    }
    let earliest = alternatives.iter().map(|a| a.end_time).min().unwrap();
    let tied = alternatives
        .iter()
        .filter(|a| a.end_time == earliest)
        .count();
    for alternative in alternatives.iter_mut().filter(|a| a.end_time == earliest) {
        alternative.comparison.push(Comparison {
            label: "earliest_arrival",
            metric: "end_time",
            value: json!(earliest),
            tied,
        });
    }
    if arrive_by {
        let latest = alternatives.iter().map(|a| a.start_time).max().unwrap();
        let tied = alternatives
            .iter()
            .filter(|a| a.start_time == latest)
            .count();
        for alternative in alternatives.iter_mut().filter(|a| a.start_time == latest) {
            alternative.comparison.push(Comparison {
                label: "latest_departure",
                metric: "start_time",
                value: json!(latest),
                tied,
            });
        }
    }
    let fewest = alternatives.iter().map(|a| a.transfers).min().unwrap();
    let tied = alternatives
        .iter()
        .filter(|a| a.transfers == fewest)
        .count();
    for alternative in alternatives.iter_mut().filter(|a| a.transfers == fewest) {
        alternative.comparison.push(Comparison {
            label: "fewest_transfers",
            metric: "transfers",
            value: json!(fewest),
            tied,
        });
    }
    if alternatives
        .iter()
        .all(|alternative| alternative.walk_distance_m.is_some())
    {
        let least = alternatives
            .iter()
            .filter_map(|a| a.walk_distance_m)
            .min_by(f64::total_cmp)
            .unwrap();
        let tied = alternatives
            .iter()
            .filter(|a| a.walk_distance_m == Some(least))
            .count();
        for alternative in alternatives
            .iter_mut()
            .filter(|a| a.walk_distance_m == Some(least))
        {
            alternative.comparison.push(Comparison {
                label: "least_walking",
                metric: "walk_distance_m",
                value: json!(least),
                tied,
            });
        }
    } else {
        comparison_warning(warnings, "least_walking", "walk_distance_m");
    }
}

fn label_i64(
    alternatives: &mut [Alternative],
    label: &'static str,
    metric: &'static str,
    best: i64,
    value: impl Fn(&Alternative) -> Option<i64>,
) {
    let tied = alternatives
        .iter()
        .filter(|a| value(a) == Some(best))
        .count();
    for alternative in alternatives.iter_mut().filter(|a| value(a) == Some(best)) {
        alternative.comparison.push(Comparison {
            label,
            metric,
            value: json!(best),
            tied,
        });
    }
}

fn comparison_warning(warnings: &mut Vec<Warning>, label: &str, metric: &str) {
    warnings.push(Warning {
        code: "comparison_incomplete".into(),
        message: format!("The {label} label was omitted because {metric} is unknown for at least one returned alternative."),
        details: json!({"label":label,"metric":metric}),
    });
}

fn journey_text(
    alternatives: &[Alternative],
    from: &str,
    to: &str,
    request_time: &RequestTime,
    timezone: chrono_tz::Tz,
    source: &ProviderSource,
) -> String {
    let request = DateTime::parse_from_rfc3339(&request_time.value)
        .expect("request metadata contains datetime")
        .with_timezone(&timezone);
    let mut lines = vec![format!(
        "{} alternative{} · {} → {} · {} {}",
        alternatives.len(),
        if alternatives.len() == 1 { "" } else { "s" },
        crate::command::escape_text(from),
        crate::command::escape_text(to),
        if request_time.kind == "arrive_by" {
            "arrive by"
        } else {
            "depart"
        },
        request.format("%Y-%m-%d %H:%M %:z")
    )];
    for (index, alternative) in alternatives.iter().enumerate() {
        let start = alternative
            .start_time
            .with_timezone(&timezone)
            .format("%H:%M");
        let end = alternative
            .end_time
            .with_timezone(&timezone)
            .format("%H:%M");
        let duration = alternative
            .duration_seconds
            .map(format_duration)
            .unwrap_or_else(|| "unknown duration".into());
        let walking = alternative
            .walk_distance_m
            .map(|v| format!("{v:.0} m walk"))
            .unwrap_or_else(|| "walk unknown".into());
        let labels = alternative
            .comparison
            .iter()
            .map(|fact| fact.label.replace('_', " "))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "{}  {start}–{end}  {duration}  {} transfer{}  {walking}{}",
            index + 1,
            alternative.transfers,
            if alternative.transfers == 1 { "" } else { "s" },
            if labels.is_empty() {
                String::new()
            } else {
                format!("  {labels}")
            }
        ));
    }
    for (index, alternative) in alternatives.iter().enumerate() {
        lines.push(String::new());
        lines.push(format!("Alternative {}", index + 1));
        for leg in &alternative.legs {
            let route = leg
                .route
                .as_ref()
                .and_then(|route| route.short_name.as_deref())
                .unwrap_or(&leg.mode);
            let start = evidence_text(&leg.start, timezone);
            let end = evidence_text(&leg.end, timezone);
            let continuation = if leg.continues_previous_vehicle == Some(true) {
                " · stay on vehicle"
            } else {
                ""
            };
            let headsign = leg
                .headsign
                .as_ref()
                .map(|value| format!(" toward {}", crate::command::escape_text(value)))
                .unwrap_or_default();
            let from_platform = leg
                .from
                .platform
                .as_ref()
                .map(|value| format!(" platform {}", crate::command::escape_text(value)))
                .unwrap_or_default();
            let to_platform = leg
                .to
                .platform
                .as_ref()
                .map(|value| format!(" platform {}", crate::command::escape_text(value)))
                .unwrap_or_default();
            let cancelled = if leg.cancelled { " · CANCELLED" } else { "" };
            lines.push(format!(
                "  {}{headsign} · {}{from_platform} {start} → {}{to_platform} {end}{continuation}{cancelled}",
                crate::command::escape_text(route),
                crate::command::escape_text(&leg.from.name),
                crate::command::escape_text(&leg.to.name)
            ));
            for step in &leg.steps {
                if let Some(instruction) = &step.instruction {
                    lines.push(format!(
                        "    {}{}",
                        crate::command::escape_text(instruction),
                        step.distance_m
                            .map(|distance| format!(" ({distance:.0} m)"))
                            .unwrap_or_default()
                    ));
                }
            }
        }
        lines.push(format!(
            "  Realtime: {}. Accessibility: unknown{}.",
            alternative.realtime.status.replace('_', " "),
            if alternative.accessibility.wheelchair_requested {
                " (wheelchair-aware search requested)"
            } else {
                ""
            }
        ));
        if !alternative.alerts.is_empty() {
            lines.push(format!(
                "  {} relevant service alert{}:",
                alternative.alerts.len(),
                if alternative.alerts.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ));
            for alert in &alternative.alerts {
                let summary = alert
                    .header
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or(&alert.description);
                lines.push(format!(
                    "    {}: {}",
                    crate::command::escape_text(&alert.id),
                    crate::command::escape_text(summary)
                ));
            }
        }
    }
    lines.push(source.attribution.clone());
    lines.join("\n")
}

fn evidence_text(evidence: &RealtimeEvidence, timezone: chrono_tz::Tz) -> String {
    let scheduled = evidence
        .scheduled_time
        .with_timezone(&timezone)
        .format("%H:%M");
    match evidence.estimated_time {
        Some(estimated) => {
            let estimated = estimated.with_timezone(&timezone).format("%H:%M");
            let delta = match evidence.delay_seconds {
                Some(value) if value < 0 => {
                    format!(" · {} early", format_duration(value.unsigned_abs() as i64))
                }
                Some(value) if value > 0 => format!(" · {} delayed", format_duration(value)),
                Some(_) => " · on time".to_owned(),
                None => String::new(),
            };
            format!("{estimated} est ({scheduled} sched{delta})")
        }
        None => format!("{scheduled} sched"),
    }
}

fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}h{minutes:02}m")
    } else if seconds > 0 {
        format!("{minutes}m{seconds:02}s")
    } else {
        format!("{minutes}m")
    }
}

fn sources_in_request_order(
    sources: impl IntoIterator<Item = Option<ProviderSource>>,
) -> Vec<ProviderSource> {
    sources.into_iter().flatten().collect()
}

fn router<'a>(context: &'a HandlerContext<'a>) -> Result<DigitransitRouter<'a>, AppError> {
    DigitransitRouter::new(
        &context.config.routing_url.0,
        context
            .config
            .subscription_key
            .as_ref()
            .expect("credential required")
            .0
            .expose()
            .to_owned(),
        context.transport,
        context.clock,
        timeout("connect_timeout", &context.config.connect_timeout.0)?,
        timeout("request_timeout", &context.config.request_timeout.0)?,
    )
    .map_err(super::provider_error)
}

fn timeout(name: &str, value: &str) -> Result<Duration, AppError> {
    Ok(Duration::from_millis(config::validate_duration(
        name, value, None,
    )?))
}

pub(crate) use schema::schema;
