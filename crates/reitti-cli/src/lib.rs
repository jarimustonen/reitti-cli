pub mod build_provenance;
pub mod client;
pub mod command;
pub mod config;
pub mod error;
pub mod output;
pub mod support;

use std::{
    collections::BTreeSet,
    ffi::OsString,
    io::{self, BufReader, Write},
    path::Path,
};

use chrono::{DateTime, Utc};
use clap::{CommandFactory, Parser};
use reitti_core::{
    Clock, FixedClock, LocationRef, RequestIdGenerator, RouteId, StopId, SystemClock,
};
use serde_json::{json, Value};

use command::{
    AlertCommand, Cli, Command, ConfigCommand, DepartureCommand, JourneyCommand, LocationCommand,
    SchemaCommand, SkillCommand, StopCommand,
};
use config::GlobalOverrides;
use error::{AppError, ErrorDocument};
use output::{CommandOutput, Warning};

#[derive(Debug, Default)]
pub struct UlidRequestIds;
impl RequestIdGenerator for UlidRequestIds {
    fn next(&self) -> String {
        format!("req_{}", ulid::Ulid::new())
    }
}

pub fn run_from_env() -> u8 {
    let args: Vec<OsString> = std::env::args_os().collect();
    let stdin = io::stdin();
    let mut input = BufReader::new(stdin.lock());
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    run(args, &mut input, &mut stdout, &mut stderr)
}

pub fn run(
    args: Vec<OsString>,
    stdin: &mut dyn io::BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    run_with(args, stdin, stdout, stderr, &SystemClock, &UlidRequestIds)
}

pub fn run_with(
    args: Vec<OsString>,
    stdin: &mut dyn io::BufRead,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    system_clock: &dyn Clock,
    request_ids: &dyn RequestIdGenerator,
) -> u8 {
    let args = normalize_version_alias(args);
    let json_requested = semantic_flag(&args, "--json");

    if semantic_flag(&args, "--help") {
        return match execute_help(&args, json_requested) {
            Ok((output, path)) => match prepare_output(path.as_deref()) {
                Ok(prepared) => finish(Ok(output), json_requested, prepared, stdout, stderr),
                Err(error) => emit_error(&error, json_requested, stderr),
            },
            Err(error) => emit_error(&error, json_requested, stderr),
        };
    }

    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                if stdout
                    .write_all(error.to_string().as_bytes())
                    .and_then(|_| stdout.flush())
                    .is_ok()
                {
                    return 0;
                }
                return emit_error(
                    &AppError::system("io_error", "Could not write help output."),
                    json_requested,
                    stderr,
                );
            }
            let app_error = AppError::caller("usage_error", one_line(&error.to_string()));
            return emit_error(&app_error, json_requested, stderr);
        }
    };

    let prepared = match prepare_output(cli.output.as_deref()) {
        Ok(prepared) => prepared,
        Err(error) => return emit_error(&error, cli.json, stderr),
    };
    let json = cli.json;
    let clock_override = match clock_for(cli.frozen_time.as_deref()) {
        Ok(clock) => clock,
        Err(error) => return emit_error(&error, json, stderr),
    };
    let clock: &dyn Clock = clock_override.as_deref().unwrap_or(system_clock);
    if cli.verbose {
        let event = json!({"timestamp": clock.now().to_rfc3339(), "level":"INFO", "event":"invocation_started", "component":"cli", "request_id": request_ids.next(), "message":"reitti invocation started"});
        if writeln!(
            stderr,
            "{}",
            serde_json::to_string(&event).unwrap_or_default()
        )
        .is_err()
        {
            return 2;
        }
    }
    let result = execute(cli, stdin, clock, request_ids);
    finish(result, json, prepared, stdout, stderr)
}

fn prepare_output(path: Option<&Path>) -> Result<Option<output::AtomicOutput>, AppError> {
    path.map(output::AtomicOutput::prepare).transpose()
}

