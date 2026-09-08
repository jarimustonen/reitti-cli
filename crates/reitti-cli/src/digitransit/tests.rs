use super::{normalize, DigitransitGeocoder, DigitransitRouter};
use crate::client::{
    HttpFuture, HttpMethod, HttpRequest, HttpResponse, HttpTransport, ReqwestTransport,
    MAX_RESPONSE_BYTES,
};
use chrono::{DateTime, Utc};
use reitti_core::*;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Mutex,
    time::Duration,
};

#[derive(Debug)]
struct MockTransport {
    requests: Mutex<Vec<HttpRequest>>,
    responses: Mutex<VecDeque<HttpResponse>>,
}
impl MockTransport {
    fn new(responses: Vec<HttpResponse>) -> Self {
        Self {
            requests: Mutex::new(vec![]),
            responses: Mutex::new(responses.into()),
        }
    }
    fn requests(&self) -> std::sync::MutexGuard<'_, Vec<HttpRequest>> {
        self.requests.lock().unwrap()
    }
}
impl HttpTransport for MockTransport {
    fn execute(&self, request: HttpRequest) -> HttpFuture<'_> {
        self.requests.lock().unwrap().push(request);
        let response = self.responses.lock().unwrap().pop_front().unwrap();
        Box::pin(async move { Ok(response) })
    }
}
fn response(status: u16, body: Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: BTreeMap::new(),
        body: serde_json::to_vec(&body).unwrap(),
    }
}
fn clock() -> FixedClock {
    FixedClock::new(
        DateTime::parse_from_rfc3339("2026-09-08T08:13:39Z")
            .unwrap()
            .with_timezone(&Utc),
    )
}
fn fixture(name: &str) -> Value {
    let path = format!("../../../tests/fixtures/digitransit/{name}");
    serde_json::from_str(match name {
        "geocoding-ambiguity.json" => {
            include_str!("../../../../tests/fixtures/digitransit/geocoding-ambiguity.json")
        }
        "geocoding-place.json" => {
            include_str!("../../../../tests/fixtures/digitransit/geocoding-place.json")
        }
        "routing-navigation-rich.json" => {
            include_str!("../../../../tests/fixtures/digitransit/routing-navigation-rich.json")
        }
        "routing-departures.json" => {
            include_str!("../../../../tests/fixtures/digitransit/routing-departures.json")
        }
        "routing-alert-scopes.json" => {
            include_str!("../../../../tests/fixtures/digitransit/routing-alert-scopes.json")
        }
        "routing-no-result.json" => {
            include_str!("../../../../tests/fixtures/digitransit/routing-no-result.json")
        }
        _ => panic!("unknown fixture {path}"),
    })
    .unwrap()
}
fn geocoder<'a>(transport: &'a dyn HttpTransport, clock: &'a dyn Clock) -> DigitransitGeocoder<'a> {
    DigitransitGeocoder::new_for_test(
        "http://mock.test/geocoding/v1",
        "secret-canary".into(),
        transport,
        clock,
        Duration::from_secs(1),
        Duration::from_secs(2),
    )
    .unwrap()
}
fn router<'a>(transport: &'a dyn HttpTransport, clock: &'a dyn Clock) -> DigitransitRouter<'a> {
    DigitransitRouter::new_for_test(
        "http://mock.test/routing/v2/hsl/gtfs/v1",
        "secret-canary".into(),
        transport,
        clock,
        Duration::from_secs(1),
        Duration::from_secs(2),
    )
    .unwrap()
}

#[tokio::test]
async fn geocoding_preserves_v1_path_encodes_input_and_normalizes_candidates() {
    let f = fixture("geocoding-ambiguity.json");
    let transport = MockTransport::new(vec![response(200, f["response"]["body"].clone())]);
    let clock = clock();
    let result = geocoder(&transport, &clock)
        .search(LocationSearchRequest {
            query: "Kamppi & koti".into(),
            language: Language::Sv,
            limit: 3,
        })
        .await
        .unwrap();
    assert_eq!(result.value.len(), 3);
    assert_eq!(result.value[0].modes, vec![Mode::Subway]);
    assert_eq!(result.value[0].service_area, ServiceArea::Inside);
    let requests = transport.requests();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.method, HttpMethod::Get);
    let url = url::Url::parse(&request.url).unwrap();
    assert_eq!(url.path(), "/geocoding/v1/search");
    assert!(url
        .query_pairs()
        .any(|(k, v)| k == "text" && v == "Kamppi & koti"));
    assert_eq!(
        request.headers["digitransit-subscription-key"].expose(),
        "secret-canary"
    );
    assert!(!format!("{request:?}").contains("secret-canary"));
}

