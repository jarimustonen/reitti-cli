use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::Path,
    process::Command,
};

use assert_cmd::cargo::cargo_bin;
use fs2::FileExt;
use serde_json::Value;
use tempfile::TempDir;

fn reitti(home: &Path) -> Command {
    let mut command = Command::new(cargo_bin("reitti"));
    command
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("REITTI_CONFIG_FILE")
        .env_remove("DIGITRANSIT_SUBSCRIPTION_KEY")
        .env_remove("REITTI_LANGUAGE")
        .env_remove("REITTI_TIMEZONE")
        .env_remove("REITTI_ROUTING_URL")
        .env_remove("REITTI_GEOCODING_URL")
        .env_remove("REITTI_CONNECT_TIMEOUT")
        .env_remove("REITTI_REQUEST_TIMEOUT")
        .env_remove("REITTI_PRIVATE_MARKERS");
    command
}

fn run(home: &Path, args: &[&str]) -> std::process::Output {
    reitti(home).args(args).output().unwrap()
}

#[test]
fn equivalent_version_spellings_are_byte_identical() {
    let home = TempDir::new().unwrap();
    let outputs = [
        run(home.path(), &["--json", "version"]),
        run(home.path(), &["version", "--json"]),
        run(home.path(), &["--json", "--version"]),
        run(home.path(), &["--version", "--json"]),
    ];
    for output in &outputs {
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        let commit = value["data"]["commit"].as_str().unwrap();
        assert_eq!(commit.len(), 40);
        assert!(commit.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_eq!(value["data"]["skills"], serde_json::json!([]));
    }
    for output in &outputs[1..] {
        assert_eq!(output.stdout, outputs[0].stdout);
        assert_eq!(output.stderr, outputs[0].stderr);
    }
}

#[test]
fn support_commands_have_stable_text_surfaces() {
    let home = TempDir::new().unwrap();
    let version = run(home.path(), &["version"]);
    let alias = run(home.path(), &["--version"]);
    assert!(version.status.success());
    assert_eq!(version.stdout, alias.stdout);
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("reitti 0.0.0 ("));

    let path = run(home.path(), &["config", "path"]);
    assert!(path.status.success());
    assert!(String::from_utf8_lossy(&path.stdout).contains("reitti/config.toml (not created)"));

    let show = reitti(home.path())
        .env("DIGITRANSIT_SUBSCRIPTION_KEY", "synthetic-key")
        .args(["config", "show"])
        .output()
        .unwrap();
    assert!(show.status.success());
    let show_text = String::from_utf8_lossy(&show.stdout);
    assert!(show_text.contains("subscription_key"));
    assert!(show_text.contains("<redacted>"));
    assert!(!show_text.contains("synthetic-key"));

    let doctor = reitti(home.path())
        .env("DIGITRANSIT_SUBSCRIPTION_KEY", "synthetic-key")
        .env("REITTI_PRIVATE_MARKERS", "[\"fictional-private-marker\"]")
        .arg("doctor")
        .output()
        .unwrap();
    assert!(
        doctor.status.success(),
        "{}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    let doctor_text = String::from_utf8_lossy(&doctor.stdout);
    assert!(doctor_text.contains("OK    config.path"));
    assert!(doctor_text.contains("summary: 6 ok, 0 warn, 0 fail"));
}

#[test]
fn structured_help_uses_validated_path_and_exposes_hidden_test_clock() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["journey", "list", "--help", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["data"]["path"],
        serde_json::json!(["journey", "list"])
    );
    let flags = value["data"]["flags"].as_array().unwrap();
    assert!(flags
        .iter()
        .any(|flag| flag["name"] == "--from" && flag["required"] == true));
    assert!(flags
        .iter()
        .any(|flag| flag["name"] == "--frozen-time" && flag["hidden"] == true));

    let invalid = run(home.path(), &["unknown", "--help", "--json"]);
    assert_eq!(invalid.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&invalid.stderr).unwrap();
    assert_eq!(error["error"]["code"], "usage_error");
}

#[test]
fn delimiter_prevents_literal_help_and_json_values_becoming_global_flags() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["skill", "print", "--", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("error: Skill '--json'"));
}

