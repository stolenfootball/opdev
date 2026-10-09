//! Agent-guided adoption with deterministic completion verification.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use opdev_core::{Gate, Outcome};
use opdev_engine::{CheckOptions, evaluate};
use opdev_project::{
    ADOPTION_PATH, AdoptionRecord, AdoptionReview, EVIDENCE_PATH, EvidenceLedger,
    adoption_catalog_version, project_adoption_catalog, staged_fingerprint,
};

use crate::{
    OutputFormat, apply_local_ci, apply_remote_audit, load_project, print_human_report, views,
};

#[derive(Debug, Args)]
pub(super) struct AdoptionArgs {
    #[command(subcommand)]
    command: AdoptionCommand,
}

#[derive(Debug, Subcommand)]
enum AdoptionCommand {
    /// Print the exact choices and contract identifier for developer review; writes nothing.
    Plan {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Populate a read-only policy worksheet from provider observations.
        #[arg(long)]
        remote: bool,
    },
    /// Record an actual developer response or bounded delegation for an unchanged plan.
    Approve {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        plan: String,
        #[arg(long)]
        reviewer: String,
        #[arg(long)]
        reference: String,
        /// Actual scope/limits of a user grant, not permission inferred from an adoption request.
        #[arg(long)]
        delegation: Option<String>,
    },
    /// Preview schema 1 to 2 migration; preserves dispositions but never fabricates approval.
    Migrate {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        write: bool,
        /// Explicit inventory upgrade matching the already-reviewed project policy.
        #[arg(long)]
        catalog_version: Option<u32>,
    },
    /// Print unresolved, fingerprint-bound adoption evidence with the correct kind/location.
    PrepareEvidence {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Inspect the versioned, tool-neutral practice catalog.
    Catalog {
        /// Exact inventory to inspect; omitted uses project policy, or legacy 1 outside a project.
        #[arg(long)]
        catalog_version: Option<u32>,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Explicitly start assessment for an existing initialized project; preserve existing decisions.
    Start {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Preview unresolved decisions without writing anything.
        #[arg(long)]
        dry_run: bool,
    },
    /// Read decisions and gaps without executing project commands or claiming completion.
    Status {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
        /// Include read-only policy observations; never select or approve policy.
        #[arg(long)]
        remote: bool,
    },
    /// Verify all decisions, fresh reviewed evidence, executable checks and all core gates.
    Check {
        /// Inspect all required inputs without running suites or claiming completed adoption.
        #[arg(long, conflicts_with = "report")]
        preflight: bool,
        /// Diagnose the selected older policy only; never reports current adoption complete.
        #[arg(long)]
        legacy_assessment: bool,
        /// Exact provider archive for the selected external review policy; not a saved check report.
        #[arg(long, requires = "review_acceptance_sha256")]
        review_locator: Option<PathBuf>,
        /// Independently selected acceptance identity for this exact adoption change.
        #[arg(long, requires = "review_locator")]
        review_acceptance_sha256: Option<String>,
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Include read-only provider auditing.
        #[arg(long)]
        remote: bool,
        /// Save the full core check report to a new file, preferably outside the worktree.
        #[arg(long)]
        report: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
}

pub(super) fn run(args: &AdoptionArgs) -> Result<ExitCode> {
    match &args.command {
        AdoptionCommand::Plan { root, remote } => plan(root, *remote)?,
        AdoptionCommand::Approve {
            root,
            plan,
            reviewer,
            reference,
            delegation,
        } => approve(
            root,
            AdoptionReview {
                plan_id: plan.clone(),
                reviewer: reviewer.clone(),
                reference: reference.clone(),
                delegation: delegation.clone(),
            },
        )?,
        AdoptionCommand::Migrate {
            root,
            write,
            catalog_version,
        } => migrate(root, *write, *catalog_version)?,
        AdoptionCommand::PrepareEvidence { root } => prepare_evidence(root)?,
        AdoptionCommand::Catalog {
            root,
            catalog_version,
        } => print_catalog(root, *catalog_version)?,
        AdoptionCommand::Start { root, dry_run } => start(root, *dry_run)?,
        AdoptionCommand::Status {
            root,
            format,
            remote,
        } => status(root, *format, *remote)?,
        AdoptionCommand::Check {
            preflight,
            legacy_assessment,
            review_locator,
            review_acceptance_sha256,
            root,
            remote,
            report,
            format,
        } => {
            return check(
                root,
                *remote,
                report.as_ref(),
                *format,
                review_locator.as_ref(),
                review_acceptance_sha256.as_deref(),
                CheckMode {
                    legacy_assessment: *legacy_assessment,
                    preflight: *preflight,
                },
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn status(root: &std::path::Path, format: OutputFormat, remote: bool) -> Result<()> {
    let (root, manifest) = load_project(root)?;
    let record = AdoptionRecord::load(&root)?;
    let remote_gap = manifest.remote_qualification_gap();
    let worksheet = policy_worksheet(&manifest, remote)?;
    let mut blockers = record
        .as_ref()
        .map(|r| r.blockers(&manifest))
        .transpose()?
        .unwrap_or_default();
    blockers.extend(opdev_project::clean_adoption::decision_gaps(
        &manifest,
        record.as_ref(),
    ));
    if let Some(target) = record.as_ref().and_then(|r| r.clean_target.as_ref()) {
        blockers.extend(
            opdev_project::clean_adoption::retirement_gaps(&root, target)
                .map_err(anyhow::Error::msg)?,
        );
    }
    let status = if record.as_ref().is_some_and(|r| r.schema == 1) {
        "migration_required"
    } else if record.is_none() {
        "legacy_unassessed"
    } else if blockers.is_empty() {
        "decisions_ready_for_verification"
    } else {
        "pending"
    };
    if matches!(format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"schema":1,"status":status,"complete":false,
                    "approval": if record.as_ref().is_some_and(|r| r.approval_blockers(&manifest).is_ok_and(|b| b.is_empty())) { "approved" } else { "review_required" },
                    "verification":"not_run","blockers":blockers,"record":record,
                    "remote_qualification": {"verification":"not_run", "policy_ready":remote_gap.is_none(), "gap":remote_gap, "worksheet":worksheet},
                    "delivery_readiness":"not_run"})
            )?
        );
    } else {
        println!("Adoption: {status} (status does not run checks or certify completion)");
        println!(
            "Remote qualification: not run; {}",
            remote_gap.unwrap_or("reviewed policy present; provider verification still required")
        );
        println!("Delivery readiness: not run");
        if let Some(worksheet) = worksheet {
            println!(
                "Policy observations for developer review: {}",
                serde_json::to_string_pretty(&worksheet)?
            );
        }
        if let Some(record) = record {
            for practice in adoption_catalog_version(record.catalog_version)?.practices {
                println!(
                    "{}: {:?} — {}",
                    practice.id, record.practices[&practice.id].state, practice.title
                );
            }
        } else {
            println!("Use opdev adoption start to explicitly assess this existing project.");
        }
        for blocker in blockers {
            println!("  {blocker}");
        }
    }
    Ok(())
}

fn start(root: &std::path::Path, dry_run: bool) -> Result<()> {
    let (root, manifest) = load_project(root)?;
    let existing = AdoptionRecord::load(&root)?;
    let exists = existing.is_some();
    let record = existing.map_or_else(
        || AdoptionRecord::pending_for_catalog(project_adoption_catalog(&manifest)),
        Ok,
    )?;
    if dry_run {
        print!("{}", record.to_yaml()?);
    } else if exists {
        println!("Existing adoption decisions preserved; use adoption status.");
    } else {
        record.write_new(&root)?;
        println!("Created {ADOPTION_PATH}; all practices are pending. Adoption is not complete.");
    }
    Ok(())
}

fn approve(root: &std::path::Path, review: AdoptionReview) -> Result<()> {
    let (root, manifest) = load_project(root)?;
    let before = std::fs::read(root.join(ADOPTION_PATH))?;
    let mut record = AdoptionRecord::load(&root)?.context("no adoption record")?;
    if record.schema != 2 {
        bail!("migration_required: preview adoption migrate first");
    }
    if record.plan_id(&manifest)? != review.plan_id {
        bail!("stale adoption plan; present the current plan for review");
    }
    if record.scope.trim().is_empty()
        || record.practices.values().any(|d| {
            d.owner.trim().is_empty() || d.reason.trim().is_empty() || d.references.is_empty()
        })
    {
        bail!("assess scope and every proposed choice before requesting approval");
    }
    if record.clean_target.is_some() {
        let gaps = opdev_project::clean_adoption::decision_gaps(&manifest, Some(&record));
        if !gaps.is_empty() {
            bail!(
                "Resolve destination choices before approval: {}",
                gaps.join("; ")
            );
        }
    }
    record.review = Some(review);
    replace_record(&root, &before, &record)?;
    println!(
        "Recorded approval claim; implementation and adoption completion remain separately verified. This does not authenticate reviewer identity."
    );
    Ok(())
}

fn plan(root: &std::path::Path, remote: bool) -> Result<()> {
    let discovery = opdev_project::discover(root)?;
    let root = discovery.root;
    let manifest = discovery.manifest;
    let record = AdoptionRecord::load(&root)?;
    let configured = root.join(opdev_project::MANIFEST_PATH).exists();
    let mut target_gaps = opdev_project::clean_adoption::decision_gaps(&manifest, record.as_ref());
    if let Some(target) = record.as_ref().and_then(|r| r.clean_target.as_ref()) {
        target_gaps.extend(
            opdev_project::clean_adoption::retirement_gaps(&root, target)
                .map_err(anyhow::Error::msg)?,
        );
    }
    let branch_choices = if manifest.project.trunk == "main" {
        vec!["Keep main as the single integration and release source".to_string()]
    } else {
        vec![
            format!(
                "Keep {} as the single integration and release source",
                manifest.project.trunk
            ),
            "Rename the trunk to main after reviewing CI, protection and documentation changes"
                .into(),
        ]
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": 1, "plan_id": record.as_ref().map(|r| r.plan_id(&manifest)).transpose()?, "record": record,
            "target": opdev_project::clean_adoption::TARGET,
            "starting_state": if configured {"configured_project"} else {"uninitialized_project"},
            "required_migrations": target_gaps,
            "required_practices": adoption_catalog_version(2)?,
            "discovery_warnings": discovery.warnings,
            "contract": manifest, "approval": "review_required",
            "branch_choices": branch_choices,
            "remote_qualification": {"verification":"not_run", "policy_ready":manifest.remote_qualification_gap().is_none(), "gap":manifest.remote_qualification_gap(), "worksheet":policy_worksheet(&manifest, remote)?},
            "delivery_readiness":"not_run",
            "notice": "Review retained, changed and retired content against clean-2. An empty project needs real behavior before verification; setup alone is incomplete. Resolve material choices before implementation. A plan hash is not consent."
        }))?
    );
    Ok(())
}

