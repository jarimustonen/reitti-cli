use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    io::Cursor,
    sync::{Mutex, MutexGuard},
};

use chrono::{DateTime, Utc};
use reitti_cli::{
    client::{HttpFuture, HttpRequest, HttpResponse, HttpTransport},
    run_with_transport,
};
use reitti_core::{FixedClock, RequestIdGenerator};
use serde_json::{json, Value};

static ENVIRONMENT: Mutex<()> = Mutex::new(());

#[derive(Debug)]
struct MockTransport {
    requests: Mutex<Vec<HttpRequest>>,
    responses: Mutex<VecDeque<HttpResponse>>,
}
impl MockTransport {
    fn new(bodies: Vec<Value>) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            responses: Mutex::new(
                bodies
                    .into_iter()
                    .map(|body| HttpResponse {
                        status: 200,
                        headers: BTreeMap::new(),
                        body: serde_json::to_vec(&body).unwrap(),
                    })
                    .collect(),
            ),
        }
    }
    fn requests(&self) -> MutexGuard<'_, Vec<HttpRequest>> {
        self.requests.lock().unwrap()
    }
}
impl HttpTransport for MockTransport {
    fn execute(&self, request: HttpRequest) -> HttpFuture<'_> {
        self.requests.lock().unwrap().push(request);
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider request");
        Box::pin(async move { Ok(response) })
    }
}

struct FixedIds;
impl RequestIdGenerator for FixedIds {
    fn next(&self) -> String {
        "req_location_test".to_owned()
    }
}

struct EnvGuard {
    old_key: Option<std::ffi::OsString>,
}
impl EnvGuard {
    fn set() -> (MutexGuard<'static, ()>, Self) {
        let lock = ENVIRONMENT.lock().unwrap();
        let old_key = std::env::var_os("DIGITRANSIT_SUBSCRIPTION_KEY");
        // SAFETY: this integration-test executable serializes all mutations of
        // this variable with ENVIRONMENT and restores the previous value.
        unsafe { std::env::set_var("DIGITRANSIT_SUBSCRIPTION_KEY", "secret-canary") };
        (lock, Self { old_key })
    }
}
impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: guarded by ENVIRONMENT for the lifetime of this value.
        unsafe {
            match self.old_key.take() {
                Some(value) => std::env::set_var("DIGITRANSIT_SUBSCRIPTION_KEY", value),
                None => std::env::remove_var("DIGITRANSIT_SUBSCRIPTION_KEY"),
            }
        }
    }
}

