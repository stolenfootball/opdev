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

mod coordinator;

#[derive(Debug, Args)]
pub(super) struct AdoptionArgs {
    #[command(subcommand)]
    command: AdoptionCommand,
}

#[derive(Debug, Subcommand)]
enum AdoptionCommand {
    /// Derive prerequisites and next actions from current authorities; no commands, writes or consent.
    Next {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Include current read-only provider policy observations, not remote qualification.
        #[arg(long)]
        remote: bool,
    },
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
        /// Produce a complete bootstrap input with unresolved catalog and safeguard review slots.
        #[arg(long)]
        complete: bool,
        /// Exact reviewed target commit, required for the selected requirements policy.
        #[arg(long, requires = "complete")]
        base: Option<String>,
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
        /// Resolve the exact native CI review selection using the built-in job identity.
        #[arg(long, conflicts_with_all = ["review_locator", "review_acceptance_sha256"])]
        review_ci: bool,
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
        AdoptionCommand::Next { root, remote } => {
            let discovery = opdev_project::discover(root)?;
            let record = AdoptionRecord::load(&discovery.root)?;
            let mut observed =
                coordinator::inspect(&discovery.root, &discovery.manifest, record.as_ref())?;
            observed["remote_observation"] =
                serde_json::to_value(policy_worksheet(&discovery.manifest, *remote)?)?;
            observed["remote_qualification"] = serde_json::json!("not_run");
            println!("{}", serde_json::to_string_pretty(&observed)?);
        }
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
        AdoptionCommand::PrepareEvidence {
            root,
            complete,
            base,
        } => {
            if *complete {
                crate::bootstrap_evidence(&crate::EvidenceBootstrapArgs {
                    adoption: true,
                    base: base.clone(),
                    root: root.clone(),
                    output: None,
                    answers: None,
                    write: false,
                })?;
            } else {
                prepare_evidence(root)?;
            }
        }
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
            review_ci,
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
                    remote: *remote,
                    review_ci: *review_ci,
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
            "target": opdev_project::clean_adoption::target_name(&manifest),
            "starting_state": if configured {"configured_project"} else {"uninitialized_project"},
            "policy_selection": policy_selection(configured),
            "required_migrations": target_gaps,
            "required_practices": adoption_catalog_version(project_adoption_catalog(&manifest).max(2))?,
            "discovery_warnings": discovery.warnings,
            "contract": manifest, "approval": "review_required",
            "branch_choices": branch_choices,
            "remote_qualification": {"verification":"not_run", "policy_ready":manifest.remote_qualification_gap().is_none(), "gap":manifest.remote_qualification_gap(), "worksheet":policy_worksheet(&manifest, remote)?},
            "delivery_readiness":"not_run",
            "notice": "Review retained, changed and retired content against the selected destination. Engineering policy 3 selects clean-4, capability safeguards 2 and adoption inventory 3. Existing policy choices are not migrated automatically. An empty project needs real behavior before verification; setup alone is incomplete. Reuse adequate tools and actual decisions, resolve material choices, and reopen changed duties. A plan hash is not consent."
        }))?
    );
    Ok(())
}