fn policy_worksheet(
    manifest: &opdev_project::ProjectManifest,
    remote: bool,
) -> Result<Option<serde_json::Value>> {
    if remote {
        Ok(Some(opdev_remote::observe_qualification_policy(manifest)?))
    } else {
        Ok(None)
    }
}

fn print_catalog(root: &std::path::Path, version: Option<u32>) -> Result<()> {
    let version = match version {
        Some(version) => version,
        // Catalog inspection is available before initialization, but an existing
        // malformed contract must not silently select legacy policy.
        None => match opdev_project::discover(root) {
            Ok(discovery) => project_adoption_catalog(&discovery.manifest),
            Err(opdev_project::DiscoveryError::NotRepository(_)) => 1,
            Err(error) => return Err(error.into()),
        },
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&adoption_catalog_version(version)?)?
    );
    Ok(())
}

fn migrate(root: &std::path::Path, write: bool, catalog_version: Option<u32>) -> Result<()> {
    let (root, manifest) = load_project(root)?;
    let before = std::fs::read(root.join(ADOPTION_PATH))?;
    let mut record = AdoptionRecord::load(&root)?.context("no adoption record")?;
    if let Some(version) = catalog_version
        && version != project_adoption_catalog(&manifest)
    {
        bail!(
            "adoption inventory must match the reviewed project policy; preview the project policy migration first. Nothing changed."
        );
    }
    record.migrate_catalog(catalog_version.unwrap_or(record.catalog_version))?;
    if write {
        replace_record(&root, &before, &record)?;
    } else {
        print!("{}", record.to_yaml()?);
    }
    Ok(())
}

