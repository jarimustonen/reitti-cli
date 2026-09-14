use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    io::Cursor,
    path::Path,
    sync::{Mutex, MutexGuard},
};

use chrono::{DateTime, Utc};
use reitti_cli::{
    client::{HttpFuture, HttpRequest, HttpResponse, HttpTransport},
    run_with_transport, support,
};
use reitti_core::{FixedClock, FixedRequestIds};
use serde_json::{json, Value};
use tempfile::TempDir;

static ENVIRONMENT: Mutex<()> = Mutex::new(());

#[derive(Debug)]
struct MockTransport {
    requests: Mutex<Vec<HttpRequest>>,
    responses: Mutex<VecDeque<HttpResponse>>,
}

impl MockTransport {
    fn json(bodies: Vec<Value>) -> Self {
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

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
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

struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    old_key: Option<OsString>,
    old_config: Option<OsString>,
}

impl EnvGuard {
    fn credentialed() -> Self {
        let guard = Self::locked();
        unsafe { std::env::set_var("DIGITRANSIT_SUBSCRIPTION_KEY", "secret-canary") };
        guard
    }

    fn without_credential(config: &Path) -> Self {
        let guard = Self::locked();
        unsafe {
            std::env::remove_var("DIGITRANSIT_SUBSCRIPTION_KEY");
            std::env::set_var("REITTI_CONFIG_FILE", config);
        }
        guard
    }

    fn locked() -> Self {
        let lock = ENVIRONMENT.lock().unwrap();
        Self {
            _lock: lock,
            old_key: std::env::var_os("DIGITRANSIT_SUBSCRIPTION_KEY"),
            old_config: std::env::var_os("REITTI_CONFIG_FILE"),
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match self.old_key.take() {
                Some(value) => std::env::set_var("DIGITRANSIT_SUBSCRIPTION_KEY", value),
                None => std::env::remove_var("DIGITRANSIT_SUBSCRIPTION_KEY"),
            }
            match self.old_config.take() {
                Some(value) => std::env::set_var("REITTI_CONFIG_FILE", value),
                None => std::env::remove_var("REITTI_CONFIG_FILE"),
            }
        }
    }
}

fn invoke(
    transport: &dyn HttpTransport,
    args: &[&str],
    credentialed: bool,
) -> (u8, Vec<u8>, Vec<u8>) {
    let temporary = TempDir::new().unwrap();
    let _environment = if credentialed {
        EnvGuard::credentialed()
    } else {
        EnvGuard::without_credential(&temporary.path().join("missing-config.toml"))
    };
    let mut argv = vec![OsString::from("reitti")];
    argv.extend(args.iter().map(OsString::from));
    let clock = FixedClock::new(
        DateTime::parse_from_rfc3339("2026-09-14T14:00:00Z")
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
        &FixedRequestIds::new("req_stop_test"),
        transport,
    );
    (exit, stdout, stderr)
}

fn fixture(name: &str) -> Value {
    serde_json::from_str(match name {
        "ordinary" => include_str!("../../../tests/fixtures/digitransit/routing-stop-detail.json"),
        "platform" => {
            include_str!("../../../tests/fixtures/digitransit/routing-stop-detail-platform.json")
        }
        _ => unreachable!(),
    })
    .unwrap()
}

fn fixture_body(name: &str) -> Value {
    fixture(name)["response"]["body"].clone()
}

#[test]
fn show_returns_one_schema_valid_bounded_detail_request() {
    let transport = MockTransport::json(vec![fixture_body("ordinary")]);
    let (exit, stdout, stderr) = invoke(
        &transport,
        &["--json", "stop", "show", "HSL:1020453", "--language", "fi"],
        true,
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(transport.request_count(), 1);
    let document: Value = serde_json::from_slice(&stdout).unwrap();
    let data = &document["data"];
    jsonschema::validator_for(&support::schema("stop-show").unwrap())
        .unwrap()
        .validate(data)
        .unwrap();
    assert_eq!(data["stop"]["name"], "Päärautatieasema");
    assert_eq!(data["stop"]["zone"], "A");
    assert_eq!(data["stop"]["parent_station"], Value::Null);
    assert_eq!(
        data["stop"]["reittiopas_url"],
        "https://reittiopas.hsl.fi/pysakit/HSL%3A1020453"
    );
    assert_eq!(data["request"]["request_id"], "req_stop_test");
    assert_eq!(data["request"]["language"], "fi");
    assert_eq!(data["source"]["provider"], "digitransit");
    assert!(data["source"]["attribution"]
        .as_str()
        .unwrap()
        .contains("Digitransit"));

    let requests = transport.requests.lock().unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["operationName"], "StopDetail");
    assert_eq!(body["variables"], json!({"id":"HSL:1020453"}));
    assert!(body["query"]
        .as_str()
        .unwrap()
        .contains("parentStation { gtfsId name }"));
    assert!(!body["query"].as_str().unwrap().contains("patterns"));
    assert_eq!(requests[0].headers["accept-language"].expose(), "fi");
    assert!(!format!("{:?}", requests[0]).contains("secret-canary"));
}

#[test]
fn platform_response_preserves_null_unknown_and_parent_values() {
    let transport = MockTransport::json(vec![fixture_body("platform")]);
    let (exit, stdout, stderr) =
        invoke(&transport, &["--json", "stop", "show", "HSL:1174504"], true);
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let data = serde_json::from_slice::<Value>(&stdout).unwrap()["data"].clone();
    assert_eq!(data["stop"]["code"], Value::Null);
    assert_eq!(data["stop"]["coordinates"], Value::Null);
    assert_eq!(data["stop"]["zone"], Value::Null);
    assert_eq!(data["stop"]["modes"], json!([]));
    assert_eq!(data["stop"]["wheelchair_boarding"], "unknown");
    assert_eq!(data["stop"]["parent_station"]["id"], "HSL:1000202");
}

#[test]
fn raw_id_validation_and_missing_argument_fail_before_io() {
    for arguments in [
        vec![
            "--json",
            "--output",
            "/definitely-missing-reitti-parent/out.json",
            "stop",
            "show",
            "bad",
        ],
        vec!["--json", "stop", "show", "stop:HSL:1020453"],
        vec!["--json", "stop", "show", "HSL:bad/path"],
        vec!["--json", "stop", "show", ""],
        vec!["--json", "stop", "show"],
    ] {
        let transport = MockTransport::json(vec![]);
        let (exit, stdout, stderr) = invoke(&transport, &arguments, true);
        assert_eq!(exit, 1, "arguments={arguments:?}");
        assert!(stdout.is_empty());
        assert_eq!(transport.request_count(), 0);
        let error: Value = serde_json::from_slice(&stderr).unwrap();
        assert!(matches!(
            error["error"]["code"].as_str(),
            Some("invalid_stop_id" | "usage_error")
        ));
        if arguments.last() == Some(&"bad") {
            assert_eq!(error["error"]["code"], "invalid_stop_id");
        }
    }
}

#[test]
fn unknown_stop_and_provider_error_keep_existing_error_contracts() {
    let unknown = MockTransport::json(vec![json!({"data":{"stop":null}})]);
    let (exit, stdout, stderr) = invoke(&unknown, &["--json", "stop", "show", "HSL:9999999"], true);
    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&stderr).unwrap()["error"]["code"],
        "stop_not_found"
    );
    assert_eq!(unknown.request_count(), 1);

    let malformed = MockTransport::json(vec![json!({"data":{}})]);
    let (exit, stdout, stderr) =
        invoke(&malformed, &["--json", "stop", "show", "HSL:1020453"], true);
    assert_eq!(exit, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&stderr).unwrap()["error"]["code"],
        "provider_contract"
    );

    let failed = MockTransport::json(vec![json!({"errors":[{"message":"synthetic"}]})]);
    let (exit, stdout, stderr) = invoke(&failed, &["--json", "stop", "show", "HSL:1020453"], true);
    assert_eq!(exit, 2);
    assert!(stdout.is_empty());
    let error: Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(error["error"]["code"], "provider_graphql");
    assert!(!String::from_utf8_lossy(&stderr).contains("secret-canary"));
}

