use super::contract;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use reitti_core::*;
use serde::Deserialize;
use serde_json::Value;

const MUNICIPALITIES: &[&str] = &[
    "Helsinki",
    "Espoo",
    "Vantaa",
    "Kauniainen",
    "Kerava",
    "Kirkkonummi",
    "Sipoo",
    "Siuntio",
    "Tuusula",
    "Helsingfors",
    "Esbo",
    "Vanda",
    "Grankulla",
    "Kervo",
    "Kyrkslätt",
    "Sibbo",
    "Sjundeå",
    "Tusby",
];

#[derive(Deserialize)]
struct FeatureCollection {
    features: Vec<Feature>,
}
#[derive(Deserialize)]
struct Feature {
    geometry: PointGeometry,
    properties: Properties,
}
#[derive(Deserialize)]
struct PointGeometry {
    #[serde(rename = "type")]
    kind: String,
    coordinates: Vec<f64>,
}
#[derive(Deserialize)]
struct Properties {
    gid: String,
    label: String,
    name: String,
    layer: String,
    source: String,
    locality: Option<String>,
    neighbourhood: Option<String>,
    #[serde(rename = "postalcode")]
    postal_code: Option<String>,
    confidence: Option<f64>,
    addendum: Option<Addendum>,
}
#[derive(Deserialize)]
struct Addendum {
    #[serde(rename = "GTFS")]
    gtfs: Option<Gtfs>,
}
#[derive(Deserialize)]
struct Gtfs {
    #[serde(default)]
    modes: Vec<String>,
}

pub fn locations(value: &Value) -> Result<Vec<LocationCandidate>, ProviderError> {
    let collection: FeatureCollection =
        serde_json::from_value(value.clone()).map_err(|_| contract("geocoding"))?;
    collection
        .features
        .into_iter()
        .map(|f| {
            if f.geometry.kind != "Point" || f.geometry.coordinates.len() != 2 {
                return Err(contract("geocoding"));
            }
            let coordinates = Coordinates {
                longitude: f.geometry.coordinates[0],
                latitude: f.geometry.coordinates[1],
            };
            if !coordinates.is_valid() {
                return Err(contract("geocoding"));
            }
            if f.properties
                .confidence
                .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
            {
                return Err(contract("geocoding"));
            }
            let modes = sorted_modes(
                f.properties
                    .addendum
                    .and_then(|a| a.gtfs)
                    .map(|g| g.modes)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|v| mode(v))
                    .collect(),
            );
            let kind = match f.properties.layer.as_str() {
                "address" => LocationKind::Address,
                "venue" => LocationKind::Venue,
                "stop" | "station" => LocationKind::Stop,
                "locality" | "localadmin" => LocationKind::Locality,
                _ => LocationKind::Other,
            };
            let service_area = if f.properties.source == "gtfshsl" {
                ServiceArea::Inside
            } else {
                match f.properties.locality.as_deref() {
                    Some(v) if MUNICIPALITIES.contains(&v) => ServiceArea::Inside,
                    Some(_) => ServiceArea::Outside,
                    None => ServiceArea::Unknown,
                }
            };
            Ok(LocationCandidate {
                reference: format!("place:{}", f.properties.gid),
                kind,
                id: Some(f.properties.gid),
                label: f.properties.label,
                name: f.properties.name,
                locality: f.properties.locality,
                neighbourhood: f.properties.neighbourhood,
                postal_code: f.properties.postal_code,
                coordinates,
                source: f.properties.source,
                source_layer: f.properties.layer,
                confidence: f.properties.confidence,
                service_area,
                modes,
            })
        })
        .collect()
}
fn mode(value: &str) -> Option<Mode> {
    match value {
        "BUS" => Some(Mode::Bus),
        "TRAM" => Some(Mode::Tram),
        "RAIL" => Some(Mode::Rail),
        "SUBWAY" => Some(Mode::Subway),
        "FERRY" => Some(Mode::Ferry),
        _ => None,
    }
}
fn sorted_modes(mut modes: Vec<Mode>) -> Vec<Mode> {
    modes.sort();
    modes.dedup();
    modes
}