pub(super) fn policy_selection(configured: bool) -> serde_json::Value {
    serde_json::json!({
        "status": if configured {"selected_by_existing_contract"} else {"unselected_discovery_proposal"},
        "supported_current_targets":[
            {"engineering_policy":"2","target":"clean-3","project_schema":4,"safeguards":1,"adoption_inventory":2},
            {"engineering_policy":"3","target":"clean-4","project_schema":4,"safeguards":2,"adoption_inventory":3}
        ],
        "initialization":"Use init --project FILE with the actual reviewed contract. The older --engineering-policy shorthand accepts only policies 1/2; it does not limit the policies accepted by --project.",
        "meaning":"The target field describes this discovered or selected contract, not the runtime's maximum capability. Discovery is not a policy choice, approval or a reason to downgrade or install another runtime. Existing policy changes require reviewed migration."
    })
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
    let (root, manifest) = load_project(root)?;
    AdoptionRecord::load(&root)?.context("no adoption record")?;
    let policy_two = delivery_rule(&manifest) == "OPDEV-PIPELINE-001";
    let project_rules = if policy_two {
        vec![]
    } else {
        vec!["MCD-PIPELINE-001".into()]
    };
    let mut change_rules = vec!["OPDEV-WORK-001".into(), "OPDEV-TEST-002".into()];
    if policy_two {
        change_rules.push("OPDEV-PIPELINE-001".into());
    }
    let mut questionnaire = opdev_project::EvidenceBootstrap::new(
        staged_fingerprint(&root)?,
        project_rules,
        change_rules,
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
    let delivery_evidence = opdev_core::Evidence {
        kind: "delivery_gate".into(),
        summary: String::new(),
        location: None,
    };
    if policy_two {
        questionnaire.change.evidence.push(delivery_evidence);
    } else {
        questionnaire.project.evidence.push(delivery_evidence);
    }
    print!("{}", questionnaire.to_yaml()?);
    eprintln!(
        "Review-required preparation only; writes no review record and asserts no pass. Fill the actual summaries and references, including the delivery dependency path, then merge reviewed assertions into the selected review record. This partial worksheet is not a full evidence bootstrap answers file."
    );
    Ok(())
}

pub(super) fn complete_review_candidates(
    manifest: &opdev_project::ProjectManifest,
    project: &mut Vec<String>,
    change: &mut Vec<String>,
) {
    for id in ["OPDEV-WORK-001", "OPDEV-TEST-002"] {
        if !change.iter().any(|v| v == id) {
            change.push(id.into());
        }
    }
    // Current-policy qualification requires an exact-change decision even for
    // justified non-applicability. A durable project assertion cannot supply it.
    if manifest
        .assurance
        .engineering
        .as_ref()
        .is_some_and(|policy| matches!(policy.version.as_str(), "2" | "3"))
    {
        for id in [
            "OPDEV-FLOW-001",
            "OPDEV-DELIVERY-001",
            "OPDEV-PIPELINE-001",
            "OPDEV-RECOVERY-001",
        ] {
            project.retain(|candidate| candidate != id);
            if !change.iter().any(|candidate| candidate == id) {
                change.push(id.into());
            }
        }
    }
    let rule = delivery_rule(manifest);
    let destination = if rule == "OPDEV-PIPELINE-001" {
        change
    } else {
        project
    };
    if !destination.iter().any(|v| v == rule) {
        destination.push(rule.into());
    }
}

pub(super) fn complete_review_template(
    root: &std::path::Path,
    manifest: &opdev_project::ProjectManifest,
    mut questionnaire: opdev_project::EvidenceBootstrap,
    base: Option<&str>,
) -> Result<serde_json::Value> {
    use serde_json::json;
    opdev_project::AdoptionRecord::load(root)?
        .context("Start the reviewed adoption assessment before preparing its review")?;
    for kind in ["adoption_review", "adoption_cleanup_review"] {
        questionnaire.change.evidence.push(opdev_core::Evidence {
            kind: kind.into(),
            summary: String::new(),
            location: Some(ADOPTION_PATH.into()),
        });
    }
    let evidence = opdev_core::Evidence {
        kind: "delivery_gate".into(),
        summary: String::new(),
        location: None,
    };
    if delivery_rule(manifest) == "OPDEV-PIPELINE-001" {
        questionnaire.change.evidence.push(evidence);
    } else {
        questionnaire.project.evidence.push(evidence);
    }
    let mut value = serde_json::to_value(questionnaire)?;
    if manifest.assurance.requirements.is_some() {
        let base = base.context("Complete adoption preparation needs --base with the provider-observed target commit; no baseline was guessed")?;
        anyhow::ensure!(
            matches!(base.len(), 40 | 64)
                && base
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "Use the full target commit, not a moving branch name"
        );
        value["change"]["acceptance"]["requirements"] = json!({
            "catalog_sha256":opdev_project::requirements::load_index(root)?.digest()?,
            "baseline_commit":base,
            "baseline_catalog_sha256":opdev_project::requirements::load_revision(root, base)?.digest()?,
            "rationale":"", "decision_reference":"", "manual_observations":[],
        });
    }
    if let Some(policy) = &manifest.assurance.safeguards {
        let mut impacts = serde_json::Map::new();
        let mut objectives = serde_json::Map::new();
        for capability in opdev_project::Capability::for_version(policy.version) {
            if !policy
                .capabilities
                .get(capability)
                .is_some_and(|fact| fact.state == opdev_project::CapabilityState::Absent)
            {
                let name = serde_json::to_value(capability)?
                    .as_str()
                    .context("capability")?
                    .to_owned();
                // Null is an unresolved question, not an invented impact selection.
                impacts.insert(name, json!({"impact":null,"rationale":""}));
                for objective in capability.objectives_for_version(policy.version) {
                    let name = serde_json::to_value(objective)?
                        .as_str()
                        .context("objective")?
                        .to_owned();
                    objectives.insert(name, json!([]));
                }
            }
        }
        value["change"]["acceptance"]["safeguards"] =
            json!({"version":policy.version,"impacts":impacts,"objectives":objectives});
    }
    eprintln!(
        "Complete unresolved adoption questionnaire. Resolve null impacts and actual objective mappings; remove objectives only after reviewing them as unaffected. Fill the real review summaries, conditions and tests. Use evidence bootstrap --adoption --answers after review. No checks, approval or writes occurred."
    );
    Ok(value)
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
#[allow(clippy::struct_excessive_bools)] // Independent CLI controls; not persisted workflow state or permission.
struct CheckMode {
    remote: bool,
    review_ci: bool,
    legacy_assessment: bool,
    preflight: bool,
}

struct Findings<'a> {
    messages: &'a [String],
    decision_end: usize,
    input_end: usize,
}

impl Findings<'_> {
    fn structured(&self) -> Vec<serde_json::Value> {
        self.messages.iter().enumerate().map(|(index, message)| {
            let (code, next_action) = if index < self.decision_end {
                ("adoption.prerequisites", "Inspect adoption next and resolve the current policy, decision, cleanup or source prerequisite. Do not run checks yet.")
            } else if index < self.input_end {
                ("adoption.review_inputs", "Resolve the named field in the current complete review input, select its exact review identity, then repeat preflight. Do not invent a passing assertion.")
            } else {
                ("adoption.verification", "Inspect the current check report and failed boundary. Correct the cause and verify again; no saved report can replace execution.")
            };
            serde_json::json!({"code":code,"message":message,"next_action":next_action})
        }).collect()
    }
}

