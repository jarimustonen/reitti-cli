use std::{
    collections::BTreeMap,
    env,
    fs::{self, File, OpenOptions},
    io::{BufRead, Read, Write},
    path::{Path, PathBuf},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;

use crate::{command::ConfigUpdateArgs, error::AppError};

pub const DEFAULT_ROUTING_URL: &str = "https://api.digitransit.fi/routing/v2/hsl/gtfs/v1";
pub const DEFAULT_GEOCODING_URL: &str = "https://api.digitransit.fi/geocoding/v1";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PathSource {
    Env,
    Xdg,
    Default,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigPath {
    pub path: PathBuf,
    pub exists: bool,
    pub source: PathSource,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueSource {
    Flag,
    Env,
    File,
    Default,
}

#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub subscription_key: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub routing_url: Option<String>,
    pub geocoding_url: Option<String>,
    pub connect_timeout: Option<String>,
    pub request_timeout: Option<String>,
    pub private_markers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalOverrides {
    pub routing_url: Option<String>,
    pub geocoding_url: Option<String>,
    pub connect_timeout: Option<String>,
    pub request_timeout: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    pub path: ConfigPath,
    pub subscription_key: Option<(Secret, ValueSource)>,
    pub language: (String, ValueSource),
    pub timezone: (String, ValueSource),
    pub routing_url: (String, ValueSource),
    pub geocoding_url: (String, ValueSource),
    pub connect_timeout: (String, ValueSource),
    pub request_timeout: (String, ValueSource),
    pub private_markers: (Vec<String>, ValueSource),
}

#[derive(Debug, Serialize)]
pub struct DisplayValue {
    pub value: Value,
    pub source: ValueSource,
    pub secret: bool,
}

#[derive(Debug, Serialize)]
pub struct ConfigShowData {
    pub path: PathBuf,
    pub values: BTreeMap<String, DisplayValue>,
}

#[derive(Debug)]
pub struct UpdateResult {
    pub path: PathBuf,
    pub updated: Vec<String>,
    pub unchanged: Vec<String>,
    pub values: BTreeMap<String, Value>,
    pub dry_run: bool,
}

pub fn resolve_path() -> Result<ConfigPath, AppError> {
    if let Some(raw) = env::var_os("REITTI_CONFIG_FILE") {
        let path = PathBuf::from(raw);
        if !path.is_absolute() {
            return Err(AppError::invalid(
                "invalid_config_path",
                format!(
                    "REITTI_CONFIG_FILE must be absolute, got '{}'.",
                    path.display()
                ),
                path.display().to_string(),
                "absolute path",
            ));
        }
        return Ok(ConfigPath {
            exists: path.exists(),
            path,
            source: PathSource::Env,
        });
    }

    let (base, source) = match env::var_os("XDG_CONFIG_HOME") {
        Some(raw) => {
            let path = PathBuf::from(raw);
            if path.as_os_str().is_empty() || !path.is_absolute() {
                return Err(AppError::invalid(
                    "invalid_config_path",
                    "XDG_CONFIG_HOME must be a non-empty absolute path.",
                    path.display().to_string(),
                    "non-empty absolute path",
                ));
            }
            (path, PathSource::Xdg)
        }
        None => {
            let home = env::var_os("HOME").ok_or_else(|| {
                AppError::caller(
                    "config_path_unresolved",
                    "Neither REITTI_CONFIG_FILE, XDG_CONFIG_HOME, nor HOME resolves a config path.",
                )
            })?;
            let home = PathBuf::from(home);
            if !home.is_absolute() {
                return Err(AppError::invalid(
                    "invalid_config_path",
                    "HOME must be absolute to resolve the config path.",
                    home.display().to_string(),
                    "absolute path",
                ));
            }
            (home.join(".config"), PathSource::Default)
        }
    };

    let path = base.join("reitti/config.toml");
    Ok(ConfigPath {
        exists: path.exists(),
        path,
        source,
    })
}

pub fn inspect_path(path: &Path) -> Result<(), AppError> {
    reject_insecure_file(path)?;
    if path.exists() {
        let file = open_existing_nofollow(path)?;
        validate_opened_file(path, &file)?;
    }
    Ok(())
}

pub fn read_file(path: &Path) -> Result<FileConfig, AppError> {
    reject_insecure_file(path)?;
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let file = open_existing_nofollow(path)?;
    validate_opened_file(path, &file)?;
    const MAX_CONFIG_BYTES: u64 = 1_048_576;
    let mut contents = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|error| AppError::io("Could not read config file", &error))?;
    if contents.len() as u64 > MAX_CONFIG_BYTES {
        return Err(AppError::caller(
            "config_too_large",
            format!("Config file '{}' exceeds the 1 MiB limit.", path.display()),
        ));
    }
    toml::from_str(&contents).map_err(|_| {
        AppError::caller(
            "invalid_config",
            format!(
                "Config file '{}' could not be parsed; secret values are omitted from this error.",
                path.display()
            ),
        )
    })
}

pub fn load(overrides: &GlobalOverrides) -> Result<EffectiveConfig, AppError> {
    let path = resolve_path()?;
    let file = read_file(&path.path)?;

    let subscription_key = choose_secret(file.subscription_key)?;
    let language = choose("REITTI_LANGUAGE", None, file.language, "en")?;
    let timezone = choose("REITTI_TIMEZONE", None, file.timezone, "Europe/Helsinki")?;
    let routing_url = choose(
        "REITTI_ROUTING_URL",
        overrides.routing_url.clone(),
        file.routing_url,
        DEFAULT_ROUTING_URL,
    )?;
    let geocoding_url = choose(
        "REITTI_GEOCODING_URL",
        overrides.geocoding_url.clone(),
        file.geocoding_url,
        DEFAULT_GEOCODING_URL,
    )?;
    let connect_timeout = choose(
        "REITTI_CONNECT_TIMEOUT",
        overrides.connect_timeout.clone(),
        file.connect_timeout,
        "5s",
    )?;
    let request_timeout = choose(
        "REITTI_REQUEST_TIMEOUT",
        overrides.request_timeout.clone(),
        file.request_timeout,
        "20s",
    )?;
    let private_markers = match env::var("REITTI_PRIVATE_MARKERS") {
        Ok(raw) => {
            let markers: Vec<String> = serde_json::from_str(&raw).map_err(|_| {
                AppError::caller(
                    "invalid_private_markers",
                    "REITTI_PRIVATE_MARKERS must be a JSON array of non-empty strings; values are redacted.",
                )
            })?;
            validate_markers(&markers)?;
            (markers, ValueSource::Env)
        }
        Err(env::VarError::NotPresent) => match file.private_markers {
            Some(markers) => (markers, ValueSource::File),
            None => (Vec::new(), ValueSource::Default),
        },
        Err(env::VarError::NotUnicode(_)) => {
            return Err(AppError::caller(
                "invalid_private_markers",
                "REITTI_PRIVATE_MARKERS must be UTF-8 JSON; values are redacted.",
            ));
        }
    };

    validate_language(&language.0)?;
    validate_timezone(&timezone.0)?;
    validate_url("routing_url", &routing_url.0)?;
    validate_url("geocoding_url", &geocoding_url.0)?;
    validate_duration("connect_timeout", &connect_timeout.0, None)?;
    validate_duration("request_timeout", &request_timeout.0, None)?;
    validate_markers(&private_markers.0)?;

    Ok(EffectiveConfig {
        path,
        subscription_key,
        language,
        timezone,
        routing_url,
        geocoding_url,
        connect_timeout,
        request_timeout,
        private_markers,
    })
}

fn choose(
    env_name: &str,
    flag: Option<String>,
    file: Option<String>,
    default: &str,
) -> Result<(String, ValueSource), AppError> {
    if let Some(value) = flag {
        return Ok((value, ValueSource::Flag));
    }
    match env::var(env_name) {
        Ok(value) => Ok((value, ValueSource::Env)),
        Err(env::VarError::NotPresent) => Ok(file
            .map(|value| (value, ValueSource::File))
            .unwrap_or_else(|| (default.to_owned(), ValueSource::Default))),
        Err(env::VarError::NotUnicode(_)) => Err(AppError::caller(
            "invalid_env",
            format!("{env_name} must be UTF-8; value is redacted."),
        )),
    }
}

fn choose_secret(file: Option<String>) -> Result<Option<(Secret, ValueSource)>, AppError> {
    let selected = match env::var("DIGITRANSIT_SUBSCRIPTION_KEY") {
        Ok(value) => Some((value, ValueSource::Env)),
        Err(env::VarError::NotPresent) => file.map(|value| (value, ValueSource::File)),
        Err(env::VarError::NotUnicode(_)) => {
            return Err(AppError::caller(
                "invalid_credential",
                "Subscription key must be UTF-8; value is redacted.",
            ));
        }
    };
    selected
        .map(|(value, source)| {
            validate_secret(&value)?;
            Ok((Secret(value), source))
        })
        .transpose()
}

pub fn resolve_language(
    flag: Option<crate::command::LanguageArg>,
    config: &EffectiveConfig,
) -> Result<reitti_core::Language, AppError> {
    if let Some(language) = flag {
        return Ok(language.into());
    }
    match config.language.0.as_str() {
        "en" => Ok(reitti_core::Language::En),
        "fi" => Ok(reitti_core::Language::Fi),
        "sv" => Ok(reitti_core::Language::Sv),
        value => Err(AppError::invalid(
            "invalid_language",
            format!(
                "Invalid resolved language '{}'.",
                crate::command::escape_text(value)
            ),
            crate::command::escape_text(value),
            json!(["en", "fi", "sv"]),
        )),
    }
}

pub fn display(config: &EffectiveConfig, show_secrets: bool) -> ConfigShowData {
    let mut values = BTreeMap::new();
    let secret_value = config
        .subscription_key
        .as_ref()
        .map(|(secret, _)| secret.expose())
        .unwrap_or("");
    let secret_source = config
        .subscription_key
        .as_ref()
        .map(|(_, source)| *source)
        .unwrap_or(ValueSource::Default);
    values.insert(
        "subscription_key".to_owned(),
        display_value(
            if show_secrets {
                json!(secret_value)
            } else {
                json!("<redacted>")
            },
            secret_source,
            true,
        ),
    );
    for (name, (value, source)) in [
        ("language", &config.language),
        ("timezone", &config.timezone),
        ("routing_url", &config.routing_url),
        ("geocoding_url", &config.geocoding_url),
        ("connect_timeout", &config.connect_timeout),
        ("request_timeout", &config.request_timeout),
    ] {
        values.insert(name.to_owned(), display_value(json!(value), *source, false));
    }
    values.insert(
        "private_markers".to_owned(),
        display_value(
            if show_secrets {
                json!(config.private_markers.0)
            } else {
                json!("<redacted>")
            },
            config.private_markers.1,
            true,
        ),
    );
    ConfigShowData {
        path: config.path.path.clone(),
        values,
    }
}

fn display_value(value: Value, source: ValueSource, secret: bool) -> DisplayValue {
    DisplayValue {
        value,
        source,
        secret,
    }
}

pub fn update(args: &ConfigUpdateArgs, stdin: &mut dyn BufRead) -> Result<UpdateResult, AppError> {
    let path = resolve_path()?;
    let secret = if args.subscription_key_stdin {
        Some(read_secret_line(stdin)?)
    } else {
        None
    };
    let update_count = usize::from(args.language.is_some())
        + usize::from(args.timezone.is_some())
        + usize::from(args.routing_url.is_some())
        + usize::from(args.geocoding_url.is_some())
        + usize::from(args.connect_timeout.is_some())
        + usize::from(args.request_timeout.is_some())
        + usize::from(!args.private_marker.is_empty())
        + usize::from(secret.is_some());
    if update_count == 0 {
        return Err(AppError::caller(
            "usage_error",
            "config update requires at least one update option.",
        ));
    }
    validate_update(args)?;

    if args.dry_run {
        let existing = read_file(&path.path)?;
        let (updated, unchanged, next) = apply(existing, args, secret);
        let values = update_values(&updated, &next);
        return Ok(UpdateResult {
            path: path.path,
            updated,
            unchanged,
            values,
            dry_run: true,
        });
    }

    let parent = path
        .path
        .parent()
        .ok_or_else(|| AppError::caller("invalid_config_path", "Config path has no parent."))?;
    ensure_owned_parent(parent)?;
    let lock_path = parent.join("config.lock");
    let lock = open_secure(&lock_path)?;
    lock.try_lock_exclusive().map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            AppError::caller(
                "config_conflict",
                "Another reitti process holds the config lock; retry after it completes.",
            )
        } else {
            AppError::io("Could not lock config", &error)
        }
    })?;
    reject_insecure_file(&path.path)?;
    let existing = read_file(&path.path)?;
    let (updated, unchanged, next) = apply(existing, args, secret);
    let values = update_values(&updated, &next);
    let serialized = toml::to_string_pretty(&next).map_err(|error| {
        AppError::system(
            "internal_error",
            format!("Could not serialize config: {error}"),
        )
    })?;
    atomic_replace(&path.path, serialized.as_bytes())?;
    drop(lock);
    Ok(UpdateResult {
        path: path.path,
        updated,
        unchanged,
        values,
        dry_run: false,
    })
}

