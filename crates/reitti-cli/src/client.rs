use std::{collections::BTreeMap, future::Future, pin::Pin, time::Duration};

use crate::error::AppError;

/// Provider-neutral request passed to the injected HTTP transport.
///
/// The next client slice owns concrete Digitransit operations, size caps, and
/// timeout enforcement. Keeping this port free of reqwest allows exact offline mocks.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: BTreeMap<String, SecretHeader>,
    pub body: Vec<u8>,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}

#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Clone)]
pub struct SecretHeader(String);

impl SecretHeader {
    pub fn new(value: String) -> Self {
        Self(value)
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretHeader {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

pub type HttpFuture<'a> = Pin<Box<dyn Future<Output = Result<HttpResponse, AppError>> + Send + 'a>>;

pub trait HttpTransport: Send + Sync {
    fn execute(&self, request: HttpRequest) -> HttpFuture<'_>;
}

/// Test transport that proves offline commands cannot accidentally escape to HTTP.
#[derive(Debug, Default)]
pub struct DenyNetwork;

impl HttpTransport for DenyNetwork {
    fn execute(&self, _request: HttpRequest) -> HttpFuture<'_> {
        Box::pin(async {
            Err(AppError::system(
                "network_forbidden",
                "HTTP transport was invoked by an offline operation.",
            ))
        })
    }
}
