use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildStamp {
    pub commit: Option<String>,
    pub kind: String,
    pub note: String,
    pub tracked_paths: Vec<PathBuf>,
}

pub fn discover(workspace: &Path) -> Result<BuildStamp, String> {
    let marker = workspace.join(".git");
    if !marker.exists() {
        return Ok(BuildStamp {
            commit: None,
            kind: "tarball".to_owned(),
            note: "no .git context in source archive".to_owned(),
            tracked_paths: Vec::new(),
        });
    }

    let top = git(workspace, &["rev-parse", "--show-toplevel"])?;
    let top = fs::canonicalize(top.trim())
        .map_err(|error| format!("could not canonicalize git top-level: {error}"))?;
    let expected = fs::canonicalize(workspace)
        .map_err(|error| format!("could not canonicalize workspace: {error}"))?;
    if top != expected {
        return Err(format!(
            "workspace .git marker resolved to a different top-level: expected '{}', got '{}'",
            expected.display(),
            top.display()
        ));
    }

    let commit = git(workspace, &["rev-parse", "HEAD"])?;
    let commit = commit.trim().to_owned();
    validate_sha(&commit)?;

    let head = git_path(workspace, "HEAD")?;
    let mut tracked_paths = vec![head.clone()];
    if let Ok(contents) = fs::read_to_string(&head) {
        if let Some(reference) = contents.strip_prefix("ref: ").map(str::trim) {
            tracked_paths.push(git_path(workspace, reference)?);
        }
    }
    let common = git(workspace, &["rev-parse", "--git-common-dir"])?;
    let common = absolutize(workspace, Path::new(common.trim()));
    tracked_paths.push(common.join("packed-refs"));
    tracked_paths.sort();
    tracked_paths.dedup();

    Ok(BuildStamp {
        commit: Some(commit),
        kind: "git".to_owned(),
        note: "git source tree".to_owned(),
        tracked_paths,
    })
}

pub fn validate_sha(value: &str) -> Result<(), String> {
    if value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!(
            "build commit must be a full 40-character hexadecimal SHA, got length {}",
            value.len()
        ))
    }
}

fn git_path(workspace: &Path, name: &str) -> Result<PathBuf, String> {
    let value = git(workspace, &["rev-parse", "--git-path", name])?;
    Ok(absolutize(workspace, Path::new(value.trim())))
}

fn absolutize(workspace: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        workspace.join(path)
    }
}

fn git(workspace: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(args)
        .output()
        .map_err(|error| format!("could not execute git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed for workspace '{}': {}",
            args.join(" "),
            workspace.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("git output was not UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn archive_nested_in_unrelated_repository_does_not_inherit_ancestor_sha() {
        let root = tempdir().unwrap();
        run(root.path(), &["init", "-q"]);
        let archive = root.path().join("downloads/reitti");
        fs::create_dir_all(&archive).unwrap();
        let stamp = discover(&archive).unwrap();
        assert_eq!(stamp.commit, None);
        assert_eq!(stamp.kind, "tarball");
    }

    #[test]
    fn broken_workspace_marker_is_a_hard_error() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join(".git"),
            "gitdir: /missing/reitti-git-dir\n",
        )
        .unwrap();
        let error = discover(root.path()).unwrap_err();
        assert!(error.contains("git rev-parse --show-toplevel failed"));
    }

    #[test]
    fn real_workspace_reports_available_source_provenance() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let stamp = discover(&workspace).unwrap();
        if stamp.kind == "git" {
            assert!(stamp.commit.is_some());
            assert!(stamp
                .tracked_paths
                .iter()
                .any(|path| path.ends_with("HEAD")));
            assert!(stamp
                .tracked_paths
                .iter()
                .any(|path| path.ends_with("packed-refs")));
        } else {
            assert_eq!(stamp.kind, "tarball");
            assert_eq!(stamp.commit, None);
            assert!(stamp.tracked_paths.is_empty());
        }
    }

    fn run(directory: &Path, args: &[&str]) {
        assert!(Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(args)
            .status()
            .unwrap()
            .success());
    }
}
