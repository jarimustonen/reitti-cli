use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{json, Value};

use crate::{command::AgentArg, error::AppError, output::Warning};

pub const NAME: &str = "reitti";
pub const DESCRIPTION: &str = "Resolve HSL-area places and stops, compare public-transport journeys, check departure or arrival plans, and inspect live departures and service alerts with the reitti CLI. Use for travel in Helsinki, Espoo, Vantaa, Kauniainen, Kerava, Kirkkonummi, Sipoo, Siuntio, or Tuusula.";
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SKILL_SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Copy)]
pub struct Resource {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

pub const RESOURCES: &[Resource] = &[
    Resource {
        path: "SKILL.md",
        bytes: include_bytes!("../skills/reitti/SKILL.md"),
    },
    Resource {
        path: "references/workflows.md",
        bytes: include_bytes!("../skills/reitti/references/workflows.md"),
    },
];

#[derive(Debug, Serialize)]
pub struct SkillMetadata {
    pub name: &'static str,
    pub description: &'static str,
    pub cli_version: &'static str,
    pub schema_version: u8,
}

pub fn metadata() -> SkillMetadata {
    let frontmatter =
        crate::skill_manifest::verify(include_str!("../skills/reitti/SKILL.md"), CLI_VERSION)
            .expect("build.rs validated bundled skill frontmatter");
    assert_eq!(frontmatter.description, DESCRIPTION);
    SkillMetadata {
        name: NAME,
        description: DESCRIPTION,
        cli_version: CLI_VERSION,
        schema_version: SKILL_SCHEMA_VERSION,
    }
}

pub fn resource(name: &str, path: Option<&str>) -> Result<Resource, AppError> {
    require_name(name)?;
    let path = path.unwrap_or("SKILL.md");
    RESOURCES
        .iter()
        .copied()
        .find(|resource| resource.path == path)
        .ok_or_else(|| {
            AppError::invalid(
                "skill_resource_not_found",
                format!("Resource '{path}' is not bundled for skill '{NAME}'."),
                path,
                json!(resource_paths()),
            )
        })
}

pub fn require_name(name: &str) -> Result<(), AppError> {
    if name == NAME {
        Ok(())
    } else {
        Err(AppError::invalid(
            "skill_not_found",
            format!("Skill '{name}' is not bundled in this build."),
            name,
            json!([NAME]),
        ))
    }
}

pub fn resource_paths() -> Vec<&'static str> {
    RESOURCES.iter().map(|resource| resource.path).collect()
}

pub fn print_data(resource: Resource) -> Result<Value, AppError> {
    let content = std::str::from_utf8(resource.bytes).map_err(|_| {
        AppError::system(
            "internal_error",
            format!("Bundled skill resource '{}' is not UTF-8.", resource.path),
        )
    })?;
    Ok(json!({
        "name": NAME,
        "cli_version": CLI_VERSION,
        "schema_version_skill": SKILL_SCHEMA_VERSION,
        "content": content,
        "path_in_repo": format!("crates/reitti-cli/skills/{NAME}/{}", resource.path),
        "resources": resource_paths(),
    }))
}

#[derive(Debug)]
pub struct InstallOutcome {
    pub text: String,
    pub data: Value,
    pub dry_run: bool,
    pub mutation_applied: bool,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Copy)]
struct Layout {
    agent: &'static str,
    relative: &'static str,
}

const LAYOUTS: &[Layout] = &[
    Layout {
        agent: "claude",
        relative: ".claude/skills",
    },
    Layout {
        agent: "pi",
        relative: ".pi/agent/skills",
    },
    Layout {
        agent: "codex",
        relative: ".codex/skills",
    },
];

#[derive(Debug)]
enum PlanState {
    Create,
    Replace,
    Unchanged,
}

#[derive(Debug)]
struct PlannedDestination {
    agent: &'static str,
    destination: PathBuf,
    state: PlanState,
}

