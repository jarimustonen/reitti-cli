use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use serde::Serialize;
use serde_json::Value;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub code: String,
    pub message: String,
    pub details: Value,
}

#[derive(Debug, Serialize)]
pub struct Envelope<T: Serialize> {
    pub schema_version: u8,
    pub data: T,
    pub warnings: Vec<Warning>,
}

impl<T: Serialize> Envelope<T> {
    pub fn new(data: T) -> Self {
        Self {
            schema_version: 1,
            data,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct CommandOutput {
    pub text: String,
    pub json: Value,
    pub exit: u8,
    pub text_warnings: Vec<String>,
    pub dry_run: bool,
    pub mutation_applied: bool,
}

impl CommandOutput {
    pub fn success<T: Serialize>(text: impl Into<String>, data: T) -> Result<Self, AppError> {
        let json = serde_json::to_value(Envelope::new(data)).map_err(|error| {
            AppError::system(
                "internal_error",
                format!("Could not serialize output: {error}"),
            )
        })?;
        Ok(Self {
            text: text.into(),
            json,
            exit: 0,
            text_warnings: Vec::new(),
            dry_run: false,
            mutation_applied: false,
        })
    }

    pub fn with_warnings(mut self, warnings: Vec<Warning>) -> Result<Self, AppError> {
        if let Some(object) = self.json.as_object_mut() {
            object.insert(
                "warnings".to_owned(),
                serde_json::to_value(&warnings).map_err(|error| {
                    AppError::system(
                        "internal_error",
                        format!("Could not serialize warnings: {error}"),
                    )
                })?,
            );
        }
        self.text_warnings = warnings
            .into_iter()
            .map(|warning| warning.message)
            .collect();
        Ok(self)
    }
}

pub fn render(output: &CommandOutput, json: bool) -> Result<Vec<u8>, AppError> {
    let mut bytes = if json {
        serde_json::to_vec_pretty(&output.json).map_err(|error| {
            AppError::system(
                "internal_error",
                format!("Could not serialize output: {error}"),
            )
        })?
    } else {
        output.text.as_bytes().to_vec()
    };
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    Ok(bytes)
}

pub struct AtomicOutput {
    destination: std::path::PathBuf,
    temporary: std::path::PathBuf,
    file: Option<std::fs::File>,
}

impl AtomicOutput {
    pub fn prepare(path: &Path) -> Result<Self, AppError> {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if !parent.is_dir() {
            return Err(AppError::caller(
                "output_parent_not_found",
                format!("Output parent '{}' does not exist.", parent.display()),
            ));
        }
        if fs::symlink_metadata(path)
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
        {
            return Err(AppError::caller(
                "invalid_output_path",
                format!("Output path '{}' is a a directory.", path.display()),
            ));
        }
        let file_name = path.file_name().ok_or_else(|| {
            AppError::caller(
                "invalid_output_path",
                format!("Invalid output path '{}'.", path.display()),
            )
        })?;
        for attempt in 0..100 {
            let temporary = parent.join(format!(
                ".{}.reitti-tmp-{}-{attempt}",
                file_name.to_string_lossy(),
                std::process::id()
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
            }
            match options.open(&temporary) {
                Ok(file) => {
                    return Ok(Self {
                        destination: path.to_owned(),
                        temporary,
                        file: Some(file),
                    })
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(AppError::io(
                        "Could not reserve output temporary file",
                        &error,
                    ))
                }
            }
        }
        Err(AppError::system(
            "io_error",
            "Could not allocate a collision-free output temporary file.",
        ))
    }

    pub fn destination(&self) -> &Path {
        &self.destination
    }

    pub fn commit(mut self, bytes: &[u8]) -> Result<(), AppError> {
        let mut file = self.file.take().expect("prepared output owns a file");
        file.write_all(bytes)
            .map_err(|error| AppError::io("Could not write output", &error))?;
        file.sync_all()
            .map_err(|error| AppError::io("Could not sync output", &error))?;
        drop(file);
        fs::rename(&self.temporary, &self.destination)
            .map_err(|error| AppError::io("Could not atomically replace output", &error))?;
        sync_parent(&self.destination)?;
        Ok(())
    }
}

impl Drop for AtomicOutput {
    fn drop(&mut self) {
        if self.file.is_some() || self.temporary.exists() {
            let _ = fs::remove_file(&self.temporary);
        }
    }
}

fn sync_parent(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let directory = std::fs::File::open(parent)
            .map_err(|error| AppError::io("Could not open output directory for sync", &error))?;
        directory
            .sync_all()
            .map_err(|error| AppError::io("Could not sync output directory", &error))?;
    }
    Ok(())
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    AtomicOutput::prepare(path)?.commit(bytes)
}
