//! Command-level boundary tests assert side effects, not prompt wording.
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn run(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .arg("check")
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
}
fn fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(dir.path())
            .status()?
            .success()
    );
    fs::create_dir(dir.path().join(".opdev"))?;
    fs::write(
        dir.path().join(".opdev/guidance.md"),
        "Synthetic test guidance",
    )?;
    fs::write(dir.path().join(".gitignore"), "runs\n")?;
    let python = if cfg!(windows) { "python" } else { "python3" };
    let command = |id: &str| {
        json!({"argv":[python,"-c",
        format!("from pathlib import Path; p=Path('runs'); p.open('a').write('{id}\\n')")],"timeout_seconds":5})
    };
    fs::write(
        dir.path().join(".opdev/project.yaml"),
        serde_json::to_vec(&json!({
            "schema":5,"policy_version":1,"preset":"standard","trunk":"main",
            "decision_reference":"synthetic", "commands":{
                "focused":command("focused"),"full":command("full"),"other":command("other")},
            "checks":[{"id":"full","command":"full","stages":["local","pre_merge","post_merge"]},
                {"id":"other","command":"other","stages":["local","pre_merge","post_merge"]}],
            "requirements":[],"policies":[],"authorities":{}
        }))?,
    )?;
    Ok(dir)
}
fn receipts(root: &Path) -> String {
    fs::read_to_string(root.join("runs"))
        .unwrap_or_default()
        .replace("\r\n", "\n")
}

#[test]
fn ambiguous_or_conflicting_selection_never_executes() -> TestResult {
    let dir = fixture()?;
    for args in [
        vec![],
        vec!["--purpose", "feedback"],
        vec!["--purpose", "feedback", "--command", "missing"],
        vec![
            "--purpose",
            "feedback",
            "--command",
            "focused",
            "--command",
            "missing",
        ],
        vec![
            "--purpose",
            "feedback",
            "--command",
            "focused",
            "--command",
            "focused",
        ],
        vec!["--purpose", "feedback", "--command", "full"],
        vec![
            "--purpose",
            "feedback",
            "--command",
            "full",
            "--reason",
            " ",
        ],
        vec!["--purpose", "integration", "--command", "focused"],
        vec!["--purpose", "integration", "--evaluation"],
        vec!["--purpose", "feedback", "--command", "focused", "--ci"],
        vec!["--purpose", "design", "--command", "focused", "--remote"],
        vec![
            "--purpose",
            "feedback",
            "--command",
            "focused",
            "--retain-state",
        ],
        vec![
            "--purpose",
            "feedback",
            "--command",
            "focused",
            "--evaluation",
        ],
    ] {
        let output = run(dir.path(), &args)?;
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(receipts(dir.path()), "", "{args:?}");
    }
    Ok(())
}

#[test]
fn focused_run_and_early_probe_never_claim_integration() -> TestResult {
    let dir = fixture()?;
    for (purpose, id, extra) in [
        ("feedback", "focused", vec![]),
        (
            "design",
            "full",
            vec!["--reason", "User requested compatibility uncertainty probe"],
        ),
    ] {
        let mut args = vec!["--purpose", purpose, "--command", id, "--format", "json"];
        args.extend(extra);
        let output = run(dir.path(), &args)?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(report["kind"], "preparation_check");
        assert_eq!(report["purpose"], purpose);
        assert_eq!(report["integration_status"], "unverified");
        assert!(report.get("gates").is_none());
        assert_eq!(report["checks"].as_array().ok_or("checks array")?.len(), 1);
        assert_eq!(report["checks"][0]["command"], id);
    }
    assert_eq!(receipts(dir.path()), "focused\nfull\n");
    Ok(())
}

#[test]
fn plan_and_no_exec_do_not_run_commands() -> TestResult {
    let dir = fixture()?;
    let output = run(
        dir.path(),
        &["--purpose", "feedback", "--command", "focused", "--plan"],
    )?;
    assert!(output.status.success());
    let plan: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(plan["kind"], "preparation_plan");
    assert_eq!(plan["commands"][0]["id"], "focused");
    assert_eq!(plan["commands"][0]["required_at_integration"], false);
    assert_eq!(plan["commands"][0]["command"]["argv"][1], "-c");
    assert_eq!(
        run(
            dir.path(),
            &["--purpose", "feedback", "--command", "focused", "--no-exec"]
        )?
        .status
        .code(),
        Some(1)
    );
    assert_eq!(receipts(dir.path()), "");
    assert!(run(dir.path(), &["--plan"])?.status.success());
    Ok(())
}

