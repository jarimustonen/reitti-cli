use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Fi,
    Sv,
}
impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fi => "fi",
            Self::Sv => "sv",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Bus,
    Tram,
    Rail,
    Subway,
    Ferry,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}
impl Coordinates {
    pub fn is_valid(self) -> bool {
        self.latitude.is_finite()
            && self.longitude.is_finite()
            && (-90.0..=90.0).contains(&self.latitude)
            && (-180.0..=180.0).contains(&self.longitude)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceArea {
    Inside,
    Outside,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationKind {
    Address,
    Venue,
    Stop,
    Locality,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationCandidate {
    #[serde(rename = "ref")]
    pub reference: String,
    pub kind: LocationKind,
    pub id: Option<String>,
    pub label: String,
    pub name: String,
    pub locality: Option<String>,
    pub neighbourhood: Option<String>,
    pub postal_code: Option<String>,
    pub coordinates: Coordinates,
    pub source: String,
    pub source_layer: String,
    pub confidence: Option<f64>,
    pub service_area: ServiceArea,
    pub modes: Vec<Mode>,
}
pub type Location = LocationCandidate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WheelchairBoarding {
    Accessible,
    NotAccessible,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    #[serde(rename = "ref")]
    pub reference: String,
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    pub platform: Option<String>,
    pub coordinates: Option<Coordinates>,
    pub distance_m: Option<f64>,
    pub modes: Vec<Mode>,
    pub wheelchair_boarding: WheelchairBoarding,
    pub service_area: ServiceArea,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    pub id: String,
    pub short_name: Option<String>,
    pub long_name: Option<String>,
    pub mode: Option<Mode>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeState {
    Scheduled,
    Updated,
    Cancelled,
    Added,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealtimeEvidence {
    pub state: RealtimeState,
    pub scheduled_time: DateTime<FixedOffset>,
    pub estimated_time: Option<DateTime<FixedOffset>>,
    pub delay_seconds: Option<i64>,
    pub observed_realtime: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    pub stop_ref: Option<String>,
    pub platform: Option<String>,
    pub coordinates: Option<Coordinates>,
    pub wheelchair_boarding: WheelchairBoarding,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopCall {
    pub kind: String,
    pub stop: Option<Stop>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WalkStep {
    pub distance_m: Option<f64>,
    pub street_name: Option<String>,
    pub generated_name: Option<bool>,
    pub relative_direction: Option<String>,
    pub absolute_direction: Option<String>,
    pub coordinates: Option<Coordinates>,
    pub area: Option<bool>,
    pub stay_on: Option<bool>,
    pub exit: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    #[serde(rename = "type")]
    pub geometry_type: String,
    pub coordinates: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Severe,
    Warning,
    Info,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertEffect {
    Detour,
    NoService,
    ReducedService,
    SignificantDelays,
    ModifiedService,
    StopMoved,
    OtherEffect,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertEntity {
    pub kind: String,
    pub id: Option<String>,
    pub route_id: Option<String>,
    pub stop_id: Option<String>,
    pub trip_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub header: Option<String>,
    pub description: String,
    pub severity: AlertSeverity,
    pub source_severity: Option<String>,
    pub effect: AlertEffect,
    pub source_effect: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub entities: Vec<AlertEntity>,
    pub source_feed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Leg {
    pub index: usize,
    pub mode: String,
    pub from: Place,
    pub to: Place,
    pub route: Option<Route>,
    pub trip_id: Option<String>,
    pub headsign: Option<String>,
    pub duration_seconds: Option<f64>,
    pub distance_m: Option<f64>,
    pub start: RealtimeEvidence,
    pub end: RealtimeEvidence,
    pub continues_previous_vehicle: Option<bool>,
    pub intermediate_stops: Vec<StopCall>,
    pub steps: Vec<WalkStep>,
    pub navigation_complete: bool,
    pub geometry: Option<Geometry>,
    pub alerts: Vec<Alert>,
    pub cancelled: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Itinerary {
    pub source_index: usize,
    pub start_time: DateTime<FixedOffset>,
    pub end_time: DateTime<FixedOffset>,
    pub duration_seconds: Option<i64>,
    pub transfers: i32,
    pub walk_seconds: Option<i64>,
    pub wait_seconds: Option<i64>,
    pub walk_distance_m: Option<f64>,
    pub legs: Vec<Leg>,
    pub alerts: Vec<Alert>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingError {
    pub code: String,
    pub description: Option<String>,
    pub input_field: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanResult {
    pub itineraries: Vec<Itinerary>,
    pub routing_errors: Vec<RoutingError>,
    pub complete: bool,
    pub search_date_time: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepartureBoard {
    pub stop: Stop,
    pub departures: Vec<Departure>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Departure {
    pub trip_id: String,
    pub route: Route,
    pub headsign: Option<String>,
    pub platform: Option<String>,
    pub service_date: NaiveDate,
    pub departure: RealtimeEvidence,
    pub cancelled: bool,
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSource {
    pub provider: String,
    pub dataset: String,
    pub product: String,
    pub retrieved_at: DateTime<Utc>,
    pub attribution: String,
    pub licenses: Vec<String>,
    pub realtime_included: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderResult<T> {
    pub value: T,
    pub source: ProviderSource,
}