fn prepare_evidence(root: &std::path::Path) -> Result<()> {
    let (root, _) = load_project(root)?;
    AdoptionRecord::load(&root)?.context("no adoption record")?;
    let mut questionnaire = opdev_project::EvidenceBootstrap::new(
        staged_fingerprint(&root)?,
        ["MCD-PIPELINE-001".into()],
        ["OPDEV-WORK-001".into(), "OPDEV-TEST-002".into()],
    );
    questionnaire.change.evidence.push(opdev_core::Evidence {
        kind: "adoption_review".into(),
        summary: String::new(),
        location: Some(ADOPTION_PATH.into()),
    });
    questionnaire.change.evidence.push(opdev_core::Evidence {
        kind: "adoption_cleanup_review".into(),
        summary: String::new(),
        location: Some(ADOPTION_PATH.into()),
    });
    questionnaire.project.evidence.push(opdev_core::Evidence {
        kind: "delivery_gate".into(),
        summary: String::new(),
        location: None,
    });
    print!("{}", questionnaire.to_yaml()?);
    eprintln!(
        "Review-required preparation only; writes no review record and asserts no pass. Fill the actual summaries and references, including the delivery dependency path, then merge reviewed assertions into the selected review record. This partial worksheet is not a full evidence bootstrap answers file."
    );
    Ok(())
}