#[test]
fn integration_runs_all_required_commands_and_ci_keeps_its_stage() -> TestResult {
    let dir = fixture()?;
    let output = run(
        dir.path(),
        &["--purpose", "integration", "--format", "json"],
    )?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(receipts(dir.path()), "full\nother\n");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["kind"], "practical_check");
    assert_eq!(report["stage"], "local");
    assert!(
        Command::new("git")
            .current_dir(dir.path())
            .args(["add", "."])
            .status()?
            .success()
    );
    assert!(
        Command::new("git")
            .current_dir(dir.path())
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "fixture"
            ])
            .status()?
            .success()
    );
    let output = run(dir.path(), &["--ci", "--format", "json"])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["stage"], "pre_merge");
    assert_eq!(receipts(dir.path()), "full\nother\nfull\nother\n");
    Ok(())
}

#[test]
fn failure_timeout_and_policy_change_cannot_pass() -> TestResult {
    for (script, timeout, expected) in [
        ("raise SystemExit(7)", 5, 1),
        ("import time; time.sleep(3)", 1, 2),
        (
            "from pathlib import Path; Path('.opdev/project.yaml').write_text('schema: 99')",
            5,
            2,
        ),
    ] {
        let dir = fixture()?;
        let path = dir.path().join(".opdev/project.yaml");
        let mut contract: Value = serde_json::from_slice(&fs::read(&path)?)?;
        contract["commands"]["focused"]["argv"][2] = json!(script);
        contract["commands"]["focused"]["timeout_seconds"] = json!(timeout);
        fs::write(path, serde_json::to_vec(&contract)?)?;
        let output = run(
            dir.path(),
            &[
                "--purpose",
                "feedback",
                "--command",
                "focused",
                "--command",
                "other",
                "--reason",
                "test bounded failure behavior",
                "--fail-fast",
                "--format",
                "json",
            ],
        )?;
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(report["checks"][1]["status"], "not_run");
        assert_eq!(report["integration_status"], "unverified");
        assert_eq!(receipts(dir.path()), "");
    }
    Ok(())
}

#[test]
fn report_collision_and_invalid_cwd_fail_before_any_execution() -> TestResult {
    let dir = fixture()?;
    let outside = tempfile::tempdir()?;
    let report = outside.path().join("report.json");
    fs::write(&report, "preserve")?;
    let output = run(
        dir.path(),
        &[
            "--purpose",
            "feedback",
            "--command",
            "focused",
            "--report",
            report.to_str().ok_or("report path")?,
        ],
    )?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(report)?, "preserve");
    let path = dir.path().join(".opdev/project.yaml");
    let mut contract: Value = serde_json::from_slice(&fs::read(&path)?)?;
    contract["commands"]["other"]["working_directory"] = json!("missing-dir");
    fs::write(path, serde_json::to_vec(&contract)?)?;
    let output = run(
        dir.path(),
        &[
            "--purpose",
            "feedback",
            "--command",
            "focused",
            "--command",
            "other",
            "--reason",
            "test",
        ],
    )?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(receipts(dir.path()), "");
    Ok(())
}

#[test]
fn integration_alias_needs_reason_but_can_be_inspected_without_execution() -> TestResult {
    let dir = fixture()?;
    let path = dir.path().join(".opdev/project.yaml");
    let mut contract: Value = serde_json::from_slice(&fs::read(&path)?)?;
    contract["commands"]["alias"] = contract["commands"]["full"].clone();
    fs::write(path, serde_json::to_vec(&contract)?)?;
    let args = ["--purpose", "feedback", "--command", "alias"];
    assert_eq!(run(dir.path(), &args)?.status.code(), Some(2));
    let mut plan_args = args.to_vec();
    plan_args.push("--plan");
    let output = run(dir.path(), &plan_args)?;
    assert!(output.status.success());
    let plan: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(plan["commands"][0]["required_at_integration"], true);
    assert_eq!(receipts(dir.path()), "");
    Ok(())
}

#[test]
fn legacy_preparation_runs_only_named_command_without_legacy_gate_claims() -> TestResult {
    let dir = fixture()?;
    let path = dir.path().join(".opdev/project.yaml");
    let contract: Value = serde_json::from_slice(&fs::read(&path)?)?;
    let mut legacy: opdev_project::ProjectManifest =
        opdev_project::ProjectManifest::from_yaml(include_str!("../../../.opdev/project.yaml"))?;
    legacy.commands.insert(
        "focused".into(),
        serde_json::from_value(contract["commands"]["focused"].clone())?,
    );
    fs::write(path, serde_json::to_vec(&legacy)?)?;
    let output = run(
        dir.path(),
        &[
            "--purpose",
            "feedback",
            "--command",
            "focused",
            "--format",
            "json",
        ],
    )?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(receipts(dir.path()), "focused\n");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["kind"], "preparation_check");
    assert!(report.get("gates").is_none());
    Ok(())
}