pub fn install(
    name: Option<&str>,
    agent: AgentArg,
    target: Option<&Path>,
    dry_run: bool,
    force: bool,
) -> Result<InstallOutcome, AppError> {
    if let Some(name) = name {
        require_name(name)?;
    }
    let base = match target {
        Some(target) if target.as_os_str().is_empty() => {
            return Err(AppError::invalid(
                "invalid_target",
                "Install base must not be empty.",
                "",
                "non-empty directory path",
            ));
        }
        Some(target) if target.is_absolute() => target.to_owned(),
        Some(target) => env::current_dir()
            .map_err(|error| AppError::io("Could not resolve relative --target", &error))?
            .join(target),
        None => PathBuf::from(env::var_os("HOME").ok_or_else(|| {
            AppError::caller(
                "home_unresolved",
                "HOME is not set; pass --target with an explicit install base.",
            )
        })?),
    };

    let selected = selected_layouts(agent);
    let mut plan = Vec::with_capacity(selected.len());
    for layout in selected {
        let destination = base.join(layout.relative).join(NAME);
        let state = inspect_destination(&destination)?;
        if matches!(state, PlanState::Replace) && !force {
            return Err(AppError::caller(
                "skill_exists",
                format!(
                    "Skill destination '{}' differs from the bundled tree; rerun with --force to replace it.",
                    destination.display()
                ),
            )
            .with_detail("destination", destination.to_string_lossy().into_owned())
            .with_detail("agent", layout.agent));
        }
        plan.push(PlannedDestination {
            agent: layout.agent,
            destination,
            state,
        });
    }

    if dry_run {
        let would = plan
            .iter()
            .map(|item| {
                let action = match item.state {
                    PlanState::Create => "create",
                    PlanState::Replace => "replace",
                    PlanState::Unchanged => "none",
                };
                json!({
                    "action": action,
                    "resource": "agent-skill-tree",
                    "input": {"name": NAME, "agent": item.agent, "destination": item.destination},
                    "known_effects": {"status": format!("would_{action}")},
                    "unknown_until_apply": []
                })
            })
            .collect::<Vec<_>>();
        return Ok(InstallOutcome {
            text: plan
                .iter()
                .map(|item| {
                    format!(
                        "{}  {}",
                        state_word(&item.state),
                        item.destination.display()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            data: json!({"dry_run": true, "would": would}),
            dry_run: true,
            mutation_applied: false,
            warnings: Vec::new(),
        });
    }

    let mut prepared = Vec::new();
    for item in &plan {
        if matches!(item.state, PlanState::Unchanged) {
            prepared.push(None);
        } else {
            match prepare_tree(&item.destination) {
                Ok(tree) => prepared.push(Some(tree)),
                Err(error) => {
                    cleanup_prepared(&mut prepared);
                    return Err(error);
                }
            }
        }
    }

    let mut installed = Vec::new();
    let mut existed = Vec::new();
    for index in 0..plan.len() {
        let item = &plan[index];
        if matches!(item.state, PlanState::Unchanged) {
            existed.push(display_skill_file(&item.destination));
            continue;
        }
        let temporary = prepared[index]
            .take()
            .expect("changed destination has a prepared tree");
        let applied = match item.state {
            PlanState::Create => atomic_install_new(&temporary, &item.destination),
            PlanState::Replace => atomic_exchange(&temporary, &item.destination),
            PlanState::Unchanged => unreachable!(),
        };
        if let Err(error) = applied {
            let _ = remove_tree(&temporary);
            cleanup_prepared(&mut prepared);
            return Err(AppError::io(
                &format!(
                    "Could not atomically apply skill destination '{}'",
                    item.destination.display()
                ),
                &error,
            )
            .with_detail(
                "failed_destination",
                item.destination.to_string_lossy().into_owned(),
            )
            .with_detail("applied", json!(installed)));
        }
        installed.push(display_skill_file(&item.destination));
        if matches!(item.state, PlanState::Replace) {
            if let Err(error) =
                finish_replacement_cleanup(&temporary, &item.destination, &installed, remove_tree)
            {
                cleanup_prepared(&mut prepared);
                return Err(error);
            }
        }
    }

    let data = json!({
        "name": NAME,
        "agent": agent.as_str(),
        "installed": installed,
        "existed": existed,
        "skipped": [],
    });
    let text = if installed.is_empty() {
        format!("Skill '{NAME}' is already identical at all selected destinations.")
    } else {
        format!(
            "Installed skill '{NAME}' at {} destination(s).",
            installed.len()
        )
    };
    Ok(InstallOutcome {
        text,
        data,
        dry_run: false,
        mutation_applied: !installed.is_empty(),
        warnings: Vec::new(),
    })
}

impl AgentArg {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Pi => "pi",
            Self::Codex => "codex",
            Self::All => "all",
        }
    }
}

fn selected_layouts(agent: AgentArg) -> Vec<Layout> {
    match agent {
        AgentArg::Claude => vec![LAYOUTS[0]],
        AgentArg::Pi => vec![LAYOUTS[1]],
        AgentArg::Codex => vec![LAYOUTS[2]],
        AgentArg::All => LAYOUTS.to_vec(),
    }
}

fn state_word(state: &PlanState) -> &'static str {
    match state {
        PlanState::Create => "CREATE",
        PlanState::Replace => "REPLACE",
        PlanState::Unchanged => "UNCHANGED",
    }
}

