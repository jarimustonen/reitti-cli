use std::time::Duration;

use reitti_core::{Clock, Geocoder, Router};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{client::HttpTransport, config, error::AppError};

pub const SCHEMA_VERSION: u8 = 1;
pub const SCHEMA_NAMES: &[&str] = &[
    "location-list",
    "journey-list",
    "stop-list",
    "departure-list",
    "alert-list",
    "config-path",
    "config-show",
    "config-update",
    "schema-list",
    "schema-show",
    "version",
    "doctor",
    "skill-list",
    "skill-print",
    "skill-install",
    "error",
    "help",
];

#[derive(Debug, Serialize)]
pub struct BuildProvenance {
    pub kind: &'static str,
    pub note: &'static str,
}

#[derive(Debug, Serialize)]
pub struct VersionData {
    pub version: &'static str,
    pub commit: Option<&'static str>,
    pub build_provenance: BuildProvenance,
    pub schema_version: u8,
    pub supported_schemas: [u8; 1],
    pub skills: Vec<Value>,
}

pub fn version_data() -> VersionData {
    let raw = option_env!("REITTI_GIT_COMMIT").unwrap_or("");
    VersionData {
        version: env!("CARGO_PKG_VERSION"),
        commit: (!raw.is_empty()).then_some(raw),
        build_provenance: BuildProvenance {
            kind: env!("REITTI_BUILD_KIND"),
            note: env!("REITTI_BUILD_NOTE"),
        },
        schema_version: SCHEMA_VERSION,
        supported_schemas: [SCHEMA_VERSION],
        skills: vec![serde_json::to_value(crate::skill::metadata())
            .expect("static skill metadata serializes")],
    }
}

pub fn version_text() -> String {
    let version = version_data();
    match version.commit {
        Some(commit) => format!("reitti {} ({commit})", version.version),
        None => format!(
            "reitti {} ({} source)",
            version.version, version.build_provenance.kind
        ),
    }
}

