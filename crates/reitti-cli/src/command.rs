use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "reitti",
    disable_help_flag = true,
    disable_help_subcommand = true,
    disable_version_flag = true,
    subcommand_required = true
)]
pub struct Cli {
    /// Emit schema-versioned JSON.
    #[arg(long, global = true)]
    pub json: bool,
    /// Atomically write the complete result to this path.
    #[arg(long, global = true, value_name = "PATH")]
    pub output: Option<PathBuf>,
    /// Emit diagnostic JSONL on stderr.
    #[arg(long, global = true)]
    pub verbose: bool,
    /// Override the routing API URL for this invocation (REITTI_ROUTING_URL).
    #[arg(long, global = true, value_name = "HTTPS_URL")]
    pub routing_url: Option<String>,
    /// Override the geocoding API URL for this invocation (REITTI_GEOCODING_URL).
    #[arg(long, global = true, value_name = "HTTPS_URL")]
    pub geocoding_url: Option<String>,
    /// Override the connection timeout for this invocation; use ms, s, m, or h (default: 5s).
    #[arg(long, global = true, value_name = "DURATION")]
    pub connect_timeout: Option<String>,
    /// Override the whole-request timeout for this invocation; use ms, s, m, or h (default: 20s).
    #[arg(long, global = true, value_name = "DURATION")]
    pub request_timeout: Option<String>,
    /// Freeze the injected clock for tests only.
    #[arg(long, global = true, hide = true, value_name = "RFC3339")]
    pub frozen_time: Option<String>,
    /// Show help for the root or selected command.
    #[arg(long, global = true)]
    pub help: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Resolve place candidates.
    Location {
        #[command(subcommand)]
        command: LocationCommand,
    },
    /// Plan journeys.
    Journey {
        #[command(subcommand)]
        command: JourneyCommand,
    },
    /// Search stops.
    Stop {
        #[command(subcommand)]
        command: StopCommand,
    },
    /// Inspect departures.
    Departure {
        #[command(subcommand)]
        command: DepartureCommand,
    },
    /// Inspect service alerts.
    Alert {
        #[command(subcommand)]
        command: AlertCommand,
    },
    /// Inspect or selectively update configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Discover bundled JSON schemas.
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    /// Run read-only diagnostics (offline unless --online is passed).
    Doctor(DoctorArgs),
    /// Show version and build provenance.
    Version,
    /// Inspect or install bundled Agent Skills.
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum LocationCommand {
    /// List bounded place candidates.
    List(LocationListArgs),
}

#[derive(Debug, Args)]
pub struct LocationListArgs {
    #[arg(long, value_parser = nonblank, value_name = "TEXT")]
    pub query: String,
    #[arg(long, value_enum, default_value = "any")]
    pub kind: LocationKind,
    #[arg(long, value_enum)]
    pub language: Option<LanguageArg>,
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=10))]
    pub limit: u8,
}

#[derive(Debug, Subcommand)]
pub enum JourneyCommand {
    /// List and compare bounded journey alternatives.
    List(JourneyListArgs),
}

#[derive(Debug, Args)]
pub struct JourneyListArgs {
    #[arg(long, value_name = "LOCATION_REF")]
    pub from: String,
    #[arg(long, value_name = "LOCATION_REF")]
    pub to: String,
    #[arg(long, conflicts_with = "arrive_by", value_name = "RFC3339")]
    pub depart_at: Option<String>,
    #[arg(long, conflicts_with = "depart_at", value_name = "RFC3339")]
    pub arrive_by: Option<String>,
    #[arg(long, value_enum, action = clap::ArgAction::Append)]
    pub mode: Vec<ModeArg>,
    #[arg(long, value_parser = clap::value_parser!(u32).range(..=20_000))]
    pub max_walk_m: Option<u32>,
    #[arg(long)]
    pub wheelchair: bool,
    #[arg(long)]
    pub include_geometry: bool,
    #[arg(long, value_enum)]
    pub language: Option<LanguageArg>,
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(1..=6))]
    pub limit: u8,
}

#[derive(Debug, Subcommand)]
pub enum StopCommand {
    /// List named or nearby stops.
    List(StopListArgs),
}

#[derive(Debug, Args)]
pub struct StopListArgs {
    #[arg(long, value_parser = nonblank, value_name = "TEXT")]
    pub query: Option<String>,
    #[arg(long, value_name = "COORDINATES", conflicts_with = "query")]
    pub near: Option<String>,
    #[arg(long, requires = "near", default_value_if("near", clap::builder::ArgPredicate::IsPresent, "1000"), value_parser = clap::value_parser!(u32).range(1..=5000))]
    pub radius_m: Option<u32>,
    #[arg(long, value_enum)]
    pub language: Option<LanguageArg>,
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=10))]
    pub limit: u8,
}

#[derive(Debug, Subcommand)]
pub enum DepartureCommand {
    /// List bounded departures for one raw stop ID.
    List(DepartureListArgs),
}

#[derive(Debug, Args)]
pub struct DepartureListArgs {
    #[arg(long, value_name = "STOP_ID")]
    pub stop: String,
    #[arg(long, value_name = "RFC3339")]
    pub at: Option<String>,
    #[arg(long, default_value = "2h", value_name = "DURATION")]
    pub window: String,
    #[arg(long, value_enum, action = clap::ArgAction::Append)]
    pub mode: Vec<ModeArg>,
    #[arg(long, value_enum)]
    pub language: Option<LanguageArg>,
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u8).range(1..=50))]
    pub limit: u8,
}

