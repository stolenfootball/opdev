//! Portable inspection envelopes. No saved bundle can construct qualifying execution.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use clap::{Args, Subcommand};
use opdev_core::Outcome;
use opdev_project::{EvidenceLedger, ProjectManifest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{local_state, workflow_records::Subject};

const LIMITS: &str = "Integrity checked; origin and qualification remain unverified offline. Attributed reviews are not authenticated decisions. Saved reports cannot qualify another check. No upload, cleanup or release is authorized.";

#[derive(Debug, Args)]
pub(super) struct BundleArgs {
    #[command(subcommand)]
    command: BundleCommand,
}

#[derive(Debug, Subcommand)]
enum BundleCommand {
    /// Capture an explicitly selected work excerpt with provider provenance, not approval.
    ObserveWork {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Strict JSON provider/repository/kind/number/note selection.
        #[arg(long)]
        selector: PathBuf,
        /// UTF-8 file containing only the exact needed excerpt.
        #[arg(long)]
        excerpt: PathBuf,
        /// Optional original whole-body identity for a freshness recheck.
        #[arg(long)]
        expected_body_sha256: Option<String>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Export only current semantic review; no command output or execution authority.
    ExportReview {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        stage: String,
        /// Explicit reviewed schema-2 ledger outside product source, or the legacy ledger by default.
        #[arg(long)]
        ledger: Option<PathBuf>,
        /// Explicit minimal observations to bind; saved attribution is not authenticated consent.
        #[arg(long)]
        work_observation: Vec<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Export one exact-current review and optional retained attempt without altering history.
    Export {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        stage: String,
        #[arg(long)]
        attempt: Option<String>,
        /// New file outside source and Git storage; parent must already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Inspect explicit bytes against current source and independently selected expectations.
    Inspect {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        stage: String,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        sha256: String,
        #[arg(long)]
        acceptance_sha256: String,
        /// When selecting an attempt, compare its actual recorded CLI identity too.
        #[arg(long)]
        attempt_runtime_sha256: Option<String>,
    },
    /// Retrieve exact provider-owned Git bytes; never select a newer or older report.
    Retrieve {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        stage: String,
        /// Reviewed locator JSON from the existing evidence authority, not from the bundle.
        #[arg(long)]
        locator: PathBuf,
        #[arg(long)]
        acceptance_sha256: String,
        #[arg(long)]
        attempt_runtime_sha256: Option<String>,
        /// New local file outside product source and Git metadata.
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    schema: u32,
    kind: String,
    subject: Subject,
    configuration: ProjectManifest,
    acceptance_sha256: String,
    // Exactly one existing change record, plus original project assertions.
    ledger: EvidenceLedger,
    attempt: Option<local_state::AttemptSnapshot>,
    exporter: ExportObservation,
    qualification: Outcome,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportObservation {
    runtime_sha256: String,
    runtime_version: String,
    observed_at_ms: u128,
    os: String,
    arch: String,
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

impl Bundle {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == 1
                && self.kind == "change_evidence"
                && self.qualification == Outcome::Unverified,
            "Unsupported evidence bundle schema or qualification claim"
        );
        self.subject.validate()?;
        ensure!(
            valid_digest(&self.exporter.runtime_sha256)
                && [
                    &self.exporter.runtime_version,
                    &self.exporter.os,
                    &self.exporter.arch
                ]
                .iter()
                .all(|s| !s.trim().is_empty()),
            "Export observation is incomplete; no producer identity inferred"
        );
        let _: opdev_project::TestStage = serde_json::from_value(json!(self.subject.stage))?;
        // Reuse the strict manifest parser, including schema/policy compatibility.
        ProjectManifest::from_yaml(&self.configuration.to_yaml()?)?;
        ensure!(
            sha(&serde_json::to_vec(&self.configuration)?) == self.subject.configuration_sha256,
            "Bundled configuration does not match its identity"
        );
        self.ledger.validate(&self.configuration.catalog()?)?;
        ensure!(
            self.ledger.schema == 2 && self.ledger.changes.len() == 1,
            "A bundle needs one schema-2 change; do not replace or combine attempt history"
        );
        let change = &self.ledger.changes[0];
        let acceptance = change
            .acceptance
            .as_ref()
            .context("Bundle acceptance inventory is missing")?;
        ensure!(
            change.fingerprint == self.subject.source_sha256
                && acceptance.digest(&change.fingerprint, &change.work)? == self.acceptance_sha256,
            "Bundled acceptance does not match its change identity"
        );
        if let Some(attempt) = &self.attempt {
            attempt.validate()?;
            ensure!(
                attempt.subject() == &self.subject,
                "Attempt does not describe this exact source, configuration and stage"
            );
        }
        Ok(())
    }

    fn verify_sources(&self, root: &Path) -> Result<()> {
        let acceptance = self.ledger.changes[0]
            .acceptance
            .as_ref()
            .context("Missing acceptance")?;
        for condition in &acceptance.conditions {
            condition.source.verify(root)?;
        }
        for mapping in &acceptance.verifications {
            mapping.target.verify(root)?;
        }
        Ok(())
    }
}

fn export(root: &Path, stage: &str, attempt: Option<&str>, output: &Path) -> Result<()> {
    let (root, configuration) = crate::load_project(root)?;
    let subject = local_state::subject(&root, &configuration, stage)?;
    let ledger = EvidenceLedger::load_optional(&root, &configuration.catalog()?)?
        .context("No evidence ledger to export; nothing changed")?;
    let original_ledger = ledger.clone();
    let change = ledger
        .changes
        .iter()
        .find(|c| c.fingerprint == subject.source_sha256)
        .context("No exact-current change to export; no older record substituted")?
        .clone();
    let acceptance_sha256 = change
        .acceptance
        .as_ref()
        .context("Current acceptance inventory is missing")?
        .digest(&change.fingerprint, &change.work)?;
    let bundle = Bundle {
        schema: 1,
        kind: "change_evidence".into(),
        subject,
        configuration,
        acceptance_sha256,
        ledger: EvidenceLedger {
            schema: 2,
            project: ledger.project,
            changes: vec![change],
        },
        attempt: attempt
            .map(|id| local_state::snapshot(&root, id))
            .transpose()?,
        exporter: ExportObservation {
            runtime_sha256: crate::workflow::content_digest(&std::env::current_exe()?)?,
            runtime_version: env!("CARGO_PKG_VERSION").into(),
            observed_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
        },
        qualification: Outcome::Unverified,
    };
    bundle.validate()?;
    bundle.verify_sources(&root)?;
    let output = export_destination(&root, output)?;
    let bytes = serde_json::to_vec_pretty(&bundle)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "Bundle exceeds the 8 MiB supported input limit; nothing written"
    );
    ensure!(
        current_subject(&root, stage)? == bundle.subject,
        "Source changed before export; nothing written"
    );
    ensure!(
        EvidenceLedger::load_optional(&root, &bundle.configuration.catalog()?)?.as_ref()
            == Some(&original_ledger),
        "Review changed before export; nothing written"
    );
    local_state::write_new(&output, &bytes)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"schema":1,"output":output,"sha256":sha(&bytes),
        "acceptance_sha256":bundle.acceptance_sha256,"qualification":"unverified",
        "limits":"Evidence exported locally. Nothing uploaded; no check or release was authorized. Original history remains unchanged; inspect private content before sharing."}))?
    );
    Ok(())
}

