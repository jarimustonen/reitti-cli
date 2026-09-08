mod schema;
use super::{await_provider, HandlerContext};
use crate::{
    command::{self, DepartureListArgs, ModeArg, StopListArgs},
    config,
    digitransit::DigitransitRouter,
    error::AppError,
    output::{CommandOutput, Warning},
};
use chrono::{DateTime, FixedOffset};
use reitti_core::{
    Departure, DepartureRequest, Language, LocationRef, Mode, Router, StopSearch, StopSearchRequest,
};
use serde_json::{json, Value};
use std::time::Duration;

pub fn execute_list(
    context: HandlerContext<'_>,
    args: StopListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    let (search, nearby_coordinates) = match (&args.query, &args.near) {
        (Some(query), None) => (StopSearch::Name(query.clone()), None),
        (None, Some(raw)) => match format!("coord:{raw}").parse::<LocationRef>() {
            Ok(LocationRef::Coordinate(coordinates)) => (
                StopSearch::Nearby {
                    coordinates,
                    radius_m: args.radius_m.unwrap_or(1_000),
                },
                Some(coordinates),
            ),
            _ => {
                return Err(AppError::invalid(
                    "invalid_coordinates",
                    format!(
                        "Invalid --near value '{}'; expected LAT,LON without whitespace.",
                        command::escape_text(raw)
                    ),
                    command::escape_text(raw),
                    "LAT,LON without whitespace",
                ))
            }
        },
        _ => {
            return Err(AppError::caller(
                "usage_error",
                "stop list requires exactly one of --query or --near.",
            ))
        }
    };
    let request_id = context.request_ids.next();
    let router = router(&context)?;
    let result = await_provider(router.search_stops(StopSearchRequest {
        search,
        language,
        limit: args.limit,
    }))?;

    let search_json = if let Some(query) = &args.query {
        json!({"kind":"query","query":query,"coordinates":null,"radius_m":null,"limit":args.limit})
    } else {
        json!({"kind":"near","query":null,"coordinates":nearby_coordinates,"radius_m":args.radius_m.unwrap_or(1_000),"limit":args.limit})
    };
    let count = result.value.len();
    let text = render_stops(&result.value, &search_json, &result.source.attribution);
    CommandOutput::success(
        text,
        json!({
            "search": search_json,
            "count": count,
            "complete": false,
            "stops": result.value,
            "request": request_metadata(request_id, language, &context),
            "source": result.source,
        }),
    )
}

pub fn execute_departures(
    context: HandlerContext<'_>,
    args: DepartureListArgs,
    language: Language,
) -> Result<CommandOutput, AppError> {
    let stop = args
        .stop
        .parse()
        .map_err(|error: reitti_core::ReferenceError| {
            AppError::invalid(
                "invalid_stop_id",
                error.to_string(),
                command::escape_text(&args.stop),
                "raw HSL GTFS ID, for example HSL:1020453",
            )
        })?;
    let (at, time_source) = match &args.at {
        Some(value) => (
            DateTime::parse_from_rfc3339(value).map_err(|_| {
                AppError::invalid(
                    "invalid_datetime",
                    format!(
                        "Invalid --at value '{}'; expected RFC 3339 with an explicit offset.",
                        command::escape_text(value)
                    ),
                    command::escape_text(value),
                    "RFC3339 with explicit offset",
                )
            })?,
            "argument",
        ),
        None => (now_in_timezone(&context)?, "clock"),
    };
    let window_seconds =
        (config::validate_duration("--window", &args.window, Some((60_000, 24 * 60 * 60_000)))?
            / 1_000) as u32;
    let requested_modes = args.mode.iter().map(mode).collect::<Vec<_>>();
    let request_id = context.request_ids.next();
    let router = router(&context)?;
    // Fetch the provider's maximum bounded board once so local mode filtering
    // happens before the caller's output limit without introducing refetches.
    let result = await_provider(router.departures(DepartureRequest {
        stop,
        language,
        at,
        window_seconds,
        limit: 50,
    }))?;
    let Some(mut board) = result.value else {
        return Err(AppError::caller(
            "stop_not_found",
            format!(
                "Stop '{}' was not found in the HSL dataset.",
                command::escape_text(&args.stop)
            ),
        ));
    };

    let provider_count = board.departures.len();
    board.departures.retain(|departure| {
        requested_modes.is_empty()
            || departure
                .route
                .mode
                .is_some_and(|candidate| requested_modes.contains(&candidate))
    });
    board.departures.sort_by(|left, right| {
        operational_time(left)
            .cmp(&operational_time(right))
            .then_with(|| left.trip_id.cmp(&right.trip_id))
    });
    let available_matches = board.departures.len();
    board.departures.truncate(args.limit as usize);
    let count = board.departures.len();
    let display_timezone: chrono_tz::Tz = context.config.timezone.0.parse().map_err(|_| {
        AppError::caller(
            "invalid_timezone",
            "The configured timezone is not a valid IANA timezone.",
        )
    })?;
    let text = render_departures(
        &board.stop.name,
        at,
        &board.departures,
        display_timezone,
        &result.source.attribution,
    );
    let mut warnings = Vec::new();
    if provider_count == 50 {
        warnings.push(Warning {
            code: "bounded_departure_search".into(),
            message: "The provider board was bounded at 50 departures; additional matching departures may exist in the requested window.".into(),
            details: json!({"provider_rows":provider_count,"available_matches":available_matches,"complete":false}),
        });
    }
    if available_matches > args.limit as usize {
        warnings.push(Warning {
            code: "results_truncated".into(),
            message: format!("{} matching departures were available; only {} are included.", available_matches, args.limit),
            details: json!({"available_matches":available_matches,"returned":count,"complete":false}),
        });
    }
    CommandOutput::success(
        text,
        json!({
            "stop": board.stop,
            "at": at,
            "time_source": time_source,
            "window_seconds": window_seconds,
            "count": count,
            "complete": false,
            "departures": board.departures,
            "request": request_metadata(request_id, language, &context),
            "source": result.source,
        }),
    )?
    .with_warnings(warnings)
}