fn update_values(updated: &[String], file: &FileConfig) -> BTreeMap<String, Value> {
    updated
        .iter()
        .map(|name| {
            let value = match name.as_str() {
                "subscription_key" | "private_markers" => json!("<redacted>"),
                "language" => json!(file.language),
                "timezone" => json!(file.timezone),
                "routing_url" => json!(file.routing_url),
                "geocoding_url" => json!(file.geocoding_url),
                "connect_timeout" => json!(file.connect_timeout),
                "request_timeout" => json!(file.request_timeout),
                _ => Value::Null,
            };
            (name.clone(), value)
        })
        .collect()
}

fn apply(
    mut file: FileConfig,
    args: &ConfigUpdateArgs,
    secret: Option<String>,
) -> (Vec<String>, Vec<String>, FileConfig) {
    let mut updated = Vec::new();
    let mut unchanged = Vec::new();

    macro_rules! set_value {
        ($field:ident, $name:literal, $value:expr) => {
            if let Some(value) = $value {
                if file.$field.as_ref() == Some(&value) {
                    unchanged.push($name.to_owned());
                } else {
                    file.$field = Some(value);
                    updated.push($name.to_owned());
                }
            }
        };
    }
    set_value!(subscription_key, "subscription_key", secret);
    set_value!(
        language,
        "language",
        args.language.map(|value| value.as_str().to_owned())
    );
    set_value!(timezone, "timezone", args.timezone.clone());
    set_value!(routing_url, "routing_url", args.routing_url.clone());
    set_value!(geocoding_url, "geocoding_url", args.geocoding_url.clone());
    set_value!(
        connect_timeout,
        "connect_timeout",
        args.connect_timeout.clone()
    );
    set_value!(
        request_timeout,
        "request_timeout",
        args.request_timeout.clone()
    );
    if !args.private_marker.is_empty() {
        if file.private_markers.as_ref() == Some(&args.private_marker) {
            unchanged.push("private_markers".to_owned());
        } else {
            file.private_markers = Some(args.private_marker.clone());
            updated.push("private_markers".to_owned());
        }
    }
    (updated, unchanged, file)
}

