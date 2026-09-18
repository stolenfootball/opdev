//! Canonical execution receipts preserve failures and reject stale report reuse.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use opdev_project::{CommandSpec, MANIFEST_PATH, TestStage, TestSuite, discover};
use serde_json::Value;

const WRITER: &str = r#"
import pathlib, sys, time
mode = sys.argv[1]
target = pathlib.Path('target')
target.mkdir(exist_ok=True)
with (target / 'executed').open('a') as marker: marker.write('yes\n')
if mode == 'timeout':
    time.sleep(10)
elif mode != 'missing':
    xml = "<testsuite><testcase name='ok'/></testsuite>"
    if mode == 'failure': xml = "<testsuite><testcase name='bad'><failure/></testcase></testsuite>"
    if mode in ('malformed', 'nonzero-malformed'): xml = '<testsuite>'
    if mode == 'empty': xml = '<testsuite/>'
    if mode == 'skipped': xml = "<testsuite><testcase name='skip'><skipped/></testcase></testsuite>"
    if mode == 'retry': xml = "<testsuite><testcase name='flaky'><flakyFailure/></testcase></testsuite>"
    (target / 'junit.xml').write_text(xml)
if mode == 'mutate':
    pathlib.Path('source.txt').write_text('changed')
sys.exit(9 if mode in ('nonzero', 'nonzero-malformed') else 0)
"#;

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()?
            .status
            .success()
    );
    Ok(())
}

fn project(mode: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    git(root, &["init", "--quiet"])?;
    git(root, &["config", "user.name", "OpDev Test"])?;
    git(root, &["config", "user.email", "test@example.invalid"])?;
    fs::write(root.join("writer.py"), WRITER)?;
    fs::write(root.join("source.txt"), "original")?;
    fs::write(root.join(".gitignore"), "target/\n")?;
    if mode == "cwd" {
        fs::create_dir(root.join("nested"))?;
        fs::write(root.join("nested/writer.py"), WRITER)?;
    }
    let mut manifest = discover(root)?.manifest;
    manifest.commands.insert(
        "verify".into(),
        CommandSpec {
            argv: vec![
                if mode == "spawn" {
                    "opdev-no-such-program-44"
                } else if cfg!(windows) {
                    "python"
                } else {
                    "python3"
                }
                .into(),
                "writer.py".into(),
                mode.into(),
            ],
            working_directory: (mode == "cwd").then(|| "nested".into()),
            timeout_seconds: Some(if mode == "timeout" { 1 } else { 30 }),
        },
    );
    manifest.testing.suites = vec![TestSuite {
        id: "tests".into(),
        command: "verify".into(),
        stages: vec![TestStage::Local],
    }];
    manifest.write_new(&root.join(MANIFEST_PATH))?;
    git(root, &["add", "."])?;
    git(root, &["commit", "--quiet", "-m", "fixture"])?;
    Ok(temp)
}

fn run(root: &Path) -> Result<Output, std::io::Error> {
    run_with_report(root, "target/junit.xml")
}

fn run_with_report(root: &Path, report: &str) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args([
            "test-report",
            "run",
            "--suite",
            "tests",
            "--junit",
            report,
            "--root",
        ])
        .arg(root)
        .output()
}

#[test]
fn command_and_report_outcomes_are_reconciled_without_hiding_failures()
-> Result<(), Box<dyn std::error::Error>> {
    for (mode, command_outcome, report_outcome, combined, exit) in [
        ("pass", "passed", "passed", "passed", 0),
        ("nonzero", "failed", "passed", "failed", 1),
        ("nonzero-malformed", "failed", "error", "failed", 1),
        ("cwd", "passed", "passed", "passed", 0),
        ("failure", "passed", "failed", "failed", 1),
        ("missing", "passed", "unverified", "unverified", 1),
        ("empty", "passed", "unverified", "unverified", 1),
        ("malformed", "passed", "error", "error", 2),
        ("mutate", "passed", "passed", "unverified", 1),
        ("timeout", "error", "unverified", "error", 2),
        ("spawn", "error", "unverified", "error", 2),
    ] {
        let project = project(mode)?;
        let output = if mode == "cwd" {
            run_with_report(project.path(), "nested/target/junit.xml")?
        } else {
            run(project.path())?
        };
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout)?;
        let mut schema: Value = serde_json::from_str(include_str!(
            "../../../schema/test-execution-receipt.schema.json"
        ))?;
        let inspection: Value = serde_json::from_str(include_str!(
            "../../../schema/test-report-inspection.schema.json"
        ))?;
        // Resolve the checked-in schema locally; validation never fetches a URL.
        assert_eq!(
            schema["properties"]["report"]["anyOf"][1]["$ref"],
            inspection["$id"]
        );
        schema["properties"]["report"]["anyOf"][1] = inspection;
        assert!(jsonschema::is_valid(&schema, &value));
        assert_eq!(value["command_outcome"], command_outcome, "{mode}");
        assert_eq!(value["report_outcome"], report_outcome, "{mode}");
        assert_eq!(value["outcome"], combined, "{mode}");
        assert_eq!(value["qualification"], "unverified");
        assert_eq!(value["source_unchanged"], mode != "mutate");
        assert_eq!(value["command_sha256"].as_str().map(str::len), Some(64));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("writer.py"));
    }
    Ok(())
}