pub(super) fn export_destination(root: &Path, output: &Path) -> Result<PathBuf> {
    let output = local_state::observed_absolute(&std::path::absolute(output)?)?;
    for owner in [
        root.to_owned(),
        local_state::git_path(root, "--git-common-dir")?,
        local_state::git_path(root, "--absolute-git-dir")?,
    ] {
        ensure!(
            !output.starts_with(owner),
            "Export outside source and Git storage; no fingerprint exclusion or project archive was created"
        );
    }
    ensure!(
        !output.exists(),
        "Destination already exists; no earlier evidence replaced"
    );
    Ok(output)
}

fn observe_work(
    root: &Path,
    selector: &Path,
    excerpt: &Path,
    expected: Option<&str>,
    output: &Path,
) -> Result<()> {
    ensure!(
        expected.is_none_or(valid_digest),
        "Expected work body identity must be a SHA-256 digest"
    );
    let (root, _) = crate::load_project(root)?;
    let output = export_destination(&root, output)?;
    let selection =
        local_state::read(&std::path::absolute(selector)?)?.context("Work selector is missing")?;
    let selection: opdev_project::WorkSelector =
        serde_json::from_slice(&selection).map_err(|_| {
            anyhow::anyhow!("Work selector is malformed or unsupported; no private content echoed")
        })?;
    let excerpt =
        local_state::read(&std::path::absolute(excerpt)?)?.context("Work excerpt is missing")?;
    let excerpt = std::str::from_utf8(&excerpt).context("Work excerpt must be UTF-8")?;
    let observation =
        opdev_remote::observe_work(&selection, excerpt).map_err(anyhow::Error::msg)?;
    ensure!(
        expected.is_none_or(|digest| digest == observation.body_sha256),
        "Work content changed since the selected observation; inspect the current decision and scope. No earlier text substituted"
    );
    let bytes = serde_json::to_vec_pretty(&observation)?;
    local_state::write_new(&output, &bytes)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"schema":1,"output":output,"sha256":sha(&bytes),
        "body_sha256":observation.body_sha256,"qualification":"unverified",
        "limits":"Selected work excerpt observed through the provider. Authorship is not approval, current authority or release permission. No full conversation copied; retain the original scope and any later revocation."}))?
    );
    Ok(())
}

