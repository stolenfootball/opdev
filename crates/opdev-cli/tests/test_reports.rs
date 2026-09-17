//! Consumer acceptance for read-only `JUnit` observations, not gate qualification.

use std::fs;
use std::process::Command;

use serde_json::Value;

#[test]
fn inspection_exposes_outcomes_without_writing_or_qualifying()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("report.xml");
    for (xml, expected, exit) in [
        (include_str!("fixtures/junit/clean.xml"), "passed", 0),
        (
            "<testsuite><testcase name='bad'><failure>PRIVATE_FAILURE_TEXT</failure></testcase></testsuite>",
            "failed",
            1,
        ),
        (
            "<testsuite><testcase name='skip'><skipped/></testcase></testsuite>",
            "unverified",
            1,
        ),
        ("<testsuite/>", "unverified", 1),
        (
            "<testsuite><testcase name='retry'><flakyFailure/></testcase></testsuite>",
            "unverified",
            1,
        ),
    ] {
        fs::write(&path, xml)?;
        let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
            .args(["test-report", "inspect"])
            .arg(&path)
            .args(["--format", "json"])
            .current_dir(directory.path())
            .output()?;
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(report["outcome"], expected);
        assert_eq!(report["qualification"], "unverified");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_FAILURE_TEXT"));
        assert_eq!(fs::read_to_string(&path)?, xml);
        assert_eq!(fs::read_dir(directory.path())?.count(), 1);
    }
    let human = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["test-report", "inspect"])
        .arg(&path)
        .output()?;
    let text = String::from_utf8(human.stdout)?;
    assert!(text.contains("Report inspection only"));
    assert!(text.contains("remain unverified"));
    assert!(!text.contains('\u{1b}'));
    Ok(())
}

#[test]
fn invalid_missing_and_oversized_inputs_have_verifier_error_exit()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("report.xml");
    for input in [
        None,
        Some("<testsuite>"),
        Some("<other/>"),
        Some(
            "<!DOCTYPE testsuite [<!ENTITY PRIVATE_NAME 'secret'>]><testsuite>&PRIVATE_NAME;</testsuite>",
        ),
    ] {
        if let Some(xml) = input {
            fs::write(&path, xml)?;
        }
        let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
            .args(["test-report", "inspect"])
            .arg(&path)
            .args(["--format", "json"])
            .output()?;
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_NAME"));
    }
    let file = fs::File::create(&path)?;
    file.set_len(8 * 1024 * 1024 + 1)?;
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["test-report", "inspect"])
        .arg(&path)
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("8 MiB"));
    Ok(())
}