#[test]
fn credential_failure_precedes_transport() {
    let transport = MockTransport::json(vec![]);
    let (exit, stdout, stderr) = invoke(
        &transport,
        &["--json", "stop", "show", "HSL:1020453"],
        false,
    );
    assert_eq!(exit, 1);
    assert!(stdout.is_empty());
    assert_eq!(transport.request_count(), 0);
    assert_eq!(
        serde_json::from_slice::<Value>(&stderr).unwrap()["error"]["code"],
        "credential_missing"
    );
}

#[test]
fn text_escapes_controls_preserves_unicode_and_output_file_is_atomic() {
    let mut body = fixture_body("ordinary");
    body["data"]["stop"]["name"] = json!("Päärautatieasema\nraide");
    body["data"]["stop"]["wheelchairBoarding"] = json!("NOT_POSSIBLE");
    let text_transport = MockTransport::json(vec![body.clone()]);
    let (exit, stdout, stderr) = invoke(&text_transport, &["stop", "show", "HSL:1020453"], true);
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let text = String::from_utf8(stdout).unwrap();
    assert!(text.contains("Päärautatieasema\\nraide"));
    assert!(!text.contains("Päärautatieasema\nraide"));
    assert!(text.contains("wheelchair: not_accessible"));
    assert!(text.contains("Reittiopas: https://reittiopas.hsl.fi/pysakit/HSL%3A1020453"));
    assert!(text.lines().last().unwrap().contains("Digitransit"));

    let directory = TempDir::new().unwrap();
    let destination = directory.path().join("stop.json");
    let output_transport = MockTransport::json(vec![body]);
    let destination_text = destination.to_string_lossy().into_owned();
    let (exit, stdout, stderr) = invoke(
        &output_transport,
        &[
            "--json",
            "--output",
            &destination_text,
            "stop",
            "show",
            "HSL:1020453",
        ],
        true,
    );
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&stderr));
    let metadata: Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(metadata["data"]["path"], destination_text);
    let saved: Value = serde_json::from_slice(&std::fs::read(destination).unwrap()).unwrap();
    assert_eq!(saved["data"]["stop"]["id"], "HSL:1020453");
}