fn display_skill_file(destination: &Path) -> String {
    destination.join("SKILL.md").to_string_lossy().into_owned()
}

fn inspect_destination(destination: &Path) -> Result<PlanState, AppError> {
    match fs::symlink_metadata(destination) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(PlanState::Create),
        Err(error) => Err(AppError::io("Could not inspect skill destination", &error)),
        Ok(metadata) if metadata.is_dir() && tree_is_identical(destination)? => {
            Ok(PlanState::Unchanged)
        }
        Ok(_) => Ok(PlanState::Replace),
    }
}

fn tree_is_identical(root: &Path) -> Result<bool, AppError> {
    let mut found = Vec::new();
    collect_entries(root, root, &mut found)?;
    found.sort();
    let mut expected = vec![("references".to_owned(), EntryKind::Directory)];
    expected.extend(
        RESOURCES
            .iter()
            .map(|resource| (resource.path.to_owned(), EntryKind::File)),
    );
    expected.sort();
    if found != expected {
        return Ok(false);
    }
    for resource in RESOURCES {
        let bytes = fs::read(root.join(resource.path))
            .map_err(|error| AppError::io("Could not read installed skill resource", &error))?;
        if bytes != resource.bytes {
            return Ok(false);
        }
    }
    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EntryKind {
    Directory,
    File,
    Other,
}

fn collect_entries(
    root: &Path,
    current: &Path,
    found: &mut Vec<(String, EntryKind)>,
) -> Result<(), AppError> {
    let entries = fs::read_dir(current)
        .map_err(|error| AppError::io("Could not inspect installed skill tree", &error))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| AppError::io("Could not inspect installed skill tree", &error))?;
        let kind = entry
            .file_type()
            .map_err(|error| AppError::io("Could not inspect installed skill resource", &error))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .expect("descendant")
            .to_string_lossy()
            .replace('\\', "/");
        let entry_kind = if kind.is_dir() {
            EntryKind::Directory
        } else if kind.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        found.push((relative, entry_kind));
        if kind.is_dir() {
            collect_entries(root, &path, found)?;
        }
    }
    Ok(())
}

