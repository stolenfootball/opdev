//! Acceptance tests for portable, read-only experiment record validation.

use std::fs;
use std::process::Command;

use opdev_project::{
    CommandSpec, MANIFEST_PATH, TestStage, TestSuite, discover, validate_experiment,
};
use serde_json::{Value, json};

const TEMPLATE: &str =
    include_str!("../../../plugins/opdev/skills/opdev/references/experiment.yaml");

fn record() -> Result<Value, Box<dyn std::error::Error>> {
    let mut value: Value = serde_saphyr::from_str(TEMPLATE)?;
    value["review_by"] = json!("2099-01-01");
    Ok(value)
}

fn project() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(repo.path())
            .status()?
            .success()
    );
    let mut manifest = discover(repo.path())?.manifest;
    manifest.commands.insert(
        "test".into(),
        CommandSpec {
            // Execution would leave an observable marker; the validator must not run it.
            argv: ["git", "config", "--local", "opdev.executed", "true"]
                .map(str::to_owned)
                .to_vec(),
            working_directory: None,
            timeout_seconds: Some(30),
        },
    );
    manifest.testing.suites = ["stable-tests", "experimental-tests"]
        .map(|id| TestSuite {
            id: id.into(),
            command: "test".into(),
            stages: vec![TestStage::PreMerge, TestStage::PostMerge],
        })
        .to_vec();
    manifest.write_new(&repo.path().join(MANIFEST_PATH))?;
    Ok(repo)
}

#[test]
fn cli_validates_record_without_executing_or_writing_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = project()?;
    let input = repo.path().join("experiment.yaml");
    let text = serde_saphyr::to_string(&record()?)?;
    fs::write(&input, &text)?;
    let before = fs::read(repo.path().join(MANIFEST_PATH))?;
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["experiment", "validate"])
        .arg(&input)
        .arg("--root")
        .arg(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("record validation only"));
    assert!(stdout.contains("remain unverified"));
    assert_eq!(fs::read_to_string(input)?, text);
    assert_eq!(fs::read(repo.path().join(MANIFEST_PATH))?, before);
    assert!(!repo.path().join(".opdev/evidence.yaml").exists());
    let marker = Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["config", "--get", "opdev.executed"])
        .output()?;
    assert_eq!(marker.status.code(), Some(1));
    Ok(())
}

#[test]
fn schema_rejects_incomplete_unknown_and_invalid_records() -> Result<(), Box<dyn std::error::Error>>
{
    let repo = project()?;
    let manifest = discover(repo.path())?.manifest;
    for field in [
        "owner",
        "review_by",
        "purpose",
        "decision",
        "isolation",
        "configurations",
        "cleanup",
        "work",
    ] {
        let mut value = record()?;
        value
            .as_object_mut()
            .ok_or("record must be an object")?
            .remove(field);
        assert!(
            validate_experiment(&value.to_string(), &manifest, 0).is_err(),
            "missing {field}"
        );
    }
    for (pointer, replacement) in [
        ("/schema", json!(2)),
        ("/owner", json!(" \t")),
        ("/review_by", json!("2025-02-29")),
        ("/review_by", json!("2024-13-01")),
        ("/decision/promote", json!("")),
        ("/isolation/mechanism", json!("long_lived_branch")),
        ("/configurations/0/suites", json!([])),
        (
            "/configurations/0/suites",
            json!(["stable-tests", "stable-tests"]),
        ),
    ] {
        let mut value = record()?;
        *value
            .pointer_mut(pointer)
            .ok_or("missing fixture pointer")? = replacement;
        assert!(
            validate_experiment(&value.to_string(), &manifest, 0).is_err(),
            "{pointer}"
        );
    }
    let mut value = record()?;
    value["waive_core"] = json!(true);
    assert!(validate_experiment(&value.to_string(), &manifest, 0).is_err());
    assert!(validate_experiment(TEMPLATE, &manifest, 0).is_err());
    assert!(validate_experiment("not: [valid", &manifest, 0).is_err());
    Ok(())
}

#[test]
fn semantic_validation_requires_supported_configurations_and_integration_suites()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = project()?;
    let mut manifest = discover(repo.path())?.manifest;
    for (pointer, replacement, diagnostic) in [
        (
            "/configurations/0/suites",
            json!(["missing"]),
            "unknown test suite",
        ),
        (
            "/configurations/1/id",
            json!("stable"),
            "duplicate configuration",
        ),
        (
            "/configurations/1/exposure",
            json!("stable"),
            "both stable and experimental",
        ),
        (
            "/configurations/0/exposure",
            json!("experimental"),
            "both stable and experimental",
        ),
    ] {
        let mut value = record()?;
        *value
            .pointer_mut(pointer)
            .ok_or("missing fixture pointer")? = replacement;
        let error = validate_experiment(&value.to_string(), &manifest, 0)
            .err()
            .ok_or("expected rejection")?;
        assert!(error.to_string().contains(diagnostic), "{error}");
    }
    manifest.testing.suites[0].stages = vec![TestStage::Local, TestStage::PreMerge];
    assert!(validate_experiment(&record()?.to_string(), &manifest, 0).is_err());
    manifest.testing.suites[0].stages = vec![TestStage::PostMerge];
    assert!(validate_experiment(&record()?.to_string(), &manifest, 0).is_err());
    Ok(())
}

#[test]
fn review_day_is_inclusive_and_shared_suites_are_supported()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = project()?;
    let manifest = discover(repo.path())?.manifest;
    let mut value = record()?;
    value["review_by"] = json!("2000-01-01");
    value["configurations"][1]["suites"] = json!(["stable-tests"]);
    assert_eq!(
        validate_experiment(&value.to_string(), &manifest, 10_957)?,
        "alternative-engine"
    );
    let error = validate_experiment(&value.to_string(), &manifest, 10_958)
        .err()
        .ok_or("expected overdue rejection")?;
    assert!(error.to_string().contains("review overdue"));
    Ok(())
}

#[test]
fn cli_rejects_overdue_and_missing_records_with_error_exit()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = project()?;
    let input = repo.path().join("experiment.yaml");
    let mut value = record()?;
    value["review_by"] = json!("2000-01-01");
    fs::write(&input, serde_saphyr::to_string(&value)?)?;
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["experiment", "validate"])
        .arg(&input)
        .arg("--root")
        .arg(repo.path())
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)?.contains("review overdue"));
    let missing = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["experiment", "validate"])
        .arg(repo.path().join("missing.yaml"))
        .arg("--root")
        .arg(repo.path())
        .output()?;
    assert_eq!(missing.status.code(), Some(2));
    Ok(())
}