pub fn schema(name: &str) -> Result<Value, AppError> {
    if !SCHEMA_NAMES.contains(&name) {
        return Err(AppError::invalid(
            "schema_not_found",
            format!("Schema '{name}' was not found."),
            name,
            json!(SCHEMA_NAMES),
        ));
    }
    let body = match name {
        "version" => object_schema(
            &[
                "version",
                "commit",
                "build_provenance",
                "schema_version",
                "supported_schemas",
                "skills",
            ],
            json!({
                "version":{"type":"string"},
                "commit":{"type":["string","null"],"pattern":"^[0-9a-fA-F]{40}$"},
                "build_provenance":{"type":"object","required":["kind","note"],"properties":{"kind":{"enum":["git","tarball","vendored","ci-injected"]},"note":{"type":"string"}},"additionalProperties":true},
                "schema_version":{"const":1},
                "supported_schemas":{"type":"array","items":{"type":"integer"}},
                "skills":{"type":"array","minItems":1,"maxItems":1,"items":skill_metadata_schema()}
            }),
        ),
        "config-path" => object_schema(
            &["path", "exists", "source"],
            json!({"path":{"type":"string"},"exists":{"type":"boolean"},"source":{"enum":["env","xdg","default"]}}),
        ),
        "config-show" => object_schema(
            &["path", "values"],
            json!({"path":{"type":"string"},"values":{"type":"object","required":["subscription_key","language","timezone","routing_url","geocoding_url","connect_timeout","request_timeout","private_markers"],"additionalProperties":false,"properties":{"subscription_key":config_value_schema(),"language":config_value_schema(),"timezone":config_value_schema(),"routing_url":config_value_schema(),"geocoding_url":config_value_schema(),"connect_timeout":config_value_schema(),"request_timeout":config_value_schema(),"private_markers":config_value_schema()}}}),
        ),
        "config-update" => json!({"oneOf":[
            object_schema(&["path","updated","unchanged","values"], json!({"path":{"type":"string"},"updated":string_array_schema(),"unchanged":string_array_schema(),"values":{"type":"object"}})),
            object_schema(&["path","dry_run","would","unchanged"], json!({"path":{"type":"string"},"dry_run":{"const":true},"would":{"type":"array","items":planning_item_schema()},"unchanged":string_array_schema()}))
        ]}),
        "schema-list" => object_schema(
            &["schemas"],
            json!({"schemas":{"type":"array","items":object_schema(&["name","schema_version"],json!({"name":{"type":"string"},"schema_version":{"const":1}}))}}),
        ),
        "schema-show" => object_schema(
            &["name", "dialect", "schema"],
            json!({"name":{"type":"string"},"dialect":{"const":"https://json-schema.org/draft/2020-12/schema"},"schema":{"type":"object"}}),
        ),
        "doctor" => object_schema(
            &["online", "checks", "summary"],
            json!({"online":{"type":"boolean"},"checks":{"type":"array","items":object_schema(&["id","status","message","fix_suggestion","details"],json!({"id":{"type":"string"},"status":{"enum":["ok","warn","fail"]},"message":{"type":"string"},"fix_suggestion":{"type":["string","null"]},"details":{"type":"object"}}))},"summary":object_schema(&["ok","warn","fail"],json!({"ok":{"type":"integer","minimum":0},"warn":{"type":"integer","minimum":0},"fail":{"type":"integer","minimum":0}}))}),
        ),
        "skill-list" => strict_object_schema(
            &["skills", "supported_agents", "install"],
            json!({
                "skills":{"type":"array","minItems":1,"maxItems":1,"items":skill_metadata_schema()},
                "supported_agents":{"const":["claude","pi","codex"]},
                "install":strict_object_schema(
                    &["selection_flag","default","accepted_values","target_flag","dry_run_flag","force_flag","interactive","no_clobber_default","overwrite_requires_force","layouts"],
                    json!({
                        "selection_flag":{"const":"--agent"},"default":{"const":"all"},
                        "accepted_values":{"const":["claude","pi","codex","all"]},
                        "target_flag":{"const":"--target"},"dry_run_flag":{"const":"--dry-run"},"force_flag":{"const":"--force"},
                        "interactive":{"const":false},"no_clobber_default":{"const":true},"overwrite_requires_force":{"const":true},
                        "layouts":{"type":"array","minItems":3,"maxItems":3,"items":strict_object_schema(&["agent","path","form"],json!({"agent":{"enum":["claude","pi","codex"]},"path":{"type":"string","minLength":1},"form":{"const":"agent-skills-tree"}}))}
                    })
                )
            }),
        ),
        "skill-print" => strict_object_schema(
            &[
                "name",
                "cli_version",
                "schema_version_skill",
                "content",
                "path_in_repo",
                "resources",
            ],
            json!({"name":{"const":"reitti"},"cli_version":{"const":env!("CARGO_PKG_VERSION")},"schema_version_skill":{"const":1},"content":{"type":"string"},"path_in_repo":{"type":"string","pattern":"^crates/reitti-cli/skills/reitti/"},"resources":{"const":crate::skill::resource_paths()}}),
        ),
        "skill-install" => json!({"oneOf":[
            strict_object_schema(
                &["name", "agent", "installed", "existed", "skipped"],
                json!({"name":{"const":"reitti"},"agent":{"enum":["claude","pi","codex","all"]},"installed":string_array_schema(),"existed":string_array_schema(),"skipped":string_array_schema()}),
            ),
            strict_object_schema(
                &["dry_run", "would"],
                json!({"dry_run":{"const":true},"would":{"type":"array","minItems":1,"maxItems":3,"items":strict_planning_item_schema()}}),
            )
        ]}),
        "error" => object_schema(
            &["schema_version", "error"],
            json!({"schema_version":{"const":1},"error":{"type":"object","required":["code","message","retryable","details"],"properties":{"code":{"type":"string"},"message":{"type":"string"},"invalid_value":{},"expected":{},"retryable":{"type":"boolean"},"details":{"type":"object"}},"additionalProperties":true}}),
        ),
        "help" => object_schema(
            &["schema_version", "data", "warnings"],
            json!({"schema_version":{"const":1},"data":object_schema(&["path","summary","usage","args","flags","subcommands","exit_codes","examples"],json!({"path":string_array_schema(),"summary":{"type":"string"},"usage":{"type":"string"},"args":{"type":"array"},"flags":{"type":"array"},"subcommands":{"type":"array"},"exit_codes":{"type":"array"},"examples":{"type":"array"}})),"warnings":{"type":"array"}}),
        ),
        "location-list" => crate::handlers::location::schema(),
        "journey-list" => crate::handlers::journey::schema(),
        "stop-list" => crate::handlers::stop::stop_schema(),
        "departure-list" => crate::handlers::stop::departure_schema(),
        "alert-list" => crate::handlers::alert::schema(),
        _ => unreachable!("schema name was validated above"),
    };
    let wrapped = matches!(name, "error" | "help");
    let mut document = body;
    document["$schema"] = json!("https://json-schema.org/draft/2020-12/schema");
    document["$id"] = json!(format!("urn:reitti:schema:v1:{name}"));
    document["title"] = json!(format!(
        "reitti {name} {}",
        if wrapped { "document" } else { "data" }
    ));
    if !document.get("$defs").is_some_and(Value::is_object) {
        document["$defs"] = json!({});
    }
    document["$defs"]["file_output"] = file_output_schema();
    Ok(document)
}