fn router<'a>(context: &'a HandlerContext<'a>) -> Result<DigitransitRouter<'a>, AppError> {
    let key = context
        .config
        .subscription_key
        .as_ref()
        .ok_or_else(AppError::credential_missing)?
        .0
        .expose()
        .to_owned();
    let connect =
        config::validate_duration("connect_timeout", &context.config.connect_timeout.0, None)?;
    let request =
        config::validate_duration("request_timeout", &context.config.request_timeout.0, None)?;
    await_provider(async {
        DigitransitRouter::new(
            &context.config.routing_url.0,
            key,
            context.transport,
            context.clock,
            Duration::from_millis(connect),
            Duration::from_millis(request),
        )
    })
}

fn now_in_timezone(context: &HandlerContext<'_>) -> Result<DateTime<FixedOffset>, AppError> {
    let timezone: chrono_tz::Tz = context.config.timezone.0.parse().map_err(|_| {
        AppError::caller(
            "invalid_timezone",
            "The configured timezone is not a valid IANA timezone.",
        )
    })?;
    Ok(context.clock.now().with_timezone(&timezone).fixed_offset())
}

fn mode(value: &ModeArg) -> Mode {
    match value {
        ModeArg::Bus => Mode::Bus,
        ModeArg::Tram => Mode::Tram,
        ModeArg::Rail => Mode::Rail,
        ModeArg::Subway => Mode::Subway,
        ModeArg::Ferry => Mode::Ferry,
    }
}

fn operational_time(departure: &Departure) -> DateTime<FixedOffset> {
    departure
        .departure
        .estimated_time
        .unwrap_or(departure.departure.scheduled_time)
}

fn request_metadata(id: String, language: Language, context: &HandlerContext<'_>) -> Value {
    json!({"request_id":id,"language":language.code(),"timezone":context.config.timezone.0})
}

fn render_stops(stops: &[reitti_core::Stop], search: &Value, attribution: &str) -> String {
    let heading = if search["kind"] == "near" {
        format!("{} stops within {} m", stops.len(), search["radius_m"])
    } else {
        format!(
            "{} stops matching ‘{}’",
            stops.len(),
            command::escape_text(search["query"].as_str().unwrap_or(""))
        )
    };
    let rows = stops
        .iter()
        .map(|stop| {
            let modes = if stop.modes.is_empty() {
                "—".into()
            } else {
                stop.modes
                    .iter()
                    .map(|mode| format!("{mode:?}").to_lowercase())
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let platform = stop
                .platform
                .as_deref()
                .map(|value| format!(" platform {}", command::escape_text(value)))
                .unwrap_or_default();
            let distance = stop
                .distance_m
                .map(|value| format!(" · {:.0} m", value))
                .unwrap_or_default();
            format!(
                "{}  {}{}  {}{}  {}",
                stop.id,
                command::escape_text(&stop.name),
                platform,
                modes,
                distance,
                stop.reference
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if rows.is_empty() {
        format!("{heading}\n{}", command::escape_text(attribution))
    } else {
        format!("{heading}\n{rows}\n{}", command::escape_text(attribution))
    }
}

fn render_departures(
    name: &str,
    at: DateTime<FixedOffset>,
    departures: &[Departure],
    display_timezone: chrono_tz::Tz,
    attribution: &str,
) -> String {
    let mut lines = vec![format!(
        "{} · {} departures from {}",
        command::escape_text(name),
        departures.len(),
        at.format("%Y-%m-%d %H:%M %:z")
    )];
    for departure in departures {
        let time = operational_time(departure)
            .with_timezone(&display_timezone)
            .format("%Y-%m-%d %H:%M %:z");
        let route = departure
            .route
            .short_name
            .as_deref()
            .unwrap_or(&departure.route.id);
        let headsign = departure
            .headsign
            .as_deref()
            .unwrap_or("destination unavailable");
        let marker = if departure.departure.estimated_time.is_some() {
            "est"
        } else {
            "sched"
        };
        let delay = match departure.departure.delay_seconds {
            Some(seconds) if seconds < 0 => format!(" · {}s early", seconds.unsigned_abs()),
            Some(seconds) if seconds > 0 => format!(" · {seconds}s late"),
            Some(_) => " · on time".to_owned(),
            None => String::new(),
        };
        let evidence = format!("{:?}", departure.departure.state).to_lowercase();
        lines.push(format!(
            "{time} {marker}  {}  {}{delay}  {evidence}",
            command::escape_text(route),
            command::escape_text(headsign)
        ));
    }
    lines.push(command::escape_text(attribution));
    lines.join("\n")
}

pub(crate) use schema::{departure_schema, stop_schema};