fn replace_record(root: &std::path::Path, before: &[u8], record: &AdoptionRecord) -> Result<()> {
    use std::io::Write;
    let path = root.join(ADOPTION_PATH);
    if path.is_symlink() || root.join(".opdev").is_symlink() {
        bail!("refusing to replace adoption state through a symlink");
    }
    let yaml = record.to_yaml()?;
    let mut file = tempfile::NamedTempFile::new_in(root.join(".opdev"))?;
    file.write_all(yaml.as_bytes())?;
    file.as_file().sync_all()?;
    if std::fs::read(&path)? != before {
        bail!("adoption record changed during review; retry from a fresh plan");
    }
    file.persist(&path)?;
    Ok(())
}

fn adoption_core_report(
    root: &std::path::Path,
    manifest: &opdev_project::ProjectManifest,
    remote: bool,
    review: Option<&opdev_engine::ValidatedReview>,
) -> Result<opdev_engine::CheckReport> {
    let revision = remote.then(|| crate::clean_remote_revision(root)).flatten();
    let mut report = if let Some(review) = review {
        opdev_engine::evaluate_with_review(root, manifest, CheckOptions::pre_merge(), review, None)?
    } else {
        evaluate(root, manifest, CheckOptions::pre_merge())?
    };
    apply_local_ci(root, manifest, &mut report)?;
    if remote {
        apply_remote_audit(root, manifest, &mut report, revision.as_deref())?;
    }
    Ok(report)
}

#[derive(Clone, Copy)]
struct CheckMode {
    legacy_assessment: bool,
    preflight: bool,
}