fn read_secret_line(stdin: &mut dyn BufRead) -> Result<String, AppError> {
    let mut bytes = Vec::new();
    stdin
        .take(4099)
        .read_to_end(&mut bytes)
        .map_err(|error| AppError::io("Could not read subscription key from stdin", &error))?;
    if bytes.len() > 4098 {
        return Err(AppError::caller(
            "invalid_credential",
            "Subscription key exceeds the 4096-byte limit; value is redacted.",
        ));
    }
    let mut value = String::from_utf8(bytes).map_err(|_| {
        AppError::caller(
            "invalid_credential",
            "Subscription key must be UTF-8; value is redacted.",
        )
    })?;
    if value.ends_with("\r\n") {
        value.truncate(value.len() - 2);
    } else if value.ends_with('\n') {
        value.pop();
    }
    if value.contains(['\r', '\n']) {
        return Err(AppError::caller(
            "invalid_credential",
            "Subscription key stdin must contain exactly one line; value is redacted.",
        ));
    }
    validate_secret(&value)?;
    Ok(value)
}

fn validate_secret(value: &str) -> Result<(), AppError> {
    if value.is_empty()
        || value.len() > 4096
        || value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        Err(AppError::caller(
            "invalid_credential",
            "Subscription key must be 1–4096 UTF-8 bytes without whitespace or control characters; value is redacted.",
        ))
    } else {
        Ok(())
    }
}