#[test]
fn stale_report_or_dirty_source_prevents_execution() -> Result<(), Box<dyn std::error::Error>> {
    for stale in [true, false] {
        let project = project("pass")?;
        if stale {
            fs::create_dir(project.path().join("target"))?;
            fs::write(project.path().join("target/junit.xml"), "old report")?;
        } else {
            fs::write(project.path().join("source.txt"), "dirty")?;
        }
        let output = run(project.path())?;
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!project.path().join("target/executed").exists());
        if stale {
            assert_eq!(
                fs::read_to_string(project.path().join("target/junit.xml"))?,
                "old report"
            );
        }
    }
    Ok(())
}

#[test]
fn stage_gates_and_contract_snapshot_are_enforced() -> Result<(), Box<dyn std::error::Error>> {
    for (stage, args, gate) in [
        (TestStage::PreMerge, vec!["--ci"], "integration"),
        (TestStage::Delivery, vec!["--ci", "--delivery"], "delivery"),
    ] {
        let project = project("pass")?;
        let root = project.path();
        let manifest_path = root.join(MANIFEST_PATH);
        let mut manifest = opdev_project::ProjectManifest::load(&manifest_path)?;
        manifest.testing.suites[0].stages = vec![stage];
        fs::write(&manifest_path, manifest.to_yaml()?)?;
        git(root, &["add", "."])?;
        git(root, &["commit", "--quiet", "-m", "stage selection"])?;
        let mut args = args;
        args.extend(["--junit", "tests=target/junit.xml"]);
        let output = check(root, &args)?;
        assert_eq!(output.status.code(), Some(1));
        let value: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(value["checks"][0]["gates"], serde_json::json!([gate]));
        assert_eq!(value["checks"][0]["outcome"], "passed");
    }
    let project = project("pass")?;
    let root = project.path();
    let mut stale_manifest = opdev_project::ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    stale_manifest
        .commands
        .get_mut("verify")
        .ok_or("missing command")?
        .timeout_seconds = Some(31);
    let report = opdev_engine::evaluate_with_junit(
        root,
        &stale_manifest,
        opdev_engine::CheckOptions::local(),
        &[opdev_engine::JunitBinding {
            suite: "tests".into(),
            path: "target/junit.xml".into(),
        }],
    )?;
    assert_eq!(report.checks[0].outcome, opdev_core::Outcome::Error);
    assert!(!root.join("target/executed").exists());
    Ok(())
}

fn check(root: &Path, arguments: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["check", "--format", "json", "--root"])
        .arg(root)
        .args(arguments)
        .output()
}