fn check(
    root: &std::path::Path,
    remote: bool,
    report_path: Option<&PathBuf>,
    format: OutputFormat,
    review_locator: Option<&PathBuf>,
    review_acceptance_sha256: Option<&str>,
    mode: CheckMode,
) -> Result<ExitCode> {
    let legacy_assessment = mode.legacy_assessment;
    if report_path.is_some_and(|path| path.symlink_metadata().is_ok()) {
        bail!("report output already exists; choose a new path");
    }
    let (root, manifest) = load_project(root)?;
    let record = AdoptionRecord::load(&root)?
        .context("no adoption assessment; run opdev adoption start explicitly")?;
    let mut blockers = decision_blockers(&root, &manifest, &record, legacy_assessment)?;
    if remote && let Some(gap) = manifest.remote_qualification_gap() {
        blockers.push(format!("remote qualification: {gap}; resolve developer choices before running adoption verification"));
    }
    let fingerprint = match staged_fingerprint(&root) {
        Ok(value) => Some(value),
        Err(error) => {
            blockers.push(format!("freshness: {error}"));
            None
        }
    };
    // Do not retrieve provider evidence or execute commands for unresolved choices.
    let review = if blockers.is_empty() {
        crate::selected_review(
            review_locator,
            review_acceptance_sha256,
            &root,
            &manifest,
            opdev_project::TestStage::PreMerge,
        )?
    } else {
        None
    };
    let ledger = if manifest.assurance.review_storage.is_some() {
        if root.join(EVIDENCE_PATH).symlink_metadata().is_ok() {
            blockers.push("adoption review: both external storage and a legacy ledger exist. Complete the reviewed storage migration before verification; no evidence source was silently chosen".into());
        }
        if let Some(review) = &review {
            Some(
                review
                    .reviewed_ledger(&root, &manifest, opdev_project::TestStage::PreMerge)
                    .map_err(anyhow::Error::msg)?
                    .clone(),
            )
        } else {
            blockers.push("adoption review: selected external storage needs --review-locator and --review-acceptance-sha256 for this change. No checks ran".into());
            None
        }
    } else {
        EvidenceLedger::load_optional(&root, &manifest.catalog()?)?
    };
    blockers.extend(input_blockers(
        &root,
        &manifest,
        ledger.as_ref(),
        fingerprint.as_deref(),
        !legacy_assessment,
    ));
    if mode.preflight
        && !preflight_unchanged(
            &root,
            &manifest,
            review.as_ref(),
            ledger.as_ref(),
            fingerprint.as_deref(),
        )?
    {
        blockers.push("Adoption inputs changed during inspection; inspect the current source and selected review before retrying. No project checks ran.".into());
    }
    // Never run project commands until decisions and their explicit review are ready.
    let core_report = if blockers.is_empty() && !mode.preflight {
        let (report, verification_gaps) = verify_core(
            &root,
            &manifest,
            &record,
            remote,
            review.as_ref(),
            fingerprint.as_deref(),
        )?;
        blockers.extend(verification_gaps);
        if let Some(path) = report_path {
            views::save_report(&report, path)?;
        }
        Some(report)
    } else {
        None
    };
    let passed = blockers.is_empty();
    present_adoption(
        format,
        passed,
        mode,
        fingerprint.as_deref(),
        &blockers,
        core_report.as_ref(),
    )?;
    Ok(if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn decision_blockers(
    root: &std::path::Path,
    manifest: &opdev_project::ProjectManifest,
    record: &AdoptionRecord,
    legacy: bool,
) -> Result<Vec<String>> {
    let mut blockers = record.blockers(manifest)?;
    if !legacy {
        blockers.extend(opdev_project::clean_adoption::decision_gaps(
            manifest,
            Some(record),
        ));
        if let Some(target) = &record.clean_target {
            blockers.extend(
                opdev_project::clean_adoption::retirement_gaps(root, target)
                    .map_err(anyhow::Error::msg)?,
            );
        }
    }
    Ok(blockers)
}

fn preflight_unchanged(
    root: &std::path::Path,
    manifest: &opdev_project::ProjectManifest,
    review: Option<&opdev_engine::ValidatedReview>,
    ledger: Option<&EvidenceLedger>,
    fingerprint: Option<&str>,
) -> Result<bool> {
    if let Some(review) = review {
        // Input gaps were already collected; this repeats current authority/source
        // checks at the end, never turns the readiness inspection into execution.
        review
            .preflight(root, manifest, opdev_project::TestStage::PreMerge)
            .map_err(anyhow::Error::msg)?;
    } else if manifest.assurance.review_storage.is_none()
        && EvidenceLedger::load_optional(root, &manifest.catalog()?)?.as_ref() != ledger
    {
        return Ok(false);
    }
    Ok(staged_fingerprint(root).ok().as_deref() == fingerprint)
}

fn input_blockers(
    root: &std::path::Path,
    manifest: &opdev_project::ProjectManifest,
    ledger: Option<&EvidenceLedger>,
    fingerprint: Option<&str>,
    clean_target: bool,
) -> Vec<String> {
    let mut blockers = review_blockers(ledger, fingerprint, clean_target);
    blockers.extend(delivery_input_blockers(manifest, ledger, fingerprint));
    blockers.extend(opdev_engine::acceptance_input_gaps(
        root,
        manifest,
        ledger,
        fingerprint,
        opdev_project::TestStage::PreMerge,
    ));
    for name in ["AGENTS.md", "CLAUDE.md"] {
        if !root.join(name).is_file() {
            blockers.push(format!("agents: missing {name}"));
        }
    }
    blockers
}

fn present_adoption(
    format: OutputFormat,
    passed: bool,
    mode: CheckMode,
    fingerprint: Option<&str>,
    blockers: &[String],
    core_report: Option<&opdev_engine::CheckReport>,
) -> Result<()> {
    let complete = passed && !mode.legacy_assessment && !mode.preflight;
    if matches!(format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema":1,"complete":complete,"target":opdev_project::clean_adoption::TARGET,
                "legacy_policy_passed":(mode.legacy_assessment && !mode.preflight).then_some(passed),
                "inputs_ready": mode.preflight.then_some(passed),
                "checks_ran": core_report.is_some(),
                "fingerprint":fingerprint,"blockers":blockers,"core_report":core_report
            }))?
        );
    } else {
        if let Some(report) = core_report {
            print_human_report(report);
        }
        if mode.preflight {
            println!(
                "Adoption inputs are {} for verification. No project checks ran; adoption is not complete.",
                if passed { "ready" } else { "not ready" }
            );
        } else if mode.legacy_assessment {
            println!(
                "Legacy policy assessment: {}. Current adoption is not verified.",
                if passed { "passed" } else { "blocked" }
            );
        } else {
            println!(
                "Adoption: {}",
                if complete {
                    "passed for the evaluated staged state"
                } else {
                    "incomplete"
                }
            );
        }
        for blocker in blockers {
            println!("  {blocker}");
        }
    }
    Ok(())
}

