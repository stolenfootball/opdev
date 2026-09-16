//! Agent-guided adoption with deterministic completion verification.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use opdev_core::{Gate, Outcome, embedded_catalog};
use opdev_engine::{CheckOptions, evaluate};
use opdev_project::{
    ADOPTION_PATH, AdoptionRecord, AdoptionReview, EVIDENCE_PATH, EvidenceLedger, adoption_catalog,
    staged_fingerprint,
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
    },
    /// Print unresolved, fingerprint-bound adoption evidence with the correct kind/location.
    PrepareEvidence {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Inspect the versioned, tool-neutral practice catalog.
    Catalog,
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
    },
    /// Verify all decisions, fresh reviewed evidence, executable checks and all core gates.
    Check {
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
        AdoptionCommand::Plan { root } => plan(root)?,
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
        AdoptionCommand::Migrate { root, write } => migrate(root, *write)?,
        AdoptionCommand::PrepareEvidence { root } => prepare_evidence(root)?,
        AdoptionCommand::Catalog => {
            println!("{}", serde_json::to_string_pretty(&adoption_catalog()?)?);
        }
        AdoptionCommand::Start { root, dry_run } => {
            let (root, _) = load_project(root)?;
            let existing = AdoptionRecord::load(&root)?;
            let exists = existing.is_some();
            let record = existing.map_or_else(AdoptionRecord::pending, Ok)?;
            if *dry_run {
                print!("{}", record.to_yaml()?);
            } else if exists {
                println!("Existing adoption decisions preserved; use adoption status.");
            } else {
                record.write_new(&root)?;
                println!(
                    "Created {ADOPTION_PATH}; all practices are pending. Adoption is not complete."
                );
            }
        }
        AdoptionCommand::Status { root, format } => {
            let (root, manifest) = load_project(root)?;
            let record = AdoptionRecord::load(&root)?;
            let blockers = record
                .as_ref()
                .map(|r| r.blockers(&manifest))
                .transpose()?
                .unwrap_or_default();
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
                            "verification":"not_run","blockers":blockers,"record":record})
                    )?
                );
            } else {
                println!("Adoption: {status} (status does not run checks or certify completion)");
                if let Some(record) = record {
                    for practice in adoption_catalog()?.practices {
                        println!(
                            "{}: {:?} — {}",
                            practice.id, record.practices[&practice.id].state, practice.title
                        );
                    }
                } else {
                    println!(
                        "Use opdev adoption start to explicitly assess this existing project."
                    );
                }
                for blocker in blockers {
                    println!("  {blocker}");
                }
            }
        }
        AdoptionCommand::Check {
            root,
            remote,
            report,
            format,
        } => return check(root, *remote, report.as_ref(), *format),
    }
    Ok(ExitCode::SUCCESS)
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
    record.review = Some(review);
    replace_record(&root, &before, &record)?;
    println!(
        "Recorded approval claim; implementation and adoption completion remain separately verified. This does not authenticate reviewer identity."
    );
    Ok(())
}

fn plan(root: &std::path::Path) -> Result<()> {
    let (root, manifest) = load_project(root)?;
    let record = AdoptionRecord::load(&root)?.context("no adoption record")?;
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
            "schema": 1, "plan_id": record.plan_id(&manifest)?, "record": record,
            "contract": manifest, "approval": "review_required",
            "branch_choices": branch_choices,
            "notice": "Present preserve/change/ignore/unresolved choices and wait for an actual response. A plan hash is not consent."
        }))?
    );
    Ok(())
}