fn execute(
    cli: Cli,
    stdin: &mut dyn io::BufRead,
    _clock: &dyn Clock,
    _request_ids: &dyn RequestIdGenerator,
) -> Result<CommandOutput, AppError> {
    let overrides = GlobalOverrides {
        routing_url: cli.routing_url,
        geocoding_url: cli.geocoding_url,
        connect_timeout: cli.connect_timeout,
        request_timeout: cli.request_timeout,
    };
    validate_overrides(&overrides)?;
    match cli.command {
        Command::Version => execute_version(),
        Command::Config { command } => execute_config(command, &overrides, stdin),
        Command::Doctor(args) => execute_doctor(args.online, &overrides),
        Command::Schema { command } => execute_schema(command),
        Command::Skill { command } => execute_skill(command),
        Command::Location { command } => match command {
            LocationCommand::List(args) => {
                command::nonblank(&args.query)
                    .map_err(|message| AppError::caller("invalid_query", message))?;
                let config = require_credential(&overrides)?;
                let _language = config::resolve_language(args.language, &config)?;
                Err(AppError::feature_incomplete("location list"))
            }
        },
        Command::Journey { command } => match command {
            JourneyCommand::List(args) => {
                args.from.parse::<LocationRef>().map_err(|error| {
                    AppError::invalid(
                        "invalid_location_ref",
                        error.to_string(),
                        command::escape_text(&args.from),
                        "query:, place:, stop:, or coord: tagged reference",
                    )
                })?;
                args.to.parse::<LocationRef>().map_err(|error| {
                    AppError::invalid(
                        "invalid_location_ref",
                        error.to_string(),
                        command::escape_text(&args.to),
                        "query:, place:, stop:, or coord: tagged reference",
                    )
                })?;
                validate_datetime(args.depart_at.as_deref().or(args.arrive_by.as_deref()))?;
                reject_duplicates(args.mode.iter().map(|mode| format!("{mode:?}")), "mode")?;
                let config = require_credential(&overrides)?;
                let _language = config::resolve_language(args.language, &config)?;
                Err(AppError::feature_incomplete("journey list"))
            }
        },
        Command::Stop { command } => match command {
            StopCommand::List(args) => {
                if args.query.is_none() == args.near.is_none() {
                    return Err(AppError::caller(
                        "usage_error",
                        "stop list requires exactly one of --query or --near.",
                    ));
                }
                if let Some(near) = &args.near {
                    format!("coord:{near}")
                        .parse::<LocationRef>()
                        .map_err(|error| {
                            AppError::invalid(
                                "invalid_coordinates",
                                error.to_string(),
                                command::escape_text(near),
                                "LAT,LON without whitespace",
                            )
                        })?;
                }
                let config = require_credential(&overrides)?;
                let _language = config::resolve_language(args.language, &config)?;
                Err(AppError::feature_incomplete("stop list"))
            }
        },
        Command::Departure { command } => match command {
            DepartureCommand::List(args) => {
                args.stop.parse::<StopId>().map_err(|error| {
                    AppError::invalid(
                        "invalid_stop_id",
                        error.to_string(),
                        command::escape_text(&args.stop),
                        "raw HSL GTFS ID, for example HSL:1020453",
                    )
                })?;
                validate_datetime(args.at.as_deref())?;
                config::validate_duration(
                    "--window",
                    &args.window,
                    Some((60_000, 24 * 60 * 60_000)),
                )?;
                reject_duplicates(args.mode.iter().map(|mode| format!("{mode:?}")), "mode")?;
                let config = require_credential(&overrides)?;
                let _language = config::resolve_language(args.language, &config)?;
                Err(AppError::feature_incomplete("departure list"))
            }
        },
        Command::Alert { command } => match command {
            AlertCommand::List(args) => {
                for route in &args.route {
                    route.parse::<RouteId>().map_err(|error| {
                        AppError::invalid(
                            "invalid_route_id",
                            error.to_string(),
                            command::escape_text(route),
                            "raw HSL GTFS ID",
                        )
                    })?;
                }
                for stop in &args.stop {
                    stop.parse::<StopId>().map_err(|error| {
                        AppError::invalid(
                            "invalid_stop_id",
                            error.to_string(),
                            command::escape_text(stop),
                            "raw HSL GTFS ID",
                        )
                    })?;
                }
                reject_duplicates(args.route.iter().cloned(), "route")?;
                reject_duplicates(args.stop.iter().cloned(), "stop")?;
                validate_datetime(args.active_at.as_deref())?;
                let config = require_credential(&overrides)?;
                let _language = config::resolve_language(args.language, &config)?;
                Err(AppError::feature_incomplete("alert list"))
            }
        },
    }
}

fn execute_version() -> Result<CommandOutput, AppError> {
    CommandOutput::success(support::version_text(), support::version_data())
}