#[test]
fn lightweight_evidence_can_pass_a_gate_but_never_overrides_required_extensions()
-> Result<(), Box<dyn std::error::Error>> {
    use opdev_core::{Evidence, Outcome, VerificationMethod, embedded_catalog};
    use opdev_project::{
        ChangeEvidence, EVIDENCE_PATH, EvidenceAssertion, EvidenceLedger, ExtensionCheck,
        ExtensionStage, staged_fingerprint,
    };
    for extension in [None, Some("unverified"), Some("failed"), Some("error")] {
        let project = project("pass")?;
        let root = project.path();
        if let Some(outcome) = extension {
            let path = root.join(MANIFEST_PATH);
            let mut manifest = opdev_project::ProjectManifest::load(&path)?;
            let response = serde_json::json!({
                "protocol_version": opdev_core::EXTENSION_PROTOCOL_VERSION,
                "outcome": outcome, "summary": "Synthetic required complete-history check", "evidence": []
            });
            manifest.commands.insert(
                "history".into(),
                CommandSpec {
                    argv: vec![
                        manifest.commands["verify"].argv[0].clone(),
                        "-c".into(),
                        format!("print({:?})", response.to_string()),
                    ],
                    working_directory: None,
                    timeout_seconds: Some(30),
                },
            );
            manifest.extensions.checks.push(ExtensionCheck {
                id: "complete-history".into(),
                stage: ExtensionStage::Verify,
                command: "history".into(),
                blocking: true,
                authority: None,
                timeout_seconds: None,
            });
            fs::write(path, manifest.to_yaml()?)?;
        }
        git(root, &["add", "."])?;
        let ledger = EvidenceLedger {
            schema: 1,
            project: vec![],
            changes: vec![ChangeEvidence {
                fingerprint: staged_fingerprint(root)?,
                work: "synthetic gate fixture".into(),
                assertions: embedded_catalog()?
                    .rules
                    .iter()
                    .filter(|rule| {
                        rule.verification.contains(&VerificationMethod::Evidence)
                            || rule.verification.contains(&VerificationMethod::Agent)
                    })
                    .map(|rule| EvidenceAssertion {
                        rule_id: rule.id.clone(),
                        outcome: Outcome::Passed,
                        summary: "Synthetic unrelated gate prerequisites".into(),
                        evidence: vec![Evidence {
                            kind: "fixture".into(),
                            summary: "Synthetic fixture evidence, not a real project attestation"
                                .into(),
                            location: None,
                        }],
                    })
                    .collect(),
            }],
        };
        fs::write(root.join(EVIDENCE_PATH), ledger.to_yaml()?)?;
        git(root, &["add", "."])?;
        git(
            root,
            &["commit", "--quiet", "-m", "synthetic gate prerequisites"],
        )?;
        let output = check(root, &["--junit", "tests=target/junit.xml"])?;
        let value: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(value["checks"][0]["outcome"], "passed");
        let gate = value["gates"]
            .as_array()
            .ok_or("missing gates")?
            .iter()
            .find(|gate| gate["gate"] == "development")
            .ok_or("development gate")?;
        assert_eq!(
            gate["verdict"],
            if extension.is_none() {
                "passed"
            } else {
                "blocked"
            },
            "{value}"
        );
        assert_eq!(output.status.code(), Some(i32::from(extension.is_some())));
        if let Some(outcome) = extension {
            assert_eq!(value["checks"][1]["outcome"], outcome);
            assert_eq!(
                gate["blocking_checks"],
                serde_json::json!(["complete-history"])
            );
        }
    }
    Ok(())
}

#[test]
fn check_ingests_once_and_blocks_incomplete_or_failed_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    for (mode, expected, observations) in [
        ("pass", "passed", "passed"),
        ("nonzero", "failed", "failed"),
        ("nonzero-malformed", "failed", "failed"),
        ("failure", "failed", "failed"),
        ("missing", "unverified", "unverified"),
        ("empty", "unverified", "unverified"),
        ("malformed", "error", "error"),
        ("mutate", "unverified", "unverified"),
        ("skipped", "unverified", "unverified"),
        ("retry", "unverified", "unverified"),
        ("timeout", "error", "error"),
        ("spawn", "error", "error"),
    ] {
        let project = project(mode)?;
        let output = check(project.path(), &["--junit", "tests=target/junit.xml"])?;
        assert_eq!(
            output.status.code(),
            Some(1),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout)?;
        let report_schema: Value =
            serde_json::from_str(include_str!("../../../schema/report.schema.json"))?;
        assert!(jsonschema::is_valid(&report_schema, &value), "{mode}");
        let checks = value["checks"].as_array().ok_or("missing checks")?;
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0]["outcome"], expected, "{mode}");
        assert_eq!(checks[0]["blocking"], true);
        let flake_rule = value["rules"]
            .as_array()
            .ok_or("missing rules")?
            .iter()
            .find(|rule| rule["rule_id"] == "OPDEV-TEST-005")
            .ok_or("missing flake rule")?;
        assert_eq!(
            flake_rule["outcome"],
            if expected == "passed" {
                "passed"
            } else {
                "unverified"
            }
        );
        assert!(
            checks[0]["summary"]
                .as_str()
                .ok_or("missing summary")?
                .contains("not established")
        );
        assert!(checks[0]["stdout"].is_null());
        let evidence = checks[0]["evidence"].as_array().ok_or("missing evidence")?;
        assert_eq!(evidence[0]["kind"], "test_execution_attempt");
        assert_eq!(evidence[1]["kind"], "test_execution_receipt_v1");
        let receipt: Value =
            serde_json::from_str(evidence[1]["summary"].as_str().ok_or("missing receipt")?)?;
        assert_eq!(receipt["outcome"], observations);
        assert_eq!(receipt["qualification"], "unverified");
        for gate in value["gates"].as_array().ok_or("missing gates")? {
            if checks[0]["gates"]
                .as_array()
                .ok_or("missing check gates")?
                .contains(&gate["gate"])
            {
                assert_eq!(gate["verdict"], "blocked", "{mode}: {gate}");
            }
        }
        if mode != "spawn" {
            assert_eq!(
                fs::read_to_string(project.path().join("target/executed"))?
                    .lines()
                    .collect::<Vec<_>>(),
                vec!["yes"],
                "command must run exactly once"
            );
        }
    }
    Ok(())
}