fn migrate(root: &std::path::Path, write: bool) -> Result<()> {
    let (root, _) = load_project(root)?;
    let before = std::fs::read(root.join(ADOPTION_PATH))?;
    let mut record = AdoptionRecord::load(&root)?.context("no adoption record")?;
    if record.schema == 1 {
        record.schema = 2;
        record.review = None;
    }
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
        [],
        ["OPDEV-WORK-001".into(), "OPDEV-TEST-002".into()],
    );
    questionnaire.change.evidence.push(opdev_core::Evidence {
        kind: "adoption_review".into(),
        summary: String::new(),
        location: Some(ADOPTION_PATH.into()),
    });
    print!("{}", questionnaire.to_yaml()?);
    eprintln!(
        "Review-required preparation only; writes no ledger and asserts no pass. Fill the actual review summary/work reference, then merge reviewed assertions into the current ledger. This partial worksheet is not a full evidence bootstrap answers file."
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

fn check(
    root: &std::path::Path,
    remote: bool,
    report_path: Option<&PathBuf>,
    format: OutputFormat,
) -> Result<ExitCode> {
    if report_path.is_some_and(|path| path.symlink_metadata().is_ok()) {
        bail!("report output already exists; choose a new path");
    }
    let (root, manifest) = load_project(root)?;
    let record = AdoptionRecord::load(&root)?
        .context("no adoption assessment; run opdev adoption start explicitly")?;
    let mut blockers = record.blockers(&manifest)?;
    let fingerprint = match staged_fingerprint(&root) {
        Ok(value) => Some(value),
        Err(error) => {
            blockers.push(format!("freshness: {error}"));
            None
        }
    };
    blockers.extend(review_blockers(&root, fingerprint.as_deref())?);
    for name in ["AGENTS.md", "CLAUDE.md"] {
        if !root.join(name).is_file() {
            blockers.push(format!("agents: missing {name}"));
        }
    }
    // Never run project commands until decisions and their explicit review are ready.
    let core_report = if blockers.is_empty() {
        let evidence_before = std::fs::read(root.join(EVIDENCE_PATH))?;
        let mut report = evaluate(&root, &manifest, CheckOptions::pre_merge())?;
        apply_local_ci(&root, &manifest, &mut report)?;
        if remote {
            apply_remote_audit(&manifest, &mut report)?;
        }
        if !report.rules.iter().any(|rule| {
            rule.rule_id.as_str() == "MCD-PIPELINE-001"
                && rule.outcome == Outcome::Passed
                && rule.evidence.iter().any(|e| {
                    e.kind == "delivery_gate"
                        && e.location.as_ref().is_some_and(|p| !p.trim().is_empty())
                })
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
        for decision in record.practices.values() {
            for suite in &decision.suites {
                if !report
                    .checks
                    .iter()
                    .any(|check| check.id == *suite && check.outcome == Outcome::Passed)
                {
                    blockers.push(format!("suite `{suite}` did not pass in this evaluation"));
                }
            }
        }
        if staged_fingerprint(&root).ok() != fingerprint
            || std::fs::read(root.join(EVIDENCE_PATH))? != evidence_before
        {
            blockers.push("repository or reviewed evidence changed during verification; stage and review again".into());
        }
        if let Some(path) = report_path {
            views::save_report(&report, path)?;
        }
        Some(report)
    } else {
        None
    };
    let complete = blockers.is_empty();
    if matches!(format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema":1,"complete":complete,"fingerprint":fingerprint,"blockers":blockers,"core_report":core_report
            }))?
        );
    } else {
        if let Some(report) = &core_report {
            print_human_report(report);
        }
        println!(
            "Adoption: {}",
            if complete {
                "passed for the evaluated staged state"
            } else {
                "incomplete"
            }
        );
        for blocker in blockers {
            println!("  {blocker}");
        }
    }
    Ok(if complete {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn review_blockers(root: &std::path::Path, fingerprint: Option<&str>) -> Result<Vec<String>> {
    let ledger = EvidenceLedger::load_optional(root, &embedded_catalog()?)?;
    let reviewed =
        fingerprint.and_then(|fingerprint| ledger.as_ref()?.matching_change(fingerprint));
    let mut blockers = Vec::new();
    if fingerprint.is_none() {
        return Ok(vec!["adoption review: staged fingerprint unavailable; resolve the reported unstaged/untracked inputs first".into()]);
    }
    if reviewed.is_none() {
        return Ok(vec!["adoption review: no ledger change matches the current staged fingerprint; review this state rather than copying old assertions".into()]);
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
    Ok(blockers)
}
