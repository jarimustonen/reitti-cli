use std::{future::Future, pin::Pin};

use chrono::{DateTime, FixedOffset};
use thiserror::Error;

use crate::{
    Alert, Coordinates, DepartureBoard, Language, LocationCandidate, Mode, PlanResult,
    ProviderResult, Stop, StopId,
};

pub type ProviderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ProviderError>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("provider operation '{operation}' failed: {kind}")]
pub struct ProviderError {
    pub operation: &'static str,
    pub kind: ProviderErrorKind,
    pub retryable: bool,
    pub retry_after_seconds: Option<u64>,
    pub retry_after_at: Option<DateTime<chrono::Utc>>,
    pub http_status: Option<u16>,
}
impl ProviderError {
    pub fn new(operation: &'static str, kind: ProviderErrorKind) -> Self {
        Self {
            operation,
            kind,
            retryable: false,
            retry_after_seconds: None,
            retry_after_at: None,
            http_status: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderErrorKind {
    Network,
    Authentication,
    RateLimited,
    Http,
    Graphql,
    Contract,
}
impl std::fmt::Display for ProviderErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Network => "network",
            Self::Authentication => "authentication",
            Self::RateLimited => "rate_limited",
            Self::Http => "http",
            Self::Graphql => "graphql",
            Self::Contract => "contract",
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocationSearchRequest {
    pub query: String,
    pub language: Language,
    pub limit: u8,
}
#[derive(Debug, Clone)]
pub struct PlanEndpoint {
    pub coordinates: Coordinates,
    pub label: Option<String>,
}
#[derive(Debug, Clone)]
pub enum PlanTime {
    DepartAt(DateTime<FixedOffset>),
    ArriveBy(DateTime<FixedOffset>),
}
#[derive(Debug, Clone)]
pub struct PlanRequest {
    pub from: PlanEndpoint,
    pub to: PlanEndpoint,
    pub language: Language,
    pub limit: u8,
    pub time: PlanTime,
    pub modes: Vec<Mode>,
    pub wheelchair: bool,
    pub include_geometry: bool,
}
#[derive(Debug, Clone)]
pub enum StopSearch {
    Name(String),
    Nearby {
        coordinates: Coordinates,
        radius_m: u32,
    },
}
#[derive(Debug, Clone)]
pub struct StopSearchRequest {
    pub search: StopSearch,
    pub language: Language,
    pub limit: u8,
}
#[derive(Debug, Clone)]
pub struct DepartureRequest {
    pub stop: StopId,
    pub language: Language,
    pub at: DateTime<FixedOffset>,
    pub window_seconds: u32,
    pub limit: u8,
    pub modes: Vec<Mode>,
}
#[derive(Debug, Clone)]
pub struct AlertRequest {
    /// Low-level retrieval is deliberately unfiltered: the handler owns the
    /// route/stop relevance union, active-time filtering, ordering and limit.
    pub language: Language,
}

pub trait Geocoder: Send + Sync {
    fn search(
        &self,
        request: LocationSearchRequest,
    ) -> ProviderFuture<'_, ProviderResult<Vec<LocationCandidate>>>;
    fn place(
        &self,
        id: &str,
        language: Language,
    ) -> ProviderFuture<'_, ProviderResult<Option<LocationCandidate>>>;
    fn probe(&self) -> ProviderFuture<'_, ProviderResult<()>>;
}
pub trait Router: Send + Sync {
    fn plan(&self, request: PlanRequest) -> ProviderFuture<'_, ProviderResult<PlanResult>>;
    fn search_stops(
        &self,
        request: StopSearchRequest,
    ) -> ProviderFuture<'_, ProviderResult<Vec<Stop>>>;
    fn stop(
        &self,
        id: &StopId,
        language: Language,
    ) -> ProviderFuture<'_, ProviderResult<Option<Stop>>>;
    fn departures(
        &self,
        request: DepartureRequest,
    ) -> ProviderFuture<'_, ProviderResult<Option<DepartureBoard>>>;
    fn alerts(&self, request: AlertRequest) -> ProviderFuture<'_, ProviderResult<Vec<Alert>>>;
    fn probe(&self) -> ProviderFuture<'_, ProviderResult<()>>;
}
