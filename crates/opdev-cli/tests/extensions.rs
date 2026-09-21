//! Additive extension execution and the optional producer-independent fixtures.

use opdev_project::{CommandSpec, ExtensionCheck, ExtensionStage, MANIFEST_PATH, discover};
use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn python() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

fn check(root: &Path, ci: bool) -> Result<Value, Box<dyn std::error::Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opdev"));
    command
        .args(["check", "--format", "json", "--root"])
        .arg(root);
    if ci {
        command.arg("--ci");
    }
    let output = command.output()?;
    assert_eq!(output.status.code(), Some(1)); // Unconfigured core rules remain blocked.
    Ok(serde_json::from_slice(&output.stdout)?)
}

const SCRIPT: &str = r"import json, sys, time
request = json.load(sys.stdin)
assert request['stage'] in ('verify', 'pre_merge') and request['check_id'] == 'strength'
mode = sys.argv[1]
if mode == 'crash': sys.exit(7)
if mode == 'timeout': time.sleep(10)
if mode == 'malformed': print('not json'); sys.exit(0)
if mode == 'oversized': print('a' * 100000); sys.exit(0)
print(json.dumps({'protocol_version': '9.0.0' if mode == 'version' else '1.0.0',
 'outcome': mode if mode in ('passed', 'failed', 'unverified', 'error') else 'passed',
 'summary': ' ' if mode == 'empty' else 'Selected test-strength finding',
 'evidence': [{'kind': 'fixture', 'summary': 'Retained synthetic observation', 'location': 'result.json'}]}))
";

fn assert_core_outcomes_unchanged(report: &Value, baseline: &Value) {
    for id in ["OPDEV-TEST-002", "OPDEV-TEST-003", "MCD-TRUNK-001"] {
        let find = |value: &Value| {
            value["rules"]
                .as_array()
                .and_then(|rules| rules.iter().find(|r| r["rule_id"] == id))
                .map(|r| r["outcome"].clone())
        };
        assert_eq!(
            find(report),
            find(baseline),
            "extension must not qualify {id}"
        );
    }
}

#[test]
fn optional_test_strength_adapter_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(python())
        .arg(root.join("tests/test_strength_test.py"))
        .output()?;
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn extensions_preserve_findings_errors_stages_and_core_verdicts()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(root.path())
            .status()?
            .success()
    );
    let base = discover(root.path())?.manifest;
    base.write_new(&root.path().join(MANIFEST_PATH))?;
    let baseline = check(root.path(), false)?;
    assert_eq!(baseline["checks"], serde_json::json!([]));
    for (mode, expected) in [
        ("passed", "passed"),
        ("failed", "failed"),
        ("unverified", "unverified"),
        ("error", "error"),
        ("crash", "error"),
        ("malformed", "error"),
        ("version", "error"),
        ("empty", "error"),
        ("timeout", "error"),
        ("oversized", "error"),
        ("missing", "error"),
    ] {
        for blocking in [false, true] {
            let mut manifest = base.clone();
            manifest.commands.insert(
                "strength".into(),
                CommandSpec {
                    argv: vec![
                        if mode == "missing" {
                            "opdev-missing-producer-48"
                        } else {
                            python()
                        }
                        .into(),
                        "-c".into(),
                        SCRIPT.into(),
                        mode.into(),
                    ],
                    working_directory: None,
                    timeout_seconds: Some(if mode == "timeout" { 1 } else { 30 }),
                },
            );
            manifest.extensions.checks.push(ExtensionCheck {
                id: "strength".into(),
                stage: ExtensionStage::Verify,
                command: "strength".into(),
                blocking,
                authority: None,
                timeout_seconds: None,
            });
            fs::write(root.path().join(MANIFEST_PATH), manifest.to_yaml()?)?;
            let report = check(root.path(), false)?;
            assert_eq!(report["checks"][0]["outcome"], expected, "{mode}");
            assert_eq!(report["checks"][0]["blocking"], blocking);
            assert_eq!(report["checks"][0]["kind"], "extension");
            let gate = report["gates"]
                .as_array()
                .ok_or("gates")?
                .iter()
                .find(|g| g["gate"] == "development")
                .ok_or("development")?;
            assert_eq!(
                gate["blocking_checks"],
                if blocking && expected != "passed" {
                    serde_json::json!(["strength"])
                } else {
                    serde_json::json!([])
                }
            );
            assert_core_outcomes_unchanged(&report, &baseline);
            if matches!(mode, "passed" | "failed" | "unverified" | "error") {
                assert_eq!(
                    report["checks"][0]["evidence"][0]["location"],
                    "result.json"
                );
            }
            if mode == "passed" {
                assert_eq!(
                    check(root.path(), true)?["checks"],
                    serde_json::json!([]),
                    "verify extension must not run in pre_merge"
                );
                manifest.extensions.checks[0].stage = ExtensionStage::PreMerge;
                fs::write(root.path().join(MANIFEST_PATH), manifest.to_yaml()?)?;
                assert_eq!(check(root.path(), true)?["checks"][0]["outcome"], "passed");
                assert_eq!(check(root.path(), false)?["checks"], serde_json::json!([]));
            }
        }
    }
    Ok(())
}