fn run(transport: &dyn HttpTransport, args: &[&str]) -> (u8, Vec<u8>, Vec<u8>) {
    let (_lock, _guard) = EnvGuard::set();
    let mut argv = vec![OsString::from("reitti")];
    argv.extend(args.iter().map(OsString::from));
    let clock = FixedClock::new(
        DateTime::parse_from_rfc3339("2026-09-08T08:13:39Z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let mut stdin = Cursor::new(Vec::new());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = run_with_transport(
        argv,
        &mut stdin,
        &mut stdout,
        &mut stderr,
        &clock,
        &FixedIds,
        transport,
    );
    (exit, stdout, stderr)
}

#[allow(clippy::too_many_arguments)]
fn feature(
    gid: &str,
    name: &str,
    label: &str,
    layer: &str,
    source: &str,
    locality: Option<&str>,
    confidence: Option<f64>,
    latitude: f64,
    longitude: f64,
) -> Value {
    json!({
        "type": "Feature",
        "geometry": {"type": "Point", "coordinates": [longitude, latitude]},
        "properties": {
            "gid": gid,
            "name": name,
            "label": label,
            "layer": layer,
            "source": source,
            "locality": locality,
            "neighbourhood": null,
            "postalcode": null,
            "confidence": confidence
        }
    })
}

fn collection(features: Vec<Value>) -> Value {
    json!({"features": features})
}

fn parse(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

#[test]
fn location_list_filters_in_provider_order_preserves_unicode_and_validates_schema() {
    let transport = MockTransport::new(vec![collection(vec![
        feature(
            "osm:venue:1",
            "Kulturhuset Ääni",
            "Kulturhuset Ääni, Helsingfors",
            "venue",
            "openstreetmap",
            Some("Helsingfors"),
            Some(0.97),
            60.17,
            24.94,
        ),
        feature(
            "osm:address:2",
            "Mannerheimvägen 1",
            "Mannerheimvägen 1, Helsingfors",
            "address",
            "openstreetmap",
            Some("Helsingfors"),
            Some(0.95),
            60.18,
            24.93,
        ),
        feature(
            "osm:venue:3",
            "Andra platsen",
            "Andra platsen, Stockholm",
            "venue",
            "openstreetmap",
            Some("Stockholm"),
            Some(0.91),
            59.33,
            18.07,
        ),
    ])]);
    let (exit, stdout, stderr) = run(
        &transport,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "location",
            "list",
            "--query",
            "Ääni",
            "--kind",
            "venue",
            "--language",
            "sv",
            "--limit",
            "2",
        ],
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let document = parse(&stdout);
    let data = &document["data"];
    assert_eq!(data["count"], 2);
    assert_eq!(data["complete"], false);
    assert_eq!(data["candidates"][0]["name"], "Kulturhuset Ääni");
    assert_eq!(data["candidates"][0]["service_area"], "inside");
    assert_eq!(data["candidates"][1]["name"], "Andra platsen");
    assert_eq!(data["candidates"][1]["service_area"], "outside");
    assert_eq!(
        data["request"],
        json!({"request_id":"req_location_test","language":"sv","timezone":"Europe/Helsinki"})
    );
    assert_eq!(data["source"]["product"], "geocoding-v1");
    assert_eq!(transport.requests().len(), 1);
    assert!(transport.requests()[0].url.contains("size=10"));

    let schema = reitti_cli::support::schema("location-list").unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(data)
        .unwrap();
}

#[test]
fn empty_location_search_is_a_successful_truthfully_incomplete_list() {
    let transport = MockTransport::new(vec![collection(Vec::new())]);
    let (exit, stdout, stderr) = run(
        &transport,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "location",
            "list",
            "--query",
            "Ei löydy",
        ],
    );
    assert_eq!(exit, 0);
    assert!(stderr.is_empty());
    let data = &parse(&stdout)["data"];
    assert_eq!(data["count"], 0);
    assert_eq!(data["candidates"], json!([]));
    assert_eq!(data["complete"], false);
    assert_eq!(transport.requests().len(), 1);
}

#[test]
fn journey_query_refuses_multiple_and_untrusted_singletons_with_copyable_refs() {
    let multiple = MockTransport::new(vec![collection(vec![
        feature(
            "osm:venue:1",
            "A",
            "A, Helsinki",
            "venue",
            "openstreetmap",
            Some("Helsinki"),
            Some(0.99),
            60.1,
            24.9,
        ),
        feature(
            "osm:venue:2",
            "B",
            "B, Helsinki",
            "venue",
            "openstreetmap",
            Some("Helsinki"),
            Some(0.98),
            60.2,
            24.8,
        ),
    ])]);
    let (exit, stdout, stderr) = run(
        &multiple,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "journey",
            "list",
            "--from",
            "query:A",
            "--to",
            "coord:60,24",
        ],
    );
    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
    let error = parse(&stderr);
    assert_eq!(error["error"]["code"], "location_ambiguous");
    assert_eq!(
        error["error"]["details"]["retry_refs"],
        json!(["place:osm:venue:1", "place:osm:venue:2"])
    );
    assert!(!String::from_utf8_lossy(&stderr).contains("secret-canary"));
    assert_eq!(multiple.requests().len(), 1);

    let low = MockTransport::new(vec![collection(vec![feature(
        "osm:venue:3",
        "Låg",
        "Låg",
        "venue",
        "openstreetmap",
        None,
        Some(0.89),
        60.3,
        24.7,
    )])]);
    let (exit, _, stderr) = run(
        &low,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "journey",
            "list",
            "--from",
            "query:Låg",
            "--to",
            "coord:60,24",
        ],
    );
    assert_eq!(exit, 1);
    let error = parse(&stderr);
    assert_eq!(error["error"]["code"], "location_ambiguous");
    assert_eq!(
        error["error"]["details"]["reason"],
        "The only candidate is below the 0.9 confidence threshold."
    );
    assert_eq!(
        error["error"]["details"]["retry_refs"],
        json!(["place:osm:venue:3"])
    );
}

