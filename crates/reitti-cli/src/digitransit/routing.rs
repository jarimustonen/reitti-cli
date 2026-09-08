use super::{contract, normalize, ClientBase};
use crate::client::HttpTransport;
use reitti_core::*;
use serde_json::{json, Value};
use std::time::Duration;

const STOP_FIELDS: &str = "gtfsId name code platformCode lat lon vehicleMode wheelchairBoarding";
const ALERT_FIELDS: &str = r#"id feed alertHeaderText alertDescriptionText alertSeverityLevel alertEffect effectiveStartDate effectiveEndDate entities { __typename ... on Agency { gtfsId } ... on Pattern { route { gtfsId } } ... on Route { gtfsId } ... on Stop { gtfsId } ... on StopOnRoute { route { gtfsId } stop { gtfsId } } ... on StopOnTrip { trip { gtfsId } stop { gtfsId } } ... on Trip { gtfsId } }"#;
const PLAN_QUERY: &str = r#"query NavigationPlan($origin:PlanLabeledLocationInput!,$destination:PlanLabeledLocationInput!,$dateTime:PlanDateTimeInput!,$modes:PlanModesInput!,$preferences:PlanPreferencesInput!,$geometry:Boolean!,$first:Int!){planConnection(origin:$origin destination:$destination dateTime:$dateTime modes:$modes preferences:$preferences first:$first){routingErrors{code description inputField}pageInfo{hasNextPage endCursor}searchDateTime edges{node{start end duration numberOfTransfers waitingTime walkTime walkDistance legs{mode duration distance realTime realtimeState interlineWithPreviousLeg headsign trip{gtfsId}route{gtfsId shortName longName mode}from{name lat lon stop{gtfsId name code platformCode lat lon vehicleMode wheelchairBoarding}}to{name lat lon stop{gtfsId name code platformCode lat lon vehicleMode wheelchairBoarding}}start{scheduledTime estimated{time delay}}end{scheduledTime estimated{time delay}}stopCalls{stopLocation{__typename ... on Stop{gtfsId name code platformCode lat lon vehicleMode wheelchairBoarding}}}steps{distance streetName relativeDirection absoluteDirection lat lon bogusName area stayOn exit}legGeometry @include(if:$geometry){length points}alerts{id feed alertHeaderText alertDescriptionText alertSeverityLevel alertEffect effectiveStartDate effectiveEndDate entities{__typename ... on Agency{gtfsId}... on Pattern{route{gtfsId}}... on Route{gtfsId}... on Stop{gtfsId}... on StopOnRoute{route{gtfsId}stop{gtfsId}}... on StopOnTrip{trip{gtfsId}stop{gtfsId}}... on Trip{gtfsId}}}}}}}}"#;

