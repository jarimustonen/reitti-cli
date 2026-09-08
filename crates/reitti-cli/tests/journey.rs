use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    io::Cursor,
    sync::{Mutex, MutexGuard},
};

use chrono::{DateTime, Utc};
use reitti_cli::{
    client::{HttpFuture, HttpRequest, HttpResponse, HttpTransport},
    run_with_transport, support,
};
use reitti_core::{FixedClock, RequestIdGenerator};
use serde_json::{json, Value};

static ENVIRONMENT: Mutex<()> = Mutex::new(());

struct Transport {
    requests: Mutex<Vec<HttpRequest>>,
    responses: Mutex<VecDeque<Result<HttpResponse, reitti_cli::error::AppError>>>,
}
impl Transport {
    fn json(bodies: Vec<Value>) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            responses: Mutex::new(
                bodies
                    .into_iter()
                    .map(|body| {
                        Ok(HttpResponse {
                            status: 200,
                            headers: BTreeMap::new(),
                            body: serde_json::to_vec(&body).unwrap(),
                        })
                    })
                    .collect(),
            ),
        }
    }
    fn failure() -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            responses: Mutex::new(
                vec![Err(reitti_cli::error::AppError::system(
                    "network_error",
                    "synthetic transport failure",
                ))]
                .into(),
            ),
        }
    }
    fn requests(&self) -> MutexGuard<'_, Vec<HttpRequest>> {
        self.requests.lock().unwrap()
    }
}
impl HttpTransport for Transport {
    fn execute(&self, request: HttpRequest) -> HttpFuture<'_> {
        self.requests.lock().unwrap().push(request);
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected request");
        Box::pin(async move { response })
    }
}
struct Ids;
impl RequestIdGenerator for Ids {
    fn next(&self) -> String {
        "req_journey_test".into()
    }
}

