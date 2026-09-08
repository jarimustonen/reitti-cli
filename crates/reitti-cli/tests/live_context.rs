use chrono::{DateTime, Utc};
use reitti_cli::{
    client::{HttpFuture, HttpRequest, HttpResponse, HttpTransport},
    run_with_transport, support,
};
use reitti_core::{FixedClock, FixedRequestIds};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    io::Cursor,
    sync::Mutex,
};

struct FixtureTransport {
    responses: Mutex<VecDeque<HttpResponse>>,
    requests: Mutex<Vec<HttpRequest>>,
}

impl FixtureTransport {
    fn new(responses: Vec<HttpResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl HttpTransport for FixtureTransport {
    fn execute(&self, request: HttpRequest) -> HttpFuture<'_> {
        self.requests.lock().unwrap().push(request);
        Box::pin(async move {
            self.responses.lock().unwrap().pop_front().ok_or_else(|| {
                reitti_cli::error::AppError::system("network_error", "No fixture response.")
            })
        })
    }
}

fn response(status: u16, body: Value) -> HttpResponse {
    HttpResponse {
        status,
        headers: BTreeMap::new(),
        body: serde_json::to_vec(&body).unwrap(),
    }
}

fn invoke(transport: &dyn HttpTransport, args: &[&str]) -> (u8, String, String) {
    let mut stdin = Cursor::new(Vec::<u8>::new());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let clock = FixedClock::new(
        DateTime::parse_from_rfc3339("2026-09-08T06:56:40Z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let exit = run_with_transport(
        args.iter().map(OsString::from).collect(),
        &mut stdin,
        &mut stdout,
        &mut stderr,
        &clock,
        &FixedRequestIds::new("req_live_context"),
        transport,
    );
    (
        exit,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn validate_schema(name: &str, output: &str) -> Value {
    let document: Value = serde_json::from_str(output).unwrap();
    let schema = support::schema(name).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    validator.validate(&document["data"]).unwrap();
    document
}

#[test]
fn live_context_handlers_are_bounded_truthful_and_schema_valid() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("config.toml");
    std::fs::write(
        &config,
        "subscription_key = \"secret-never-print\"\ntimezone = \"UTC\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    // This integration test is one process-local serial test; restore the only
    // environment key it changes before returning.
    let previous = std::env::var_os("REITTI_CONFIG_FILE");
    unsafe { std::env::set_var("REITTI_CONFIG_FILE", &config) };

    let stop_body = json!({"data":{"stops":[{
        "gtfsId":"HSL:1020453","name":"Päärautatieasema","code":"Rautatieasema",
        "platformCode":"3","lat":60.1702,"lon":24.9391,"vehicleMode":"TRAM",
        "wheelchairBoarding":"POSSIBLE"
    }]}});
    let departure_body = json!({"data":{"stop":{
        "gtfsId":"HSL:1020453","name":"Päärautatieasema","code":null,
        "platformCode":"3","lat":60.1702,"lon":24.9391,"vehicleMode":"TRAM",
        "wheelchairBoarding":"NO_INFORMATION","stoptimesWithoutPatterns":[
            {"headsign":"Scheduled only","realtime":null,"realtimeDeparture":null,"realtimeState":null,"scheduledDeparture":35880,"serviceDay":1788814800,
             "trip":{"gtfsId":"HSL:scheduled","route":{"gtfsId":"HSL:route3","shortName":"3","longName":null,"mode":"TRAM"}}},
            {"headsign":"Cancelled service","realtime":true,"realtimeDeparture":35980,"realtimeState":"CANCELED","scheduledDeparture":35940,"serviceDay":1788814800,
             "trip":{"gtfsId":"HSL:cancelled","route":{"gtfsId":"HSL:route5","shortName":"5","longName":null,"mode":"TRAM"}}}
        ]
    }}});
    let alert_body = json!({"data":{"alerts":[
        {"id":"route-warning","feed":"HSL","alertHeaderText":"Route notice","alertDescriptionText":"Route description","alertSeverityLevel":"WARNING","alertEffect":"OTHER_EFFECT","effectiveStartDate":1788844680,"effectiveEndDate":1788850800,"entities":[{"__typename":"Pattern","route":{"gtfsId":"HSL:31M1"}}]},
        {"id":"stop-info","feed":"HSL","alertHeaderText":"Stop notice","alertDescriptionText":"Stop description","alertSeverityLevel":"INFO","alertEffect":"STOP_MOVED","effectiveStartDate":null,"effectiveEndDate":null,"entities":[{"__typename":"StopOnTrip","trip":{"gtfsId":"HSL:trip"},"stop":{"gtfsId":"HSL:1020453"}}]},
        {"id":"unknown-scope","feed":"HSL","alertHeaderText":null,"alertDescriptionText":"Unknown scope","alertSeverityLevel":null,"alertEffect":"NEW_EFFECT","effectiveStartDate":null,"effectiveEndDate":null,"entities":null},
        {"id":"severe-late-source-row","feed":"HSL","alertHeaderText":"Severe notice","alertDescriptionText":"Severe description","alertSeverityLevel":"SEVERE","alertEffect":"NO_SERVICE","effectiveStartDate":1788844680,"effectiveEndDate":1788850800,"entities":[{"__typename":"Route","gtfsId":"HSL:31M1"}]},
        {"id":"future-open","feed":"HSL","alertHeaderText":"Future","alertDescriptionText":"Future","alertSeverityLevel":"SEVERE","alertEffect":null,"effectiveStartDate":1790000000,"effectiveEndDate":null,"entities":[{"__typename":"Route","gtfsId":"HSL:31M1"}]},
        {"id":"expired-open","feed":"HSL","alertHeaderText":"Expired","alertDescriptionText":"Expired","alertSeverityLevel":"SEVERE","alertEffect":null,"effectiveStartDate":null,"effectiveEndDate":1700000000,"entities":[{"__typename":"Stop","gtfsId":"HSL:1020453"}]}
    ]}});
    let missing_stop = json!({"data":{"stop":null}});
    let transport = FixtureTransport::new(vec![
        response(200, stop_body),
        response(200, departure_body.clone()),
        response(200, alert_body),
        response(200, missing_stop),
        response(
            500,
            json!({"secret-never-print":"provider echoed credential"}),
        ),
    ]);

    let (exit, stdout, stderr) = invoke(
        &transport,
        &[
            "reitti", "--json", "stop", "list", "--query", "Railway", "--limit", "2",
        ],
    );
    assert_eq!(exit, 0, "{stderr}");
    let stop = validate_schema("stop-list", &stdout);
    assert_eq!(stop["data"]["stops"][0]["platform"], "3");
    assert_eq!(stop["data"]["stops"][0]["ref"], "stop:HSL:1020453");
    assert_eq!(transport.request_count(), 1);

    let (exit, stdout, stderr) = invoke(
        &transport,
        &[
            "reitti",
            "--json",
            "departure",
            "list",
            "--stop",
            "HSL:1020453",
            "--at",
            "2026-09-08T09:55:00+03:00",
            "--limit",
            "2",
        ],
    );
    assert_eq!(exit, 0, "{stderr}");
    let departure = validate_schema("departure-list", &stdout);
    assert_eq!(departure["data"]["window_seconds"], 7200);
    assert_eq!(
        departure["data"]["departures"][0]["departure"]["observed_realtime"],
        false
    );
    assert!(departure["data"]["departures"][0]["departure"]["estimated_time"].is_null());
    assert_eq!(departure["data"]["departures"][1]["cancelled"], true);
    assert_eq!(
        departure["data"]["departures"][0]["service_date"],
        "2026-09-08"
    );
    assert_eq!(transport.request_count(), 2);

    let text_transport = FixtureTransport::new(vec![response(200, departure_body.clone())]);
    let (exit, text, stderr) = invoke(
        &text_transport,
        &[
            "reitti",
            "departure",
            "list",
            "--stop",
            "HSL:1020453",
            "--at",
            "2026-09-08T09:55:00+03:00",
        ],
    );
    assert_eq!(exit, 0, "{stderr}");
    assert!(text.contains("from 2026-09-08 09:55 +03:00"));
    assert!(text.contains("2026-09-08 06:58 +00:00 sched"));
    assert!(text.contains("40s late"));
    assert!(!text.contains("cancelled · cancelled"));
    assert_eq!(text_transport.request_count(), 1);

    let alert_transport = FixtureTransport::new(vec![response(
        200,
        json!({"data":{"alerts":[
            {"id":"route-warning","feed":"HSL","alertHeaderText":"Route notice","alertDescriptionText":"Route description","alertSeverityLevel":"WARNING","alertEffect":"OTHER_EFFECT","effectiveStartDate":1788844680,"effectiveEndDate":1788850800,"entities":[{"__typename":"Pattern","route":{"gtfsId":"HSL:31M1"}}]},
            {"id":"stop-info","feed":"HSL","alertHeaderText":"Stop notice","alertDescriptionText":"Stop description","alertSeverityLevel":"INFO","alertEffect":"STOP_MOVED","effectiveStartDate":null,"effectiveEndDate":null,"entities":[{"__typename":"StopOnRoute","route":{"gtfsId":"HSL:other"},"stop":{"gtfsId":"HSL:1020453"}}]},
            {"id":"unknown-scope","feed":"HSL","alertHeaderText":null,"alertDescriptionText":"Unknown scope","alertSeverityLevel":null,"alertEffect":"NEW_EFFECT","effectiveStartDate":null,"effectiveEndDate":null,"entities":null},
            {"id":"severe-late-source-row","feed":"HSL","alertHeaderText":"Severe notice","alertDescriptionText":"Severe description","alertSeverityLevel":"SEVERE","alertEffect":"NO_SERVICE","effectiveStartDate":1788844680,"effectiveEndDate":1788850800,"entities":[{"__typename":"Route","gtfsId":"HSL:31M1"}]},
            {"id":"future-open","feed":"HSL","alertHeaderText":"Future","alertDescriptionText":"Future","alertSeverityLevel":"SEVERE","alertEffect":null,"effectiveStartDate":1790000000,"effectiveEndDate":null,"entities":[{"__typename":"Route","gtfsId":"HSL:31M1"}]},
            {"id":"expired-open","feed":"HSL","alertHeaderText":"Expired","alertDescriptionText":"Expired","alertSeverityLevel":"SEVERE","alertEffect":null,"effectiveStartDate":null,"effectiveEndDate":1700000000,"entities":[{"__typename":"Stop","gtfsId":"HSL:1020453"}]}
        ]}}),
    )]);
    let (exit, stdout, stderr) = invoke(
        &alert_transport,
        &[
            "reitti",
            "--json",
            "alert",
            "list",
            "--route",
            "HSL:31M1",
            "--stop",
            "HSL:1020453",
            "--active-at",
            "2026-09-08T09:00:00+03:00",
            "--limit",
            "4",
        ],
    );
    assert_eq!(exit, 0, "{stderr}");
    let alert = validate_schema("alert-list", &stdout);
    let ids = alert["data"]["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "severe-late-source-row",
            "route-warning",
            "stop-info",
            "unknown-scope"
        ]
    );
    assert!(!ids.contains(&"future-open"));
    assert!(!ids.contains(&"expired-open"));
    assert_eq!(alert_transport.request_count(), 1);
    assert!(stdout.contains("alert_scope_unknown"));
    assert!(stdout.contains("alert_validity_unknown"));

    let missing_transport =
        FixtureTransport::new(vec![response(200, json!({"data":{"stop":null}}))]);
    let (exit, stdout, stderr) = invoke(
        &missing_transport,
        &[
            "reitti",
            "--json",
            "departure",
            "list",
            "--stop",
            "HSL:does-not-exist",
        ],
    );
    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(&stderr).unwrap()["error"]["code"],
        "stop_not_found"
    );
    assert_eq!(missing_transport.request_count(), 1);

    let failing_transport =
        FixtureTransport::new(vec![response(500, json!({"secret-never-print":"echo"}))]);
    let (exit, stdout, stderr) = invoke(&failing_transport, &["reitti", "--json", "alert", "list"]);
    assert_eq!(exit, 2);
    assert!(stdout.is_empty());
    assert!(!stderr.contains("secret-never-print"));
    assert!(!stderr.contains("echo"));
    assert_eq!(failing_transport.request_count(), 1);

    let nearby_transport = FixtureTransport::new(vec![response(
        200,
        json!({"data":{"stopsByRadius":{"edges":[{"node":{"distance":125.0,"stop":{
            "gtfsId":"HSL:nearby","name":"Nearby stop","code":null,"platformCode":null,
            "lat":60.17,"lon":24.94,"vehicleMode":"BUS","wheelchairBoarding":"NOT_POSSIBLE"
        }}}]}}}),
    )]);
    let (exit, stdout, stderr) = invoke(
        &nearby_transport,
        &[
            "reitti",
            "--json",
            "stop",
            "list",
            "--near",
            "60.17,24.94",
            "--radius-m",
            "500",
            "--limit",
            "1",
        ],
    );
    assert_eq!(exit, 0, "{stderr}");
    let nearby = validate_schema("stop-list", &stdout);
    assert_eq!(nearby["data"]["search"]["kind"], "near");
    assert_eq!(nearby["data"]["stops"][0]["distance_m"], 125.0);
    assert_eq!(nearby_transport.request_count(), 1);

    let mode_transport = FixtureTransport::new(vec![response(200, departure_body)]);
    let (exit, stdout, stderr) = invoke(
        &mode_transport,
        &[
            "reitti",
            "--json",
            "departure",
            "list",
            "--stop",
            "HSL:1020453",
            "--mode",
            "bus",
            "--limit",
            "1",
        ],
    );
    assert_eq!(exit, 0, "{stderr}");
    let mode_filtered = validate_schema("departure-list", &stdout);
    assert_eq!(mode_filtered["data"]["time_source"], "clock");
    assert_eq!(mode_filtered["data"]["count"], 0);
    assert_eq!(mode_filtered["data"]["complete"], false);
    assert_eq!(mode_transport.request_count(), 1);

    let malformed_transport = FixtureTransport::new(vec![response(
        200,
        json!({"data":{"alerts":[{
            "id":"malformed","feed":"HSL","alertHeaderText":42,"alertDescriptionText":"Description",
            "alertSeverityLevel":null,"alertEffect":null,"effectiveStartDate":null,"effectiveEndDate":null,"entities":[]
        }]}}),
    )]);
    let (exit, stdout, stderr) =
        invoke(&malformed_transport, &["reitti", "--json", "alert", "list"]);
    assert_eq!(exit, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(&stderr).unwrap()["error"]["code"],
        "provider_contract"
    );
    assert_eq!(malformed_transport.request_count(), 1);

    let unresolved_transport = FixtureTransport::new(vec![response(
        200,
        json!({"data":{"alerts":[
            {"id":"trip-only","feed":"HSL","alertHeaderText":"","alertDescriptionText":"Trip relationship unavailable","alertSeverityLevel":null,"alertEffect":null,"effectiveStartDate":null,"effectiveEndDate":null,"entities":[{"__typename":"Trip","gtfsId":"HSL:trip"}]},
            {"id":"stop-on-trip","feed":"HSL","alertHeaderText":"Stop on trip","alertDescriptionText":"Stop on trip","alertSeverityLevel":"INFO","alertEffect":null,"effectiveStartDate":null,"effectiveEndDate":null,"entities":[{"__typename":"StopOnTrip","trip":{"gtfsId":"HSL:trip2"},"stop":{"gtfsId":"HSL:other-stop"}}]},
            {"id":"empty-scope","feed":"HSL","alertHeaderText":"Feed","alertDescriptionText":"Feed","alertSeverityLevel":"INFO","alertEffect":null,"effectiveStartDate":null,"effectiveEndDate":null,"entities":[]}
        ]}}),
    )]);
    let (exit, stdout, stderr) = invoke(
        &unresolved_transport,
        &["reitti", "--json", "alert", "list", "--route", "HSL:31M1"],
    );
    assert_eq!(exit, 0, "{stderr}");
    let unresolved = validate_schema("alert-list", &stdout);
    assert_eq!(unresolved["data"]["count"], 3);
    let scope_warning = unresolved["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|warning| warning["code"] == "alert_scope_unknown")
        .unwrap();
    assert_eq!(
        scope_warning["details"]["alert_ids"],
        json!(["empty-scope", "stop-on-trip", "trip-only"])
    );
    assert_eq!(unresolved_transport.request_count(), 1);

    let empty_transport = FixtureTransport::new(vec![response(
        200,
        json!({"data":{"alerts":[
            {"id":"other-route","feed":"HSL","alertHeaderText":"Other","alertDescriptionText":"Other","alertSeverityLevel":"INFO","alertEffect":null,"effectiveStartDate":1788844680,"effectiveEndDate":1788850800,"entities":[{"__typename":"Route","gtfsId":"HSL:other"}]}
        ]}}),
    )]);
    let (exit, stdout, stderr) = invoke(
        &empty_transport,
        &["reitti", "--json", "alert", "list", "--route", "HSL:31M1"],
    );
    assert_eq!(exit, 0, "{stderr}");
    let empty = validate_schema("alert-list", &stdout);
    assert_eq!(empty["data"]["count"], 0);
    assert!(empty["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["code"] == "normal_service_not_guaranteed"));
    assert_eq!(empty_transport.request_count(), 1);

    match previous {
        Some(value) => unsafe { std::env::set_var("REITTI_CONFIG_FILE", value) },
        None => unsafe { std::env::remove_var("REITTI_CONFIG_FILE") },
    }
}
