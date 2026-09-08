use std::future::Future;

use reitti_core::{Clock, RequestIdGenerator};

use crate::{client::HttpTransport, config::EffectiveConfig, error::AppError};

pub mod alert;
pub mod journey;
pub mod location;
pub mod stop;

/// Complete invocation dependencies shared by the separately owned handlers.
/// It keeps provider I/O out of core without requiring later handlers to edit
/// the common dispatcher or rediscover ambient config/time/id state.
pub struct HandlerContext<'a> {
    pub config: &'a EffectiveConfig,
    pub clock: &'a dyn Clock,
    pub request_ids: &'a dyn RequestIdGenerator,
    pub transport: &'a dyn HttpTransport,
}

/// One small synchronous CLI-to-async-provider bridge. Provider clients remain
/// async and tests can await them directly; a CLI invocation creates one local
/// runtime and performs no background work.
pub fn await_provider<F, T>(future: F) -> Result<T, AppError>
where
    F: Future<Output = Result<T, reitti_core::ProviderError>>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| {
            AppError::system(
                "internal_error",
                "Could not initialize the provider runtime.",
            )
        })?;
    runtime.block_on(future).map_err(provider_error)
}

fn provider_error(error: reitti_core::ProviderError) -> AppError {
    use reitti_core::ProviderErrorKind as K;
    let (code, exit_one, message) = match error.kind {
        K::Authentication => ("provider_authentication", true, "Digitransit rejected the subscription key. Update DIGITRANSIT_SUBSCRIPTION_KEY or pipe one line to reitti config update --subscription-key-stdin."),
        K::RateLimited => ("provider_rate_limited", false, "Digitransit rate-limited the request; retry later using Retry-After when supplied."),
        K::Network => ("network_error", false, "The Digitransit network request failed or timed out."),
        K::Http => ("provider_http", false, "Digitransit returned an HTTP provider failure."),
        K::Graphql => ("provider_graphql", false, "Digitransit returned GraphQL errors; partial data was not served."),
        K::Contract => ("provider_contract", false, "The Digitransit response did not match the required typed data contract."),
    };
    let message = format!("{message} Operation: {}.", error.operation);
    let mut app = if exit_one {
        AppError::caller(code, message)
    } else {
        AppError::system(code, message)
    };
    app.retryable = error.retryable;
    if let Some(status) = error.http_status {
        app = app.with_detail("http_status", status);
    }
    if let Some(seconds) = error.retry_after_seconds {
        app = app.with_detail("retry_after_seconds", seconds);
    }
    if let Some(at) = error.retry_after_at {
        app = app.with_detail("retry_after_at", at.to_rfc3339());
    }
    app
}