pub fn validate_url(name: &str, value: &str) -> Result<(), AppError> {
    let expected = "HTTPS URL without userinfo, query, or fragment";
    let url = Url::parse(value).map_err(|_| {
        AppError::invalid(
            "invalid_url",
            format!("Invalid {name} '<redacted>'; expected {expected}."),
            "<redacted>",
            expected,
        )
    })?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        let sensitive = !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some();
        let shown = if sensitive {
            "<redacted>".to_owned()
        } else {
            crate::command::escape_text(value)
        };
        return Err(AppError::invalid(
            "invalid_url",
            format!("Invalid {name} '{shown}'; expected {expected}."),
            shown,
            expected,
        ));
    }
    Ok(())
}

pub fn validate_duration(
    name: &str,
    value: &str,
    bounds_ms: Option<(u64, u64)>,
) -> Result<u64, AppError> {
    let split = value
        .bytes()
        .position(|byte| !byte.is_ascii_digit())
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(split);
    let multiplier = match unit {
        "ms" => Some(1),
        "s" => Some(1_000),
        "m" => Some(60_000),
        "h" => Some(3_600_000),
        _ => None,
    };
    let millis = number
        .parse::<u64>()
        .ok()
        .filter(|number| *number > 0)
        .and_then(|number| multiplier.and_then(|multiplier| number.checked_mul(multiplier)))
        .filter(|millis| {
            bounds_ms
                .map(|(minimum, maximum)| (minimum..=maximum).contains(millis))
                .unwrap_or(true)
        });
    millis.ok_or_else(|| {
        AppError::invalid(
            "invalid_duration",
            format!(
                "Invalid {name} value '{}'; expected a positive integer followed by ms, s, m, or h.",
                crate::command::escape_text(value)
            ),
            crate::command::escape_text(value),
            "positive ASCII integer followed by ms, s, m, or h",
        )
    })
}

