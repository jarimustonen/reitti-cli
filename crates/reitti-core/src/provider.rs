use std::{future::Future, pin::Pin};

use thiserror::Error;

use crate::{Language, LocationCandidate, LocationRef, Stop, StopId};

pub type ProviderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ProviderError>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("provider operation '{operation}' failed: {kind}")]
pub struct ProviderError {
    pub operation: &'static str,
    pub kind: ProviderErrorKind,
    pub retryable: bool,
    pub retry_after_seconds: Option<u64>,
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
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                Self::Network => "network",
                Self::Authentication => "authentication",
                Self::RateLimited => "rate_limited",
                Self::Http => "http",
                Self::Graphql => "graphql",
                Self::Contract => "contract",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct LocationSearchRequest {
    pub query: String,
    pub language: Language,
    pub limit: u8,
}

#[derive(Debug, Clone)]
pub struct PlanRequest {
    pub from: LocationRef,
    pub to: LocationRef,
    pub language: Language,
    pub limit: u8,
}

#[derive(Debug, Clone, Default)]
pub struct PlanResult;

pub trait Geocoder: Send + Sync {
    fn search(&self, request: LocationSearchRequest) -> ProviderFuture<'_, Vec<LocationCandidate>>;
    fn place(&self, id: &str, language: Language) -> ProviderFuture<'_, Option<LocationCandidate>>;
}

pub trait Router: Send + Sync {
    fn plan(&self, request: PlanRequest) -> ProviderFuture<'_, PlanResult>;
    fn stop(&self, id: &StopId, language: Language) -> ProviderFuture<'_, Option<Stop>>;
}
