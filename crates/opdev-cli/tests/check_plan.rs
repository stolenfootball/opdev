//! Preview and execution agree without trusting a preview as qualification.
use opdev_project::{
    CommandSpec, ExtensionCheck, ExtensionStage, MANIFEST_PATH, TestStage, TestSuite, discover,
};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

const COUNTER: &str = r"import json, pathlib, sys
assert sys.argv[2] == 'literal $value; argument with spaces'
with pathlib.Path('executed').open('a') as f: f.write(sys.argv[1] + '\n')
if sys.argv[1] == 'extension':
    request = json.load(sys.stdin)
    assert request['check_id'] == 'review'
    print(json.dumps({'protocol_version':'1.0.0', 'outcome':'passed', 'summary':'counted extension'}))
sys.exit(7 if sys.argv[1] == 'failure' else 0)
";

fn invoke(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["check", "--root"])
        .arg(root)
        .args(args)
        .output()
}

fn fixture(
    extension_stage: ExtensionStage,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(root)
            .status()?
            .success()
    );
    fs::create_dir(root.join("work"))?;
    let mut manifest = discover(root)?.manifest;
    manifest.testing.suites.clear();
    for mode in ["suite", "failure", "extension"] {
        manifest.commands.insert(
            mode.into(),
            CommandSpec {
                argv: vec![
                    if cfg!(windows) { "python" } else { "python3" }.into(),
                    "-c".into(),
                    COUNTER.into(),
                    mode.into(),
                    "literal $value; argument with spaces".into(),
                ],
                working_directory: Some("work".into()),
                timeout_seconds: None,
            },
        );
    }
    for (id, selected, command) in [
        ("first", TestStage::Local, "suite"),
        ("second", TestStage::Local, "suite"),
        ("ci", TestStage::PreMerge, "failure"),
        ("delivery", TestStage::Delivery, "suite"),
        ("post", TestStage::PostMerge, "suite"),
    ] {
        manifest.testing.suites.push(TestSuite {
            id: id.into(),
            command: command.into(),
            stages: vec![selected],
        });
    }
    manifest.extensions.checks.push(ExtensionCheck {
        id: "review".into(),
        command: "extension".into(),
        stage: extension_stage,
        blocking: true,
        timeout_seconds: Some(12),
        authority: None,
    });
    manifest.write_new(&root.join(MANIFEST_PATH))?;
    fs::write(root.join(".opdev/evidence.yaml"), "malformed: [")?;
    Ok(temp)
}

#[test]
fn plan_is_read_only_and_execution_preserves_selection_and_distinct_invocations()
-> Result<(), Box<dyn std::error::Error>> {
    for (flags, stage, extension_stage, expected_ids) in [
        (
            vec![],
            TestStage::Local,
            ExtensionStage::Verify,
            vec!["first", "second", "review"],
        ),
        (
            vec!["--ci"],
            TestStage::PreMerge,
            ExtensionStage::PreMerge,
            vec!["ci", "review"],
        ),
        (
            vec!["--ci", "--delivery"],
            TestStage::Delivery,
            ExtensionStage::Deliver,
            vec!["delivery", "review"],
        ),
        (
            vec!["--ci", "--post-merge"],
            TestStage::PostMerge,
            ExtensionStage::PostMerge,
            vec!["post", "review"],
        ),
    ] {
        let temp = fixture(extension_stage)?;
        let root = temp.path();
        // Planning needs only a valid contract, not a manufactured acceptance record.
        let mut plan_args = flags.clone();
        plan_args.extend(["--plan", "--format", "json"]);
        let output = invoke(root, &plan_args)?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !root.join("work/executed").exists(),
            "planning executed code"
        );
        assert_eq!(
            fs::read_to_string(root.join(".opdev/evidence.yaml"))?,
            "malformed: ["
        );
        let plan: Value = serde_json::from_slice(&output.stdout)?;
        validate_plan_schema(&plan)?;
        assert_eq!(plan["kind"], "check_plan");
        assert_eq!(plan["qualification"], "unverified");
        assert!(plan.get("gates").is_none());
        assert_eq!(plan["test_stage"], serde_json::to_value(stage)?);
        assert_eq!(
            plan["extension_stage"],
            serde_json::to_value(extension_stage)?
        );
        assert_planned_commands(&plan, &expected_ids)?;
        fs::remove_file(root.join(".opdev/evidence.yaml"))?;
        let mut run_args = flags;
        run_args.extend(["--format", "json"]);
        let actual = invoke(root, &run_args)?;
        assert_eq!(
            actual.status.code(),
            Some(1),
            "missing core evidence must remain blocked"
        );
        let report: Value = serde_json::from_slice(&actual.stdout)?;
        let checks = report["checks"].as_array().ok_or("checks")?;
        assert_eq!(
            checks
                .iter()
                .filter_map(|c| c["id"].as_str())
                .collect::<Vec<_>>(),
            expected_ids
        );
        assert_eq!(
            fs::read_to_string(root.join("work/executed"))?
                .lines()
                .count(),
            expected_ids.len()
        );
        assert_eq!(
            checks[0]["outcome"],
            if stage == TestStage::PreMerge {
                "failed"
            } else {
                "passed"
            }
        );
        assert_eq!(checks.last().ok_or("extension")?["outcome"], "passed");
        if stage == TestStage::PostMerge {
            for check in checks {
                assert_eq!(
                    check["gates"],
                    serde_json::json!(["integration", "delivery"])
                );
            }
        }
    }
    Ok(())
}