fn validate_language(value: &str) -> Result<(), AppError> {
    if matches!(value, "en" | "fi" | "sv") {
        Ok(())
    } else {
        Err(AppError::invalid(
            "invalid_language",
            format!("Invalid language '{value}'; expected en, fi, or sv."),
            value,
            json!(["en", "fi", "sv"]),
        ))
    }
}

fn validate_timezone(value: &str) -> Result<(), AppError> {
    value.parse::<chrono_tz::Tz>().map(|_| ()).map_err(|_| {
        AppError::invalid(
            "invalid_timezone",
            format!(
                "Invalid timezone '{}'; expected an IANA timezone name.",
                crate::command::escape_text(value)
            ),
            crate::command::escape_text(value),
            "IANA timezone, for example Europe/Helsinki",
        )
    })
}

fn validate_markers(values: &[String]) -> Result<(), AppError> {
    if values
        .iter()
        .any(|value| value.trim().is_empty() || value.chars().any(char::is_control))
    {
        Err(AppError::caller(
            "invalid_private_markers",
            "Private markers must be non-empty; values are redacted.",
        ))
    } else {
        Ok(())
    }
}

fn validate_update(args: &ConfigUpdateArgs) -> Result<(), AppError> {
    if let Some(value) = &args.timezone {
        validate_timezone(value)?;
    }
    if let Some(value) = &args.routing_url {
        validate_url("routing_url", value)?;
    }
    if let Some(value) = &args.geocoding_url {
        validate_url("geocoding_url", value)?;
    }
    if let Some(value) = &args.connect_timeout {
        validate_duration("connect_timeout", value, None)?;
    }
    if let Some(value) = &args.request_timeout {
        validate_duration("request_timeout", value, None)?;
    }
    validate_markers(&args.private_marker)
}