#[test]
fn query_resolution_handles_no_match_broad_locality_and_two_trusted_endpoints() {
    let no_match = MockTransport::new(vec![collection(Vec::new())]);
    let (exit, _, stderr) = run(
        &no_match,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "journey",
            "list",
            "--from",
            "query:Puuttuu",
            "--to",
            "coord:60,24",
        ],
    );
    assert_eq!(exit, 1);
    let error = parse(&stderr);
    assert_eq!(error["error"]["code"], "location_not_found");
    assert_eq!(
        error["error"]["details"]["source"]["product"],
        "geocoding-v1"
    );

    let locality = MockTransport::new(vec![collection(vec![feature(
        "osm:locality:1",
        "Helsinki",
        "Helsinki",
        "locality",
        "openstreetmap",
        Some("Helsinki"),
        Some(1.0),
        60.17,
        24.94,
    )])]);
    let (exit, _, stderr) = run(
        &locality,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "journey",
            "list",
            "--from",
            "query:Helsinki",
            "--to",
            "coord:60,24",
        ],
    );
    assert_eq!(exit, 1);
    let error = parse(&stderr);
    assert_eq!(error["error"]["code"], "location_ambiguous");
    assert_eq!(
        error["error"]["details"]["selected_ref_retry"],
        "place:osm:locality:1"
    );

    let trusted = MockTransport::new(vec![
        collection(vec![feature(
            "osm:venue:from",
            "Lähtö",
            "Lähtö",
            "venue",
            "openstreetmap",
            None,
            Some(0.95),
            60.1,
            24.9,
        )]),
        collection(vec![feature(
            "osm:address:to",
            "Mål",
            "Mål",
            "address",
            "openstreetmap",
            None,
            Some(0.9),
            60.2,
            24.8,
        )]),
    ]);
    let (exit, _, stderr) = run(
        &trusted,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "journey",
            "list",
            "--from",
            "query:Lähtö",
            "--to",
            "query:Mål",
        ],
    );
    assert_eq!(exit, 2);
    let error = parse(&stderr);
    assert_eq!(
        error["error"]["details"]["from"]["resolution"],
        "unique_query"
    );
    assert_eq!(
        error["error"]["details"]["to"]["resolution"],
        "unique_query"
    );
    assert_eq!(error["error"]["details"]["from"]["service_area"], "unknown");
    assert_eq!(trusted.requests().len(), 2);
}

#[test]
fn text_output_escapes_provider_controls_without_damaging_unicode_json() {
    let transport = MockTransport::new(vec![collection(vec![feature(
        "osm:venue:text",
        "Ääni",
        "Ääni\nRivi",
        "venue",
        "openstreetmap",
        Some("Helsinki"),
        Some(1.0),
        60.1,
        24.9,
    )])]);
    let (exit, stdout, stderr) = run(
        &transport,
        &[
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "location",
            "list",
            "--query",
            "Ääni",
        ],
    );
    assert_eq!(exit, 0);
    assert!(stderr.is_empty());
    let text = String::from_utf8(stdout).unwrap();
    assert!(text.contains("Ääni\\nRivi"));
    assert!(!text.contains("Ääni\nRivi"));
}