fn delivery_input_blockers(
    manifest: &opdev_project::ProjectManifest,
    ledger: Option<&EvidenceLedger>,
    fingerprint: Option<&str>,
) -> Vec<String> {
    let assertion = ledger.and_then(|ledger| {
        fingerprint
            .and_then(|f| ledger.matching_change(f))
            .and_then(|c| {
                c.assertions
                    .iter()
                    .find(|a| a.rule_id.as_str() == "MCD-PIPELINE-001")
            })
            .or_else(|| {
                ledger
                    .project
                    .iter()
                    .find(|a| a.rule_id.as_str() == "MCD-PIPELINE-001")
            })
    });
    if assertion.is_some_and(|a| {
        (a.outcome == Outcome::Passed
            && a.evidence.iter().any(|e| {
                e.kind == "delivery_gate"
                    && e.location.as_ref().is_some_and(|p| !p.trim().is_empty())
            }))
            || (manifest.assurance.engineering.is_some() && a.outcome == Outcome::NotApplicable)
    }) {
        Vec::new()
    } else {
        vec!["delivery: review the actual release/tag publication dependency path and provide MCD-PIPELINE-001 delivery_gate evidence; integration-only CI is insufficient".into()]
    }
}

fn verify_core(
    root: &std::path::Path,
    manifest: &opdev_project::ProjectManifest,
    record: &AdoptionRecord,
    remote: bool,
    review: Option<&opdev_engine::ValidatedReview>,
    fingerprint: Option<&str>,
) -> Result<(opdev_engine::CheckReport, Vec<String>)> {
    let evidence_before = if review.is_some() {
        None
    } else {
        Some(std::fs::read(root.join(EVIDENCE_PATH))?)
    };
    let report = adoption_core_report(root, manifest, remote, review)?;
    let mut blockers = Vec::new();
    if !report.rules.iter().any(|rule| {
        rule.rule_id.as_str() == "MCD-PIPELINE-001"
            && ((rule.outcome == Outcome::Passed
                && rule.evidence.iter().any(|e| {
                    e.kind == "delivery_gate"
                        && e.location.as_ref().is_some_and(|p| !p.trim().is_empty())
                }))
                || (manifest.assurance.engineering.is_some()
                    && rule.outcome == Outcome::NotApplicable))
    }) {
        blockers.push("delivery: review the actual release/tag publication dependency path and provide MCD-PIPELINE-001 delivery_gate evidence; integration-only CI is insufficient".into());
    }
    for gate in [
        Gate::Development,
        Gate::Integration,
        Gate::Delivery,
        Gate::Compliance,
    ] {
        if !report.gate_passed(gate) {
            blockers.push(format!("core gate {gate:?} is blocked"));
        }
    }
    blockers.extend(referenced_suite_blockers(record, manifest, &report));
    let evidence_unchanged = if let Some(review) = review {
        root.join(EVIDENCE_PATH).symlink_metadata().is_err()
            && review
                .reviewed_ledger(root, manifest, opdev_project::TestStage::PreMerge)
                .is_ok()
    } else {
        Some(std::fs::read(root.join(EVIDENCE_PATH))?) == evidence_before
    };
    if staged_fingerprint(root).ok().as_deref() != fingerprint || !evidence_unchanged {
        blockers.push(
            "repository or reviewed evidence changed during verification; stage and review again"
                .into(),
        );
    }
    Ok((report, blockers))
}