#[tokio::test]
async fn stable_place_lookup_uses_ids_query_pair() {
    let f = fixture("geocoding-place.json");
    let transport = MockTransport::new(vec![response(200, f["response"]["body"].clone())]);
    let clock = clock();
    let result = geocoder(&transport, &clock)
        .place("gtfshsl:station:GTFS:HSL:1000102", Language::Fi)
        .await
        .unwrap();
    assert!(result.value.is_some());
    let requests = transport.requests();
    let url = url::Url::parse(&requests[0].url).unwrap();
    assert!(url
        .query_pairs()
        .any(|(key, value)| key == "ids" && value == "gtfshsl:station:GTFS:HSL:1000102"));
}

#[tokio::test]
async fn rich_plan_uses_typed_variables_and_preserves_navigation() {
    let f = fixture("routing-navigation-rich.json");
    let transport = MockTransport::new(vec![response(200, f["response"]["body"].clone())]);
    let clock = clock();
    let result = router(&transport, &clock)
        .plan(PlanRequest {
            from: PlanEndpoint {
                coordinates: Coordinates {
                    latitude: 60.168992,
                    longitude: 24.932366,
                },
                label: None,
            },
            to: PlanEndpoint {
                coordinates: Coordinates {
                    latitude: 60.175294,
                    longitude: 24.684855,
                },
                label: None,
            },
            language: Language::En,
            limit: 3,
            time: PlanTime::DepartAt(DateTime::parse_from_rfc3339("2026-09-08T08:13:37Z").unwrap()),
            modes: vec![Mode::Bus, Mode::Tram, Mode::Rail, Mode::Subway, Mode::Ferry],
            wheelchair: true,
            include_geometry: true,
        })
        .await
        .unwrap();
    let itinerary = &result.value.itineraries[0];
    assert_eq!(itinerary.transfers, 1);
    assert_eq!(itinerary.duration_seconds, Some(3065));
    assert_eq!(itinerary.walk_distance_m, Some(840.21));
    assert!(itinerary
        .legs
        .iter()
        .any(|leg| leg.intermediate_stops.len() >= 10));
    assert!(itinerary.legs.iter().any(|leg| leg
        .geometry
        .as_ref()
        .is_some_and(|g| g.coordinates.len() > 100)));
    assert_eq!(itinerary.alerts[0].id, "QWxlcnQ6SFNMOjMwNjQwOQ");
    let requests = transport.requests();
    assert_eq!(requests.len(), 1);
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["operationName"], "NavigationPlan");
    assert_eq!(
        body["variables"]["preferences"]["accessibility"]["wheelchair"]["enabled"],
        true
    );
    assert_eq!(
        body["variables"]["modes"]["transit"]["transit"][0]["mode"],
        "BUS"
    );
    assert!(body["query"].as_str().unwrap().contains("stopCalls"));
    assert!(!body["query"]
        .as_str()
        .unwrap()
        .contains("intermediateStops"));
    assert!(body.get("maxWalkDistance").is_none());
}