#[derive(Debug)]
pub struct DigitransitRouter<'a> {
    base: ClientBase<'a>,
}
impl<'a> DigitransitRouter<'a> {
    pub fn new(
        endpoint: &str,
        key: String,
        transport: &'a dyn HttpTransport,
        clock: &'a dyn Clock,
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
        clock: &'a dyn Clock,
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
    async fn plan_impl(&self, r: PlanRequest) -> Result<ProviderResult<PlanResult>, ProviderError> {
        if !r.from.coordinates.is_valid()
            || !r.to.coordinates.is_valid()
            || r.limit == 0
            || r.limit > 6
            || r.modes.is_empty()
        {
            return Err(contract("NavigationPlan"));
        }
        let endpoint = |e: PlanEndpoint| json!({"label":e.label,"location":{"coordinate":{"latitude":e.coordinates.latitude,"longitude":e.coordinates.longitude}}});
        let date = match r.time {
            PlanTime::DepartAt(v) => json!({"earliestDeparture":v.to_rfc3339()}),
            PlanTime::ArriveBy(v) => json!({"latestArrival":v.to_rfc3339()}),
        };
        let modes = r
            .modes
            .iter()
            .map(|m| json!({"mode":provider_mode(*m)}))
            .collect::<Vec<_>>();
        let vars = json!({"origin":endpoint(r.from),"destination":endpoint(r.to),"dateTime":date,"modes":{"transit":{"transit":modes}},"preferences":{"accessibility":{"wheelchair":{"enabled":r.wheelchair}}},"geometry":r.include_geometry,"first":r.limit});
        let data = self
            .base
            .graphql("NavigationPlan", PLAN_QUERY, vars, r.language.code())
            .await?;
        Ok(ProviderResult {
            value: normalize::plan(&data, r.include_geometry)?,
            source: self.base.source("routing-v2-hsl-gtfs", true),
        })
    }
    async fn stop_impl(
        &self,
        id: &StopId,
        lang: Language,
    ) -> Result<ProviderResult<Option<Stop>>, ProviderError> {
        let query = "query StopDetail($id:String!){stop(id:$id){$STOP_FIELDS}}"
            .replace("$STOP_FIELDS", STOP_FIELDS);
        let data = self
            .graphql_owned("StopDetail", query, json!({"id":id.as_str()}), lang)
            .await?;
        let value = data
            .get("stop")
            .filter(|v| !v.is_null())
            .map(|v| normalize::stop(v, None))
            .transpose()?;
        Ok(ProviderResult {
            value,
            source: self.base.source("routing-v2-hsl-gtfs", true),
        })
    }
    async fn graphql_owned(
        &self,
        operation: &'static str,
        query: String,
        variables: Value,
        lang: Language,
    ) -> Result<Value, ProviderError> {
        // Queries assembled here contain only compile-time field lists; user values remain variables.
        let body = serde_json::to_vec(
            &json!({"operationName":operation,"query":query,"variables":variables}),
        )
        .map_err(|_| contract(operation))?;
        let mut headers = self.base.headers(Some(lang.code()));
        headers.insert(
            "content-type".into(),
            crate::client::SecretHeader::new("application/json".into()),
        );
        let value = self
            .base
            .execute_json(
                operation,
                crate::client::HttpMethod::Post,
                self.base.endpoint.clone(),
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
    async fn search_impl(
        &self,
        r: StopSearchRequest,
    ) -> Result<ProviderResult<Vec<Stop>>, ProviderError> {
        if r.limit == 0 || r.limit > 10 {
            return Err(contract("StopSearch"));
        }
        let (query, vars, near) = match r.search {
            StopSearch::Name(name) if !name.trim().is_empty() => ("query StopSearch($name:String!){stops(name:$name){$STOP_FIELDS}}".replace("$STOP_FIELDS", STOP_FIELDS), json!({"name":name}), false),
            StopSearch::Name(_) => return Err(contract("StopSearch")),
            StopSearch::Nearby { coordinates, radius_m } if coordinates.is_valid() && (1..=5_000).contains(&radius_m) => ("query StopNearby($lat:Float!,$lon:Float!,$radius:Int!,$first:Int!){stopsByRadius(lat:$lat lon:$lon radius:$radius first:$first){edges{node{distance stop{$STOP_FIELDS}}}}}".replace("$STOP_FIELDS", STOP_FIELDS), json!({"lat":coordinates.latitude,"lon":coordinates.longitude,"radius":radius_m,"first":r.limit}), true),
            StopSearch::Nearby { .. } => return Err(contract("StopNearby")),
        };
        let data = self
            .graphql_owned(
                if near { "StopNearby" } else { "StopSearch" },
                query,
                vars,
                r.language,
            )
            .await?;
        let mut stops = if near {
            data.pointer("/stopsByRadius/edges")
                .and_then(Value::as_array)
                .ok_or_else(|| contract("StopNearby"))?
                .iter()
                .map(|e| {
                    let n = e.get("node").ok_or_else(|| contract("StopNearby"))?;
                    normalize::stop(
                        n.get("stop").ok_or_else(|| contract("StopNearby"))?,
                        n.get("distance").and_then(Value::as_f64),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            data.get("stops")
                .and_then(Value::as_array)
                .ok_or_else(|| contract("StopSearch"))?
                .iter()
                .map(|v| normalize::stop(v, None))
                .collect::<Result<Vec<_>, _>>()?
        };
        stops.truncate(r.limit as usize);
        Ok(ProviderResult {
            value: stops,
            source: self.base.source("routing-v2-hsl-gtfs", true),
        })
    }
    async fn departures_impl(
        &self,
        r: DepartureRequest,
    ) -> Result<ProviderResult<Option<DepartureBoard>>, ProviderError> {
        if r.limit == 0 || r.limit > 50 || !(60..=86_400).contains(&r.window_seconds) {
            return Err(contract("StopDepartures"));
        }
        let query = "query StopDepartures($id:String!,$count:Int!,$start:Long!,$range:Int!){stop(id:$id){$STOP_FIELDS stoptimesWithoutPatterns(numberOfDepartures:$count startTime:$start timeRange:$range omitCanceled:false omitNonPickups:true){headsign realtime realtimeDeparture realtimeState scheduledDeparture serviceDay trip{gtfsId route{gtfsId shortName longName mode}}}}}".replace("$STOP_FIELDS", STOP_FIELDS);
        let vars = json!({"id":r.stop.as_str(),"count":r.limit,"start":r.at.timestamp(),"range":r.window_seconds});
        let data = self
            .graphql_owned("StopDepartures", query, vars, r.language)
            .await?;
        let Some(v) = data.get("stop").filter(|v| !v.is_null()) else {
            return Ok(ProviderResult {
                value: None,
                source: self.base.source("routing-v2-hsl-gtfs", true),
            });
        };
        let stop = normalize::stop(v, None)?;
        let ds = v
            .get("stoptimesWithoutPatterns")
            .and_then(Value::as_array)
            .ok_or_else(|| contract("StopDepartures"))?
            .iter()
            .map(|d| normalize::departure(d, stop.platform.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ProviderResult {
            value: Some(DepartureBoard {
                stop,
                departures: ds,
            }),
            source: self.base.source("routing-v2-hsl-gtfs", true),
        })
    }
    async fn alerts_impl(
        &self,
        r: AlertRequest,
    ) -> Result<ProviderResult<Vec<Alert>>, ProviderError> {
        let query = "query Alerts($feeds:[String!]){alerts(feeds:$feeds){$ALERT_FIELDS}}"
            .replace("$ALERT_FIELDS", ALERT_FIELDS);
        let data = self
            .graphql_owned("Alerts", query, json!({"feeds":["HSL"]}), r.language)
            .await?;
        // Do not truncate here: the handler must retain feed-wide and unknown
        // scopes while applying its relevance union, active time and ordering.
        let values = data
            .get("alerts")
            .and_then(Value::as_array)
            .ok_or_else(|| contract("Alerts"))?
            .iter()
            .map(normalize::alert)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ProviderResult {
            value: values,
            source: self.base.source("routing-v2-hsl-gtfs", true),
        })
    }
}
impl Router for DigitransitRouter<'_> {
    fn plan(&self, r: PlanRequest) -> ProviderFuture<'_, ProviderResult<PlanResult>> {
        Box::pin(self.plan_impl(r))
    }
    fn search_stops(&self, r: StopSearchRequest) -> ProviderFuture<'_, ProviderResult<Vec<Stop>>> {
        Box::pin(self.search_impl(r))
    }
    fn stop(&self, id: &StopId, l: Language) -> ProviderFuture<'_, ProviderResult<Option<Stop>>> {
        let id = id.clone();
        Box::pin(async move { self.stop_impl(&id, l).await })
    }
    fn departures(
        &self,
        r: DepartureRequest,
    ) -> ProviderFuture<'_, ProviderResult<Option<DepartureBoard>>> {
        Box::pin(self.departures_impl(r))
    }
    fn alerts(&self, r: AlertRequest) -> ProviderFuture<'_, ProviderResult<Vec<Alert>>> {
        Box::pin(self.alerts_impl(r))
    }
    fn probe(&self) -> ProviderFuture<'_, ProviderResult<()>> {
        Box::pin(async move {
            let data = self
                .base
                .graphql(
                    "RoutingProbe",
                    "query RoutingProbe { feeds { feedId } }",
                    json!({}),
                    "en",
                )
                .await?;
            if !data.get("feeds").is_some_and(Value::is_array) {
                return Err(contract("RoutingProbe"));
            }
            Ok(ProviderResult {
                value: (),
                source: self.base.source("routing-v2-hsl-gtfs", true),
            })
        })
    }
}
fn provider_mode(m: Mode) -> &'static str {
    match m {
        Mode::Bus => "BUS",
        Mode::Tram => "TRAM",
        Mode::Rail => "RAIL",
        Mode::Subway => "SUBWAY",
        Mode::Ferry => "FERRY",
    }
}
