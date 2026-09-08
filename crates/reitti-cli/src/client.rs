use std::{collections::BTreeMap, future::Future, pin::Pin, time::Duration};

use futures_util::StreamExt;

use crate::error::AppError;

pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: BTreeMap<String, SecretHeader>,
    pub body: Vec<u8>,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

#[derive(Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
impl std::fmt::Debug for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("headers", &"<redacted>")
            .field(
                "body",
                &format_args!("<{} bytes redacted>", self.body.len()),
            )
            .finish()
    }
}
pub type HttpFuture<'a> = Pin<Box<dyn Future<Output = Result<HttpResponse, AppError>> + Send + 'a>>;
pub trait HttpTransport: Send + Sync {
    fn execute(&self, request: HttpRequest) -> HttpFuture<'_>;
}

/// Production transport: exactly one request, redirects disabled, finite
/// connect/operation deadlines, and a streaming 8 MiB response cap.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReqwestTransport;
impl HttpTransport for ReqwestTransport {
    fn execute(&self, request: HttpRequest) -> HttpFuture<'_> {
        Box::pin(async move {
            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .connect_timeout(request.connect_timeout)
                .build()
                .map_err(|_| {
                    AppError::system("network_error", "Could not initialize the HTTPS client.")
                })?;
            let mut builder = match request.method {
                HttpMethod::Get => client.get(&request.url),
                HttpMethod::Post => client.post(&request.url),
            }
            .timeout(request.request_timeout);
            for (name, value) in &request.headers {
                let mut header =
                    reqwest::header::HeaderValue::from_str(value.expose()).map_err(|_| {
                        AppError::system(
                            "internal_error",
                            "Could not construct a provider request header.",
                        )
                    })?;
                if name.eq_ignore_ascii_case("digitransit-subscription-key") {
                    header.set_sensitive(true);
                }
                builder = builder.header(name, header);
            }
            if !request.body.is_empty() {
                builder = builder.body(request.body);
            }
            let response = builder.send().await.map_err(|error| {
                let kind = if error.is_timeout() {
                    "Provider request timed out."
                } else {
                    "Provider network request failed."
                };
                AppError::system("network_error", kind)
            })?;
            let status = response.status().as_u16();
            let mut headers = BTreeMap::new();
            // Only retain headers needed by policy. Never preserve arbitrary
            // response headers, which may echo credentials.
            if let Some(value) = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
            {
                headers.insert("retry-after".into(), value.into());
            }
            if let Some(length) = response.content_length() {
                if length > MAX_RESPONSE_BYTES as u64 {
                    return Err(AppError::system(
                        "provider_response_too_large",
                        "Provider response exceeded the 8 MiB limit.",
                    ));
                }
            }
            let mut body = Vec::new();
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|_| {
                    AppError::system("network_error", "Provider response stream failed.")
                })?;
                if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                    return Err(AppError::system(
                        "provider_response_too_large",
                        "Provider response exceeded the 8 MiB limit.",
                    ));
                }
                body.extend_from_slice(&chunk);
            }
            Ok(HttpResponse {
                status,
                headers,
                body,
            })
        })
    }
}

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