#[test]
fn binding_validation_and_no_exec_cannot_drop_required_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    for args in [
        vec!["--junit", "missing=target/junit.xml"],
        vec!["--junit", "tests="],
        vec!["--junit", "=target/junit.xml"],
        vec!["--junit", "tests"],
        vec![
            "--junit",
            "tests=target/junit.xml",
            "--junit",
            "tests=target/other.xml",
        ],
        vec!["--ci", "--junit", "tests=target/junit.xml"],
    ] {
        let project = project("pass")?;
        let output = check(project.path(), &args)?;
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!project.path().join("target/executed").exists());
    }
    let project = project("pass")?;
    let output = check(
        project.path(),
        &["--no-exec", "--junit", "tests=target/junit.xml"],
    )?;
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(value["checks"][0]["outcome"], "unverified");
    assert!(!project.path().join("target/executed").exists());
    Ok(())
}

#[test]
fn check_refuses_stale_or_dirty_execution_and_preserves_default_behavior()
-> Result<(), Box<dyn std::error::Error>> {
    for stale in [true, false] {
        let project = project("pass")?;
        if stale {
            fs::create_dir(project.path().join("target"))?;
            fs::write(project.path().join("target/junit.xml"), "old report")?;
        } else {
            fs::write(project.path().join("source.txt"), "dirty")?;
        }
        let output = check(project.path(), &["--junit", "tests=target/junit.xml"])?;
        let value: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(value["checks"][0]["outcome"], "error");
        assert!(!project.path().join("target/executed").exists());
        if stale {
            assert_eq!(
                fs::read_to_string(project.path().join("target/junit.xml"))?,
                "old report"
            );
        }
    }
    let project = project("pass")?;
    let output = check(project.path(), &[])?;
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["checks"][0]["outcome"], "passed");
    assert_eq!(value["checks"][0]["evidence"][0]["kind"], "command");
    Ok(())
}

#[test]
fn repeated_bindings_retain_independent_attempts_and_exact_report_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let project = project("pass")?;
    let root = project.path();
    fs::create_dir(root.join("nested"))?;
    fs::write(root.join("nested/writer.py"), WRITER)?;
    let manifest_path = root.join(MANIFEST_PATH);
    let mut manifest = opdev_project::ProjectManifest::load(&manifest_path)?;
    let mut nested = manifest.commands["verify"].clone();
    nested.working_directory = Some("nested".into());
    manifest.commands.insert("nested".into(), nested);
    manifest.testing.suites.push(TestSuite {
        id: "nested".into(),
        command: "nested".into(),
        stages: vec![TestStage::Local],
    });
    // Fixtures deliberately rewrite their own temp manifest, never user state.
    fs::write(&manifest_path, manifest.to_yaml()?)?;
    git(root, &["add", "."])?;
    git(root, &["commit", "--quiet", "-m", "second suite"])?;
    fs::create_dir(root.join("target"))?;
    let output = check(
        root,
        &[
            "--junit",
            "tests=target/junit.xml",
            "--junit",
            "nested=nested/target/junit.xml",
            "--report",
            root.join("target/check.json")
                .to_str()
                .ok_or("non-UTF8 fixture path")?,
        ],
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read(root.join("target/check.json"))?, output.stdout);
    let value: Value = serde_json::from_slice(&output.stdout)?;
    let checks = value["checks"].as_array().ok_or("missing checks")?;
    assert_eq!(checks.len(), 2);
    assert_ne!(
        checks[0]["evidence"][0]["summary"],
        checks[1]["evidence"][0]["summary"]
    );
    for (index, directory) in ["target", "nested/target"].iter().enumerate() {
        assert_eq!(checks[index]["outcome"], "passed");
        assert_eq!(
            fs::read_to_string(root.join(directory).join("executed"))?
                .lines()
                .count(),
            1
        );
    }
    Ok(())
}
