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
(target / 'executed').write_text('yes')
if mode == 'timeout':
    time.sleep(10)
elif mode != 'missing':
    xml = "<testsuite><testcase name='ok'/></testsuite>"
    if mode == 'failure': xml = "<testsuite><testcase name='bad'><failure/></testcase></testsuite>"
    if mode in ('malformed', 'nonzero-malformed'): xml = '<testsuite>'
    if mode == 'empty': xml = '<testsuite/>'
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