fn export_review(
    root: &Path,
    stage: &str,
    ledger: Option<&PathBuf>,
    observations: &[PathBuf],
    output: &Path,
) -> Result<()> {
    let (root, manifest) = crate::load_project(root)?;
    let stage = serde_json::from_value(json!(stage))?;
    let input = ledger.map_or_else(|| root.join(opdev_project::EVIDENCE_PATH), Clone::clone);
    let input = std::path::absolute(input)?;
    let bytes = local_state::read(&input)?.context("Explicit review input is missing")?;
    let ledger: EvidenceLedger = serde_saphyr::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("Review input is malformed; no private content echoed"))?;
    let mut record = opdev_project::ReviewRecord::prepare(&root, &manifest, stage, &ledger)?;
    let observations = observations
        .iter()
        .map(|path| -> Result<_> {
            let bytes = local_state::read(&std::path::absolute(path)?)?
                .context("Work observation is missing")?;
            serde_json::from_slice::<opdev_project::WorkObservation>(&bytes).map_err(|_| {
                anyhow::anyhow!(
                    "Work observation is malformed or unsupported; no private content echoed"
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    record.bind_observations(observations)?;
    let output = export_destination(&root, output)?;
    let serialized = serde_json::to_vec_pretty(&record)?;
    ensure!(
        serialized.len() <= 8 * 1024 * 1024,
        "Semantic review exceeds the 8 MiB supported limit"
    );
    record.verify_current(&root, &manifest, stage, &record.acceptance_sha256)?;
    ensure!(
        local_state::read(&input)?.as_ref() == Some(&bytes),
        "Review input changed before export; nothing written"
    );
    local_state::write_new(&output, &serialized)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"schema":1, "output":output,
        "sha256":sha(&serialized), "acceptance_sha256":record.acceptance_sha256,
        "qualification":"unverified", "limits":"Semantic review exported; attributed judgments are not authenticated consent. No upload, execution, history deletion or release occurred."}))?
    );
    Ok(())
}