fn execute_config(
    command: ConfigCommand,
    overrides: &GlobalOverrides,
    stdin: &mut dyn io::BufRead,
) -> Result<CommandOutput, AppError> {
    match command {
        ConfigCommand::Path => {
            let path = config::resolve_path()?;
            let text = format!(
                "{} ({})",
                command::escape_text(&path.path.to_string_lossy()),
                if path.exists { "exists" } else { "not created" }
            );
            CommandOutput::success(text, path)
        }
        ConfigCommand::Show(args) => {
            let effective = config::load(overrides)?;
            let shown = config::display(&effective, args.show_secrets);
            let mut text = String::new();
            for (name, value) in &shown.values {
                text.push_str(&format!(
                    "{name:<20} {:<64} {}\n",
                    command::escape_text(&scalar_text(&value.value)),
                    format!("{:?}", value.source).to_lowercase()
                ));
            }
            let output = CommandOutput::success(text, shown)?;
            if args.show_secrets {
                output.with_warnings(vec![Warning {
                    code: "secrets_exposed".to_owned(),
                    message: "config show --show-secrets exposes sensitive values".to_owned(),
                    details: json!({}),
                }])
            } else {
                Ok(output)
            }
        }
        ConfigCommand::Update(args) => {
            let result = config::update(&args, stdin)?;
            let values: serde_json::Map<String, Value> = result
                .values
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        json!({
                            "value": value,
                            "source": "file",
                            "secret": matches!(name.as_str(), "subscription_key" | "private_markers")
                        }),
                    )
                })
                .collect();
            let data = if result.dry_run {
                json!({"path": result.path, "dry_run": true, "would": result.updated.iter().map(|name| json!({"action":"update", "resource":"config", "input":{"key":name, "value": result.values.get(name)}, "known_effects":{"status":"would_update"}, "unknown_until_apply":[]})).collect::<Vec<_>>(), "unchanged": result.unchanged})
            } else {
                json!({"path": result.path, "updated": result.updated, "unchanged": result.unchanged, "values": values})
            };
            let mut output = CommandOutput::success(
                format!(
                    "Updated {}",
                    command::escape_text(&result.path.to_string_lossy())
                ),
                data,
            )?;
            output.dry_run = result.dry_run;
            output.mutation_applied = !result.dry_run;
            Ok(output)
        }
    }
}

fn execute_doctor(online: bool, overrides: &GlobalOverrides) -> Result<CommandOutput, AppError> {
    let data = support::doctor(online, overrides);
    let text = data
        .checks
        .iter()
        .map(|check| {
            format!(
                "{:<4}  {}  {}",
                format!("{:?}", check.status).to_uppercase(),
                check.id,
                command::escape_text(&check.message)
            )
        })
        .chain(std::iter::once(format!(
            "summary: {} ok, {} warn, {} fail",
            data.summary.ok, data.summary.warn, data.summary.fail
        )))
        .collect::<Vec<_>>()
        .join("\n");
    let exit = if data.summary.fail > 0 { 1 } else { 0 };
    let mut output = CommandOutput::success(text, data)?;
    output.exit = exit;
    Ok(output)
}

fn execute_schema(command: SchemaCommand) -> Result<CommandOutput, AppError> {
    match command {
        SchemaCommand::List => CommandOutput::success(
            support::SCHEMA_NAMES.join("\n"),
            json!({"schemas": support::SCHEMA_NAMES.iter().map(|name| json!({"name":name,"schema_version":1})).collect::<Vec<_>>()}),
        ),
        SchemaCommand::Show { name } => {
            let schema = support::schema(&name)?;
            CommandOutput::success(
                serde_json::to_string_pretty(&schema).unwrap_or_default(),
                json!({"name":name,"dialect":"https://json-schema.org/draft/2020-12/schema","schema":schema}),
            )
        }
    }
}