#[tokio::test]
async fn stop_departure_alert_and_no_result_methods_keep_handler_owned_semantics() {
    let mut departures = fixture("routing-departures.json")["response"]["body"].clone();
    let stop = &mut departures["data"]["stop"];
    stop["code"] = Value::Null;
    stop["platformCode"] = Value::Null;
    stop["lat"] = json!(60.170347);
    stop["lon"] = json!(24.941008);
    stop["vehicleMode"] = json!("TRAM");
    stop["wheelchairBoarding"] = json!("POSSIBLE");
    for (index, row) in stop["stoptimesWithoutPatterns"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        row["trip"]["route"]["gtfsId"] = json!(format!("HSL:route{index}"));
        row["trip"]["route"]["longName"] = Value::Null;
        row["trip"]["route"]["mode"] = json!("TRAM");
    }
    let alerts = fixture("routing-alert-scopes.json")["response"]["body"].clone();
    let no_result = fixture("routing-no-result.json")["response"]["body"].clone();
    let transport = MockTransport::new(vec![
        response(200, departures),
        response(200, alerts),
        response(200, no_result),
    ]);
    let clock = clock();
    let router = router(&transport, &clock);
    let board = router
        .departures(DepartureRequest {
            stop: "HSL:1020453".parse().unwrap(),
            language: Language::Fi,
            at: DateTime::parse_from_rfc3339("2026-09-08T09:55:00+03:00").unwrap(),
            window_seconds: 7200,
            limit: 3,
            modes: vec![Mode::Tram],
        })
        .await
        .unwrap()
        .value
        .unwrap();
    assert_eq!(board.departures.len(), 3);
    assert_eq!(board.departures[0].service_date.to_string(), "2026-09-08");
    assert!(board.departures[0].departure.observed_realtime);
    let all_alerts = router
        .alerts(AlertRequest {
            language: Language::En,
        })
        .await
        .unwrap()
        .value;
    assert_eq!(all_alerts.len(), 4);
    assert!(all_alerts
        .iter()
        .any(|alert| alert.entities.iter().any(|entity| entity.kind == "unknown")));
    let missing = router
        .stop(&"HSL:does-not-exist".parse().unwrap(), Language::En)
        .await
        .unwrap();
    assert!(missing.value.is_none());
    let requests = transport.requests();
    assert_eq!(requests.len(), 3);
    let departure_body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(departure_body["query"]
        .as_str()
        .unwrap()
        .contains("omitCanceled:false"));
    let alert_body: Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert_eq!(alert_body["variables"], json!({"feeds":["HSL"]}));
    assert!(!alert_body["query"].as_str().unwrap().contains("$routes"));
}

#[tokio::test]
async fn stop_search_nearby_is_one_bounded_typed_query() {
    let body = json!({"data":{"stopsByRadius":{"edges":[{"node":{"distance":123,"stop":{"gtfsId":"HSL:1","name":"A","code":null,"platformCode":"2","lat":60.1,"lon":24.9,"vehicleMode":"BUS","wheelchairBoarding":"NO_INFORMATION"}}}]}}});
    let transport = MockTransport::new(vec![response(200, body)]);
    let clock = clock();
    let result = router(&transport, &clock)
        .search_stops(StopSearchRequest {
            search: StopSearch::Nearby {
                coordinates: Coordinates {
                    latitude: 60.1,
                    longitude: 24.9,
                },
                radius_m: 500,
            },
            language: Language::En,
            limit: 5,
        })
        .await
        .unwrap();
    assert_eq!(result.value[0].distance_m, Some(123.0));
    let requests = transport.requests();
    assert_eq!(requests.len(), 1);
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["variables"]["radius"], 500);
    assert_eq!(body["variables"]["first"], 5);
}

#[test]
fn nullable_provider_evidence_remains_unknown_and_early_delay_is_signed() {
    let alert=normalize::alert(&json!({"id":"a1","alertHeaderText":null,"alertDescriptionText":"Description","alertSeverityLevel":null,"alertEffect":"NEW_EFFECT","effectiveStartDate":null,"effectiveEndDate":null,"entities":null,"feed":"HSL"})).unwrap();
    assert_eq!(alert.header, None);
    assert_eq!(alert.entities[0].kind, "unknown");
    assert_eq!(alert.source_effect.as_deref(), Some("NEW_EFFECT"));
    let departure=normalize::departure(&json!({"headsign":null,"realtime":null,"realtimeDeparture":null,"realtimeState":null,"scheduledDeparture":3600,"serviceDay":1788814800,"trip":{"gtfsId":"HSL:t","route":{"gtfsId":"HSL:r","shortName":null,"longName":null,"mode":"BUS"}}}),None).unwrap();
    assert_eq!(departure.departure.state, RealtimeState::Unknown);
    assert_eq!(departure.departure.estimated_time, None);
    let data = json!({"planConnection":{"routingErrors":[],"pageInfo":{"hasNextPage":false},"searchDateTime":null,"edges":[{"node":{"start":"2026-09-08T10:00:00+03:00","end":"2026-09-08T10:10:00+03:00","duration":null,"numberOfTransfers":0,"waitingTime":null,"walkTime":null,"walkDistance":null,"legs":[{"mode":"BUS","duration":null,"distance":null,"realtimeState":"UPDATED","interlineWithPreviousLeg":null,"headsign":null,"trip":null,"route":null,"from":{"name":"A","lat":null,"lon":null,"stop":null},"to":{"name":"B","lat":null,"lon":null,"stop":null},"start":{"scheduledTime":"2026-09-08T10:00:00+03:00","estimated":{"time":"2026-09-08T09:59:30+03:00","delay":"-PT30S"}},"end":{"scheduledTime":"2026-09-08T10:10:00+03:00","estimated":null},"stopCalls":[],"steps":null,"legGeometry":null,"alerts":[]}]}}]}});
    let plan = normalize::plan(&data, false).unwrap();
    assert_eq!(plan.itineraries[0].walk_distance_m, None);
    assert_eq!(plan.itineraries[0].legs[0].start.delay_seconds, Some(-30));
    assert!(!plan.itineraries[0].legs[0].navigation_complete);
    assert_eq!(plan.itineraries[0].legs[0].continues_previous_vehicle, None);
}

