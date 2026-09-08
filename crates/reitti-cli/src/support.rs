use serde::Serialize;
use serde_json::{json, Value};

use crate::{config, error::AppError};

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
        // The skill implementation is owned by @ship-agent-skill. An empty
        // catalogue is truthful until a resource is compiled into the binary.
        skills: Vec::new(),
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
                "skills":{"type":"array","items":{"type":"object"}}
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
        "skill-list" => object_schema(
            &["skills", "supported_agents", "install"],
            json!({"skills":{"type":"array"},"supported_agents":{"type":"array","items":{"enum":["claude","pi","codex"]}},"install":{"type":"object","required":["selection_flag","default","accepted_values","target_flag","dry_run_flag","force_flag","interactive","no_clobber_default","overwrite_requires_force","layouts"]}}),
        ),
        "skill-print" => object_schema(
            &[
                "name",
                "cli_version",
                "schema_version_skill",
                "content",
                "path_in_repo",
                "resources",
            ],
            json!({"name":{"type":"string"},"cli_version":{"type":"string"},"schema_version_skill":{"type":"integer"},"content":{"type":"string"},"path_in_repo":{"type":"string"},"resources":string_array_schema()}),
        ),
        "skill-install" => object_schema(
            &["name", "agent", "installed", "existed", "skipped"],
            json!({"name":{"type":"string"},"agent":{"enum":["claude","pi","codex","all"]},"installed":string_array_schema(),"existed":string_array_schema(),"skipped":string_array_schema()}),
        ),
        "error" => object_schema(
            &["schema_version", "error"],
            json!({"schema_version":{"const":1},"error":{"type":"object","required":["code","message","retryable","details"],"properties":{"code":{"type":"string"},"message":{"type":"string"},"invalid_value":{},"expected":{},"retryable":{"type":"boolean"},"details":{"type":"object"}},"additionalProperties":true}}),
        ),
        "help" => object_schema(
            &["schema_version", "data", "warnings"],
            json!({"schema_version":{"const":1},"data":object_schema(&["path","summary","usage","args","flags","subcommands","exit_codes","examples"],json!({"path":string_array_schema(),"summary":{"type":"string"},"usage":{"type":"string"},"args":{"type":"array"},"flags":{"type":"array"},"subcommands":{"type":"array"},"exit_codes":{"type":"array"},"examples":{"type":"array"}})),"warnings":{"type":"array"}}),
        ),
        // Domain owners replace these explicitly identified scaffolds before
        // claiming their provider-backed output contracts complete.
        _ => {
            let mut placeholder = object_schema(&[], json!({}));
            placeholder["description"] = json!("Temporary domain schema placeholder; provider/domain implementation issue owns the exact body.");
            placeholder
        }
    };
    let wrapped = matches!(name, "error" | "help");
    let mut document = body;
    document["$schema"] = json!("https://json-schema.org/draft/2020-12/schema");
    document["$id"] = json!(format!("urn:reitti:schema:v1:{name}"));
    document["title"] = json!(format!(
        "reitti {name} {}",
        if wrapped { "document" } else { "data" }
    ));
    document["$defs"] = json!({"file_output": file_output_schema()});
    Ok(document)
}

fn object_schema(required: &[&str], properties: Value) -> Value {
    json!({"type":"object","required":required,"properties":properties,"additionalProperties":true})
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

pub fn doctor(online: bool, overrides: &config::GlobalOverrides) -> DoctorData {
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
    checks.push(DoctorCheck {
        id: "skill.sync",
        status: CheckStatus::Ok,
        message: "No companion skill is bundled in this foundation build; no sync claim is made."
            .to_owned(),
        fix_suggestion: None,
        details: json!({"bundled": []}),
    });
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
                "{}\n{}\n{}\n{}\n{}\n{}\n{}",
                include_str!("../../../README.md"),
                include_str!("../../../CONTRIBUTING.md"),
                include_str!("../../../SECURITY.md"),
                include_str!("../../../LICENSE"),
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
        for (id, provider) in [
            ("provider.geocoding", "geocoding"),
            ("provider.routing_v2", "routing-v2"),
        ] {
            checks.push(DoctorCheck { id, status: CheckStatus::Fail, message: format!("The {provider} online probe is not implemented in this foundation build; no request was sent."), fix_suggestion: Some("Use a build containing the Digitransit client slice.".to_owned()), details: json!({"requests_sent": 0}) });
        }
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