fn execute_skill(command: SkillCommand) -> Result<CommandOutput, AppError> {
    match command {
        SkillCommand::List => CommandOutput::success(
            "No companion skills are bundled in this foundation build.",
            json!({"skills":[],"supported_agents":["claude","pi","codex"],"install":{"selection_flag":"--agent","default":"all","accepted_values":["claude","pi","codex","all"],"target_flag":"--target","dry_run_flag":"--dry-run","force_flag":"--force","interactive":false,"no_clobber_default":true,"overwrite_requires_force":true,"layouts":[{"agent":"claude","path":".claude/skills/<name>/...","form":"agent-skills-tree"},{"agent":"pi","path":".pi/agent/skills/<name>/...","form":"agent-skills-tree"},{"agent":"codex","path":".codex/skills/<name>/...","form":"agent-skills-tree"}]}}),
        ),
        SkillCommand::Print { name, .. } => Err(AppError::invalid(
            "skill_not_found",
            format!("Skill '{name}' is not bundled in this build."),
            name,
            json!([]),
        )),
        SkillCommand::Install(_) => Err(AppError::caller(
            "no_skills_bundled",
            "No companion skills are bundled in this foundation build; nothing was installed.",
        )),
    }
}

fn validate_overrides(overrides: &GlobalOverrides) -> Result<(), AppError> {
    if let Some(value) = &overrides.routing_url {
        config::validate_url("--routing-url", value)?;
    }
    if let Some(value) = &overrides.geocoding_url {
        config::validate_url("--geocoding-url", value)?;
    }
    if let Some(value) = &overrides.connect_timeout {
        config::validate_duration("--connect-timeout", value, None)?;
    }
    if let Some(value) = &overrides.request_timeout {
        config::validate_duration("--request-timeout", value, None)?;
    }
    Ok(())
}

fn require_credential(overrides: &GlobalOverrides) -> Result<config::EffectiveConfig, AppError> {
    let config = config::load(overrides)?;
    if config.subscription_key.is_none() {
        Err(AppError::caller("credential_missing", "Digitransit subscription key is missing. Set DIGITRANSIT_SUBSCRIPTION_KEY or pipe one line to reitti config update --subscription-key-stdin."))
    } else {
        Ok(config)
    }
}

fn clock_for(value: Option<&str>) -> Result<Option<Box<dyn Clock>>, AppError> {
    match value {
        Some(value) => {
            let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| AppError::invalid("invalid_datetime", format!("Invalid --frozen-time value '{}'; expected RFC 3339 with an explicit offset.", command::escape_text(value)), command::escape_text(value), "RFC3339 with explicit offset"))?;
            Ok(Some(Box::new(FixedClock::new(parsed.with_timezone(&Utc)))))
        }
        None => Ok(None),
    }
}

fn validate_datetime(value: Option<&str>) -> Result<(), AppError> {
    if let Some(value) = value {
        DateTime::parse_from_rfc3339(value).map_err(|_| {
            AppError::invalid(
                "invalid_datetime",
                format!(
                    "Invalid datetime '{}'; expected RFC 3339 with an explicit offset.",
                    command::escape_text(value)
                ),
                command::escape_text(value),
                "RFC3339 with explicit offset",
            )
        })?;
    }
    Ok(())
}

fn reject_duplicates(values: impl Iterator<Item = String>, kind: &str) -> Result<(), AppError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.clone()) {
            return Err(AppError::invalid(
                "duplicate_value",
                format!(
                    "Repeated --{kind} value '{}'.",
                    command::escape_text(&value)
                ),
                value,
                "distinct repeated values",
            ));
        }
    }
    Ok(())
}

fn finish(
    result: Result<CommandOutput, AppError>,
    json: bool,
    prepared_output: Option<output::AtomicOutput>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    match result {
        Ok(output) => {
            let bytes = match output::render(&output, json) {
                Ok(bytes) => bytes,
                Err(error) => return emit_error(&error, json, stderr),
            };
            if let Some(prepared) = prepared_output {
                let path = prepared.destination().to_owned();
                if let Err(error) = prepared.commit(&bytes) {
                    let error = if output.mutation_applied {
                        AppError::system(
                            "output_write_failed",
                            format!("The command mutation was applied, but writing output '{}' failed: {}", command::escape_text(&path.to_string_lossy()), error.message),
                        )
                        .with_detail("mutation_applied", true)
                        .with_detail("output_path", path.to_string_lossy().into_owned())
                    } else {
                        error
                    };
                    return emit_error(&error, json, stderr);
                }
                let metadata = json!({"path":path,"bytes":bytes.len(),"content_type":if json {"application/json"} else {"text/plain"},"schema_version_written":1});
                let file_output = CommandOutput::success(
                    format!(
                        "Wrote {} bytes to {}",
                        bytes.len(),
                        command::escape_text(&path.to_string_lossy())
                    ),
                    metadata,
                )
                .unwrap();
                let envelope = output::render(&file_output, json).unwrap();
                if let Err(error) = stdout.write_all(&envelope).and_then(|_| stdout.flush()) {
                    return emit_error(
                        &AppError::io("Could not write file-result output", &error),
                        json,
                        stderr,
                    );
                }
            } else if let Err(error) = stdout.write_all(&bytes).and_then(|_| stdout.flush()) {
                return emit_error(
                    &AppError::io("Could not write command output", &error),
                    json,
                    stderr,
                );
            }
            if !json {
                for warning in &output.text_warnings {
                    let _ = writeln!(stderr, "warning: {}", one_line(warning));
                }
            }
            output.exit
        }
        Err(error) => emit_error(&error, json, stderr),
    }
}