#[allow(clippy::too_many_lines)] // Keep prerequisite, review, execution and freshness boundaries in one auditable order.
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
    let decision_end = blockers.len();
    let review_deferred = !blockers.is_empty()
        && manifest.assurance.review_storage.is_some()
        && (mode.review_ci || (review_locator.is_some() && review_acceptance_sha256.is_some()));
    // Do not retrieve provider evidence or execute commands for unresolved choices.
    let (review, ci_selection) = if blockers.is_empty() && mode.review_ci {
        crate::selected_ci_review(&root, &manifest, opdev_project::TestStage::PreMerge)?
    } else if blockers.is_empty() {
        (
            crate::selected_review(
                review_locator,
                review_acceptance_sha256,
                &root,
                &manifest,
                opdev_project::TestStage::PreMerge,
            )?,
            None,
        )
    } else {
        (None, None)
    };
    let ledger = adoption_ledger(
        &root,
        &manifest,
        review.as_ref(),
        review_deferred,
        &mut blockers,
    )?;
    blockers.extend(input_blockers(
        &root,
        &manifest,
        ledger.as_ref(),
        fingerprint.as_deref(),
        !legacy_assessment,
        review_deferred,
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
    let input_end = blockers.len();
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
    if let Some(selection) = ci_selection {
        selection.recheck().map_err(anyhow::Error::msg)?;
    }
    let passed = blockers.is_empty();
    present_adoption(
        format,
        passed,
        mode,
        fingerprint.as_deref(),
        &Findings {
            messages: &blockers,
            decision_end,
            input_end,
        },
        core_report.as_ref(),
        opdev_project::clean_adoption::target_name(&manifest),
    )?;
    Ok(if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn adoption_ledger(
    root: &std::path::Path,
    manifest: &opdev_project::ProjectManifest,
    review: Option<&opdev_engine::ValidatedReview>,
    review_deferred: bool,
    blockers: &mut Vec<String>,
) -> Result<Option<EvidenceLedger>> {
    if manifest.assurance.review_storage.is_none() {
        return Ok(EvidenceLedger::load_optional(root, &manifest.catalog()?)?);
    }
    if root.join(EVIDENCE_PATH).symlink_metadata().is_ok() {
        blockers.push("adoption review: both external storage and a legacy ledger exist. Complete the reviewed storage migration before verification; no evidence source was silently chosen".into());
    }
    if let Some(review) = review {
        return Ok(Some(
            review
                .reviewed_ledger(root, manifest, opdev_project::TestStage::PreMerge)
                .map_err(anyhow::Error::msg)?
                .clone(),
        ));
    }
    blockers.push(if review_deferred {
        "adoption review: review arguments were supplied, but the selected record was not inspected because adoption decisions or source readiness are unresolved. Resolve those blockers, then retry; review validity is unknown and no checks ran".into()
    } else {
        "adoption review: selected external storage needs --review-locator and --review-acceptance-sha256 for this change. No checks ran".into()
    });
    Ok(None)
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
    review_deferred: bool,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if !review_deferred {
        blockers.extend(review_blockers(ledger, fingerprint, clean_target));
        blockers.extend(delivery_input_blockers(manifest, ledger, fingerprint));
        blockers.extend(opdev_engine::acceptance_input_gaps(
            root,
            manifest,
            ledger,
            fingerprint,
            opdev_project::TestStage::PreMerge,
        ));
    }
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
    findings: &Findings<'_>,
    core_report: Option<&opdev_engine::CheckReport>,
    target: &str,
) -> Result<()> {
    let blockers = findings.messages;
    let controls_verified = passed && !mode.legacy_assessment && !mode.preflight;
    let integration_qualified =
        mode.remote && core_report.is_some_and(|report| integrated_qualification(&report.rules));
    let complete = controls_verified && integration_qualified;
    if matches!(format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema":1,"complete":complete,"target":target,
                "controls_verified":controls_verified,
                "integration_qualified":integration_qualified,
                "legacy_policy_passed":(mode.legacy_assessment && !mode.preflight).then_some(passed),
                "inputs_ready": mode.preflight.then_some(passed),
                "qualification_scope": "project_adoption",
                "artifact_delivery": "not_evaluated",
                "checks_ran": core_report.is_some(),
                "fingerprint":fingerprint,"blockers":blockers,"diagnostics":findings.structured(),"core_report":core_report
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
                "Qualification scope: current adoption controls and any explicitly requested integrated CI observation. Artifact delivery was not evaluated; no release is authorized."
            );
            println!(
                "Adoption: {}",
                if complete {
                    "passed for the evaluated staged state"
                } else if controls_verified {
                    "local controls verified; integrated CI not yet qualified. Reconcile current trunk with ci qualify before reporting adoption complete. Do not rerun suites merely to refresh remote status"
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

fn integrated_qualification(rules: &[opdev_core::RuleResult]) -> bool {
    rules.iter().any(|rule| {
        rule.rule_id.as_str() == "MCD-CI-001"
            && rule.outcome == Outcome::Passed
            && rule.verifier == opdev_core::VerificationSource::Remote
            && rule
                .evidence
                .iter()
                .any(|e| e.kind == "remote_ci_qualification")
    })
}

fn delivery_input_blockers(
    manifest: &opdev_project::ProjectManifest,
    ledger: Option<&EvidenceLedger>,
    fingerprint: Option<&str>,
) -> Vec<String> {
    let rule_id = delivery_rule(manifest);
    let assertion = ledger.and_then(|ledger| {
        fingerprint
            .and_then(|f| ledger.matching_change(f))
            .and_then(|c| c.assertions.iter().find(|a| a.rule_id.as_str() == rule_id))
            .or_else(|| {
                // Policy 2 requires current-change review; historical project
                // assertions cannot stand in for this boundary's verification.
                (rule_id == "MCD-PIPELINE-001")
                    .then(|| {
                        ledger
                            .project
                            .iter()
                            .find(|a| a.rule_id.as_str() == rule_id)
                    })
                    .flatten()
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
        vec![delivery_review_gap(manifest)]
    }
}

fn delivery_rule(manifest: &opdev_project::ProjectManifest) -> &'static str {
    if manifest
        .assurance
        .engineering
        .as_ref()
        .is_some_and(|p| matches!(p.version.as_str(), "2" | "3"))
    {
        "OPDEV-PIPELINE-001"
    } else {
        "MCD-PIPELINE-001"
    }
}

fn delivery_review_gap(manifest: &opdev_project::ProjectManifest) -> String {
    let current = if delivery_rule(manifest) == "OPDEV-PIPELINE-001" {
        " Current policy-2 acceptance links and execution are also required."
    } else {
        ""
    };
    format!(
        "delivery: review the actual supported publication dependency path and provide {} delivery_gate evidence; integration-only CI is insufficient.{current} This review does not authorize publication",
        delivery_rule(manifest)
    )
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
        rule.rule_id.as_str() == delivery_rule(manifest)
            && ((rule.outcome == Outcome::Passed
                && rule.evidence.iter().any(|e| {
                    e.kind == "delivery_gate"
                        && e.location.as_ref().is_some_and(|p| !p.trim().is_empty())
                }))
                || (manifest.assurance.engineering.is_some()
                    && rule.outcome == Outcome::NotApplicable))
    }) {
        blockers.push(delivery_review_gap(manifest));
    }
    for gate in [
        Gate::Development,
        Gate::Integration,
        Gate::Delivery,
        Gate::Compliance,
    ] {
        if adoption_gate_blocked(manifest, &report, gate) {
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

fn adoption_gate_blocked(
    manifest: &opdev_project::ProjectManifest,
    report: &opdev_engine::CheckReport,
    gate: Gate,
) -> bool {
    if manifest
        .assurance
        .engineering
        .as_ref()
        .is_none_or(|p| !matches!(p.version.as_str(), "2" | "3"))
    {
        return !report.gate_passed(gate);
    }
    // Adoption checks the implemented controls, not a new release's bytes. Keep
    // every original gate result visible; do not relabel it passed. Known artifact
    // failures and required additional-standard findings still block adoption.
    report
        .gates
        .iter()
        .find(|g| g.gate == gate)
        .is_none_or(|g| {
            !g.blocking_checks.is_empty()
                || g.blocking_rules.iter().any(|id| {
                    !opdev_engine::artifact_scoped_rule(id.as_str())
                        || !report
                            .rules
                            .iter()
                            .any(|r| r.rule_id == *id && r.outcome == Outcome::Unverified)
                })
        })
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

#[cfg(test)]
mod scope_tests {
    use super::*;
    #[test]
    fn local_or_attributed_ci_success_cannot_claim_integrated_adoption() -> Result<()> {
        use opdev_core::{Evidence, RuleResult, VerificationSource};
        let mut rule = RuleResult {
            rule_id: "MCD-CI-001".parse()?,
            catalog_version: 2,
            outcome: Outcome::Passed,
            subject: "synthetic exact revision".into(),
            verifier: VerificationSource::Remote,
            evaluated_at: 0,
            evidence: vec![Evidence {
                kind: "remote_ci_qualification".into(),
                summary: "synthetic observed run".into(),
                location: None,
            }],
            diagnostic: None,
        };
        assert!(integrated_qualification(&[rule.clone()]));
        assert!(!integrated_qualification(&[]));
        for source in [
            VerificationSource::Ci,
            VerificationSource::Evidence,
            VerificationSource::Agent,
        ] {
            rule.verifier = source;
            assert!(!integrated_qualification(&[rule.clone()]));
        }
        rule.verifier = VerificationSource::Remote;
        for outcome in [
            Outcome::Unverified,
            Outcome::Failed,
            Outcome::Error,
            Outcome::NotApplicable,
        ] {
            rule.outcome = outcome;
            assert!(!integrated_qualification(&[rule.clone()]));
        }
        rule.outcome = Outcome::Passed;
        rule.evidence.clear();
        assert!(!integrated_qualification(&[rule]));
        Ok(())
    }
    #[test]
    fn adoption_scope_preserves_delivery_gaps_and_rejects_other_blockers() -> Result<()> {
        let temp = tempfile::tempdir()?;
        anyhow::ensure!(
            std::process::Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(temp.path())
                .status()?
                .success(),
            "fixture Git initialization failed"
        );
        let mut manifest = opdev_project::discover(temp.path())?.manifest;
        manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
            version: "2".into(),
            minimumcd: None,
            review_reference: "fixture".into(),
            maintenance_branches: vec![],
        });
        let id: opdev_core::RuleId = "MCD-ARTIFACT-001".parse()?;
        let mut report = opdev_engine::CheckReport {
            schema: 3,
            catalog_version: 4,
            engineering: None,
            subject: "fixture".into(),
            evaluated_at: 1,
            rules: vec![opdev_core::RuleResult {
                rule_id: id.clone(),
                catalog_version: 4,
                outcome: Outcome::Unverified,
                subject: "fixture".into(),
                verifier: opdev_core::VerificationSource::Evidence,
                evaluated_at: 1,
                evidence: vec![],
                diagnostic: None,
            }],
            checks: vec![],
            gates: vec![opdev_core::GateVerdict {
                gate: Gate::Delivery,
                verdict: opdev_core::AggregateVerdict::Blocked,
                blocking_rules: vec![id],
                blocking_checks: vec![],
            }],
        };
        assert!(!adoption_gate_blocked(&manifest, &report, Gate::Delivery));
        assert!(
            !report.gate_passed(Gate::Delivery),
            "adoption must not rewrite artifact readiness"
        );
        for outcome in [Outcome::Failed, Outcome::Error, Outcome::MigrationRequired] {
            report.rules[0].outcome = outcome;
            assert!(adoption_gate_blocked(&manifest, &report, Gate::Delivery));
        }
        report.rules[0].outcome = Outcome::Unverified;
        report.gates[0]
            .blocking_checks
            .push("standard:required".into());
        assert!(adoption_gate_blocked(&manifest, &report, Gate::Delivery));
        report.gates[0].blocking_checks.clear();
        report.gates[0]
            .blocking_rules
            .push("OPDEV-RECOVERY-001".parse()?);
        assert!(adoption_gate_blocked(&manifest, &report, Gate::Delivery));
        report.gates[0].blocking_rules.pop();
        manifest.assurance.engineering = None;
        assert!(
            adoption_gate_blocked(&manifest, &report, Gate::Delivery),
            "legacy semantics changed"
        );
        Ok(())
    }
}