#[derive(Debug, Subcommand)]
pub enum AlertCommand {
    /// List bounded relevant alerts.
    List(AlertListArgs),
}

#[derive(Debug, Args)]
pub struct AlertListArgs {
    #[arg(long, action = clap::ArgAction::Append, value_name = "ROUTE_ID")]
    pub route: Vec<String>,
    #[arg(long, action = clap::ArgAction::Append, value_name = "STOP_ID")]
    pub stop: Vec<String>,
    #[arg(long, value_name = "RFC3339")]
    pub active_at: Option<String>,
    #[arg(long, value_enum)]
    pub language: Option<LanguageArg>,
    #[arg(long, default_value_t = 25, value_parser = clap::value_parser!(u8).range(1..=100))]
    pub limit: u8,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show the resolved config path without creating it.
    Path,
    /// Show effective values and per-key provenance.
    Show(ConfigShowArgs),
    /// Selectively and atomically update the config file.
    Update(ConfigUpdateArgs),
}

#[derive(Debug, Args)]
pub struct ConfigShowArgs {
    #[arg(long)]
    pub show_secrets: bool,
}

#[derive(Debug, Args)]
pub struct ConfigUpdateArgs {
    /// Persist the default response language.
    #[arg(long, value_enum)]
    pub language: Option<LanguageArg>,
    /// Persist the IANA timezone used when interpreting provider times.
    #[arg(long, value_name = "IANA_TZ", value_parser = nonblank)]
    pub timezone: Option<String>,
    /// Persist the routing API URL. Unlike global --routing-url, this changes the config file.
    #[arg(
        id = "set_routing_url",
        long = "set-routing-url",
        value_name = "HTTPS_URL"
    )]
    pub routing_url: Option<String>,
    /// Persist the geocoding API URL. Unlike global --geocoding-url, this changes the config file.
    #[arg(
        id = "set_geocoding_url",
        long = "set-geocoding-url",
        value_name = "HTTPS_URL"
    )]
    pub geocoding_url: Option<String>,
    /// Persist the connection timeout; use ms, s, m, or h (default: 5s).
    #[arg(
        id = "set_connect_timeout",
        long = "set-connect-timeout",
        value_name = "DURATION"
    )]
    pub connect_timeout: Option<String>,
    /// Persist the whole-request timeout; use ms, s, m, or h (default: 20s).
    #[arg(
        id = "set_request_timeout",
        long = "set-request-timeout",
        value_name = "DURATION"
    )]
    pub request_timeout: Option<String>,
    /// Persist a private marker checked against bundled public text by doctor; repeatable.
    #[arg(long, action = clap::ArgAction::Append, value_parser = nonblank)]
    pub private_marker: Vec<String>,
    /// Read exactly one Digitransit subscription-key line from stdin and persist it securely.
    #[arg(long)]
    pub subscription_key_stdin: bool,
    /// Validate and print the planned selective update without writing the config file.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Subcommand)]
pub enum SchemaCommand {
    /// List stable schema names.
    List,
    /// Print one bundled schema.
    Show { name: String },
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Add bounded provider probes. Offline is the default.
    #[arg(long)]
    pub online: bool,
}

#[derive(Debug, Subcommand)]
pub enum SkillCommand {
    /// List skills actually bundled in this binary.
    List,
    /// Print a bundled skill resource.
    #[command(visible_alias = "show")]
    Print {
        #[arg(value_parser = nonblank)]
        name: String,
        #[arg(long, value_parser = nonblank, value_name = "PATH")]
        resource: Option<String>,
    },
    /// Install bundled skills without clobbering by default.
    Install(SkillInstallArgs),
}

#[derive(Debug, Args)]
pub struct SkillInstallArgs {
    #[arg(value_parser = nonblank)]
    pub name: Option<String>,
    #[arg(long, value_enum, default_value = "all")]
    pub agent: AgentArg,
    #[arg(long)]
    pub target: Option<PathBuf>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LanguageArg {
    En,
    Fi,
    Sv,
}

impl LanguageArg {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fi => "fi",
            Self::Sv => "sv",
        }
    }
}

impl From<LanguageArg> for reitti_core::Language {
    fn from(value: LanguageArg) -> Self {
        match value {
            LanguageArg::En => Self::En,
            LanguageArg::Fi => Self::Fi,
            LanguageArg::Sv => Self::Sv,
        }
    }
}
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ModeArg {
    Bus,
    Tram,
    Rail,
    Subway,
    Ferry,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LocationKind {
    Any,
    Address,
    Venue,
    Stop,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AgentArg {
    Claude,
    Pi,
    Codex,
    All,
}

pub fn nonblank(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err("value must contain a non-whitespace character".to_owned())
    } else if value.chars().any(char::is_control) {
        Err(format!(
            "value '{}' must not contain control characters",
            escape_text(value)
        ))
    } else {
        Ok(value.to_owned())
    }
}

pub fn escape_text(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                character.escape_default().to_string()
            } else {
                character.to_string()
            }
        })
        .collect()
}