pub fn stop(value: &Value, distance: Option<f64>) -> Result<Stop, ProviderError> {
    let id = req_str(value, "gtfsId", "stop")?;
    let name = req_str(value, "name", "stop")?;
    let coordinates = coords_optional(value.get("lat"), value.get("lon"), "stop")?;
    let modes = match value
        .get("vehicleMode")
        .and_then(Value::as_str)
        .and_then(mode)
    {
        Some(m) => vec![m],
        None => Vec::new(),
    };
    let wheelchair_boarding = wheelchair(value.get("wheelchairBoarding").and_then(Value::as_str));
    Ok(Stop {
        reference: format!("stop:{id}"),
        id,
        name,
        code: opt_str(value, "code")?,
        platform: opt_str(value, "platformCode")?,
        coordinates,
        distance_m: distance,
        modes,
        wheelchair_boarding,
        service_area: ServiceArea::Inside,
    })
}
fn place(value: &Value) -> Result<Place, ProviderError> {
    let stop_value = value.get("stop");
    let parsed = stop_value
        .filter(|v| !v.is_null())
        .map(|v| stop(v, None))
        .transpose()?;
    Ok(Place {
        name: req_str(value, "name", "plan")?,
        stop_ref: parsed.as_ref().map(|s| s.reference.clone()),
        platform: parsed.as_ref().and_then(|s| s.platform.clone()),
        coordinates: coords_optional(value.get("lat"), value.get("lon"), "plan")?,
        wheelchair_boarding: parsed.map_or(WheelchairBoarding::Unknown, |s| s.wheelchair_boarding),
    })
}
fn wheelchair(value: Option<&str>) -> WheelchairBoarding {
    match value {
        Some("POSSIBLE") => WheelchairBoarding::Accessible,
        Some("NOT_POSSIBLE") => WheelchairBoarding::NotAccessible,
        _ => WheelchairBoarding::Unknown,
    }
}

pub fn alert(value: &Value) -> Result<Alert, ProviderError> {
    let severity_raw = opt_str(value, "alertSeverityLevel")?;
    let (severity, source_severity) = match severity_raw.as_deref() {
        Some("SEVERE") => (AlertSeverity::Severe, None),
        Some("WARNING") => (AlertSeverity::Warning, None),
        Some("INFO") => (AlertSeverity::Info, None),
        Some(v) => (AlertSeverity::Unknown, Some(v.into())),
        None => (AlertSeverity::Unknown, None),
    };
    let effect_raw = opt_str(value, "alertEffect")?;
    let (effect, source_effect) = match effect_raw.as_deref() {
        Some("DETOUR") => (AlertEffect::Detour, None),
        Some("NO_SERVICE") => (AlertEffect::NoService, None),
        Some("REDUCED_SERVICE") => (AlertEffect::ReducedService, None),
        Some("SIGNIFICANT_DELAYS") => (AlertEffect::SignificantDelays, None),
        Some("MODIFIED_SERVICE") => (AlertEffect::ModifiedService, None),
        Some("STOP_MOVED") => (AlertEffect::StopMoved, None),
        Some("OTHER_EFFECT") => (AlertEffect::OtherEffect, None),
        Some(v) => (AlertEffect::Unknown, Some(v.into())),
        None => (AlertEffect::Unknown, None),
    };
    let entities = match value.get("entities").filter(|value| !value.is_null()) {
        Some(value) => value
            .as_array()
            .ok_or_else(|| contract("alerts"))?
            .iter()
            .map(entity)
            .collect::<Result<_, _>>()?,
        // A missing scope is retained explicitly so later relevance filtering
        // treats the alert conservatively instead of silently dropping it.
        None => vec![AlertEntity {
            kind: "unknown".into(),
            id: None,
            route_id: None,
            stop_id: None,
            trip_id: None,
        }],
    };
    Ok(Alert {
        id: req_str(value, "id", "alerts")?,
        header: opt_str(value, "alertHeaderText")?,
        description: value
            .get("alertDescriptionText")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| contract("alerts"))?,
        severity,
        source_severity,
        effect,
        source_effect,
        valid_from: epoch_opt(value.get("effectiveStartDate"), "alerts")?,
        valid_until: epoch_opt(value.get("effectiveEndDate"), "alerts")?,
        entities,
        source_feed: opt_str(value, "feed")?,
    })
}
fn entity(v: &Value) -> Result<AlertEntity, ProviderError> {
    let kind = req_str(v, "__typename", "alerts")?;
    let direct = opt_str(v, "gtfsId")?;
    let route_id = v
        .pointer("/route/gtfsId")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            if kind == "Route" {
                direct.clone()
            } else {
                None
            }
        });
    let stop_id = v
        .pointer("/stop/gtfsId")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| if kind == "Stop" { direct.clone() } else { None });
    let trip_id = v
        .pointer("/trip/gtfsId")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| if kind == "Trip" { direct.clone() } else { None });
    Ok(AlertEntity {
        kind: snake(&kind),
        id: direct,
        route_id,
        stop_id,
        trip_id,
    })
}