fn emit_error(error: &AppError, json: bool, stderr: &mut dyn Write) -> u8 {
    if json {
        let document = ErrorDocument::from(error);
        if let Ok(bytes) = serde_json::to_vec(&document) {
            let _ = stderr.write_all(&bytes);
            let _ = stderr.write_all(b"\n");
        }
    } else {
        let _ = writeln!(stderr, "error: {}", one_line(&error.message));
    }
    error.exit
}

fn one_line(value: &str) -> String {
    command::escape_text(value)
}
fn scalar_text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

fn semantic_flag(args: &[OsString], flag: &str) -> bool {
    args.iter()
        .skip(1)
        .take_while(|value| value.to_string_lossy() != "--")
        .any(|value| value == flag)
}

fn normalize_version_alias(mut args: Vec<OsString>) -> Vec<OsString> {
    let value_options = [
        "--output",
        "--routing-url",
        "--geocoding-url",
        "--connect-timeout",
        "--request-timeout",
        "--frozen-time",
    ];
    let mut index = 1;
    while index < args.len() {
        let token = args[index].to_string_lossy();
        if token == "--" {
            break;
        }
        if token == "--version" {
            args[index] = OsString::from("version");
            break;
        }
        if token.starts_with("--") {
            let consumes_next = value_options.contains(&token.as_ref());
            index += if consumes_next && index + 1 < args.len() {
                2
            } else {
                1
            };
            continue;
        }
        break;
    }
    args
}

fn execute_help(
    args: &[OsString],
    json: bool,
) -> Result<(CommandOutput, Option<std::path::PathBuf>), AppError> {
    let parsing_command = relax_for_help(Cli::command());
    let matches = parsing_command
        .try_get_matches_from(args)
        .map_err(|error| AppError::caller("usage_error", one_line(&error.to_string())))?;
    let output_path = matches.get_one::<std::path::PathBuf>("output").cloned();
    let path = match_path(&matches);
    let mut command = Cli::command();
    let global_flags = command
        .get_arguments()
        .filter(|arg| arg.is_global_set())
        .cloned()
        .collect::<Vec<_>>();
    let target = find_command(&mut command, &path)?;
    if json {
        let mut flags = target
            .get_arguments()
            .filter(|arg| arg.get_long().is_some())
            .map(help_flag)
            .collect::<Vec<_>>();
        if !path.is_empty() {
            flags.extend(global_flags.iter().map(help_flag));
        }
        let subcommands = target.get_subcommands().map(|sub| json!({"name":sub.get_name(),"summary":sub.get_about().map(|s|s.to_string()).unwrap_or_default()})).collect::<Vec<_>>();
        CommandOutput::success(
            "",
            json!({"path":path,"summary":target.get_about().map(|s|s.to_string()).unwrap_or_else(||"Agent-first HSL journey planner.".to_owned()),"usage":target.clone().render_usage().to_string(),"args":help_args(target),"flags":flags,"subcommands":subcommands,"exit_codes":[{"code":0,"meaning":"success"},{"code":1,"meaning":"caller/domain-actionable error"},{"code":2,"meaning":"system/provider/internal error"},{"code":130,"meaning":"SIGINT cancellation"},{"code":143,"meaning":"SIGTERM cancellation"}],"examples":examples(&path)}),
        ).map(|output| (output, output_path))
    } else {
        let mut bytes = Vec::new();
        target
            .write_long_help(&mut bytes)
            .map_err(|error| AppError::io("Could not render help", &error))?;
        if let Some(argv) = examples(&path)
            .first()
            .and_then(|example| example["argv"].as_array())
        {
            let command = argv
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            write!(&mut bytes, "\n\nExample:\n  {command}\n")
                .map_err(|error| AppError::io("Could not render help example", &error))?;
        }
        CommandOutput::success(String::from_utf8_lossy(&bytes), json!({}))
            .map(|output| (output, output_path))
    }
}