fn prepare_tree(destination: &Path) -> Result<PathBuf, AppError> {
    let parent = destination.parent().expect("skill destination has parent");
    fs::create_dir_all(parent)
        .map_err(|error| AppError::io("Could not create skill layout directory", &error))?;
    for attempt in 0..100 {
        let temporary = parent.join(format!(
            ".{NAME}.reitti-tmp-{}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&temporary) {
            Ok(()) => {
                if let Err(error) = write_tree(&temporary) {
                    let _ = remove_tree(&temporary);
                    return Err(error);
                }
                return Ok(temporary);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(AppError::io("Could not prepare skill tree", &error)),
        }
    }
    Err(AppError::system(
        "io_error",
        "Could not allocate a collision-free temporary skill tree.",
    ))
}

fn cleanup_prepared(prepared: &mut [Option<PathBuf>]) {
    for path in prepared.iter_mut().filter_map(Option::take) {
        let _ = remove_tree(&path);
    }
}

fn finish_replacement_cleanup(
    temporary: &Path,
    destination: &Path,
    installed: &[String],
    cleanup: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), AppError> {
    cleanup(temporary).map_err(|error| {
        AppError::io(
            "Skill was replaced but the old temporary tree could not be removed",
            &error,
        )
        .with_detail("destination", destination.to_string_lossy().into_owned())
        .with_detail("applied", json!(installed))
    })
}

fn write_tree(root: &Path) -> Result<(), AppError> {
    for resource in RESOURCES {
        let path = root.join(resource.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AppError::io("Could not prepare skill resource directory", &error)
            })?;
        }
        fs::write(&path, resource.bytes)
            .map_err(|error| AppError::io("Could not prepare skill resource", &error))?;
    }
    sync_tree(root)?;
    Ok(())
}

fn sync_tree(root: &Path) -> Result<(), AppError> {
    for resource in RESOURCES {
        fs::File::open(root.join(resource.path))
            .and_then(|file| file.sync_all())
            .map_err(|error| AppError::io("Could not sync skill resource", &error))?;
    }
    #[cfg(unix)]
    fs::File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| AppError::io("Could not sync skill tree", &error))?;
    Ok(())
}

fn remove_tree(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn atomic_install_new(source: &Path, destination: &Path) -> io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let source = CString::new(source.as_os_str().as_bytes())?;
    let destination = CString::new(destination.as_os_str().as_bytes())?;
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn atomic_exchange(source: &Path, destination: &Path) -> io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let source = CString::new(source.as_os_str().as_bytes())?;
    let destination = CString::new(destination.as_os_str().as_bytes())?;
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn atomic_install_new(source: &Path, destination: &Path) -> io::Result<()> {
    apple_rename(source, destination, libc::RENAME_EXCL)
}

#[cfg(target_os = "macos")]
fn atomic_exchange(source: &Path, destination: &Path) -> io::Result<()> {
    apple_rename(source, destination, libc::RENAME_SWAP)
}

#[cfg(target_os = "macos")]
fn apple_rename(source: &Path, destination: &Path, flags: libc::c_uint) -> io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let source = CString::new(source.as_os_str().as_bytes())?;
    let destination = CString::new(destination.as_os_str().as_bytes())?;
    let result = unsafe {
        libc::renameatx_np(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            flags,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
fn atomic_install_new(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic no-replace directory rename is unsupported on this platform",
    ))
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
fn atomic_exchange(_source: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic directory exchange is unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn replacement_cleanup_failure_reports_already_applied_destination() {
        let installed = vec!["/base/.claude/skills/reitti/SKILL.md".to_owned()];
        let error = finish_replacement_cleanup(
            Path::new("/temporary"),
            Path::new("/base/.claude/skills/reitti"),
            &installed,
            |_| Err(io::Error::other("synthetic cleanup failure")),
        )
        .unwrap_err();
        assert_eq!(error.details["applied"], json!(installed));
        assert_eq!(error.details["destination"], "/base/.claude/skills/reitti");
    }

    #[test]
    fn cleanup_guard_removes_every_staged_tree() {
        let root = tempdir().unwrap();
        let first = root.path().join("first");
        let second = root.path().join("second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        fs::write(first.join("resource"), b"one").unwrap();
        fs::write(second.join("resource"), b"two").unwrap();
        let mut prepared = vec![Some(first.clone()), None, Some(second.clone())];
        cleanup_prepared(&mut prepared);
        assert!(!first.exists());
        assert!(!second.exists());
        assert!(prepared.iter().all(Option::is_none));
    }
}
