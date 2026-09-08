mod geocoding;
mod normalize;
mod routing;

pub use geocoding::DigitransitGeocoder;
pub use routing::DigitransitRouter;

#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use reitti_core::{Clock, ProviderError, ProviderErrorKind, ProviderSource};
use serde_json::{json, Value};
use std::{collections::BTreeMap, time::Duration};
use url::Url;

use crate::client::{HttpMethod, HttpRequest, HttpTransport, SecretHeader};

const KEY_HEADER: &str = "digitransit-subscription-key";

struct ClientBase<'a> {
    endpoint: Url,
    key: SecretHeader,
    transport: &'a dyn HttpTransport,
    clock: &'a dyn Clock,
    connect_timeout: Duration,
    request_timeout: Duration,
}
impl std::fmt::Debug for ClientBase<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientBase")
            .field("endpoint", &self.endpoint)
            .field("key", &"<redacted>")
            .field("connect_timeout", &self.connect_timeout)
            .field("request_timeout", &self.request_timeout)
            .finish()
    }
}
impl<'a> ClientBase<'a> {
    fn new(
        endpoint: &str,
        key: String,
        transport: &'a dyn HttpTransport,
        clock: &'a dyn Clock,
        connect_timeout: Duration,
        request_timeout: Duration,
        allow_http: bool,
    ) -> Result<Self, ProviderError> {
        let endpoint = Url::parse(endpoint).map_err(|_| contract("client_init"))?;
        if endpoint.scheme() != "https" && !(allow_http && endpoint.scheme() == "http") {
            return Err(contract("client_init"));
        }
        if key.trim().is_empty() || key.contains(['\r', '\n', '\0']) {
            return Err(ProviderError::new(
                "client_init",
                ProviderErrorKind::Authentication,
            ));
        }
        if connect_timeout.is_zero() || request_timeout.is_zero() {
            return Err(contract("client_init"));
        }
        Ok(Self {
            endpoint,
            key: SecretHeader::new(key),
            transport,
            clock,
            connect_timeout,
            request_timeout,
        })
    }
    fn headers(&self, language: Option<&str>) -> BTreeMap<String, SecretHeader> {
        let mut headers = BTreeMap::from([
            (
                "accept".into(),
                SecretHeader::new("application/json".into()),
            ),
            (KEY_HEADER.into(), self.key.clone()),
        ]);
        if let Some(language) = language {
            headers.insert("accept-language".into(), SecretHeader::new(language.into()));
        }
        headers
    }
    async fn get_json(
        &self,
        operation: &'static str,
        url: Url,
        language: &str,
    ) -> Result<Value, ProviderError> {
        self.execute_json(
            operation,
            HttpMethod::Get,
            url,
            self.headers(Some(language)),
            Vec::new(),
        )
        .await
    }
    async fn graphql(
        &self,
        operation: &'static str,
        query: &'static str,
        variables: Value,
        language: &str,
    ) -> Result<Value, ProviderError> {
        let body = serde_json::to_vec(
            &json!({"operationName":operation,"query":query,"variables":variables}),
        )
        .map_err(|_| contract(operation))?;
        let mut headers = self.headers(Some(language));
        headers.insert(
            "content-type".into(),
            SecretHeader::new("application/json".into()),
        );
        let value = self
            .execute_json(
                operation,
                HttpMethod::Post,
                self.endpoint.clone(),
                headers,
                body,
            )
            .await?;
        if value
            .get("errors")
            .and_then(Value::as_array)
            .is_some_and(|e| !e.is_empty())
        {
            return Err(ProviderError::new(operation, ProviderErrorKind::Graphql));
        }
        value
            .get("data")
            .cloned()
            .ok_or_else(|| contract(operation))
    }
    async fn execute_json(
        &self,
        operation: &'static str,
        method: HttpMethod,
        url: Url,
        headers: BTreeMap<String, SecretHeader>,
        body: Vec<u8>,
    ) -> Result<Value, ProviderError> {
        let response = self
            .transport
            .execute(HttpRequest {
                method,
                url: url.into(),
                headers,
                body,
                connect_timeout: self.connect_timeout,
                request_timeout: self.request_timeout,
            })
            .await
            .map_err(|source| {
                if source.code == "provider_response_too_large" {
                    ProviderError::new(operation, ProviderErrorKind::Contract)
                } else {
                    let mut error = ProviderError::new(operation, ProviderErrorKind::Network);
                    error.retryable = true;
                    error
                }
            })?;
        if response.status != 200 {
            return Err(http_error(
                operation,
                response.status,
                response.headers.get("retry-after").map(String::as_str),
                self.clock.now(),
            ));
        }
        serde_json::from_slice(&response.body).map_err(|_| contract(operation))
    }
    fn source(&self, product: &str, realtime: bool) -> ProviderSource {
        let retrieved_at = self.clock.now();
        ProviderSource {
            provider: "digitransit".into(),
            dataset: "hsl".into(),
            product: product.into(),
            retrieved_at,
            attribution: format!(
                "© Digitransit; retrieved {}",
                retrieved_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            ),
            licenses: vec!["CC-BY-4.0".into(), "ODbL-1.0".into()],
            realtime_included: realtime,
        }
    }
}
fn contract(operation: &'static str) -> ProviderError {
    ProviderError::new(operation, ProviderErrorKind::Contract)
}
fn http_error(
    operation: &'static str,
    status: u16,
    retry_after: Option<&str>,
    now: DateTime<Utc>,
) -> ProviderError {
    let kind = match status {
        401 => ProviderErrorKind::Authentication,
        429 => ProviderErrorKind::RateLimited,
        _ => ProviderErrorKind::Http,
    };
    let mut error = ProviderError::new(operation, kind);
    error.http_status = Some(status);
    error.retryable = matches!(status, 403 | 429 | 500..=599);
    if let Some(value) = retry_after {
        if let Ok(seconds) = value.parse::<u64>() {
            error.retry_after_seconds = Some(seconds);
        } else if let Ok(time) = httpdate::parse_http_date(value) {
            let at: DateTime<Utc> = time.into();
            if at > now {
                error.retry_after_at = Some(at);
            }
        }
    }
    error
}