#[test]
fn selected_place_bypasses_ambiguity_but_stale_and_proven_outside_refs_fail() {
    let selected = feature(
        "osm:venue:selected",
        "Valittu",
        "Valittu, Helsinki",
        "venue",
        "openstreetmap",
        Some("Helsinki"),
        None,
        60.17,
        24.94,
    );
    let transport = MockTransport::new(vec![collection(vec![selected])]);
    let (exit, stdout, stderr) = run(
        &transport,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "journey",
            "list",
            "--from",
            "place:osm:venue:selected",
            "--to",
            "coord:60.2,24.8",
        ],
    );
    assert_eq!(exit, 2);
    assert!(stdout.is_empty());
    let error = parse(&stderr);
    assert_eq!(error["error"]["code"], "feature_incomplete");
    assert_eq!(error["error"]["details"]["stage"], "locations_resolved");
    assert_eq!(
        error["error"]["details"]["from"]["resolution"],
        "stable_place"
    );
    assert_eq!(
        error["error"]["details"]["from"]["coordinates"],
        json!({"latitude":60.17,"longitude":24.94})
    );
    assert_eq!(
        error["error"]["details"]["to"]["resolution"],
        "exact_coordinate"
    );
    assert_eq!(error["error"]["details"]["to"]["service_area"], "unknown");
    assert_eq!(error["error"]["details"]["plan_requests_sent"], 0);
    assert_eq!(transport.requests().len(), 1);

    let stale = MockTransport::new(vec![collection(Vec::new())]);
    let (exit, _, stderr) = run(
        &stale,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "journey",
            "list",
            "--from",
            "place:stale",
            "--to",
            "coord:60,24",
        ],
    );
    assert_eq!(exit, 1);
    assert_eq!(parse(&stderr)["error"]["code"], "location_not_found");

    let outside = MockTransport::new(vec![collection(vec![feature(
        "osm:venue:outside",
        "Ute",
        "Ute, Stockholm",
        "venue",
        "openstreetmap",
        Some("Stockholm"),
        Some(1.0),
        59.3,
        18.0,
    )])]);
    let (exit, _, stderr) = run(
        &outside,
        &[
            "--json",
            "--geocoding-url",
            "https://mock.test/geocoding/v1",
            "journey",
            "list",
            "--from",
            "place:osm:venue:outside",
            "--to",
            "coord:60,24",
        ],
    );
    assert_eq!(exit, 1);
    assert_eq!(
        parse(&stderr)["error"]["code"],
        "location_outside_service_area"
    );
}

#[test]
fn coordinates_make_zero_requests_and_stop_refs_make_exactly_one() {
    let none = MockTransport::new(Vec::new());
    let (exit, _, stderr) = run(
        &none,
        &[
            "--json",
            "journey",
            "list",
            "--from",
            "coord:60.1699,24.9384",
            "--to",
            "coord:60.1776,24.6529",
        ],
    );
    assert_eq!(exit, 2);
    assert_eq!(parse(&stderr)["error"]["details"]["plan_requests_sent"], 0);
    assert!(none.requests().is_empty());

    let stop = MockTransport::new(vec![json!({"data":{"stop":{
        "gtfsId":"HSL:1020453", "name":"Päärautatieasema", "code":"H0301",
        "platformCode":null, "lat":60.170347, "lon":24.941008,
        "vehicleMode":"RAIL", "wheelchairBoarding":"POSSIBLE"
    }}})]);
    let (exit, _, stderr) = run(
        &stop,
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing/v2/hsl/gtfs/v1",
            "journey",
            "list",
            "--from",
            "stop:HSL:1020453",
            "--to",
            "coord:60,24",
        ],
    );
    assert_eq!(exit, 2, "{}", String::from_utf8_lossy(&stderr));
    let error = parse(&stderr);
    assert_eq!(
        error["error"]["details"]["from"]["resolution"],
        "stable_stop"
    );
    assert_eq!(error["error"]["details"]["from"]["ref"], "stop:HSL:1020453");
    assert_eq!(stop.requests().len(), 1);
    let body: Value = serde_json::from_slice(&stop.requests()[0].body).unwrap();
    assert_eq!(body["operationName"], "StopDetail");

    let stale = MockTransport::new(vec![json!({"data":{"stop":null}})]);
    let (exit, _, stderr) = run(
        &stale,
        &[
            "--json",
            "--routing-url",
            "https://mock.test/routing/v2/hsl/gtfs/v1",
            "journey",
            "list",
            "--from",
            "stop:HSL:9999999",
            "--to",
            "coord:60,24",
        ],
    );
    assert_eq!(exit, 1);
    let error = parse(&stderr);
    assert_eq!(error["error"]["code"], "location_not_found");
    assert!(error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("stale"));
    assert_eq!(stale.requests().len(), 1);
}