fn referenced_suite_blockers(
    record: &AdoptionRecord,
    manifest: &opdev_project::ProjectManifest,
    report: &opdev_engine::CheckReport,
) -> Vec<String> {
    let mut blockers = Vec::new();
    for decision in record.practices.values() {
        for suite in &decision.suites {
            if manifest.assurance.engineering.is_some()
                && manifest.testing.suites.iter().any(|declared| {
                    &declared.id == suite
                        && !declared
                            .stages
                            .contains(&opdev_project::TestStage::PreMerge)
                })
            {
                // This invocation verifies the pre-merge boundary only. Existing
                // post-merge gate evidence remains independently required.
                continue;
            }
            if !report
                .checks
                .iter()
                .any(|check| check.id == *suite && check.outcome == Outcome::Passed)
            {
                blockers.push(format!("suite `{suite}` did not pass in this evaluation"));
            }
        }
    }
    blockers
}

fn review_blockers(
    ledger: Option<&EvidenceLedger>,
    fingerprint: Option<&str>,
    clean_target: bool,
) -> Vec<String> {
    let reviewed = fingerprint.and_then(|fingerprint| ledger?.matching_change(fingerprint));
    let mut blockers = Vec::new();
    if fingerprint.is_none() {
        return vec!["adoption review: staged fingerprint unavailable; resolve the reported unstaged/untracked inputs first".into()];
    }
    if reviewed.is_none() {
        return vec!["adoption review: no selected review record matches the current staged fingerprint; review this state rather than copying old assertions".into()];
    }
    for id in ["OPDEV-WORK-001", "OPDEV-TEST-002"] {
        if !reviewed.is_some_and(|change| {
            change.assertions.iter().any(|assertion| {
                assertion.rule_id.as_str() == id
                    && assertion.outcome == Outcome::Passed
                    && assertion.evidence.iter().any(|evidence| {
                        evidence.kind == "adoption_review"
                            && evidence.location.as_deref() == Some(ADOPTION_PATH)
                    })
            })
        }) {
            blockers.push(format!(
                "{id}: needs a current fingerprint-bound adoption_review of {ADOPTION_PATH}; check fingerprint separately from evidence kind/location; use adoption prepare-evidence for an unresolved worksheet"
            ));
        }
    }
    if clean_target
        && !reviewed.is_some_and(|change| {
            change.assertions.iter().any(|assertion| {
                assertion.rule_id.as_str() == "OPDEV-WORK-001"
                    && assertion.outcome == Outcome::Passed
                    && assertion.evidence.iter().any(|e| {
                        e.kind == "adoption_cleanup_review"
                            && e.location.as_deref() == Some(ADOPTION_PATH)
                    })
            })
        })
    {
        blockers.push("adoption cleanup: review actual retained content, retired paths and updated references for this source; provide adoption_cleanup_review evidence. A clean filename layout alone is insufficient".into());
    }
    blockers
}