#[test]
fn post_merge_process_failures_and_skipped_execution_remain_blocking()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in ["pass", "suite_failure", "extension_error", "no_exec"] {
        let temp = fixture(ExtensionStage::PostMerge)?;
        let root = temp.path();
        fs::remove_file(root.join(".opdev/evidence.yaml"))?;
        let path = root.join(MANIFEST_PATH);
        let mut manifest = opdev_project::ProjectManifest::load(&path)?;
        if mode == "suite_failure" {
            manifest
                .testing
                .suites
                .iter_mut()
                .find(|s| s.id == "post")
                .ok_or("post suite")?
                .command = "failure".into();
        }
        if mode == "extension_error" {
            manifest.extensions.checks[0].command = "failure".into();
        }
        fs::write(path, manifest.to_yaml()?)?;
        let mut args = vec!["--ci", "--post-merge", "--format", "json"];
        if mode == "no_exec" {
            args.push("--no-exec");
        }
        let actual = invoke(root, &args)?;
        let mut report: opdev_engine::CheckReport = serde_json::from_slice(&actual.stdout)?;
        assert_eq!(actual.status.code(), Some(1), "fixture lacks core evidence");
        assert_eq!(report.checks.len(), 2);
        assert_eq!(
            report.checks[0].outcome,
            match mode {
                "suite_failure" => opdev_core::Outcome::Failed,
                "no_exec" => opdev_core::Outcome::Unverified,
                _ => opdev_core::Outcome::Passed,
            }
        );
        assert_eq!(
            report.checks[1].outcome,
            match mode {
                "extension_error" => opdev_core::Outcome::Error,
                "no_exec" => opdev_core::Outcome::Unverified,
                _ => opdev_core::Outcome::Passed,
            }
        );
        assert_eq!(root.join("work/executed").exists(), mode != "no_exec");
        // Isolate actual command results from the fixture's deliberately missing core evidence.
        for rule in &mut report.rules {
            rule.outcome = opdev_core::Outcome::Passed;
        }
        opdev_engine::reaggregate(&mut report)?;
        for gate in [opdev_core::Gate::Integration, opdev_core::Gate::Delivery] {
            assert_eq!(report.gate_passed(gate), mode == "pass", "{mode}: {gate:?}");
        }
    }
    Ok(())
}

#[test]
fn evaluation_only_keeps_selected_checks_visible_at_every_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    use opdev_core::{Gate, Outcome};
    for (flags, extension_stage, gate, expected) in [
        (
            vec![],
            ExtensionStage::Verify,
            Gate::Development,
            vec!["first", "second", "review"],
        ),
        (
            vec!["--ci"],
            ExtensionStage::PreMerge,
            Gate::Integration,
            vec!["ci", "review"],
        ),
        (
            vec!["--ci", "--post-merge"],
            ExtensionStage::PostMerge,
            Gate::Integration,
            vec!["post", "review"],
        ),
    ] {
        let temp = fixture(extension_stage)?;
        let root = temp.path();
        fs::remove_file(root.join(".opdev/evidence.yaml"))?;
        let mut args = flags.clone();
        args.extend(["--no-exec", "--format", "json"]);
        let output = invoke(root, &args)?;
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut report: opdev_engine::CheckReport = serde_json::from_slice(&output.stdout)?;
        assert_eq!(
            report
                .checks
                .iter()
                .map(|c| c.id.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(
            report
                .checks
                .iter()
                .all(|c| c.outcome == Outcome::Unverified)
        );
        assert!(!root.join("work/executed").exists());
        // Even otherwise sufficient reviews cannot replace missing command execution.
        for rule in &mut report.rules {
            rule.outcome = Outcome::Passed;
        }
        opdev_engine::reaggregate(&mut report)?;
        assert!(!report.gate_passed(gate));
        let mut human_args = flags;
        human_args.push("--no-exec");
        let human = invoke(root, &human_args)?;
        let message = String::from_utf8(human.stdout)?;
        for id in expected {
            assert!(
                message.contains(&format!("check {id}: Unverified")),
                "{message}"
            );
        }
        assert!(!root.join("work/executed").exists());
    }
    Ok(())
}

#[test]
fn plan_rejects_misleading_execution_and_remote_options() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    for option in ["--remote", "--no-exec"] {
        assert_eq!(
            invoke(temp.path(), &["--plan", option])?.status.code(),
            Some(2)
        );
    }
    assert_eq!(
        invoke(
            temp.path(),
            &["--plan", "--report", "should-not-exist.json"]
        )?
        .status
        .code(),
        Some(2)
    );
    assert!(!temp.path().join("should-not-exist.json").exists());
    Ok(())
}

fn assert_planned_commands(
    plan: &Value,
    expected_ids: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let commands = plan["commands"].as_array().ok_or("commands")?;
    assert_eq!(
        commands
            .iter()
            .filter_map(|c| c["id"].as_str())
            .collect::<Vec<_>>(),
        expected_ids
    );
    assert_eq!(commands[0]["timeout_seconds"], 900);
    assert_eq!(commands.last().ok_or("extension")?["timeout_seconds"], 12);
    for command in commands {
        assert!(
            Path::new(command["working_directory"].as_str().ok_or("directory")?).ends_with("work")
        );
        assert_eq!(command["argv"][4], "literal $value; argument with spaces");
    }
    Ok(())
}

fn validate_plan_schema(plan: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schema/check-plan.schema.json"))?;
    assert!(jsonschema::is_valid(&schema, plan));
    Ok(())
}
