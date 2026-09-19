//! Tool-neutral execution receipts preserve command outcomes and source identity.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use opdev_project::{CommandSpec, MANIFEST_PATH, TestStage, TestSuite, discover};
use serde_json::Value;
use sha2::Digest;

const WRITER: &str = r#"
import json, pathlib, subprocess, sys, time
mode = sys.argv[1]
target = pathlib.Path('target')
target.mkdir(exist_ok=True)
(target / 'argv.json').write_text(json.dumps(sys.argv[2:]))
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
if mode in ('mutate', 'mutate-nonzero', 'mutate-commit'):
    pathlib.Path('source.txt').write_text('changed')
if mode == 'mutate-commit':
    subprocess.run(['git', 'add', 'source.txt'], check=True)
    subprocess.run(['git', 'commit', '--quiet', '-m', 'fixture source change'], check=True)
sys.exit(9 if mode in ('nonzero', 'nonzero-malformed', 'mutate-nonzero') else 0)
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
                "argument with spaces".into(),
                "$literal; not a shell".into(),
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
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["test-execution", "--suite", "tests", "--root"])
        .arg(root)
        .output()
}

fn check(root: &Path, arguments: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["check", "--format", "json", "--root"])
        .arg(root)
        .args(arguments)
        .output()
}

#[test]
fn receipts_are_report_independent_and_preserve_command_and_source_outcomes()
-> Result<(), Box<dyn std::error::Error>> {
    for (mode, command_outcome, combined, exit) in [
        ("missing", "passed", "passed", 0),
        ("pass", "passed", "passed", 0),
        ("nonzero", "failed", "failed", 1),
        ("nonzero-malformed", "failed", "failed", 1),
        ("cwd", "passed", "passed", 0),
        // These files are intentionally never read by the execution observer.
        ("failure", "passed", "passed", 0),
        ("empty", "passed", "passed", 0),
        ("malformed", "passed", "passed", 0),
        ("skipped", "passed", "passed", 0),
        ("retry", "passed", "passed", 0),
        ("mutate", "passed", "unverified", 1),
        ("mutate-commit", "passed", "unverified", 1),
        ("mutate-nonzero", "failed", "failed", 1),
        ("timeout", "error", "error", 2),
        ("spawn", "error", "error", 2),
    ] {
        let project = project(mode)?;
        let output = run(project.path())?;
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout)?;
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schema/test-execution-receipt.schema.json"
        ))?;
        assert!(jsonschema::is_valid(&schema, &value), "{mode}: {value}");
        assert_eq!(value["schema"], 2);
        assert_eq!(value["command_outcome"], command_outcome, "{mode}");
        assert_eq!(value["outcome"], combined, "{mode}");
        assert_eq!(value["qualification"], "unverified");
        assert_eq!(value["source_unchanged"], !mode.starts_with("mutate"));
        assert_eq!(value["command_sha256"].as_str().map(str::len), Some(64));
        let manifest = opdev_project::ProjectManifest::load(&project.path().join(MANIFEST_PATH))?;
        assert_eq!(
            value["command_sha256"],
            format!(
                "{:x}",
                sha2::Sha256::digest(serde_json::to_vec(&manifest.commands["verify"])?)
            )
        );
        if mode == "mutate-commit" {
            assert!(value["revision_after"].is_string());
            assert_ne!(value["revision_before"], value["revision_after"]);
        }
        assert_eq!(value["suite"], "tests");
        assert_eq!(value["command_key"], "verify");
        assert!(value.get("report").is_none());
        assert!(value.get("report_outcome").is_none());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("writer.py"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("$literal"));
        assert!(!project.path().join(".opdev/evidence.yaml").exists());
        if mode != "spawn" {
            let directory = if mode == "cwd" {
                "nested/target"
            } else {
                "target"
            };
            assert_eq!(
                fs::read_to_string(project.path().join(directory).join("executed"))?
                    .lines()
                    .count(),
                1
            );
            let argv: Value = serde_json::from_slice(&fs::read(
                project.path().join(directory).join("argv.json"),
            )?)?;
            assert_eq!(
                argv,
                serde_json::json!(["argument with spaces", "$literal; not a shell"])
            );
        }
        let mut old_version = value.clone();
        old_version["schema"] = 1.into();
        assert!(!jsonschema::is_valid(&schema, &old_version));
        let mut old_report = value;
        old_report["report"] = Value::Null;
        assert!(!jsonschema::is_valid(&schema, &old_report));
    }
    Ok(())
}

