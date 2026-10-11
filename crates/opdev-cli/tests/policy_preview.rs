//! Real consumer-path previews must not execute declared commands or migrate policy.
use opdev_project::{MANIFEST_PATH, discover};
use serde_json::Value;
use std::{fs, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn explanation_works_outside_a_project_and_rejects_unknowns() -> TestResult {
    let dir = tempfile::tempdir()?;
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_opdev"))
            .current_dir(dir.path())
            .args(args)
            .output()
    };
    let output = invoke(&[
        "policy",
        "explain",
        "--engineering",
        "2",
        "--rule",
        "OPDEV-RECOVERY-001",
        "--format",
        "json",
    ])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["rule"]["id"], "OPDEV-RECOVERY-001");
    assert_eq!(value["change"]["rule"], "MCD-RECOVERY-001");
    assert_eq!(value["change"]["disposition"], "replaced");
    assert!(
        value["limits"]
            .as_str()
            .ok_or("limits")?
            .contains("No checks ran")
    );
    for args in [
        vec!["policy", "explain", "--engineering", "latest"],
        vec![
            "policy",
            "explain",
            "--engineering",
            "2",
            "--rule",
            "UNKNOWN-RULE-001",
        ],
        vec!["policy", "preview", "--engineering", "2"],
        vec!["policy", "explain", "--engineering", "2", "--apply"],
    ] {
        assert_eq!(invoke(&args)?.status.code(), Some(2));
    }
    assert_eq!(fs::read_dir(dir.path())?.count(), 0);
    Ok(())
}

#[test]
fn project_preview_preserves_contract_and_does_not_launch_commands() -> TestResult {
    let dir = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(dir.path())
            .status()?
            .success()
    );
    let mut project = discover(dir.path())?.manifest;
    for command in project.commands.values_mut() {
        command.argv = vec![
            "git".into(),
            "config".into(),
            "--local".into(),
            "opdev.preview-executed".into(),
            "true".into(),
        ];
    }
    project.write_new(&dir.path().join(MANIFEST_PATH))?;
    let before = fs::read(dir.path().join(MANIFEST_PATH))?;
    let config = fs::read(dir.path().join(".git/config"))?;
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(dir.path())
        .args([
            "policy",
            "preview",
            "--engineering",
            "2",
            "--format",
            "json",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["current_catalog"], 2);
    assert!(value["current_engineering"].is_null());
    assert_eq!(value["proposed"]["catalog_version"], 4);
    let added = value["added_engineering"].as_array().ok_or("added")?;
    assert!(added.contains(&Value::from("OPDEV-BUILD-001")));
    assert!(added.contains(&Value::from("OPDEV-BRANCH-001")));
    assert!(added.contains(&Value::from("OPDEV-FLOW-001")));
    let removed = value["removed_engineering"].as_array().ok_or("removed")?;
    assert!(removed.contains(&Value::from("MCD-FLOW-001")));
    assert!(removed.contains(&Value::from("MCD-TRUNK-003")));
    assert!(!removed.contains(&Value::from("OPDEV-TEST-002")));
    assert_eq!(
        value["proposed"]["changes"]
            .as_array()
            .ok_or("changes")?
            .len(),
        42
    );
    assert_eq!(before, fs::read(dir.path().join(MANIFEST_PATH))?);
    assert_eq!(config, fs::read(dir.path().join(".git/config"))?);
    assert_eq!(fs::read_dir(dir.path().join(".opdev"))?.count(), 1);
    Ok(())
}
