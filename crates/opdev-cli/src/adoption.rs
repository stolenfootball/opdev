//! Agent-guided adoption with deterministic completion verification.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use opdev_core::{Gate, Outcome, embedded_catalog};
use opdev_engine::{CheckOptions, evaluate};
use opdev_project::{
    ADOPTION_PATH, AdoptionRecord, EVIDENCE_PATH, EvidenceLedger, adoption_catalog,
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
            let status = if record.is_none() {
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
                        &serde_json::json!({"schema":1,"status":status,"blockers":blockers,"record":record})
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
                "{id}: needs a current fingerprint-bound adoption_review of {ADOPTION_PATH}"
            ));
        }
    }
    Ok(blockers)
}