pub fn plan(data: &Value, include_geometry: bool) -> Result<PlanResult, ProviderError> {
    let connection = data
        .get("planConnection")
        .ok_or_else(|| contract("NavigationPlan"))?;
    if connection.is_null() {
        return Ok(PlanResult {
            itineraries: vec![],
            routing_errors: vec![],
            complete: true,
            search_date_time: None,
        });
    }
    let routing_errors = connection
        .get("routingErrors")
        .and_then(Value::as_array)
        .ok_or_else(|| contract("NavigationPlan"))?
        .iter()
        .map(|e| {
            Ok(RoutingError {
                code: req_str(e, "code", "NavigationPlan")?,
                description: opt_str(e, "description")?,
                input_field: opt_str(e, "inputField")?,
            })
        })
        .collect::<Result<_, ProviderError>>()?;
    let complete = !connection
        .pointer("/pageInfo/hasNextPage")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut geometry_points = 0usize;
    let itineraries = connection
        .get("edges")
        .and_then(Value::as_array)
        .ok_or_else(|| contract("NavigationPlan"))?
        .iter()
        .enumerate()
        .map(|(index, e)| {
            itinerary(
                e.get("node").ok_or_else(|| contract("NavigationPlan"))?,
                index,
                include_geometry,
                &mut geometry_points,
            )
        })
        .collect::<Result<_, _>>()?;
    Ok(PlanResult {
        itineraries,
        routing_errors,
        complete,
        search_date_time: datetime_opt(connection.get("searchDateTime"), "NavigationPlan")?,
    })
}
fn itinerary(
    v: &Value,
    source_index: usize,
    include_geometry: bool,
    total_points: &mut usize,
) -> Result<Itinerary, ProviderError> {
    let legs = v
        .get("legs")
        .and_then(Value::as_array)
        .ok_or_else(|| contract("NavigationPlan"))?
        .iter()
        .enumerate()
        .map(|(i, l)| leg(l, i, include_geometry, total_points))
        .collect::<Result<Vec<_>, _>>()?;
    let alerts = legs.iter().flat_map(|l| l.alerts.clone()).collect();
    Ok(Itinerary {
        source_index,
        start_time: datetime(v.get("start"), "NavigationPlan")?,
        end_time: datetime(v.get("end"), "NavigationPlan")?,
        duration_seconds: opt_i64(v, "duration")?,
        transfers: req_i64(v, "numberOfTransfers", "NavigationPlan")?
            .try_into()
            .map_err(|_| contract("NavigationPlan"))?,
        walk_seconds: opt_i64(v, "walkTime")?,
        wait_seconds: opt_i64(v, "waitingTime")?,
        walk_distance_m: opt_f64(v, "walkDistance")?,
        legs,
        alerts,
    })
}
fn leg(
    v: &Value,
    index: usize,
    include_geometry: bool,
    total_points: &mut usize,
) -> Result<Leg, ProviderError> {
    let realtime = v.get("realtimeState").and_then(Value::as_str);
    let state = rt_state(realtime);
    let start = evidence(
        v.get("start").ok_or_else(|| contract("NavigationPlan"))?,
        state,
    )?;
    let end = evidence(
        v.get("end").ok_or_else(|| contract("NavigationPlan"))?,
        state,
    )?;
    let calls = v
        .get("stopCalls")
        .and_then(Value::as_array)
        .ok_or_else(|| contract("NavigationPlan"))?
        .iter()
        .map(|c| {
            let loc = c
                .get("stopLocation")
                .ok_or_else(|| contract("NavigationPlan"))?;
            let kind = req_str(loc, "__typename", "NavigationPlan")?;
            Ok(StopCall {
                kind: snake(&kind),
                stop: if kind == "Stop" {
                    Some(stop(loc, None)?)
                } else {
                    None
                },
            })
        })
        .collect::<Result<_, ProviderError>>()?;
    let all_steps = match v.get("steps").filter(|value| !value.is_null()) {
        Some(value) => Some(value.as_array().ok_or_else(|| contract("NavigationPlan"))?),
        None => None,
    };
    let navigation_complete = all_steps.is_some_and(|steps| steps.len() <= 200);
    let steps = all_steps
        .into_iter()
        .flatten()
        .take(200)
        .map(walk_step)
        .collect::<Result<_, _>>()?;
    let geometry = if include_geometry {
        match v.get("legGeometry").filter(|x| !x.is_null()) {
            Some(g) => {
                let expected =
                    g.get("length")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| contract("NavigationPlan"))? as usize;
                let points = req_str(g, "points", "NavigationPlan")?;
                let decoded = decode_polyline(&points, 10_000usize.saturating_sub(*total_points))?;
                if decoded.len() != expected {
                    return Err(contract("NavigationPlan"));
                }
                *total_points += decoded.len();
                Some(Geometry {
                    geometry_type: "LineString".into(),
                    coordinates: decoded,
                })
            }
            None => None,
        }
    } else {
        None
    };
    let route = v
        .get("route")
        .filter(|x| !x.is_null())
        .map(|r| {
            Ok(Route {
                id: req_str(r, "gtfsId", "NavigationPlan")?,
                short_name: opt_str(r, "shortName")?,
                long_name: opt_str(r, "longName")?,
                mode: r.get("mode").and_then(Value::as_str).and_then(mode),
            })
        })
        .transpose()?;
    let alerts = v
        .get("alerts")
        .and_then(Value::as_array)
        .ok_or_else(|| contract("NavigationPlan"))?
        .iter()
        .map(alert)
        .collect::<Result<_, _>>()?;
    Ok(Leg {
        index,
        mode: snake(&req_str(v, "mode", "NavigationPlan")?),
        from: place(v.get("from").ok_or_else(|| contract("NavigationPlan"))?)?,
        to: place(v.get("to").ok_or_else(|| contract("NavigationPlan"))?)?,
        route,
        trip_id: v
            .pointer("/trip/gtfsId")
            .and_then(Value::as_str)
            .map(str::to_owned),
        headsign: opt_str(v, "headsign")?,
        duration_seconds: opt_f64(v, "duration")?,
        distance_m: opt_f64(v, "distance")?,
        start,
        end,
        continues_previous_vehicle: match v.get("interlineWithPreviousLeg") {
            None | Some(Value::Null) => None,
            Some(value) => Some(value.as_bool().ok_or_else(|| contract("NavigationPlan"))?),
        },
        intermediate_stops: calls,
        steps,
        navigation_complete,
        geometry,
        alerts,
        cancelled: matches!(state, RealtimeState::Cancelled),
    })
}
fn walk_step(v: &Value) -> Result<WalkStep, ProviderError> {
    Ok(WalkStep {
        distance_m: opt_f64(v, "distance")?,
        street_name: opt_str(v, "streetName")?,
        generated_name: v.get("bogusName").and_then(Value::as_bool),
        relative_direction: opt_str(v, "relativeDirection")?.map(|v| snake(&v)),
        absolute_direction: opt_str(v, "absoluteDirection")?.map(|v| snake(&v)),
        coordinates: coords_optional(v.get("lat"), v.get("lon"), "NavigationPlan")?,
        area: v.get("area").and_then(Value::as_bool),
        stay_on: v.get("stayOn").and_then(Value::as_bool),
        exit: opt_str(v, "exit")?,
    })
}
fn evidence(v: &Value, state: RealtimeState) -> Result<RealtimeEvidence, ProviderError> {
    let scheduled = datetime(v.get("scheduledTime"), "NavigationPlan")?;
    let estimated = v.get("estimated").filter(|e| !e.is_null());
    let estimated_time = estimated
        .map(|e| datetime(e.get("time"), "NavigationPlan"))
        .transpose()?; /* Derive signed delay from the provider's explicit source timestamps. This safely handles early vehicles and avoids trusting a separately formatted duration. */
    let delay_seconds = estimated_time
        .as_ref()
        .map(|time| time.signed_duration_since(scheduled).num_seconds());
    Ok(RealtimeEvidence {
        state,
        scheduled_time: scheduled,
        estimated_time,
        delay_seconds,
        observed_realtime: estimated.is_some()
            && matches!(
                state,
                RealtimeState::Updated | RealtimeState::Added | RealtimeState::Cancelled
            ),
    })
}