fn relax_for_help(mut command: clap::Command) -> clap::Command {
    command = command
        .subcommand_required(false)
        .arg_required_else_help(false);
    let args = command
        .get_arguments()
        .map(|arg| arg.get_id().clone())
        .collect::<Vec<_>>();
    for id in args {
        command = command.mut_arg(id, |arg| arg.required(false));
    }
    let children = command
        .get_subcommands()
        .map(|sub| sub.get_name().to_owned())
        .collect::<Vec<_>>();
    for child in children {
        command = command.mut_subcommand(child, relax_for_help);
    }
    command
}
fn match_path(matches: &clap::ArgMatches) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = matches;
    while let Some((name, child)) = current.subcommand() {
        path.push(name.to_owned());
        current = child;
    }
    path
}
fn find_command<'a>(
    command: &'a mut clap::Command,
    path: &[String],
) -> Result<&'a mut clap::Command, AppError> {
    let mut current = command;
    for part in path {
        current = current.find_subcommand_mut(part).ok_or_else(|| {
            AppError::caller(
                "usage_error",
                format!("Unknown command path '{}'.", path.join(" ")),
            )
        })?;
    }
    Ok(current)
}
fn help_args(command: &clap::Command) -> Vec<Value> {
    command
        .get_positionals()
        .map(|arg| {
            json!({
                "name": arg.get_id().as_str(),
                "required": arg.is_required_set(),
                "value_name": arg.get_value_names().and_then(|names| names.first()).map(|name| name.to_string()),
                "possible_values": arg.get_possible_values().iter().map(|value| value.get_name()).collect::<Vec<_>>()
            })
        })
        .collect()
}

fn help_flag(arg: &clap::Arg) -> Value {
    json!({
        "name": format!("--{}", arg.get_long().unwrap_or(arg.get_id().as_str())),
        "required": arg.is_required_set(),
        "repeatable": matches!(arg.get_action(), clap::ArgAction::Append | clap::ArgAction::Count),
        "value_name": arg.get_value_names().and_then(|names| names.first()).map(|name| name.to_string()),
        "possible_values": arg.get_possible_values().iter().map(|value| value.get_name()).collect::<Vec<_>>(),
        "default": arg.get_default_values().first().map(|value| value.to_string_lossy()),
        "env": env_for(arg.get_long()),
        "global": arg.is_global_set(),
        "hidden": arg.is_hide_set(),
        "deprecated": Value::Null
    })
}

fn env_for(long: Option<&str>) -> Option<String> {
    match long {
        Some("language") => Some("REITTI_LANGUAGE".to_owned()),
        Some("routing-url") => Some("REITTI_ROUTING_URL".to_owned()),
        Some("geocoding-url") => Some("REITTI_GEOCODING_URL".to_owned()),
        Some("connect-timeout") => Some("REITTI_CONNECT_TIMEOUT".to_owned()),
        Some("request-timeout") => Some("REITTI_REQUEST_TIMEOUT".to_owned()),
        _ => None,
    }
}
fn examples(path: &[String]) -> Vec<Value> {
    let argv = match path
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["location", "list"] => vec![
            "reitti", "--json", "location", "list", "--query", "Kamppi", "--kind", "stop",
            "--limit", "5",
        ],
        ["journey", "list"] => vec![
            "reitti",
            "--json",
            "journey",
            "list",
            "--from",
            "place:example",
            "--to",
            "stop:HSL:1020453",
            "--arrive-by",
            "2026-09-08T10:00:00+03:00",
        ],
        ["stop", "list"] => vec![
            "reitti",
            "--json",
            "stop",
            "list",
            "--near",
            "60.1699,24.9384",
            "--radius-m",
            "500",
            "--limit",
            "5",
        ],
        ["departure", "list"] => vec![
            "reitti",
            "--json",
            "departure",
            "list",
            "--stop",
            "HSL:1020453",
            "--at",
            "2026-09-08T09:55:00+03:00",
            "--limit",
            "10",
        ],
        ["alert", "list"] => vec![
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
        ],
        ["config", "path"] => vec!["reitti", "--json", "config", "path"],
        ["config", "show"] => vec!["reitti", "--json", "config", "show"],
        ["config", "update"] => vec![
            "reitti",
            "--json",
            "config",
            "update",
            "--language",
            "fi",
            "--dry-run",
        ],
        ["schema", "list"] => vec!["reitti", "--json", "schema", "list"],
        ["schema", "show"] => vec!["reitti", "--json", "schema", "show", "journey-list"],
        ["version"] => vec!["reitti", "--json", "version"],
        ["doctor"] => vec!["reitti", "--json", "doctor", "--online"],
        ["skill", "list"] => vec!["reitti", "--json", "skill", "list"],
        ["skill", "print"] => vec!["reitti", "skill", "print", "reitti"],
        ["skill", "install"] => vec![
            "reitti",
            "--json",
            "skill",
            "install",
            "reitti",
            "--agent",
            "all",
            "--dry-run",
        ],
        _ => vec!["reitti", "--help"],
    };
    vec![json!({"description":"Example invocation","argv":argv})]
}