#[tokio::test]
async fn status_graphql_and_retry_after_are_classified_without_body_echo() {
    for (status, kind, retryable) in [
        (401, ProviderErrorKind::Authentication, false),
        (403, ProviderErrorKind::Http, true),
        (429, ProviderErrorKind::RateLimited, true),
        (503, ProviderErrorKind::Http, true),
    ] {
        let mut r = response(status, json!({"secret":"secret-canary"}));
        if status == 429 {
            r.headers.insert("retry-after".into(), "120".into());
        }
        let transport = MockTransport::new(vec![r]);
        let clock = clock();
        let error = geocoder(&transport, &clock).probe().await.unwrap_err();
        assert_eq!(error.kind, kind);
        assert_eq!(error.retryable, retryable);
        if status == 429 {
            assert_eq!(error.retry_after_seconds, Some(120));
        }
        assert!(!format!("{error:?}").contains("secret-canary"));
    }
    let transport = MockTransport::new(vec![response(
        200,
        json!({"data":{"feeds":[]},"errors":[{"message":"secret-canary"}]}),
    )]);
    let clock = clock();
    let error = router(&transport, &clock).probe().await.unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Graphql);
    assert!(!format!("{error:?}").contains("secret-canary"));
}

async fn one_shot_server(response: Vec<u8>, delay: Duration) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut input = [0u8; 4096];
        let _ = socket.read(&mut input).await;
        tokio::time::sleep(delay).await;
        let _ = socket.write_all(&response).await;
    });
    format!("http://{address}/test")
}
#[tokio::test]
async fn production_transport_disables_redirects_and_honors_timeout() {
    let url = one_shot_server(
        b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/secret\r\nContent-Length: 0\r\n\r\n"
            .to_vec(),
        Duration::ZERO,
    )
    .await;
    let result = ReqwestTransport
        .execute(HttpRequest {
            method: HttpMethod::Get,
            url,
            headers: BTreeMap::new(),
            body: vec![],
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
        })
        .await
        .unwrap();
    assert_eq!(result.status, 302);
    let url = one_shot_server(
        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}".to_vec(),
        Duration::from_millis(100),
    )
    .await;
    let error = ReqwestTransport
        .execute(HttpRequest {
            method: HttpMethod::Get,
            url,
            headers: BTreeMap::new(),
            body: vec![],
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_millis(10),
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, "network_error");
}
#[tokio::test]
async fn chunked_oversized_response_is_bounded_through_provider() {
    let mut wire = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
    let chunk = vec![b'x'; 8192];
    for _ in 0..=(MAX_RESPONSE_BYTES / 8192) {
        wire.extend_from_slice(b"2000\r\n");
        wire.extend_from_slice(&chunk);
        wire.extend_from_slice(b"\r\n");
    }
    wire.extend_from_slice(b"0\r\n\r\n");
    let url = one_shot_server(wire, Duration::ZERO).await;
    let transport = ReqwestTransport;
    let clock = clock();
    let geocoder = DigitransitGeocoder::new_for_test(
        &url,
        "secret-canary".into(),
        &transport,
        &clock,
        Duration::from_secs(1),
        Duration::from_secs(5),
    )
    .unwrap();
    let error = geocoder.probe().await.unwrap_err();
    assert_eq!(error.kind, ProviderErrorKind::Contract);
    assert!(!error.retryable);
}

#[test]
fn debug_never_exposes_response_or_credentials_and_polyline_is_bounded() {
    let response = HttpResponse {
        status: 200,
        headers: BTreeMap::from([("echo".into(), "secret-canary".into())]),
        body: b"secret-canary".to_vec(),
    };
    assert!(!format!("{response:?}").contains("secret-canary"));
    assert!(normalize::decode_polyline("_p~iF~ps|U_ulLnnqC_mqNvxq`@", 3).is_ok());
    assert!(normalize::decode_polyline("_p~iF~ps|U_ulLnnqC_mqNvxq`@", 2).is_err());
    assert!(normalize::decode_polyline("~", 10).is_err());
}
