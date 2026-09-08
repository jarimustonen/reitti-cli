#[path = "src/build_provenance.rs"]
mod build_provenance;
#[path = "src/skill_manifest.rs"]
mod skill_manifest;

use std::{env, path::Path};

fn main() {
    println!("cargo:rerun-if-env-changed=REITTI_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=REITTI_BUILD_PROVENANCE_KIND");

    verify_bundled_skill();

    if let Ok(commit) = env::var("REITTI_BUILD_COMMIT") {
        build_provenance::validate_sha(&commit)
            .unwrap_or_else(|error| panic!("invalid REITTI_BUILD_COMMIT: {error}"));
        emit(
            &commit,
            "ci-injected",
            "commit supplied by build environment",
        );
        return;
    }

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .parent()
        .and_then(Path::parent)
        .expect("reitti-cli must remain under crates/ in the workspace");
    let mut stamp = build_provenance::discover(workspace)
        .unwrap_or_else(|error| panic!("build provenance failure: {error}"));

    if stamp.commit.is_none() {
        let requested =
            env::var("REITTI_BUILD_PROVENANCE_KIND").unwrap_or_else(|_| "tarball".to_owned());
        assert!(
            matches!(requested.as_str(), "tarball" | "vendored"),
            "REITTI_BUILD_PROVENANCE_KIND must be tarball or vendored without a commit"
        );
        stamp.kind = requested;
    }
    for path in stamp.tracked_paths {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    emit(
        stamp.commit.as_deref().unwrap_or(""),
        &stamp.kind,
        &stamp.note,
    );
}

fn verify_bundled_skill() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = manifest.join("skills/reitti/SKILL.md");
    println!("cargo:rerun-if-changed={}", path.display());
    println!(
        "cargo:rerun-if-changed={}",
        manifest
            .join("skills/reitti/references/workflows.md")
            .display()
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("could not read bundled skill '{}': {error}", path.display())
    });
    skill_manifest::verify(&text, env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|error| panic!("invalid bundled skill: {error}"));
    assert!(
        !text.contains("{{") && !text.contains("}}"),
        "bundled skill contains an unexpanded template placeholder"
    );
}

fn emit(commit: &str, kind: &str, note: &str) {
    println!("cargo:rustc-env=REITTI_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=REITTI_BUILD_KIND={kind}");
    println!("cargo:rustc-env=REITTI_BUILD_NOTE={note}");
}