#[test]
fn config_show_redacts_and_resolves_each_key_independently() {
    let home = TempDir::new().unwrap();
    let directory = home.path().join("reitti");
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("config.toml"), "subscription_key = \"file-key\"\nlanguage = \"fi\"\nrouting_url = \"https://file.example/path\"\n").unwrap();
    fs::set_permissions(
        directory.join("config.toml"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let output = reitti(home.path())
        .env("DIGITRANSIT_SUBSCRIPTION_KEY", "env-key")
        .env("REITTI_LANGUAGE", "sv")
        .args([
            "--json",
            "--routing-url",
            "https://flag.example/path",
            "config",
            "show",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(!text.contains("env-key"));
    assert!(!text.contains("file-key"));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["data"]["values"]["subscription_key"]["value"],
        "<redacted>"
    );
    assert_eq!(value["data"]["values"]["subscription_key"]["source"], "env");
    assert_eq!(value["data"]["values"]["language"]["value"], "sv");
    assert_eq!(value["data"]["values"]["routing_url"]["source"], "flag");
}

#[test]
fn malformed_config_never_echoes_secret_source_text() {
    let home = TempDir::new().unwrap();
    let directory = home.path().join("reitti");
    fs::create_dir(&directory).unwrap();
    let secret = "highly-sensitive-config-value";
    fs::write(
        directory.join("config.toml"),
        format!("subscription_key = \"{secret}\n"),
    )
    .unwrap();
    fs::set_permissions(
        directory.join("config.toml"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let output = run(home.path(), &["--json", "config", "show"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
}

#[test]
fn credential_whitespace_and_url_credentials_are_redacted() {
    let home = TempDir::new().unwrap();
    let credential = reitti(home.path())
        .env("DIGITRANSIT_SUBSCRIPTION_KEY", " secret ")
        .args(["--json", "config", "show", "--show-secrets"])
        .output()
        .unwrap();
    assert_eq!(credential.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&credential.stderr).contains(" secret "));

    let url = "https://user:password@example.test/path?token=very-secret";
    let output = run(
        home.path(),
        &["--json", "--routing-url", url, "config", "show"],
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("password"));
    assert!(!stderr.contains("very-secret"));
    assert!(stderr.contains("<redacted>"));
}

#[test]
fn update_is_atomic_private_and_dry_run_writes_only_requested_report() {
    let home = TempDir::new().unwrap();
    let output_path = home.path().join("plan.json");
    let dry = reitti(home.path())
        .args([
            "--json",
            "--output",
            output_path.to_str().unwrap(),
            "config",
            "update",
            "--language",
            "fi",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(
        dry.status.success(),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
    assert!(output_path.exists());
    let plan: Value = serde_json::from_slice(&fs::read(&output_path).unwrap()).unwrap();
    assert_eq!(plan["data"]["dry_run"], true);
    assert!(!home.path().join("reitti").exists());

    let applied = reitti(home.path())
        .args([
            "--json",
            "--verbose",
            "config",
            "update",
            "--subscription-key-stdin",
            "--language",
            "fi",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"secret-key\n")?;
            child.wait_with_output()
        })
        .unwrap();
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(!String::from_utf8_lossy(&applied.stderr).contains("secret-key"));
    assert!(!String::from_utf8_lossy(&applied.stdout).contains("secret-key"));
    let config = home.path().join("reitti/config.toml");
    assert_eq!(
        fs::metadata(&config).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(home.path().join("reitti"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert!(fs::read_to_string(config).unwrap().contains("secret-key"));
}

#[test]
fn first_update_creates_secure_default_tree_without_chmodding_home() {
    let home = TempDir::new().unwrap();
    fs::set_permissions(home.path(), fs::Permissions::from_mode(0o751)).unwrap();
    let mut child = Command::new(cargo_bin("reitti"))
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("REITTI_CONFIG_FILE")
        .args(["--json", "config", "update", "--subscription-key-stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"synthetic-key\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = home.path().join(".config/reitti/config.toml");
    assert_eq!(
        fs::metadata(&config).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(home.path()).unwrap().permissions().mode() & 0o777,
        0o751
    );
    assert_eq!(
        fs::metadata(home.path().join(".config"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}

#[test]
fn held_or_non_regular_config_lock_fails_without_waiting() {
    let home = TempDir::new().unwrap();
    fs::create_dir(home.path().join("reitti")).unwrap();
    let lock_path = home.path().join("reitti/config.lock");
    let lock = fs::File::create(&lock_path).unwrap();
    lock.lock_exclusive().unwrap();
    let output = run(
        home.path(),
        &["--json", "config", "update", "--language", "fi"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("config_conflict"));
    lock.unlock().unwrap();
    fs::remove_file(&lock_path).unwrap();
    fs::create_dir(&lock_path).unwrap();
    let output = run(
        home.path(),
        &["--json", "config", "update", "--language", "fi"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("config_conflict"));
}

#[test]
fn runtime_overrides_never_become_persistent_updates() {
    let home = TempDir::new().unwrap();
    let global_only = run(home.path(), &[
        "--json", "--routing-url", "https://runtime.example/routing",
        "--request-timeout", "9s", "config", "update", "--language", "fi", "--dry-run",
    ]);
    assert!(global_only.status.success(), "{}", String::from_utf8_lossy(&global_only.stderr));
    let plan: Value = serde_json::from_slice(&global_only.stdout).unwrap();
    let keys = plan["data"]["would"].as_array().unwrap().iter().map(|item| item["input"]["key"].as_str().unwrap()).collect::<Vec<_>>();
    assert_eq!(keys, vec!["language"]);

    let persistent = run(home.path(), &[
        "--json", "config", "update", "--set-routing-url", "https://saved.example/routing",
        "--set-request-timeout", "8s", "--dry-run",
    ]);
    assert!(persistent.status.success(), "{}", String::from_utf8_lossy(&persistent.stderr));
    let plan: Value = serde_json::from_slice(&persistent.stdout).unwrap();
    let keys = plan["data"]["would"].as_array().unwrap().iter().map(|item| item["input"]["key"].as_str().unwrap()).collect::<Vec<_>>();
    assert_eq!(keys, vec!["routing_url", "request_timeout"]);
    assert_eq!(plan["data"]["would"][0]["input"]["value"], "https://saved.example/routing");
}

#[test]
fn config_rejects_symlinks_without_following_them() {
    let home = TempDir::new().unwrap();
    let directory = home.path().join("reitti");
    fs::create_dir(&directory).unwrap();
    let victim = home.path().join("victim");
    fs::write(&victim, "subscription_key = \"victim-secret\"\n").unwrap();
    symlink(&victim, directory.join("config.toml")).unwrap();
    let output = run(home.path(), &["--json", "config", "show"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("victim-secret"));
}

#[test]
fn output_files_are_private_and_failures_leave_existing_files_unchanged() {
    let home = TempDir::new().unwrap();
    let output_file = home.path().join("version.json");
    let success = run(
        home.path(),
        &[
            "--json",
            "--output",
            output_file.to_str().unwrap(),
            "version",
        ],
    );
    assert!(success.status.success());
    assert_eq!(
        fs::metadata(&output_file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::write(&output_file, "keep-me").unwrap();
    let failure = run(
        home.path(),
        &[
            "--json",
            "--output",
            output_file.to_str().unwrap(),
            "journey",
            "list",
            "--from",
            "bad",
            "--to",
            "stop:HSL:2",
        ],
    );
    assert_eq!(failure.status.code(), Some(1));
    assert_eq!(fs::read_to_string(output_file).unwrap(), "keep-me");
}

#[test]
fn controls_are_rejected_and_text_errors_stay_on_one_line() {
    let home = TempDir::new().unwrap();
    let output = run(
        home.path(),
        &["location", "list", "--query", "Töölö\n\u{1b}[31m"],
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1);
    assert!(stderr.contains("Töölö\\n\\u{1b}"));
    assert!(!stderr.contains('\u{1b}'));
}

#[test]
fn durations_reject_unknown_units_and_departure_default_reaches_credential_check() {
    let home = TempDir::new().unwrap();
    let invalid = run(
        home.path(),
        &[
            "--json",
            "departure",
            "list",
            "--stop",
            "HSL:1020453",
            "--window",
            "1d",
        ],
    );
    assert_eq!(invalid.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("invalid_duration"));
    let defaulted = run(
        home.path(),
        &["--json", "departure", "list", "--stop", "HSL:1020453"],
    );
    assert_eq!(defaulted.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&defaulted.stderr).contains("credential_missing"));
}

#[test]
fn offline_doctor_has_stable_local_checks_and_no_provider_checks() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["--json", "doctor"]);
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let ids = value["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "config.path",
            "config.values",
            "credential.subscription_key",
            "skill.sync",
            "public_artifacts.private_markers",
            "build.provenance"
        ]
    );
    assert!(!ids.iter().any(|id| id.starts_with("provider.")));
}

#[test]
fn foundation_documents_validate_against_bundled_support_schemas() {
    let home = TempDir::new().unwrap();
    let schema_for = |name: &str| {
        let output = run(home.path(), &["--json", "schema", "show", name]);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        value["data"]["schema"].clone()
    };
    let validate = |schema: Value, instance: &Value| {
        let validator = jsonschema::validator_for(&schema).unwrap();
        if let Err(error) = validator.validate(instance) {
            panic!("schema validation failed: {error}; instance={instance}");
        }
    };

    let version: Value = serde_json::from_slice(&run(home.path(), &["--json", "version"]).stdout).unwrap();
    validate(schema_for("version"), &version["data"]);
    let path: Value = serde_json::from_slice(&run(home.path(), &["--json", "config", "path"]).stdout).unwrap();
    validate(schema_for("config-path"), &path["data"]);
    let show_output = reitti(home.path()).env("DIGITRANSIT_SUBSCRIPTION_KEY", "synthetic-key").args(["--json", "config", "show"]).output().unwrap();
    let show: Value = serde_json::from_slice(&show_output.stdout).unwrap();
    validate(schema_for("config-show"), &show["data"]);
    let update: Value = serde_json::from_slice(&run(home.path(), &["--json", "config", "update", "--language", "fi", "--dry-run"]).stdout).unwrap();
    validate(schema_for("config-update"), &update["data"]);
    let doctor: Value = serde_json::from_slice(&run(home.path(), &["--json", "doctor"]).stdout).unwrap();
    validate(schema_for("doctor"), &doctor["data"]);
    let skills: Value = serde_json::from_slice(&run(home.path(), &["--json", "skill", "list"]).stdout).unwrap();
    validate(schema_for("skill-list"), &skills["data"]);
    let help: Value = serde_json::from_slice(&run(home.path(), &["--json", "config", "show", "--help"]).stdout).unwrap();
    validate(schema_for("help"), &help);
    let error_output = run(home.path(), &["--json", "departure", "list", "--stop", "bad"]);
    let error: Value = serde_json::from_slice(&error_output.stderr).unwrap();
    validate(schema_for("error"), &error);

    let output_file = home.path().join("saved.json");
    let file_result: Value = serde_json::from_slice(&run(home.path(), &["--json", "--output", output_file.to_str().unwrap(), "version"]).stdout).unwrap();
    let schema = schema_for("version");
    validate(schema["$defs"]["file_output"].clone(), &file_result["data"]);
}

#[test]
fn all_domain_default_invocations_validate_before_incomplete_handler() {
    let home = TempDir::new().unwrap();
    let cases: &[&[&str]] = &[
        &["--json", "location", "list", "--query", "Kamppi"],
        &[
            "--json",
            "journey",
            "list",
            "--from",
            "query:Kamppi",
            "--to",
            "stop:HSL:1020453",
        ],
        &["--json", "stop", "list", "--query", "Kamppi"],
        &["--json", "departure", "list", "--stop", "HSL:1020453"],
        &["--json", "alert", "list"],
    ];
    for args in cases {
        let output = reitti(home.path())
            .env("DIGITRANSIT_SUBSCRIPTION_KEY", "test-key")
            .args(*args)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "args={args:?}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("feature_incomplete"));
    }
}