fn open_existing_nofollow(path: &Path) -> Result<File, AppError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options
        .open(path)
        .map_err(|error| AppError::io("Could not securely open config file", &error))
}

fn validate_opened_file(path: &Path, file: &File) -> Result<(), AppError> {
    let metadata = file
        .metadata()
        .map_err(|error| AppError::io("Could not inspect opened config file", &error))?;
    if !metadata.is_file() {
        return Err(AppError::caller(
            "insecure_config_file",
            format!("Config path '{}' must be a regular file.", path.display()),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(AppError::caller(
                "insecure_config_permissions",
                format!("Config file '{}' must have mode 0600.", path.display()),
            ));
        }
    }
    Ok(())
}

fn reject_insecure_file(path: &Path) -> Result<(), AppError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(AppError::io("Could not inspect config file", &error)),
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(AppError::caller(
            "insecure_config_file",
            format!(
                "Config path '{}' must be a regular, non-symlink file.",
                path.display()
            ),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(AppError::caller(
                "insecure_config_permissions",
                format!("Config file '{}' must have mode 0600.", path.display()),
            ));
        }
    }
    Ok(())
}

fn ensure_owned_parent(parent: &Path) -> Result<(), AppError> {
    let mut missing = Vec::new();
    let mut cursor = parent;
    loop {
        match fs::symlink_metadata(cursor) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(AppError::caller(
                        "insecure_config_path",
                        format!(
                            "Config ancestor '{}' must be a non-symlink directory.",
                            cursor.display()
                        ),
                    ));
                }
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(cursor.to_owned());
                cursor = cursor.parent().ok_or_else(|| {
                    AppError::caller(
                        "invalid_config_path",
                        "Config path has no existing directory ancestor.",
                    )
                })?;
            }
            Err(error) => return Err(AppError::io("Could not inspect config ancestor", &error)),
        }
    }
    for directory in missing.into_iter().rev() {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory).map_err(|error| {
            AppError::io(
                &format!(
                    "Could not create secure config directory '{}'",
                    directory.display()
                ),
                &error,
            )
        })?;
    }
    Ok(())
}

fn open_secure(path: &Path) -> Result<File, AppError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(AppError::caller(
                "config_conflict",
                format!(
                    "Config lock '{}' must be a regular, non-symlink file.",
                    path.display()
                ),
            ));
        }
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options
        .open(path)
        .map_err(|error| AppError::io("Could not open config lock", &error))
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::caller("invalid_config_path", "Config path has no parent."))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            AppError::caller("invalid_config_path", "Config filename is not valid UTF-8.")
        })?;
    for attempt in 0..100 {
        let temporary = parent.join(format!(".{name}.tmp-{}-{attempt}", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        match options.open(&temporary) {
            Ok(mut file) => {
                let result = (|| {
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    drop(file);
                    fs::rename(&temporary, path)?;
                    #[cfg(unix)]
                    File::open(parent)?.sync_all()?;
                    Ok::<_, std::io::Error>(())
                })();
                if let Err(error) = result {
                    let _ = fs::remove_file(&temporary);
                    return Err(AppError::io("Could not atomically replace config", &error));
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(AppError::io(
                    "Could not create config temporary file",
                    &error,
                ))
            }
        }
    }
    Err(AppError::system(
        "io_error",
        "Could not allocate a collision-free config temporary file.",
    ))
}
