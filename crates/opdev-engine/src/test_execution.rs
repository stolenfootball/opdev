use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::execute;
use anyhow::{Context, Result, ensure};
use opdev_core::Outcome;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Serialize)]
/// An observation of one canonical execution, not authenticated qualification.
pub struct TestExecutionReceipt {
    schema: u32,
    attempt_id: String,
    suite: String,
    command_key: String,
    command_sha256: String,
    started_at_unix_ms: u128,
    revision_before: String,
    revision_after: Option<String>,
    source_unchanged: bool,
    command_outcome: Outcome,
    /// Combined command and source observation outcome.
    pub outcome: Outcome,
    exit_code: Option<i32>,
    duration_ms: Option<u128>,
    qualification: Outcome,
    diagnostics: Vec<String>,
}

/// Runs one canonical suite once, requiring clean committed source.
///
/// # Errors
/// Returns an error before execution if source or suite validation fails.
pub fn observe_test_execution(root: &Path, suite_id: &str) -> Result<TestExecutionReceipt> {
    let revision = clean_revision(root)
        .context("a clean committed source tree is required before execution")?;
    let manifest = opdev_project::ProjectManifest::load(&root.join(opdev_project::MANIFEST_PATH))?;
    let suite = manifest
        .testing
        .suites
        .iter()
        .find(|suite| suite.id == suite_id)
        .context("unknown canonical test suite; no command was executed")?;
    let command = &manifest.commands[&suite.command];
    // Independent observation identity, not a CI run ID or authentication.
    let identity = tempfile::Builder::new()
        .prefix("opdev-attempt-")
        .rand_bytes(24)
        .tempfile()
        .context("could not allocate an execution identity; no command was executed")?;
    ensure!(
        clean_revision(root)? == revision,
        "source changed while selecting the command; nothing was executed"
    );
    let mut receipt = TestExecutionReceipt {
        schema: 2,
        attempt_id: identity
            .path()
            .file_name()
            .context("missing attempt identity")?
            .to_string_lossy()
            .into_owned(),
        suite: suite.id.clone(),
        command_key: suite.command.clone(),
        command_sha256: format!("{:x}", Sha256::digest(serde_json::to_vec(command)?)),
        started_at_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        revision_before: revision,
        revision_after: None,
        source_unchanged: false,
        command_outcome: Outcome::Error,
        outcome: Outcome::Unverified,
        exit_code: None,
        duration_ms: None,
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
    receipt.revision_after = clean_revision(root).ok();
    receipt.source_unchanged = receipt.revision_after.as_ref() == Some(&receipt.revision_before);
    if !receipt.source_unchanged {
        receipt
            .diagnostics
            .push("Source identity changed or could not be verified after execution.".into());
    }
    receipt.outcome = aggregate(&receipt);
    receipt.diagnostics.push("One observed canonical command attempt only. A successful exit does not establish test counts, completeness or effectiveness. Ignored inputs, external dependencies, internal retries, quarantine completeness and CI identity are not verified. No test report was inspected. This receipt alone cannot qualify a gate; no ledger was updated.".into());
    Ok(receipt)
}

fn aggregate(receipt: &TestExecutionReceipt) -> Outcome {
    if receipt.command_outcome == Outcome::Failed {
        Outcome::Failed
    } else if receipt.command_outcome == Outcome::Error {
        Outcome::Error
    } else if !receipt.source_unchanged {
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
