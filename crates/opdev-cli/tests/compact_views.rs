//! Process-level acceptance of compact views and retained full evidence.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use opdev_project::{CommandSpec, MANIFEST_PATH, TestStage, TestSuite, discover};
use serde_json::Value;

fn opdev() -> Command {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
}

fn git(root: &Path, args: &[&str]) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?)
}

fn project() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    let root = repo.path();
    assert!(git(root, &["init", "--quiet"])?.status.success());
    let mut manifest = discover(root)?.manifest;
    manifest.commands.insert(
        "check".into(),
        CommandSpec {
            argv: ["git", "config", "--local", "--add", "opdev.runs", "once"]
                .map(str::to_owned)
                .to_vec(),
            working_directory: None,
            timeout_seconds: Some(30),
        },
    );
    manifest.commands.insert(
        "fail".into(),
        CommandSpec {
            argv: [
                "git",
                "rev-parse",
                "--verify",
                "opdev-nonexistent-reference",
            ]
            .map(str::to_owned)
            .to_vec(),
            working_directory: None,
            timeout_seconds: Some(30),
        },
    );
    manifest.testing.suites = vec![
        TestSuite {
            id: "count".into(),
            command: "check".into(),
            stages: vec![TestStage::Local],
        },
        TestSuite {
            id: "failure".into(),
            command: "fail".into(),
            stages: vec![TestStage::Local],
        },
    ];
    manifest.write_new(&root.join(MANIFEST_PATH))?;
    assert!(git(root, &["add", "."])?.status.success());
    Ok(repo)
}

#[test]
fn full_artifact_is_retained_once_and_offline_projection_does_not_execute_checks()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = project()?;
    let root = repo.path();
    let artifacts = tempfile::tempdir()?;
    let missing_path = opdev()
        .args(["check", "--root"])
        .arg(root)
        .args(["--format", "summary"])
        .output()?;
    assert_eq!(missing_path.status.code(), Some(2));
    assert!(
        git(root, &["config", "--get-all", "opdev.runs"])?
            .stdout
            .is_empty()
    );
    let path = artifacts.path().join("full.json");
    let check = opdev()
        .args(["check", "--root"])
        .arg(root)
        .args(["--format", "summary", "--report"])
        .arg(&path)
        .output()?;
    assert_eq!(
        check.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let summary: Value = serde_json::from_slice(&check.stdout)?;
    let before = fs::read(&path)?;
    let full: Value = serde_json::from_slice(&before)?;
    assert_eq!(summary["gates"], full["gates"]);
    assert_eq!(summary["checks"][1]["outcome"], "failed");
    assert!(
        full["checks"][1]["stderr"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert_eq!(
        git(root, &["config", "--get-all", "opdev.runs"])?.stdout,
        b"once\n"
    );
    let offline = opdev()
        .current_dir(artifacts.path())
        .args(["report", "summarize"])
        .arg(&path)
        .output()?;
    assert_eq!(offline.status.code(), Some(1));
    assert_eq!(serde_json::from_slice::<Value>(&offline.stdout)?, summary);
    assert_eq!(
        git(root, &["config", "--get-all", "opdev.runs"])?.stdout,
        b"once\n"
    );
    assert_eq!(fs::read(&path)?, before);
    let existing = opdev()
        .args(["check", "--root"])
        .arg(root)
        .arg("--report")
        .arg(&path)
        .output()?;
    assert_eq!(existing.status.code(), Some(2));
    assert_eq!(
        git(root, &["config", "--get-all", "opdev.runs"])?.stdout,
        b"once\n"
    );
    let evidence = opdev()
        .args(["evidence", "show", "--current", "--root"])
        .arg(root)
        .args(["--rule", "OPDEV-WORK-001"])
        .output()?;
    assert!(evidence.status.success());
    let evidence: Value = serde_json::from_slice(&evidence.stdout)?;
    assert_eq!(evidence["ledger_present"], false);
    assert_eq!(evidence["missing_requested_rule"], "OPDEV-WORK-001");
    assert!(evidence["change"].is_null());
    fs::write(root.join("unindexed"), "new material")?;
    let stale = opdev()
        .args(["evidence", "show", "--current", "--root"])
        .arg(root)
        .output()?;
    assert_eq!(stale.status.code(), Some(2));
    assert!(stale.stdout.is_empty());
    Ok(())
}