fn object_schema(required: &[&str], properties: Value) -> Value {
    json!({"type":"object","required":required,"properties":properties,"additionalProperties":true})
}
fn strict_object_schema(required: &[&str], properties: Value) -> Value {
    json!({"type":"object","required":required,"properties":properties,"additionalProperties":false})
}
fn skill_metadata_schema() -> Value {
    strict_object_schema(
        &["name", "description", "cli_version", "schema_version"],
        json!({
            "name":{"const":"reitti"},
            "description":{"type":"string","maxLength":1024},
            "cli_version":{"const":env!("CARGO_PKG_VERSION")},
            "schema_version":{"const":1}
        }),
    )
}
fn strict_planning_item_schema() -> Value {
    strict_object_schema(
        &[
            "action",
            "resource",
            "input",
            "known_effects",
            "unknown_until_apply",
        ],
        json!({"action":{"enum":["create","replace","none"]},"resource":{"const":"agent-skill-tree"},"input":{"type":"object","required":["name","agent","destination"],"properties":{"name":{"const":"reitti"},"agent":{"enum":["claude","pi","codex"]},"destination":{"type":"string"}},"additionalProperties":false},"known_effects":{"type":"object"},"unknown_until_apply":{"type":"array","maxItems":0}}),
    )
}
fn string_array_schema() -> Value {
    json!({"type":"array","items":{"type":"string"}})
}
fn config_value_schema() -> Value {
    object_schema(
        &["value", "source", "secret"],
        json!({"value":{},"source":{"enum":["flag","env","file","default"]},"secret":{"type":"boolean"}}),
    )
}
fn planning_item_schema() -> Value {
    object_schema(
        &[
            "action",
            "resource",
            "input",
            "known_effects",
            "unknown_until_apply",
        ],
        json!({"action":{"type":"string"},"resource":{"type":"string"},"input":{"type":"object"},"known_effects":{"type":"object"},"unknown_until_apply":{"type":"array","items":{"type":"string"}}}),
    )
}
fn file_output_schema() -> Value {
    object_schema(
        &["path", "bytes", "content_type", "schema_version_written"],
        json!({"path":{"type":"string"},"bytes":{"type":"integer","minimum":0},"content_type":{"enum":["application/json","text/plain"]},"schema_version_written":{"const":1}}),
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub id: &'static str,
    pub status: CheckStatus,
    pub message: String,
    pub fix_suggestion: Option<String>,
    pub details: Value,
}

#[derive(Debug, Serialize)]
pub struct DoctorSummary {
    pub ok: usize,
    pub warn: usize,
    pub fail: usize,
}

#[derive(Debug, Serialize)]
pub struct DoctorData {
    pub online: bool,
    pub checks: Vec<DoctorCheck>,
    pub summary: DoctorSummary,
}

pub fn doctor(
    online: bool,
    overrides: &config::GlobalOverrides,
    clock: &dyn Clock,
    transport: &dyn HttpTransport,
) -> DoctorData {
    let path_result = config::resolve_path();
    let mut checks = Vec::new();
    checks.push(match &path_result {
        Ok(path) => match config::inspect_path(&path.path) {
            Ok(()) => DoctorCheck {
                id: "config.path",
                status: CheckStatus::Ok,
                message: format!(
                    "Config path '{}' is absolute and has a secure file type and mode.",
                    path.path.display()
                ),
                fix_suggestion: None,
                details: json!({"path": path.path, "exists": path.exists}),
            },
            Err(error) => DoctorCheck {
                id: "config.path",
                status: CheckStatus::Fail,
                message: error.message,
                fix_suggestion: Some("Replace the config path with a mode-0600 regular, non-symlink file.".to_owned()),
                details: json!({"path": path.path, "exists": path.exists, "error_code": error.code}),
            },
        },
        Err(error) => DoctorCheck {
            id: "config.path",
            status: CheckStatus::Fail,
            message: error.message.clone(),
            fix_suggestion: Some(
                "Set REITTI_CONFIG_FILE to an absolute regular-file path.".to_owned(),
            ),
            details: json!({}),
        },
    });
    let loaded = config::load(overrides);
    checks.push(match &loaded {
        Ok(_) => DoctorCheck {
            id: "config.values",
            status: CheckStatus::Ok,
            message: "Configured values parse safely.".to_owned(),
            fix_suggestion: None,
            details: json!({}),
        },
        Err(error) => DoctorCheck {
            id: "config.values",
            status: CheckStatus::Fail,
            message: error.message.clone(),
            fix_suggestion: Some(
                "Correct the reported config source, then rerun reitti doctor.".to_owned(),
            ),
            details: json!({"error_code": error.code}),
        },
    });
    checks.push(match &loaded {
        Ok(config) if config.subscription_key.is_some() => DoctorCheck { id: "credential.subscription_key", status: CheckStatus::Ok, message: "Digitransit subscription key is present (value redacted).".to_owned(), fix_suggestion: None, details: json!({"present": true}) },
        Ok(_) => DoctorCheck { id: "credential.subscription_key", status: CheckStatus::Fail, message: "Digitransit subscription key is absent.".to_owned(), fix_suggestion: Some("Set DIGITRANSIT_SUBSCRIPTION_KEY or pipe one line to reitti config update --subscription-key-stdin.".to_owned()), details: json!({"present": false}) },
        Err(_) => DoctorCheck { id: "credential.subscription_key", status: CheckStatus::Fail, message: "Credential presence could not be determined because configuration is invalid.".to_owned(), fix_suggestion: Some("Fix config.values first.".to_owned()), details: json!({"present": null}) },
    });
    checks.push(skill_sync_check());
    checks.push(match &loaded {
        Ok(config) if config.private_markers.0.is_empty() => DoctorCheck {
            id: "public_artifacts.private_markers",
            status: CheckStatus::Warn,
            message: "Private-marker scan is unconfigured.".to_owned(),
            fix_suggestion: Some(
                "Set REITTI_PRIVATE_MARKERS to a JSON array of exact markers.".to_owned(),
            ),
            details: json!({"configured": false}),
        },
        Ok(config) => {
            let bundled_text = format!(
                "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                include_str!("../README.md"),
                include_str!("../CONTRIBUTING.md"),
                include_str!("../SECURITY.md"),
                include_str!("../LICENSE"),
                std::str::from_utf8(crate::skill::RESOURCES[0].bytes).unwrap_or(""),
                std::str::from_utf8(crate::skill::RESOURCES[1].bytes).unwrap_or(""),
                env!("CARGO_PKG_NAME"),
                config::DEFAULT_ROUTING_URL,
                config::DEFAULT_GEOCODING_URL
            )
            .to_lowercase();
            let match_count = config
                .private_markers
                .0
                .iter()
                .filter(|marker| bundled_text.contains(&marker.to_lowercase()))
                .count();
            DoctorCheck {
                id: "public_artifacts.private_markers",
                status: if match_count == 0 { CheckStatus::Ok } else { CheckStatus::Fail },
                message: if match_count == 0 {
                    "Configured private markers were not found in bundled public text.".to_owned()
                } else {
                    format!("{match_count} private marker(s) matched bundled public text; values redacted.")
                },
                fix_suggestion: (match_count > 0).then(|| "Review bundled public artifacts; marker values remain redacted.".to_owned()),
                details: json!({"configured": true, "match_count": match_count}),
            }
        },
        Err(_) => DoctorCheck {
            id: "public_artifacts.private_markers",
            status: CheckStatus::Fail,
            message: "Private-marker configuration could not be validated.".to_owned(),
            fix_suggestion: Some("Fix config.values first.".to_owned()),
            details: json!({}),
        },
    });
    let version = version_data();
    let valid_commit = version
        .commit
        .map(|commit| commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .unwrap_or(matches!(
            version.build_provenance.kind,
            "tarball" | "vendored"
        ));
    checks.push(DoctorCheck {
        id: "build.provenance",
        status: if valid_commit {
            CheckStatus::Ok
        } else {
            CheckStatus::Fail
        },
        message: if valid_commit {
            format!(
                "Build provenance is valid ({}).",
                version.build_provenance.kind
            )
        } else {
            "Build provenance is invalid.".to_owned()
        },
        fix_suggestion: (!valid_commit)
            .then(|| "Rebuild from the repository or inject a full commit SHA in CI.".to_owned()),
        details: json!({"kind": version.build_provenance.kind}),
    });
    if online {
        append_online_checks(&mut checks, &loaded, clock, transport);
    }
    let summary = DoctorSummary {
        ok: checks
            .iter()
            .filter(|check| matches!(check.status, CheckStatus::Ok))
            .count(),
        warn: checks
            .iter()
            .filter(|check| matches!(check.status, CheckStatus::Warn))
            .count(),
        fail: checks
            .iter()
            .filter(|check| matches!(check.status, CheckStatus::Fail))
            .count(),
    };
    DoctorData {
        online,
        checks,
        summary,
    }
}

fn skill_sync_check() -> DoctorCheck {
    let Some(home) = std::env::var_os("HOME") else {
        return DoctorCheck {
            id: "skill.sync",
            status: CheckStatus::Warn,
            message:
                "Installed companion-skill versions could not be checked because HOME is not set."
                    .to_owned(),
            fix_suggestion: Some("Set HOME, then rerun reitti doctor.".to_owned()),
            details: json!({"installed": []}),
        };
    };
    let layouts = [
        ("claude", ".claude/skills/reitti/SKILL.md"),
        ("pi", ".pi/agent/skills/reitti/SKILL.md"),
        ("codex", ".codex/skills/reitti/SKILL.md"),
    ];
    let mut installed = Vec::new();
    let mut mismatches = Vec::new();
    for (agent, relative) in layouts {
        let path = std::path::PathBuf::from(&home).join(relative);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let frontmatter = crate::skill_manifest::parse(&text).ok();
                let version = frontmatter
                    .as_ref()
                    .map(|value| value.cli_version.to_owned());
                let schema_version = frontmatter.as_ref().map(|value| value.schema_version);
                installed.push(json!({"agent":agent,"path":path,"cli_version":version,"schema_version":schema_version}));
                if version.as_deref() != Some(crate::skill::CLI_VERSION)
                    || schema_version != Some(crate::skill::SKILL_SCHEMA_VERSION)
                {
                    mismatches.push(agent);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                installed.push(json!({"agent":agent,"path":path,"error":error.to_string()}));
                mismatches.push(agent);
            }
        }
    }
    if mismatches.is_empty() {
        DoctorCheck {
            id: "skill.sync",
            status: CheckStatus::Ok,
            message: if installed.is_empty() {
                format!(
                    "Bundled skill '{}' is version {}; no installed copies were found.",
                    crate::skill::NAME,
                    crate::skill::CLI_VERSION
                )
            } else {
                format!(
                    "All {} installed companion-skill copy/copies match CLI version {}.",
                    installed.len(),
                    crate::skill::CLI_VERSION
                )
            },
            fix_suggestion: None,
            details: json!({"bundled": [crate::skill::metadata()], "installed": installed}),
        }
    } else {
        DoctorCheck {
            id: "skill.sync",
            status: CheckStatus::Warn,
            message: format!("Installed companion skill is missing or version-mismatched for: {}.", mismatches.join(", ")),
            fix_suggestion: Some("Review local skill changes, then run reitti skill install reitti --agent all --force if replacement is intended.".to_owned()),
            details: json!({"bundled": [crate::skill::metadata()], "installed": installed, "mismatched_agents": mismatches}),
        }
    }
}

fn append_online_checks(
    checks: &mut Vec<DoctorCheck>,
    loaded: &Result<config::EffectiveConfig, AppError>,
    clock: &dyn Clock,
    transport: &dyn HttpTransport,
) {
    match loaded {
        Ok(config) if config.subscription_key.is_some() => {
            let key = config
                .subscription_key
                .as_ref()
                .expect("checked")
                .0
                .expose()
                .to_owned();
            let connect = Duration::from_millis(
                config::validate_duration("connect_timeout", &config.connect_timeout.0, None)
                    .expect("loaded config was validated"),
            );
            let request = Duration::from_millis(
                config::validate_duration("request_timeout", &config.request_timeout.0, None)
                    .expect("loaded config was validated"),
            );
            let geocoding = crate::digitransit::DigitransitGeocoder::new(
                &config.geocoding_url.0,
                key.clone(),
                transport,
                clock,
                connect,
                request,
            )
            .map_err(|error| error.to_string())
            .and_then(|client| {
                crate::handlers::await_provider(client.probe())
                    .map(|_| ())
                    .map_err(|error| error.message)
            });
            checks.push(provider_check("provider.geocoding", "geocoding", geocoding));
            let routing = crate::digitransit::DigitransitRouter::new(
                &config.routing_url.0,
                key,
                transport,
                clock,
                connect,
                request,
            )
            .map_err(|error| error.to_string())
            .and_then(|client| {
                crate::handlers::await_provider(client.probe())
                    .map(|_| ())
                    .map_err(|error| error.message)
            });
            checks.push(provider_check("provider.routing_v2", "routing-v2", routing));
        }
        _ => {
            checks.push(skipped_provider_check("provider.geocoding", "geocoding"));
            checks.push(skipped_provider_check("provider.routing_v2", "routing-v2"));
        }
    }
}

fn provider_check(
    id: &'static str,
    provider: &'static str,
    result: Result<(), String>,
) -> DoctorCheck {
    match result {
        Ok(()) => DoctorCheck { id, status: CheckStatus::Ok, message: format!("The Digitransit {provider} probe succeeded."), fix_suggestion: None, details: json!({"requests_sent": 1}) },
        Err(message) => DoctorCheck { id, status: CheckStatus::Fail, message, fix_suggestion: Some("Check the credential, network, and provider status, then rerun reitti doctor --online.".to_owned()), details: json!({"requests_sent": 1}) },
    }
}

fn skipped_provider_check(id: &'static str, provider: &'static str) -> DoctorCheck {
    DoctorCheck { id, status: CheckStatus::Fail, message: format!("The Digitransit {provider} probe was skipped because configuration or the credential is invalid."), fix_suggestion: Some("Fix config.values and credential.subscription_key first.".to_owned()), details: json!({"requests_sent": 0}) }
}

#[cfg(test)]
mod online_tests {
    use super::*;
    use crate::{
        client::{HttpFuture, HttpRequest, HttpResponse},
        config::{ConfigPath, PathSource, Secret, ValueSource},
    };
    use chrono::{DateTime, Utc};
    use reitti_core::{Clock, FixedClock};
    use std::{
        collections::{BTreeMap, VecDeque},
        sync::Mutex,
    };

    #[derive(Debug)]
    struct Mock {
        requests: Mutex<Vec<HttpRequest>>,
        responses: Mutex<VecDeque<HttpResponse>>,
    }
    impl HttpTransport for Mock {
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
    fn config() -> config::EffectiveConfig {
        config::EffectiveConfig {
            path: ConfigPath {
                path: "/tmp/reitti-test-config".into(),
                exists: false,
                source: PathSource::Default,
            },
            subscription_key: Some((Secret::new("secret-canary".into()), ValueSource::Env)),
            language: ("en".into(), ValueSource::Default),
            timezone: ("Europe/Helsinki".into(), ValueSource::Default),
            routing_url: (config::DEFAULT_ROUTING_URL.into(), ValueSource::Default),
            geocoding_url: (config::DEFAULT_GEOCODING_URL.into(), ValueSource::Default),
            connect_timeout: ("1s".into(), ValueSource::Default),
            request_timeout: ("2s".into(), ValueSource::Default),
            private_markers: (vec![], ValueSource::Default),
        }
    }
    fn clock() -> impl Clock {
        FixedClock::new(
            DateTime::parse_from_rfc3339("2026-09-08T08:13:39Z")
                .unwrap()
                .with_timezone(&Utc),
        )
    }

    #[test]
    fn online_doctor_runs_both_probes_even_when_the_first_fails() {
        let mock = Mock {
            requests: Mutex::new(vec![]),
            responses: Mutex::new(
                vec![
                    response(503, json!({})),
                    response(200, json!({"data":{"feeds":[]}})),
                ]
                .into(),
            ),
        };
        let mut checks = vec![];
        append_online_checks(&mut checks, &Ok(config()), &clock(), &mock);
        assert_eq!(mock.requests.lock().unwrap().len(), 2);
        assert!(matches!(checks[0].status, CheckStatus::Fail));
        assert!(matches!(checks[1].status, CheckStatus::Ok));
        assert_eq!(checks[0].details["requests_sent"], 1);
    }

    #[test]
    fn invalid_config_skips_both_online_requests() {
        let mock = Mock {
            requests: Mutex::new(vec![]),
            responses: Mutex::new(VecDeque::new()),
        };
        let mut checks = vec![];
        append_online_checks(
            &mut checks,
            &Err(AppError::caller("invalid_config", "invalid")),
            &clock(),
            &mock,
        );
        assert!(mock.requests.lock().unwrap().is_empty());
        assert_eq!(checks.len(), 2);
        assert!(checks
            .iter()
            .all(|check| check.details["requests_sent"] == 0));
    }
}
