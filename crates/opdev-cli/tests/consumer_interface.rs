//! Process-level consumer interface checks; not a substitute for human review.

use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};

use opdev_project::{MANIFEST_PATH, discover};
use serde_json::Value;

fn cli(root: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(args)
        .stdin(Stdio::null())
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .output()
}

fn plain(bytes: &[u8]) -> Result<&str, std::str::Utf8Error> {
    let text = std::str::from_utf8(bytes)?;
    assert!(
        !text.contains('\u{1b}'),
        "redirected output contains ANSI escapes"
    );
    assert!(
        !text.contains('\u{8}'),
        "redirected output uses cursor backtracking"
    );
    Ok(text)
}

#[test]
fn help_and_errors_are_textual_without_interactive_input() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    for args in [
        vec!["--help"],
        vec!["check", "--help"],
        vec!["ci", "verify-run", "--help"],
        vec!["test-execution", "--help"],
        vec!["test-report", "inspect", "--help"],
        vec!["adoption", "approve", "--help"],
        vec!["upgrade", "--help"],
    ] {
        let output = cli(root.path(), &args)?;
        assert!(output.status.success());
        assert!(plain(&output.stdout)?.contains("Usage:"));
        assert!(output.stderr.is_empty());
    }
    let version = cli(root.path(), &["version"])?;
    assert_eq!(version.status.code(), Some(0));
    assert!(plain(&version.stdout)?.contains(env!("CARGO_PKG_VERSION")));
    let error = cli(root.path(), &["check", "--unknown-opdev-option"])?;
    assert_eq!(error.status.code(), Some(2));
    assert!(error.stdout.is_empty());
    let diagnostic = plain(&error.stderr)?;
    assert!(diagnostic.contains("--unknown-opdev-option"));
    assert!(diagnostic.contains("--help"));
    Ok(())
}

#[test]
fn blocked_reports_are_named_retained_and_never_overwritten()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    let artifacts = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(repo.path())
            .status()?
            .success()
    );
    let manifest = discover(repo.path())?.manifest;
    manifest.write_new(&repo.path().join(MANIFEST_PATH))?;
    let human = cli(repo.path(), &["check", "--no-exec"])?;
    assert_eq!(human.status.code(), Some(1));
    let text = plain(&human.stdout)?;
    assert!(text.contains("Blocked"));
    assert!(text.contains("OPDEV-WORK-001"));
    let report = artifacts.path().join("full report.json");
    let path = report.to_str().ok_or("non-UTF8 fixture path")?;
    let output = cli(
        repo.path(),
        &["check", "--no-exec", "--format", "json", "--report", path],
    )?;
    assert_eq!(output.status.code(), Some(1));
    let parsed: Value = serde_json::from_slice(&output.stdout)?;
    let retained = fs::read(&report)?;
    assert_eq!(parsed, serde_json::from_slice::<Value>(&retained)?);
    assert!(
        parsed["gates"]
            .as_array()
            .is_some_and(|gates| !gates.is_empty())
    );
    let retry = cli(repo.path(), &["check", "--no-exec", "--report", path])?;
    assert_eq!(retry.status.code(), Some(2));
    assert!(plain(&retry.stderr)?.contains("choose a new path"));
    assert_eq!(fs::read(&report)?, retained);
    Ok(())
}