pub fn departure(v: &Value, platform: Option<String>) -> Result<Departure, ProviderError> {
    let service = req_i64(v, "serviceDay", "departures")?;
    let scheduled = req_i64(v, "scheduledDeparture", "departures")?;
    let actual = opt_i64(v, "realtimeDeparture")?;
    let realtime = match v.get("realtime") {
        None | Some(Value::Null) => None,
        Some(value) => Some(value.as_bool().ok_or_else(|| contract("departures"))?),
    };
    let state = match v.get("realtimeState").and_then(Value::as_str) {
        Some(value) => rt_state(Some(value)),
        None if realtime.is_none() => RealtimeState::Unknown,
        None => RealtimeState::Scheduled,
    };
    let service_utc = Utc
        .timestamp_opt(service, 0)
        .single()
        .ok_or_else(|| contract("departures"))?;
    let helsinki: chrono_tz::Tz = "Europe/Helsinki"
        .parse()
        .map_err(|_| contract("departures"))?;
    let service_date = service_utc.with_timezone(&helsinki).date_naive();
    let midnight = helsinki
        .from_local_datetime(
            &service_date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| contract("departures"))?,
        )
        .single()
        .ok_or_else(|| contract("departures"))?;
    let scheduled_dt = midnight
        .checked_add_signed(
            chrono::Duration::try_seconds(scheduled).ok_or_else(|| contract("departures"))?,
        )
        .ok_or_else(|| contract("departures"))?;
    let actual_dt = actual
        .map(|seconds| {
            midnight
                .checked_add_signed(
                    chrono::Duration::try_seconds(seconds).ok_or_else(|| contract("departures"))?,
                )
                .ok_or_else(|| contract("departures"))
        })
        .transpose()?;
    let trip = v.get("trip").ok_or_else(|| contract("departures"))?;
    let route = trip.get("route").ok_or_else(|| contract("departures"))?;
    Ok(Departure {
        trip_id: req_str(trip, "gtfsId", "departures")?,
        route: Route {
            id: req_str(route, "gtfsId", "departures")?,
            short_name: opt_str(route, "shortName")?,
            long_name: opt_str(route, "longName")?,
            mode: route.get("mode").and_then(Value::as_str).and_then(mode),
        },
        headsign: opt_str(v, "headsign")?,
        platform,
        service_date,
        departure: RealtimeEvidence {
            state,
            scheduled_time: scheduled_dt.fixed_offset(),
            estimated_time: if realtime == Some(true) {
                actual_dt.map(|time| time.fixed_offset())
            } else {
                None
            },
            delay_seconds: if realtime == Some(true) {
                actual.and_then(|value| value.checked_sub(scheduled))
            } else {
                None
            },
            observed_realtime: realtime == Some(true),
        },
        cancelled: matches!(state, RealtimeState::Cancelled),
        alerts: vec![],
    })
}

