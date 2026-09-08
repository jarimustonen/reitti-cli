use super::{contract, normalize::locations, ClientBase};
use crate::client::HttpTransport;
use reitti_core::{
    Geocoder, Language, LocationSearchRequest, ProviderError, ProviderFuture, ProviderResult,
};
use serde_json::Value;
use std::time::Duration;
use url::Url;

#[derive(Debug)]
pub struct DigitransitGeocoder<'a> {
    base: ClientBase<'a>,
}
impl<'a> DigitransitGeocoder<'a> {
    pub fn new(
        endpoint: &str,
        key: String,
        transport: &'a dyn HttpTransport,
        clock: &'a dyn reitti_core::Clock,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        Ok(Self {
            base: ClientBase::new(
                endpoint,
                key,
                transport,
                clock,
                connect_timeout,
                request_timeout,
                false,
            )?,
        })
    }
    #[cfg(test)]
    pub(crate) fn new_for_test(
        endpoint: &str,
        key: String,
        transport: &'a dyn HttpTransport,
        clock: &'a dyn reitti_core::Clock,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self, ProviderError> {
        Ok(Self {
            base: ClientBase::new(
                endpoint,
                key,
                transport,
                clock,
                connect_timeout,
                request_timeout,
                true,
            )?,
        })
    }
    fn operation_url(&self, operation: &str) -> Result<Url, ProviderError> {
        let mut url = self.base.endpoint.clone();
        {
            let mut parts = url
                .path_segments_mut()
                .map_err(|_| contract("geocoding_url"))?;
            parts.pop_if_empty();
            parts.push(operation);
        }
        Ok(url)
    }
    async fn search_impl(
        &self,
        request: LocationSearchRequest,
    ) -> Result<ProviderResult<Vec<reitti_core::LocationCandidate>>, ProviderError> {
        if request.limit == 0 || request.limit > 10 || request.query.trim().is_empty() {
            return Err(contract("LocationSearch"));
        }
        let mut url = self.operation_url("search")?;
        url.query_pairs_mut()
            .append_pair("text", &request.query)
            .append_pair("lang", request.language.code())
            .append_pair("size", &request.limit.to_string());
        let value = self
            .base
            .get_json("LocationSearch", url, request.language.code())
            .await?;
        let candidates = locations(&value)?;
        if candidates.len() > request.limit as usize {
            return Err(contract("LocationSearch"));
        }
        Ok(ProviderResult {
            value: candidates,
            source: self.base.source("geocoding-v1", false),
        })
    }
    async fn place_impl(
        &self,
        id: &str,
        language: Language,
    ) -> Result<ProviderResult<Option<reitti_core::LocationCandidate>>, ProviderError> {
        let mut url = self.operation_url("place")?;
        url.query_pairs_mut()
            .append_pair("ids", id)
            .append_pair("lang", language.code());
        let value = self
            .base
            .get_json("LocationPlace", url, language.code())
            .await?;
        let mut values = locations(&value)?;
        if values.len() > 1
            || values
                .first()
                .is_some_and(|candidate| candidate.id.as_deref() != Some(id))
        {
            return Err(contract("LocationPlace"));
        }
        Ok(ProviderResult {
            value: values.pop(),
            source: self.base.source("geocoding-v1", false),
        })
    }
}
impl Geocoder for DigitransitGeocoder<'_> {
    fn search(
        &self,
        request: LocationSearchRequest,
    ) -> ProviderFuture<'_, ProviderResult<Vec<reitti_core::LocationCandidate>>> {
        Box::pin(self.search_impl(request))
    }
    fn place(
        &self,
        id: &str,
        language: Language,
    ) -> ProviderFuture<'_, ProviderResult<Option<reitti_core::LocationCandidate>>> {
        let id = id.to_owned();
        Box::pin(async move { self.place_impl(&id, language).await })
    }
    fn probe(&self) -> ProviderFuture<'_, ProviderResult<()>> {
        Box::pin(async move {
            let mut url = self.operation_url("search")?;
            url.query_pairs_mut()
                .append_pair("text", "Helsinki")
                .append_pair("lang", "en")
                .append_pair("size", "1");
            let value = self.base.get_json("GeocodingProbe", url, "en").await?;
            if !value.get("features").is_some_and(Value::is_array) {
                return Err(contract("GeocodingProbe"));
            }
            Ok(ProviderResult {
                value: (),
                source: self.base.source("geocoding-v1", false),
            })
        })
    }
}