pub(super) fn run(args: &BundleArgs) -> Result<()> {
    match &args.command {
        BundleCommand::ObserveWork {
            root,
            selector,
            excerpt,
            expected_body_sha256,
            output,
        } => observe_work(
            root,
            selector,
            excerpt,
            expected_body_sha256.as_deref(),
            output,
        ),
        BundleCommand::ExportReview {
            root,
            stage,
            ledger,
            work_observation,
            output,
        } => export_review(root, stage, ledger.as_ref(), work_observation, output),
        BundleCommand::Export {
            root,
            stage,
            attempt,
            output,
        } => export(root, stage, attempt.as_deref(), output),
        BundleCommand::Inspect {
            root,
            stage,
            input,
            sha256,
            acceptance_sha256,
            attempt_runtime_sha256,
        } => {
            let bytes = local_state::read(&std::path::absolute(input)?)?
                .context("Evidence bundle is missing; no cached result substituted")?;
            let report = inspect_bytes(
                root,
                stage,
                &bytes,
                sha256,
                acceptance_sha256,
                attempt_runtime_sha256.as_deref(),
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        BundleCommand::Retrieve {
            root,
            stage,
            locator,
            acceptance_sha256,
            attempt_runtime_sha256,
            output,
        } => {
            let bytes = local_state::read(&std::path::absolute(locator)?)?
                .context("Archive locator is missing")?;
            let locator: opdev_remote::ArchiveLocator = serde_json::from_slice(&bytes)?;
            locator.validate().map_err(anyhow::Error::msg)?;
            ensure!(
                valid_digest(acceptance_sha256)
                    && attempt_runtime_sha256.as_deref().is_none_or(valid_digest),
                "Expected identities must be SHA-256 digests"
            );
            let (root, manifest) = crate::load_project(root)?;
            let before = local_state::subject(&root, &manifest, stage)?;
            let output = export_destination(&root, output)?;
            let observed = opdev_remote::retrieve_archive(&locator).map_err(anyhow::Error::msg)?;
            let mut report = inspect_bytes(
                &root,
                stage,
                observed.bytes(),
                &locator.sha256,
                acceptance_sha256,
                attempt_runtime_sha256.as_deref(),
            )?;
            ensure!(
                current_subject(&root, stage)? == before,
                "Source changed during retrieval; nothing written"
            );
            local_state::write_new(&output, observed.bytes())?;
            report["origin"] = json!("provider_observed");
            report["archive"] = serde_json::to_value(observed.locator())?;
            report["blob"] = json!(observed.blob());
            report["output"] = json!(output);
            report["limits"] = json!(
                "Exact bytes retrieved from the selected provider repository. Retention, review authority and qualification remain unverified. No saved report qualifies a check; no upload, cleanup or release was authorized."
            );
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
    }
}

fn inspect_bytes(
    root: &Path,
    stage: &str,
    bytes: &[u8],
    sha256: &str,
    acceptance_sha256: &str,
    attempt_runtime_sha256: Option<&str>,
) -> Result<serde_json::Value> {
    ensure!(
        valid_digest(sha256)
            && valid_digest(acceptance_sha256)
            && attempt_runtime_sha256.is_none_or(valid_digest),
        "Expected identities must be SHA-256 digests"
    );
    ensure!(
        sha(bytes) == sha256.to_ascii_lowercase(),
        "Evidence bundle bytes changed; do not reuse it"
    );
    // Imported reports may contain private output. Parser diagnostics can echo
    // unknown field/variant values, so do not expose their source error chain.
    let bundle: Bundle = serde_json::from_slice(bytes).map_err(|_| {
        anyhow::anyhow!("Evidence bundle contains malformed or unsupported fields; inspect its schema locally. No private content echoed")
    })?;
    bundle.validate().map_err(|_| {
        anyhow::anyhow!("Evidence bundle schema or internal bindings are inconsistent; inspect its source, configuration, review and attempt identities locally. No earlier result substituted")
    })?;
    let (root, manifest) = crate::load_project(root)?;
    ensure!(
        local_state::subject(&root, &manifest, stage)? == bundle.subject
            && bundle.acceptance_sha256 == acceptance_sha256.to_ascii_lowercase(),
        "The bundle does not match the expected source, configuration, stage or acceptance review. No earlier result was substituted."
    );
    bundle.verify_sources(&root)?;
    ensure!(
        current_subject(&root, stage)? == bundle.subject,
        "Source or configuration changed during inspection; retry from current references"
    );
    let review = &bundle.ledger.changes[0]
        .acceptance
        .as_ref()
        .context("Missing acceptance")?
        .review;
    let review_current = matches!(review.outcome, Outcome::Passed | Outcome::Failed)
        && review.subject_sha256 == bundle.acceptance_sha256
        && [&review.reviewer, &review.reference, &review.rationale]
            .iter()
            .all(|s| !s.trim().is_empty());
    if let Some(expected) = attempt_runtime_sha256 {
        ensure!(
            bundle
                .attempt
                .as_ref()
                .is_some_and(|a| a.runtime_sha256() == expected.to_ascii_lowercase()),
            "Recorded attempt producer differs or is missing; no previous producer substituted"
        );
    }
    Ok(
        json!({"schema":1,"integrity":"matched", "subject":bundle.subject,"acceptance_sha256":bundle.acceptance_sha256,
        "review_current":review_current,"attributed_review_outcome":review.outcome,
        "attempt_state":bundle.attempt.as_ref().map(local_state::AttemptSnapshot::state),
        "export_observation":bundle.exporter,"contains_attempt":bundle.attempt.is_some(),
        "qualification":"unverified","origin":"unverified","limits":LIMITS}),
    )
}

fn current_subject(root: &Path, stage: &str) -> Result<Subject> {
    let (root, manifest) = crate::load_project(root)?;
    local_state::subject(&root, &manifest, stage)
}
