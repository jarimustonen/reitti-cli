use std::{fs, path::Path, process::Command};

use assert_cmd::cargo::cargo_bin;
use jsonschema::validator_for;
use serde_json::{json, Value};
use tempfile::TempDir;

const BUNDLED_SKILL: &[u8] = include_bytes!("../skills/reitti/SKILL.md");
const BUNDLED_WORKFLOWS: &[u8] = include_bytes!("../skills/reitti/references/workflows.md");

fn reitti(home: &Path) -> Command {
    let mut command = Command::new(cargo_bin("reitti"));
    command
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("DIGITRANSIT_SUBSCRIPTION_KEY")
        .env_remove("REITTI_CONFIG_FILE");
    command
}

fn run(home: &Path, args: &[&str]) -> std::process::Output {
    reitti(home).args(args).output().unwrap()
}

fn assert_tree(base: &Path, layout: &str) {
    let root = base.join(layout).join("reitti");
    assert_eq!(fs::read(root.join("SKILL.md")).unwrap(), BUNDLED_SKILL);
    assert_eq!(
        fs::read(root.join("references/workflows.md")).unwrap(),
        BUNDLED_WORKFLOWS
    );
}

#[test]
fn catalogue_version_and_frontmatter_match_the_binary() {
    let home = TempDir::new().unwrap();
    let list = run(home.path(), &["--json", "skill", "list"]);
    assert!(
        list.status.success(),
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );
    let list: Value = serde_json::from_slice(&list.stdout).unwrap();
    let skill = &list["data"]["skills"][0];
    assert_eq!(skill["name"], "reitti");
    assert_eq!(skill["cli_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(skill["schema_version"], 1);
    assert!(skill["description"].as_str().unwrap().len() <= 1024);
    assert_eq!(
        list["data"]["supported_agents"],
        json!(["claude", "pi", "codex"])
    );
    assert_eq!(list["data"]["install"]["default"], "all");
    assert_eq!(
        list["data"]["install"]["layouts"],
        json!([
            {"agent":"claude","path":".claude/skills/<name>/...","form":"agent-skills-tree"},
            {"agent":"pi","path":".pi/agent/skills/<name>/...","form":"agent-skills-tree"},
            {"agent":"codex","path":".codex/skills/<name>/...","form":"agent-skills-tree"}
        ])
    );

    let version: Value =
        serde_json::from_slice(&run(home.path(), &["--json", "version"]).stdout).unwrap();
    assert_eq!(version["data"]["skills"][0], *skill);
    let text = std::str::from_utf8(BUNDLED_SKILL).unwrap();
    let header = text.split("\n---\n").next().unwrap();
    assert!(header.contains(&format!("cli_version: \"{}\"", env!("CARGO_PKG_VERSION"))));
    assert!(header.contains("schema_version: 1"));
    let bundled_description = header
        .lines()
        .find_map(|line| line.strip_prefix("description: "))
        .unwrap();
    assert_eq!(skill["description"], bundled_description);
    assert!(!text.contains("{{"));
}

#[test]
fn print_show_and_declared_resources_preserve_bytes() {
    let home = TempDir::new().unwrap();
    let printed = run(home.path(), &["skill", "print", "reitti"]);
    assert!(printed.status.success());
    assert_eq!(printed.stdout, BUNDLED_SKILL);
    let shown = run(home.path(), &["skill", "show", "reitti"]);
    assert_eq!(shown.stdout, printed.stdout);

    let resource = run(
        home.path(),
        &[
            "skill",
            "print",
            "reitti",
            "--resource",
            "references/workflows.md",
        ],
    );
    assert_eq!(resource.stdout, BUNDLED_WORKFLOWS);

    let structured = run(home.path(), &["--json", "skill", "print", "reitti"]);
    let structured: Value = serde_json::from_slice(&structured.stdout).unwrap();
    assert_eq!(
        structured["data"]["content"].as_str().unwrap().as_bytes(),
        BUNDLED_SKILL
    );
    assert_eq!(
        structured["data"]["path_in_repo"],
        "crates/reitti-cli/skills/reitti/SKILL.md"
    );
    assert_eq!(
        structured["data"]["resources"],
        json!(["SKILL.md", "references/workflows.md"])
    );

    for invalid in ["../SKILL.md", "/etc/passwd", "references/../SKILL.md"] {
        let output = run(
            home.path(),
            &["--json", "skill", "print", "reitti", "--resource", invalid],
        );
        assert_eq!(output.status.code(), Some(1));
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "skill_resource_not_found");
    }
}

#[test]
fn default_and_explicit_all_install_every_native_tree() {
    for explicit_all in [false, true] {
        let home = TempDir::new().unwrap();
        let mut args = vec!["--json", "skill", "install", "reitti"];
        if explicit_all {
            args.extend(["--agent", "all"]);
        }
        let output = run(home.path(), &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["data"]["installed"].as_array().unwrap().len(), 3);
        assert_tree(home.path(), ".claude/skills");
        assert_tree(home.path(), ".pi/agent/skills");
        assert_tree(home.path(), ".codex/skills");
        assert!(!home.path().join(".agents/skills/reitti").exists());
    }
}

#[test]
fn each_explicit_agent_installs_only_its_native_layout() {
    let cases = [
        ("claude", ".claude/skills"),
        ("pi", ".pi/agent/skills"),
        ("codex", ".codex/skills"),
    ];
    for (agent, layout) in cases {
        let home = TempDir::new().unwrap();
        let base = home.path().join("stage");
        let output = run(
            home.path(),
            &[
                "skill",
                "install",
                "reitti",
                "--agent",
                agent,
                "--target",
                base.to_str().unwrap(),
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_tree(&base, layout);
        assert_eq!(
            [".claude/skills", ".pi/agent/skills", ".codex/skills"]
                .into_iter()
                .filter(|candidate| base.join(candidate).join("reitti").exists())
                .count(),
            1
        );
    }
}

#[test]
fn relative_target_replaces_base_without_changing_layout() {
    let home = TempDir::new().unwrap();
    let output = reitti(home.path())
        .current_dir(home.path())
        .args(["skill", "install", "--agent", "pi", "--target", "./stage"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_tree(&home.path().join("stage"), ".pi/agent/skills");
    assert!(!home.path().join("stage/.pi/skills/reitti").exists());

    let empty = run(home.path(), &["--json", "skill", "install", "--target", ""]);
    assert_eq!(empty.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&empty.stderr).contains("usage_error"));
}

#[test]
fn identical_rerun_is_unchanged_but_every_differing_tree_requires_force() {
    let home = TempDir::new().unwrap();
    let base = home.path().join("base");
    let base_text = base.to_str().unwrap();
    assert!(
        run(home.path(), &["skill", "install", "--target", base_text])
            .status
            .success()
    );
    let rerun = run(
        home.path(),
        &["--json", "skill", "install", "--target", base_text],
    );
    assert!(rerun.status.success());
    let rerun: Value = serde_json::from_slice(&rerun.stdout).unwrap();
    assert!(rerun["data"]["installed"].as_array().unwrap().is_empty());
    assert_eq!(rerun["data"]["existed"].as_array().unwrap().len(), 3);

    let claude = base.join(".claude/skills/reitti");
    fs::write(
        claude.join("SKILL.md"),
        "---\nname: reitti\ncli_version: \"9.0.0\"\nschema_version: 1\n---\nnewer\n",
    )
    .unwrap();
    let conflict = run(
        home.path(),
        &["--json", "skill", "install", "--target", base_text],
    );
    assert_eq!(conflict.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&conflict.stderr).unwrap()["error"]["code"],
        "skill_exists"
    );
    assert!(
        String::from_utf8(fs::read(claude.join("SKILL.md")).unwrap())
            .unwrap()
            .contains("9.0.0")
    );

    let forced = run(
        home.path(),
        &["skill", "install", "--target", base_text, "--force"],
    );
    assert!(
        forced.status.success(),
        "{}",
        String::from_utf8_lossy(&forced.stderr)
    );
    assert_tree(&base, ".claude/skills");

    fs::write(
        claude.join("SKILL.md"),
        "---\nname: reitti\ncli_version: \"0.0.0-old\"\nschema_version: 1\n---\nolder\n",
    )
    .unwrap();
    assert_eq!(
        run(home.path(), &["skill", "install", "--target", base_text])
            .status
            .code(),
        Some(1)
    );
    assert!(run(
        home.path(),
        &["skill", "install", "--target", base_text, "--force"]
    )
    .status
    .success());
}

#[cfg(unix)]
#[test]
fn extra_empty_directory_and_symlink_make_a_tree_different() {
    use std::os::unix::fs::symlink;

    for extra in ["directory", "symlink"] {
        let home = TempDir::new().unwrap();
        let base = home.path().join("base");
        let base_text = base.to_str().unwrap();
        assert!(run(
            home.path(),
            &["skill", "install", "--agent", "claude", "--target", base_text]
        )
        .status
        .success());
        let root = base.join(".claude/skills/reitti");
        if extra == "directory" {
            fs::create_dir(root.join("extra")).unwrap();
        } else {
            symlink("SKILL.md", root.join("extra-link")).unwrap();
        }
        let output = run(
            home.path(),
            &[
                "--json", "skill", "install", "--agent", "claude", "--target", base_text,
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"]["code"],
            "skill_exists"
        );
    }
}

#[test]
fn dry_run_and_explicit_output_write_only_the_plan() {
    let home = TempDir::new().unwrap();
    let base = home.path().join("base");
    let report = home.path().join("plan.json");
    let output = run(
        home.path(),
        &[
            "--json",
            "--output",
            report.to_str().unwrap(),
            "skill",
            "install",
            "--target",
            base.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!base.exists());
    let plan: Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    assert_eq!(plan["data"]["dry_run"], true);
    assert_eq!(plan["data"]["would"].as_array().unwrap().len(), 3);
    assert!(plan["data"]["would"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["action"] == "create"));
}

#[test]
fn unsupported_agent_and_name_are_actionable() {
    let home = TempDir::new().unwrap();
    let agent = run(
        home.path(),
        &["--json", "skill", "install", "--agent", "cursor"],
    );
    assert_eq!(agent.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&agent.stderr).unwrap()["error"]["code"],
        "usage_error"
    );
    let name = run(home.path(), &["--json", "skill", "install", "other"]);
    assert_eq!(name.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&name.stderr).unwrap()["error"]["code"],
        "skill_not_found"
    );
}

#[test]
fn actual_skill_results_validate_against_strict_schemas() {
    let home = TempDir::new().unwrap();
    let schema = |name: &str| -> Value {
        let output = run(home.path(), &["--json", "schema", "show", name]);
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["data"]["schema"].clone()
    };
    for (name, args) in [
        ("skill-list", vec!["--json", "skill", "list"]),
        ("skill-print", vec!["--json", "skill", "print", "reitti"]),
        (
            "skill-install",
            vec![
                "--json",
                "skill",
                "install",
                "--agent",
                "claude",
                "--dry-run",
            ],
        ),
    ] {
        let output = run(home.path(), &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let data = serde_json::from_slice::<Value>(&output.stdout).unwrap()["data"].clone();
        validator_for(&schema(name))
            .unwrap()
            .validate(&data)
            .unwrap();
        let mut extra = data.clone();
        extra
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        assert!(validator_for(&schema(name))
            .unwrap()
            .validate(&extra)
            .is_err());
    }
}

#[test]
fn doctor_checks_leading_frontmatter_version_and_schema() {
    let home = TempDir::new().unwrap();
    assert!(run(home.path(), &["skill", "install", "--agent", "claude"])
        .status
        .success());
    let path = home.path().join(".claude/skills/reitti/SKILL.md");
    fs::write(
        &path,
        format!(
            "---\nname: reitti\ncli_version: \"{}\"\nschema_version: 9\n---\ncli_version: \"{}\"\nschema_version: 1\n",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    let doctor = reitti(home.path())
        .env("DIGITRANSIT_SUBSCRIPTION_KEY", "synthetic-key")
        .env("REITTI_PRIVATE_MARKERS", "[\"fictional-private-marker\"]")
        .args(["--json", "doctor"])
        .output()
        .unwrap();
    assert!(doctor.status.success());
    let value: Value = serde_json::from_slice(&doctor.stdout).unwrap();
    let check = value["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["id"] == "skill.sync")
        .unwrap();
    assert_eq!(check["status"], "warn");
    assert_eq!(check["details"]["mismatched_agents"], json!(["claude"]));
}
