use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{TestReportInspection, execute, inspect_junit};
use anyhow::{Context, Result, ensure};
use opdev_core::Outcome;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
/// An observation of one canonical execution, not authenticated qualification.
pub struct TestExecutionReceipt {
    schema: u32,
    suite: String,
    command_key: String,
    command_sha256: String,
    started_at_unix_ms: u128,
    revision_before: String,
    revision_after: Option<String>,
    source_unchanged: bool,
    command_outcome: Outcome,
    report_outcome: Outcome,
    /// Combined command, report and source observation outcome.
    pub outcome: Outcome,
    exit_code: Option<i32>,
    pub(crate) duration_ms: Option<u128>,
    report: Option<TestReportInspection>,
    qualification: Outcome,
    diagnostics: Vec<String>,
}

/// Runs one canonical suite once, requiring clean source and a fresh report path.
///
/// # Errors
/// Returns an error before execution if source, suite or destination validation fails.
pub fn observe_test_execution(
    root: &Path,
    suite_id: &str,
    junit: &Path,
) -> Result<TestExecutionReceipt> {
    observe_with_manifest(root, suite_id, junit, None)
}

pub(crate) fn observe_with_manifest(
    root: &Path,
    suite_id: &str,
    junit: &Path,
    expected_manifest: Option<&opdev_project::ProjectManifest>,
) -> Result<TestExecutionReceipt> {
    let revision = clean_revision(root)
        .context("a clean committed source tree is required before execution")?;
    let manifest = opdev_project::ProjectManifest::load(&root.join(opdev_project::MANIFEST_PATH))?;
    ensure!(
        expected_manifest.is_none_or(|expected| expected == &manifest),
        "project contract changed since evaluation began; no command was executed"
    );
    let suite = manifest
        .testing
        .suites
        .iter()
        .find(|suite| suite.id == suite_id)
        .context("unknown canonical test suite; no command was executed")?;
    let command = &manifest.commands[&suite.command];
    let path = root.join(junit);
    ensure_absent(&path)?;
    ensure!(
        clean_revision(root)? == revision,
        "source changed while selecting the command; nothing was executed"
    );
    let mut receipt = TestExecutionReceipt {
        schema: 1,
        suite: suite.id.clone(),
        command_key: suite.command.clone(),
        command_sha256: format!("{:x}", Sha256::digest(serde_json::to_vec(command)?)),
        started_at_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        revision_before: revision,
        revision_after: None,
        source_unchanged: false,
        command_outcome: Outcome::Error,
        report_outcome: Outcome::Unverified,
        outcome: Outcome::Unverified,
        exit_code: None,
        duration_ms: None,
        report: None,
        qualification: Outcome::Unverified,
        diagnostics: vec![],
    };
    match execute(root, command, None) {
        Ok(execution) => {
            receipt.exit_code = execution.exit_code;
            receipt.duration_ms = Some(execution.duration_ms);
            receipt.command_outcome = if execution.timed_out {
                receipt
                    .diagnostics
                    .push("Canonical command exceeded its timeout.".into());
                Outcome::Error
            } else if execution.exit_code == Some(0) {
                Outcome::Passed
            } else {
                Outcome::Failed
            };
        }
        Err(_) => receipt
            .diagnostics
            .push("Canonical command execution failed; no successful exit was observed.".into()),
    }
    collect_report(&path, &mut receipt);
    receipt.revision_after = clean_revision(root).ok();
    receipt.source_unchanged = receipt.revision_after.as_ref() == Some(&receipt.revision_before);
    if !receipt.source_unchanged {
        receipt
            .diagnostics
            .push("Source identity changed or could not be verified after execution.".into());
    }
    receipt.outcome = aggregate(&receipt);
    receipt.diagnostics.push("One observed command attempt and a previously absent report only. Ignored inputs, external dependencies, producer-internal retries, quarantine completeness and CI identity are not verified. This receipt alone cannot qualify a gate; no ledger was updated.".into());
    Ok(receipt)
}

fn ensure_absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).context("could not inspect report destination; no command was executed")
        }
        Ok(_) => anyhow::bail!(
            "report destination already exists; choose a fresh path before running (nothing was removed or executed)"
        ),
    }
}

fn collect_report(path: &Path, receipt: &mut TestExecutionReceipt) {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            receipt
                .diagnostics
                .push("The canonical command did not produce the requested report.".into());
        }
        Ok(metadata) if metadata.file_type().is_file() => {
            if let Ok(report) = File::open(path).and_then(inspect_junit) {
                receipt.report_outcome = report.outcome;
                receipt.report = Some(report);
            } else {
                receipt.report_outcome = Outcome::Error;
                receipt.diagnostics.push(
                    "The report could not be parsed within the supported format and limits.".into(),
                );
            }
        }
        _ => {
            receipt.report_outcome = Outcome::Error;
            receipt
                .diagnostics
                .push("The report is unreadable or is not a regular non-symlink file.".into());
        }
    }
}

fn aggregate(receipt: &TestExecutionReceipt) -> Outcome {
    if receipt.command_outcome == Outcome::Failed || receipt.report_outcome == Outcome::Failed {
        Outcome::Failed
    } else if receipt.command_outcome == Outcome::Error || receipt.report_outcome == Outcome::Error
    {
        Outcome::Error
    } else if !receipt.source_unchanged || receipt.report_outcome != Outcome::Passed {
        Outcome::Unverified
    } else {
        Outcome::Passed
    }
}

fn clean_revision(root: &Path) -> Result<String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ])
        .output()
        .context("could not inspect source status")?;
    ensure!(
        status.status.success() && status.stdout.is_empty(),
        "source is dirty or unavailable"
    );
    let revision = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "HEAD^{commit}"])
        .output()
        .context("could not inspect source revision")?;
    ensure!(revision.status.success(), "source has no readable commit");
    let revision = String::from_utf8(revision.stdout)?.trim().to_owned();
    ensure!(
        matches!(revision.len(), 40 | 64) && revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "invalid source revision"
    );
    Ok(revision)
}