struct Environment {
    key: Option<OsString>,
    timezone: Option<OsString>,
}
impl Environment {
    fn set(timezone: &str) -> (MutexGuard<'static, ()>, Self) {
        let lock = ENVIRONMENT.lock().unwrap();
        let old = Self {
            key: std::env::var_os("DIGITRANSIT_SUBSCRIPTION_KEY"),
            timezone: std::env::var_os("REITTI_TIMEZONE"),
        };
        unsafe {
            std::env::set_var("DIGITRANSIT_SUBSCRIPTION_KEY", "secret-canary");
            std::env::set_var("REITTI_TIMEZONE", timezone);
        }
        (lock, old)
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        unsafe {
            match self.key.take() {
                Some(v) => std::env::set_var("DIGITRANSIT_SUBSCRIPTION_KEY", v),
                None => std::env::remove_var("DIGITRANSIT_SUBSCRIPTION_KEY"),
            }
            match self.timezone.take() {
                Some(v) => std::env::set_var("REITTI_TIMEZONE", v),
                None => std::env::remove_var("REITTI_TIMEZONE"),
            }
        }
    }
}
fn run(transport: &dyn HttpTransport, timezone: &str, args: &[&str]) -> (u8, Vec<u8>, Vec<u8>) {
    let (_lock, _environment) = Environment::set(timezone);
    let mut argv = vec![OsString::from("reitti")];
    argv.extend(args.iter().map(OsString::from));
    let clock = FixedClock::new(
        DateTime::parse_from_rfc3339("2026-09-08T06:56:40Z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let (mut stdin, mut stdout, mut stderr) = (Cursor::new(Vec::new()), Vec::new(), Vec::new());
    let exit = run_with_transport(
        argv,
        &mut stdin,
        &mut stdout,
        &mut stderr,
        &clock,
        &Ids,
        transport,
    );
    (exit, stdout, stderr)
}
fn parse(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
fn body(request: &HttpRequest) -> Value {
    serde_json::from_slice(&request.body).unwrap()
}
fn endpoint(name: &str, stop: Option<(&str, &str, &str)>) -> Value {
    match stop {
        Some((id, platform, boarding)) => {
            json!({"name":name,"lat":60.1,"lon":24.9,"stop":{"gtfsId":id,"name":name,"code":null,"platformCode":platform,"lat":60.1,"lon":24.9,"vehicleMode":"BUS","wheelchairBoarding":boarding}})
        }
        None => json!({"name":name,"lat":60.1,"lon":24.9,"stop":null}),
    }
}
fn evidence(scheduled: &str, estimated: Option<&str>) -> Value {
    json!({"scheduledTime":scheduled,"estimated":estimated.map(|time| json!({"time":time,"delay":"ignored"}))})
}
#[allow(clippy::too_many_arguments)]
fn leg(
    mode: &str,
    start: &str,
    estimated_start: Option<&str>,
    end: &str,
    estimated_end: Option<&str>,
    state: &str,
    interline: Option<bool>,
    route: Option<&str>,
) -> Value {
    json!({
        "mode":mode,"duration":600.0,"distance":1000.0,"realTime":estimated_start.is_some(),"realtimeState":state,"interlineWithPreviousLeg":interline,"headsign":if route.is_some(){json!("Central Station")}else{Value::Null},
        "trip":route.map(|_| json!({"gtfsId":"HSL:trip-1"})),"route":route.map(|short| json!({"gtfsId":format!("HSL:{short}"),"shortName":short,"longName":"Example route","mode":"BUS"})),
        "from":endpoint("Origin",route.map(|_|("HSL:from","4","POSSIBLE"))),"to":endpoint("Destination",route.map(|_|("HSL:to","7","NO_INFORMATION"))),
        "start":evidence(start,estimated_start),"end":evidence(end,estimated_end),"stopCalls":[],
        "steps":if mode=="WALK" { json!([{"distance":25.0,"streetName":"Generated path","relativeDirection":"HARD_RIGHT","absoluteDirection":"NORTH","lat":60.1,"lon":24.9,"bogusName":true,"area":false,"stayOn":false,"exit":null}]) } else { json!([]) },
        "legGeometry":null,"alerts":[]
    })
}
fn itinerary(
    start: &str,
    end: &str,
    duration: Option<i64>,
    transfers: i64,
    walk_distance: Option<f64>,
    legs: Vec<Value>,
) -> Value {
    json!({"start":start,"end":end,"duration":duration,"numberOfTransfers":transfers,"waitingTime":duration.map(|_|120),"walkTime":duration.map(|_|300),"walkDistance":walk_distance,"legs":legs})
}
fn plan(itineraries: Vec<Value>, routing_errors: Vec<Value>, has_next: bool) -> Value {
    json!({"data":{"planConnection":{"routingErrors":routing_errors,"pageInfo":{"hasNextPage":has_next,"endCursor":null},"searchDateTime":"2026-09-08T09:30:00+03:00","edges":itineraries.into_iter().map(|node|json!({"node":node})).collect::<Vec<_>>()}}})
}
fn basic_itinerary(duration: Option<i64>, transfers: i64, walk_distance: Option<f64>) -> Value {
    itinerary(
        "2026-09-08T10:00:00+03:00",
        "2026-09-08T10:30:00+03:00",
        duration,
        transfers,
        walk_distance,
        vec![
            leg(
                "WALK",
                "2026-09-08T10:00:00+03:00",
                None,
                "2026-09-08T10:05:00+03:00",
                None,
                "SCHEDULED",
                Some(false),
                None,
            ),
            leg(
                "BUS",
                "2026-09-08T10:05:00+03:00",
                Some("2026-09-08T10:04:00+03:00"),
                "2026-09-08T10:25:00+03:00",
                Some("2026-09-08T10:27:00+03:00"),
                "UPDATED",
                Some(false),
                Some("10"),
            ),
        ],
    )
}

#[test]
fn ordinary_journey_preserves_offset_variables_navigation_and_exact_schema() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/digitransit/routing-navigation-rich.json"
    ))
    .unwrap();
    let transport = Transport::json(vec![fixture["response"]["body"].clone()]);
    let (exit, stdout, stderr) = run(
        &transport,
        "Europe/Helsinki",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60.168992,24.932366",
            "--to",
            "coord:60.175294,24.684855",
            "--depart-at",
            "2026-09-08T09:30:00+03:00",
            "--wheelchair",
            "--include-geometry",
            "--limit",
            "1",
        ],
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let document = parse(&stdout);
    let data = &document["data"];
    jsonschema::validator_for(&support::schema("journey-list").unwrap())
        .unwrap()
        .validate(data)
        .unwrap();
    assert_eq!(
        data["request"]["time"]["value"],
        "2026-09-08T09:30:00+03:00"
    );
    assert_eq!(data["alternatives"][0]["transfers"], 1);
    assert!(data["alternatives"][0]["legs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|leg| !leg["intermediate_stops"].as_array().unwrap().is_empty()));
    assert!(data["alternatives"][0]["legs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|leg| leg["geometry"].is_object()));
    assert_eq!(
        data["alternatives"][0]["accessibility"]["status"],
        "unknown"
    );
    let generated_step = data["alternatives"][0]["legs"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|leg| leg["steps"].as_array().unwrap())
        .find(|step| step["generated_name"] == true)
        .unwrap();
    assert!(generated_step["street_name"].is_string());
    assert!(!generated_step["instruction"]
        .as_str()
        .unwrap_or_default()
        .contains(generated_step["street_name"].as_str().unwrap()));
    assert_eq!(transport.requests().len(), 1);
    let request = body(&transport.requests()[0]);
    assert_eq!(
        request["variables"]["dateTime"]["earliestDeparture"],
        "2026-09-08T09:30:00+03:00"
    );
    assert_eq!(request["variables"]["first"], 1);
    assert_eq!(
        request["variables"]["preferences"]["accessibility"]["wheelchair"]["enabled"],
        true
    );
}

#[test]
fn arrive_by_uses_first_labels_ties_and_omits_unprovable_superlatives() {
    let one = basic_itinerary(None, 0, None);
    let mut two = basic_itinerary(Some(1800), 0, Some(500.0));
    two["start"] = json!("2026-09-08T10:05:00+03:00");
    let transport = Transport::json(vec![plan(vec![one, two], vec![], false)]);
    let (exit, stdout, stderr) = run(
        &transport,
        "Europe/Helsinki",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:60.1,24.1",
            "--arrive-by",
            "2026-09-08T10:30:00+03:00",
            "--mode",
            "bus",
            "--limit",
            "2",
        ],
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let document = parse(&stdout);
    let alternatives = document["data"]["alternatives"].as_array().unwrap();
    assert_eq!(alternatives[0]["id"], "alt-1");
    assert_eq!(alternatives[1]["id"], "alt-2");
    for alternative in alternatives {
        let labels = alternative["comparison"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["label"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(!labels.contains(&"fastest"));
        assert!(!labels.contains(&"least_walking"));
        assert!(labels.contains(&"earliest_arrival"));
        assert!(labels.contains(&"fewest_transfers"));
    }
    assert_eq!(
        alternatives[1]["comparison"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["label"] == "latest_departure")
            .unwrap()["tied"],
        1
    );
    let warnings = document["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .filter(|w| w["code"] == "comparison_incomplete")
            .count()
            >= 2
    );
    let request = body(&transport.requests()[0]);
    assert_eq!(
        request["variables"]["dateTime"]["latestArrival"],
        "2026-09-08T10:30:00+03:00"
    );
    assert_eq!(request["variables"]["first"], 2);
    assert_eq!(
        request["variables"]["modes"]["transit"]["transit"],
        json!([{"mode":"BUS"}])
    );
}

#[test]
fn walking_cap_excludes_over_cap_and_unknown_and_all_excluded_is_scoped() {
    let transport = Transport::json(vec![plan(
        vec![
            basic_itinerary(Some(1800), 1, Some(100.0)),
            basic_itinerary(Some(1800), 1, Some(900.0)),
            basic_itinerary(Some(1800), 1, None),
        ],
        vec![],
        true,
    )]);
    let (exit, stdout, _) = run(
        &transport,
        "UTC",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
            "--max-walk-m",
            "100",
        ],
    );
    assert_eq!(exit, 0);
    let data = &parse(&stdout)["data"];
    assert_eq!(data["count"], 1);
    assert_eq!(data["complete"], false);
    assert_eq!(data["filtering"]["excluded_over_cap"], 1);
    assert_eq!(data["filtering"]["excluded_unknown_distance"], 1);

    let excluded = Transport::json(vec![plan(
        vec![basic_itinerary(Some(1800), 1, None)],
        vec![],
        false,
    )]);
    let (exit, stdout, stderr) = run(
        &excluded,
        "UTC",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
            "--max-walk-m",
            "0",
        ],
    );
    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
    let error = parse(&stderr);
    assert_eq!(error["error"]["code"], "no_matching_journeys");
    assert_eq!(
        error["error"]["details"]["global_route_absence_proven"],
        false
    );
}

#[test]
fn injected_default_timezone_and_text_realtime_facts_are_honest() {
    let transport = Transport::json(vec![plan(
        vec![basic_itinerary(Some(1800), 0, Some(100.0))],
        vec![],
        false,
    )]);
    let (exit, stdout, stderr) = run(
        &transport,
        "America/New_York",
        &[
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
        ],
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let text = String::from_utf8(stdout).unwrap();
    assert!(text.contains("2026-09-08 02:56 -04:00"));
    assert!(text.contains("Central Station"));
    assert!(!text.contains("Generated path"));
    assert!(text.contains("hard right"));
    assert!(text.contains("platform 4"));
    assert!(text.contains("est"));
    assert!(text.contains("1m early"));
    assert!(text.contains("Realtime: updated."));
    let request = body(&transport.requests()[0]);
    assert_eq!(
        request["variables"]["dateTime"]["earliestDeparture"],
        "2026-09-08T02:56:40-04:00"
    );
}

#[test]
fn routing_domain_errors_with_results_warn_but_empty_and_transport_fail_distinctly() {
    let routing = json!({"code":"NO_TRANSIT_CONNECTION","description":"No transit for one branch","inputField":"modes"});
    let usable = Transport::json(vec![plan(
        vec![basic_itinerary(Some(1800), 2, Some(100.0))],
        vec![routing.clone()],
        false,
    )]);
    let (exit, stdout, _) = run(
        &usable,
        "UTC",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
        ],
    );
    assert_eq!(exit, 0);
    let document = parse(&stdout);
    let warning = document["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "routing_errors")
        .unwrap();
    assert_eq!(
        warning["details"]["routing_errors"][0]["code"],
        "NO_TRANSIT_CONNECTION"
    );
    assert_eq!(document["data"]["alternatives"][0]["transfers"], 2);

    let empty = Transport::json(vec![plan(vec![], vec![routing], false)]);
    let (exit, _, stderr) = run(
        &empty,
        "UTC",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
        ],
    );
    assert_eq!(exit, 1);
    assert_eq!(parse(&stderr)["error"]["code"], "no_journeys");
    let failure = Transport::failure();
    let (exit, _, stderr) = run(
        &failure,
        "UTC",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
        ],
    );
    assert_eq!(exit, 2);
    assert_eq!(parse(&stderr)["error"]["code"], "network_error");
}

#[test]
fn cancelled_interlined_leg_and_alert_are_visible_in_json_and_text() {
    let mut journey = basic_itinerary(Some(1800), 0, Some(100.0));
    let transit = &mut journey["legs"][1];
    transit["realtimeState"] = json!("CANCELED");
    transit["interlineWithPreviousLeg"] = json!(true);
    transit["alerts"] = json!([{
        "id":"alert-1","feed":"HSL","alertHeaderText":"Bus cancelled","alertDescriptionText":"Use another service.",
        "alertSeverityLevel":"SEVERE","alertEffect":"NO_SERVICE","effectiveStartDate":null,"effectiveEndDate":null,
        "entities":[{"__typename":"Route","gtfsId":"HSL:10"}]
    }]);
    let json_transport = Transport::json(vec![plan(vec![journey.clone()], vec![], false)]);
    let (exit, stdout, stderr) = run(
        &json_transport,
        "UTC",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
        ],
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let document = parse(&stdout);
    let alternative = &document["data"]["alternatives"][0];
    assert_eq!(alternative["realtime"]["status"], "cancelled");
    assert_eq!(alternative["realtime"]["scheduled_only_legs"], 0);
    assert_eq!(alternative["legs"][1]["continues_previous_vehicle"], true);
    assert_eq!(alternative["legs"][1]["cancelled"], true);
    assert_eq!(alternative["alerts"][0]["id"], "alert-1");

    let text_transport = Transport::json(vec![plan(vec![journey], vec![], false)]);
    let (exit, stdout, _) = run(
        &text_transport,
        "UTC",
        &[
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
        ],
    );
    assert_eq!(exit, 0);
    let text = String::from_utf8(stdout).unwrap();
    assert!(text.contains("CANCELLED"));
    assert!(text.contains("stay on vehicle"));
    assert!(text.contains("Bus cancelled"));
}

#[test]
fn normalization_incompleteness_survives_as_warning() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/digitransit/routing-navigation-rich.json"
    ))
    .unwrap();
    let mut response = fixture["response"]["body"].clone();
    response["data"]["planConnection"]["edges"][0]["node"]["legs"][0]["legGeometry"]["length"] =
        json!(10_001);
    let transport = Transport::json(vec![response]);
    let (exit, stdout, stderr) = run(
        &transport,
        "UTC",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
            "--include-geometry",
            "--limit",
            "1",
        ],
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let document = parse(&stdout);
    assert!(document["data"]["alternatives"][0]["legs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|leg| leg["navigation_complete"] == false && leg["geometry"].is_null()));
    assert!(document["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["code"] == "navigation_truncated"));
}

#[test]
fn oversized_provider_response_is_locally_bounded_before_comparison() {
    let mut first = basic_itinerary(Some(1800), 1, Some(100.0));
    first["end"] = json!("2026-09-08T10:30:00+03:00");
    let mut second = basic_itinerary(Some(1200), 1, Some(200.0));
    second["end"] = json!("2026-09-08T10:25:00+03:00");
    let mut omitted = basic_itinerary(Some(600), 0, Some(10.0));
    omitted["end"] = json!("2026-09-08T10:10:00+03:00");
    let transport = Transport::json(vec![plan(vec![first, second, omitted], vec![], false)]);
    let (exit, stdout, stderr) = run(
        &transport,
        "UTC",
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing",
            "journey",
            "list",
            "--from",
            "coord:60,24",
            "--to",
            "coord:61,25",
            "--limit",
            "2",
        ],
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let document = parse(&stdout);
    assert_eq!(document["data"]["count"], 2);
    assert_eq!(document["data"]["filtering"]["provider_count"], 3);
    assert_eq!(document["data"]["filtering"]["excluded_over_limit"], 1);
    assert_eq!(document["data"]["complete"], false);
    let alternatives = document["data"]["alternatives"].as_array().unwrap();
    assert_eq!(alternatives[0]["id"], "alt-1");
    assert_eq!(alternatives[1]["id"], "alt-2");
    assert!(alternatives[1]["comparison"]
        .as_array()
        .unwrap()
        .iter()
        .any(|fact| fact["label"] == "fastest"));
    assert!(document["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning["code"] == "alternatives_truncated"));
}

#[test]
fn invalid_grammar_never_reaches_provider() {
    let transport = Transport::json(vec![]);
    let (exit, stdout, stderr) = run(
        &transport,
        "UTC",
        &[
            "--json",
            "journey",
            "list",
            "--from",
            "coord:invalid",
            "--to",
            "coord:60,24",
        ],
    );
    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
    assert_eq!(parse(&stderr)["error"]["code"], "invalid_location_ref");
    assert!(transport.requests().is_empty());
}