#[cfg(test)]
mod tests {
    use super::*;
    use reitti_core::{FixedClock, FixedRequestIds};
    use std::io::{Cursor, Error, ErrorKind};
    use tempfile::tempdir;

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(Error::new(ErrorKind::BrokenPipe, "synthetic broken pipe"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FailingFlushWriter;

    impl Write for FailingFlushWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(Error::other("synthetic flush failure"))
        }
    }

    fn fixed_clock() -> FixedClock {
        FixedClock::new(
            DateTime::parse_from_rfc3339("2026-09-08T06:56:40Z")
                .unwrap()
                .with_timezone(&Utc),
        )
    }

    #[test]
    fn stdout_failure_is_a_system_error() {
        let mut stdin = Cursor::new(Vec::<u8>::new());
        let mut stdout = FailingWriter;
        let mut stderr = Vec::new();
        let exit = run_with(
            ["reitti", "--json", "version"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            &mut stdin,
            &mut stdout,
            &mut stderr,
            &fixed_clock(),
            &FixedRequestIds::new("req_test"),
        );
        assert_eq!(exit, 2);
        let error: Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(error["error"]["code"], "io_error");
    }

    #[test]
    fn stdout_flush_failure_is_a_system_error() {
        let mut stdin = Cursor::new(Vec::<u8>::new());
        let mut stdout = FailingFlushWriter;
        let mut stderr = Vec::new();
        let exit = run_with(
            ["reitti", "--json", "version"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            &mut stdin,
            &mut stdout,
            &mut stderr,
            &fixed_clock(),
            &FixedRequestIds::new("req_test"),
        );
        assert_eq!(exit, 2);
        let error: Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(error["error"]["code"], "io_error");
    }

    #[test]
    fn post_mutation_output_failure_reports_applied_state() {
        let directory = tempdir().unwrap();
        let destination = directory.path().join("result.json");
        let prepared = output::AtomicOutput::prepare(&destination).unwrap();
        std::fs::create_dir(&destination).unwrap();
        let mut result = CommandOutput::success("done", json!({"done": true})).unwrap();
        result.mutation_applied = true;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = finish(Ok(result), true, Some(prepared), &mut stdout, &mut stderr);
        assert_eq!(exit, 2);
        assert!(stdout.is_empty());
        let error: Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(error["error"]["code"], "output_write_failed");
        assert_eq!(error["error"]["details"]["mutation_applied"], true);
    }

    #[test]
    fn version_alias_honors_verbose_with_injected_determinism() {
        fn invoke(args: &[&str]) -> (u8, Vec<u8>, Vec<u8>) {
            let mut stdin = Cursor::new(Vec::<u8>::new());
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let exit = run_with(
                args.iter().map(|arg| OsString::from(*arg)).collect(),
                &mut stdin,
                &mut stdout,
                &mut stderr,
                &fixed_clock(),
                &FixedRequestIds::new("req_fixed"),
            );
            (exit, stdout, stderr)
        }

        let alias = invoke(&["reitti", "--version", "--json", "--verbose"]);
        let command = invoke(&["reitti", "version", "--json", "--verbose"]);
        assert_eq!(alias, command);
        assert_eq!(alias.0, 0);
        assert!(String::from_utf8(alias.2)
            .unwrap()
            .contains("\"request_id\":\"req_fixed\""));
    }
}
