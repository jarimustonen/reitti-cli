use serde::{Deserialize, Serialize};

/// Language accepted by the public CLI and provider ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Fi,
    Sv,
}

/// Canonical transport ordering is the declaration order below.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceArea {
    Inside,
    Outside,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationCandidate {
    /// Tagged reference suitable for a polymorphic journey endpoint.
    pub reference: String,
    pub name: String,
    pub coordinates: Coordinates,
    /// Unknown evidence remains unknown; coordinates alone never prove scope.
    pub service_area: ServiceArea,
}

pub type Location = LocationCandidate;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stop {
    /// Raw canonical HSL GTFS ID. Tagged `stop:` references are only for
    /// polymorphic journey endpoint positions.
    pub id: String,
    pub name: String,
    pub coordinates: Option<Coordinates>,
    pub modes: Vec<Mode>,
    pub service_area: ServiceArea,
}