#[test]
fn dirty_staged_untracked_or_uncommitted_source_prevents_execution()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in ["dirty", "staged", "untracked", "uncommitted"] {
        let project = project("missing")?;
        let root = project.path();
        match mode {
            "dirty" | "staged" => {
                fs::write(root.join("source.txt"), "dirty")?;
                if mode == "staged" {
                    git(root, &["add", "."])?;
                }
            }
            "untracked" => fs::write(root.join("new.txt"), "untracked")?,
            // Only a disposable test fixture's ref is removed.
            _ => git(root, &["update-ref", "-d", "HEAD"])?,
        }
        let output = run(root)?;
        assert_eq!(output.status.code(), Some(2), "{mode}");
        assert!(output.stdout.is_empty());
        assert!(!root.join("target/executed").exists());
    }
    Ok(())
}

#[test]
fn repeated_attempts_need_no_report_path_and_preserve_existing_files()
-> Result<(), Box<dyn std::error::Error>> {
    let project = project("missing")?;
    let root = project.path();
    fs::create_dir(root.join("target"))?;
    fs::write(
        root.join("target/junit.xml"),
        "existing report is project-owned",
    )?;
    // The observer must not modify an existing ledger either.
    fs::write(
        root.join(".opdev/evidence.yaml"),
        "schema: 1\nproject: []\nchanges: []\n",
    )?;
    git(root, &["add", "."])?;
    git(root, &["commit", "--quiet", "-m", "ledger fixture"])?;
    let ledger = fs::read(root.join(".opdev/evidence.yaml"))?;
    let first = run(root)?;
    let second = run(root)?;
    assert!(first.status.success() && second.status.success());
    let first: Value = serde_json::from_slice(&first.stdout)?;
    let second: Value = serde_json::from_slice(&second.stdout)?;
    assert_ne!(first["attempt_id"], second["attempt_id"]);
    assert_eq!(first["revision_before"], first["revision_after"]);
    assert_eq!(first["revision_before"], second["revision_before"]);
    assert_eq!(first["command_sha256"], second["command_sha256"]);
    assert_eq!(
        fs::read_to_string(root.join("target/executed"))?
            .lines()
            .count(),
        2
    );
    assert_eq!(
        fs::read_to_string(root.join("target/junit.xml"))?,
        "existing report is project-owned"
    );
    assert_eq!(fs::read(root.join(".opdev/evidence.yaml"))?, ledger);
    Ok(())
}

#[test]
fn unknown_suite_and_removed_preview_interfaces_fail_before_execution()
-> Result<(), Box<dyn std::error::Error>> {
    for args in [
        vec!["test-execution", "--suite", "unknown"],
        vec![
            "test-execution",
            "--suite",
            "tests",
            "--junit",
            "target/junit.xml",
        ],
        vec!["check", "--junit", "tests=target/junit.xml"],
        vec![
            "test-report",
            "run",
            "--suite",
            "tests",
            "--junit",
            "target/junit.xml",
        ],
    ] {
        let project = project("missing")?;
        let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
            .args(args)
            .arg("--root")
            .arg(project.path())
            .output()?;
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!project.path().join("target/executed").exists());
    }
    let help = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["test-execution", "--help"])
        .output()?;
    assert!(help.status.success());
    assert!(!help.stdout.contains(&0x1b));
    assert!(!String::from_utf8_lossy(&help.stdout).contains("--junit"));
    Ok(())
}

#[test]
fn ordinary_checks_remain_command_based_and_respect_stage_and_no_exec()
-> Result<(), Box<dyn std::error::Error>> {
    for (mode, expected) in [
        ("missing", "passed"),
        ("nonzero", "failed"),
        ("spawn", "error"),
    ] {
        let project = project(mode)?;
        let output = check(project.path(), &[])?;
        let value: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(value["checks"][0]["outcome"], expected);
        if mode != "spawn" {
            assert_eq!(
                fs::read_to_string(project.path().join("target/executed"))?
                    .lines()
                    .count(),
                1
            );
        }
    }
    for args in [vec!["--no-exec"], vec!["--ci"]] {
        let project = project("missing")?;
        let output = check(project.path(), &args)?;
        let value: Value = serde_json::from_slice(&output.stdout)?;
        assert!(value["checks"].as_array().ok_or("checks")?.is_empty());
        assert!(!project.path().join("target/executed").exists());
    }
    Ok(())
}

#[test]
fn ordinary_checks_preserve_required_extension_blockers() -> Result<(), Box<dyn std::error::Error>>
{
    use opdev_core::{Evidence, Outcome, VerificationMethod, embedded_catalog};
    use opdev_project::{
        ChangeEvidence, EVIDENCE_PATH, EvidenceAssertion, EvidenceLedger, ExtensionCheck,
        ExtensionStage, staged_fingerprint,
    };
    for extension in [None, Some("unverified"), Some("failed"), Some("error")] {
        let project = project("missing")?;
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
                acceptance: None,
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
        let output = check(root, &[])?;
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