pub fn decode_polyline(value: &str, remaining: usize) -> Result<Vec<[f64; 2]>, ProviderError> {
    if value.len() > 1_000_000 {
        return Err(contract("NavigationPlan"));
    }
    let bytes = value.as_bytes();
    let (mut i, mut lat, mut lon) = (0, 0i64, 0i64);
    let mut out = Vec::new();
    while i < bytes.len() {
        let dlat = decode_component(bytes, &mut i)?;
        let dlon = decode_component(bytes, &mut i)?;
        lat = lat
            .checked_add(dlat)
            .ok_or_else(|| contract("NavigationPlan"))?;
        lon = lon
            .checked_add(dlon)
            .ok_or_else(|| contract("NavigationPlan"))?;
        if out.len() >= remaining {
            return Err(contract("NavigationPlan"));
        }
        let point = [lon as f64 / 1e5, lat as f64 / 1e5];
        if !point.iter().all(|v| v.is_finite())
            || !(-180.0..=180.0).contains(&point[0])
            || !(-90.0..=90.0).contains(&point[1])
        {
            return Err(contract("NavigationPlan"));
        }
        out.push(point);
    }
    Ok(out)
}
fn decode_component(bytes: &[u8], i: &mut usize) -> Result<i64, ProviderError> {
    let (mut result, mut shift) = (0u64, 0u32);
    loop {
        let byte = *bytes.get(*i).ok_or_else(|| contract("NavigationPlan"))?;
        *i += 1;
        if !(63..=126).contains(&byte) || shift > 60 {
            return Err(contract("NavigationPlan"));
        }
        let chunk = (byte - 63) as u64;
        result |= (chunk & 0x1f) << shift;
        if chunk < 0x20 {
            break;
        }
        shift += 5;
    }
    Ok(if result & 1 != 0 {
        !((result >> 1) as i64)
    } else {
        (result >> 1) as i64
    })
}
fn rt_state(v: Option<&str>) -> RealtimeState {
    match v {
        Some("UPDATED") => RealtimeState::Updated,
        Some("CANCELLED") => RealtimeState::Cancelled,
        Some("ADDED") => RealtimeState::Added,
        Some("SCHEDULED") | None => RealtimeState::Scheduled,
        _ => RealtimeState::Unknown,
    }
}
fn req_str(v: &Value, k: &str, op: &'static str) -> Result<String, ProviderError> {
    v.get(k)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| contract(op))
}
fn opt_str(v: &Value, k: &str) -> Result<Option<String>, ProviderError> {
    match v.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => Err(contract("provider")),
    }
}
fn req_i64(v: &Value, k: &str, op: &'static str) -> Result<i64, ProviderError> {
    v.get(k).and_then(Value::as_i64).ok_or_else(|| contract(op))
}
fn opt_i64(v: &Value, k: &str) -> Result<Option<i64>, ProviderError> {
    match v.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(x) => x.as_i64().map(Some).ok_or_else(|| contract("provider")),
    }
}
fn opt_f64(v: &Value, k: &str) -> Result<Option<f64>, ProviderError> {
    match v.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(x) => x
            .as_f64()
            .filter(|x| x.is_finite() && *x >= 0.0)
            .map(Some)
            .ok_or_else(|| contract("provider")),
    }
}
fn coords_optional(
    lat: Option<&Value>,
    lon: Option<&Value>,
    op: &'static str,
) -> Result<Option<Coordinates>, ProviderError> {
    match (lat, lon) {
        (None, None) | (Some(Value::Null), Some(Value::Null)) => Ok(None),
        (Some(a), Some(b)) => {
            let c = Coordinates {
                latitude: a.as_f64().ok_or_else(|| contract(op))?,
                longitude: b.as_f64().ok_or_else(|| contract(op))?,
            };
            if c.is_valid() {
                Ok(Some(c))
            } else {
                Err(contract(op))
            }
        }
        _ => Err(contract(op)),
    }
}
fn epoch_opt(v: Option<&Value>, op: &'static str) -> Result<Option<DateTime<Utc>>, ProviderError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(v) => Utc
            .timestamp_opt(v.as_i64().ok_or_else(|| contract(op))?, 0)
            .single()
            .map(Some)
            .ok_or_else(|| contract(op)),
    }
}
fn datetime(v: Option<&Value>, op: &'static str) -> Result<DateTime<FixedOffset>, ProviderError> {
    v.and_then(Value::as_str)
        .ok_or_else(|| contract(op))
        .and_then(|v| DateTime::parse_from_rfc3339(v).map_err(|_| contract(op)))
}
fn datetime_opt(
    v: Option<&Value>,
    op: &'static str,
) -> Result<Option<DateTime<FixedOffset>>, ProviderError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(_) => datetime(v, op).map(Some),
    }
}
fn snake(v: &str) -> String {
    v.to_ascii_lowercase()
}
