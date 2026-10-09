//! Explicit coordinated migration; recovery snapshots are not decision authorities.
use anyhow::{Context, Result, ensure};
use opdev_core::Outcome;
use opdev_project::{
    ADOPTION_PATH, AdoptionRecord, AgentFilePreview, EVIDENCE_PATH, FileChange, MANIFEST_PATH,
    ManagedFile, ProjectManifest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

mod cleanup;
mod inventory;
#[cfg(test)]
mod tests;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: u32,
    decision_reference: String,
    project: ProjectManifest,
    /// CI changes are explicit exact file contents, never inferred version bumps.
    #[serde(default)]
    ci: BTreeMap<String, String>,
    ci_review_reference: String,
    /// Original complete legacy bytes at the retained, authenticated authority.
    history: Option<opdev_remote::ArchiveLocator>,
    #[serde(default)]
    clean_target: Option<opdev_project::clean_adoption::CleanTarget>,
    #[serde(default)]
    cleanup: Vec<cleanup::Action>,
    /// Actual review of content ownership changes, not permission inferred from policy.
    #[serde(default)]
    authority_review_reference: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Change {
    path: String,
    before: Option<String>,
    after: Option<String>,
    reason: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MigrationPlan {
    schema: u32,
    kind: String,
    root: PathBuf,
    cli_sha256: String,
    request_sha256: String,
    decision_reference: String,
    ci_review_reference: String,
    plan_id: String,
    inputs: BTreeMap<String, Option<String>>,
    changes: Vec<Change>,
    findings: Vec<Finding>,
    history: Option<opdev_remote::ArchiveLocator>,
    #[serde(default)]
    cleanup: Vec<cleanup::Action>,
    #[serde(default)]
    authority_review_reference: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Finding {
    component: String,
    outcome: Outcome,
    detail: String,
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

impl MigrationPlan {
    fn identity(&self) -> Result<String> {
        let mut value = serde_json::to_value(self)?;
        value["plan_id"] = serde_json::json!("");
        Ok(sha(&serde_json::to_vec(&value)?))
    }
    fn changed(&self) -> bool {
        self.changes.iter().any(|c| c.before != c.after)
            || self.cleanup.iter().any(|a| {
                cleanup::retired_directory(self, &a.path) && self.root.join(&a.path).exists()
            })
    }
    fn blocked(&self) -> bool {
        self.findings
            .iter()
            .any(|f| matches!(f.outcome, Outcome::Failed | Outcome::Error))
    }
    fn finding(&mut self, component: &str, outcome: Outcome, detail: impl Into<String>) {
        self.findings.push(Finding {
            component: component.into(),
            outcome,
            detail: detail.into(),
        });
    }
    fn replacement(&mut self, path: &str, after: String, reason: &str) -> Result<()> {
        inventory::target(&self.root, path)?;
        let before = inventory::text(&self.root.join(path))?;
        self.changes.push(Change {
            path: path.into(),
            before,
            after: Some(after),
            reason: reason.into(),
        });
        Ok(())
    }
}

fn assess(root: &Path, request_path: &Path, plugin: Option<&Path>) -> Result<MigrationPlan> {
    let (root, original) = crate::load_project(root)?;
    let bytes = crate::local_state::read(&std::path::absolute(request_path)?)?
        .context("Migration request is missing")?;
    let request: Request = serde_json::from_slice(&bytes).map_err(|_| {
        anyhow::anyhow!("Migration request is malformed or unsupported; no input contents echoed")
    })?;
    ensure!(
        request.schema == 1
            && !request.decision_reference.trim().is_empty()
            && !request.ci_review_reference.trim().is_empty(),
        "Migration needs supported schema 1 and actual decision/CI review references"
    );
    let candidate = ProjectManifest::from_yaml(&request.project.to_yaml()?)?;
    ensure!(
        candidate.schema == 3
            && candidate.layout.is_some()
            && candidate.assurance.review_storage.is_some(),
        "Coordinated migration needs explicitly selected schema-3 engineering, layout and review storage; no choices inferred"
    );
    let mut plan = MigrationPlan {
        schema: 1,
        kind: "coordinated_migration".into(),
        cli_sha256: sha(&fs::read(std::env::current_exe()?)?),
        root,
        request_sha256: sha(&bytes),
        decision_reference: request.decision_reference,
        ci_review_reference: request.ci_review_reference,
        plan_id: String::new(),
        inputs: BTreeMap::new(),
        changes: vec![],
        findings: vec![],
        history: request.history,
        cleanup: request.cleanup,
        authority_review_reference: request.authority_review_reference,
    };
    cleanup::validate_actions(&plan.cleanup)?;
    inventory::collect(&mut plan, plugin)?;
    plan.replacement(MANIFEST_PATH, candidate.to_yaml()?, "Explicit reviewed target policy; commands and authority changes are visible in this exact diff")?;
    adoption(&mut plan, &original, &candidate, request.clean_target)?;
    for file in opdev_project::preview_agent_files_for_layout(&plan.root, true)? {
        plan.changes.push(Change {
            path: file
                .file
                .path
                .strip_prefix(&plan.root)?
                .to_string_lossy()
                .replace('\\', "/"),
            before: file.before,
            after: Some(file.after),
            reason: "Deterministic shared guidance, preserving unrelated instructions/imports"
                .into(),
        });
    }
    for (path, after) in &request.ci {
        ensure!(
            inventory::ci_path(path),
            "Migration CI edits must name existing, inspected local provider configuration files"
        );
        ensure!(
            plan.inputs.contains_key(path) && plan.inputs[path].is_some(),
            "CI target was not an inspected existing file"
        );
        inventory::validate_ci(&plan.root, path, after, &request.ci)?;
        plan.replacement(path, after.clone(), "Explicit CI transition; syntax, runtime capability and pipeline qualification remain separate checks")?;
    }
    history(&mut plan, &candidate)?;
    cleanup::prepare(&mut plan)?;
    inventory::owners(&mut plan, &original, &candidate);
    plan.finding("verification", Outcome::Unverified, "Migration writes do not verify adoption, local/CI compatibility, project tests, delivery or release. Review each separate finding and run current required checks after staging.");
    plan.plan_id = plan.identity()?;
    Ok(plan)
}

fn adoption(
    plan: &mut MigrationPlan,
    original: &ProjectManifest,
    candidate: &ProjectManifest,
    clean_target: Option<opdev_project::clean_adoption::CleanTarget>,
) -> Result<()> {
    let Some(mut record) = AdoptionRecord::load(&plan.root)? else {
        plan.finding("adoption", Outcome::Failed, "No existing assessment. Complete the explicitly authorized assessment first; migration will not invent adoption choices.");
        return Ok(());
    };
    let before = record.to_yaml()?;
    record.migrate_catalog(opdev_project::project_adoption_catalog(candidate))?;
    if let Some(target) = clean_target {
        opdev_project::clean_adoption::validate_target(&target).map_err(anyhow::Error::msg)?;
        ensure!(
            opdev_project::clean_adoption::policy_gaps_for(candidate, target.version).is_empty(),
            "Clean adoption target requires the complete reviewed policy bundle"
        );
        if serde_json::to_value(&record.clean_target)? != serde_json::to_value(&target)? {
            record.review = None;
        }
        record.clean_target = Some(target);
    }
    // Existing approvals cannot authorize a materially different project policy.
    // Preserve their original bytes in the mandatory recovery snapshot, never re-approve.
    if original != candidate {
        record.review = None;
    }
    let after = record.to_yaml()?;
    plan.replacement(ADOPTION_PATH, after, "Preserve all existing practices/scope/workflow; add only pending catalog gaps. Old approval remains historical in recovery, never rebound as new consent")?;
    plan.finding("adoption", Outcome::Unverified, if before == record.to_yaml()? {
        "Existing dispositions retained; adoption completion is not inferred"
    } else { "Catalog or policy changed; original dispositions retained, new gaps pending, prior approval not transferred. Review the new adoption plan before claiming completion" });
    Ok(())
}

fn history(plan: &mut MigrationPlan, candidate: &ProjectManifest) -> Result<()> {
    let Some(before) = inventory::text(&plan.root.join(EVIDENCE_PATH))? else {
        return Ok(());
    };
    if candidate
        .assurance
        .review_storage
        .as_ref()
        .is_some_and(|p| p.version == 2)
    {
        ensure!(
            plan.history.is_none(),
            "MR/PR migration retains existing Git history; do not select a separate archive"
        );
        plan.changes.push(Change {path: EVIDENCE_PATH.into(), before: Some(before), after: None,
            reason: "Remove the obsolete active ledger under reviewed MR/PR policy. Temporary migration recovery protects interruption, not permanent evidence storage".into()});
        plan.finding("history", Outcome::Passed, "Legacy ledger selected for removal. No archive or historical-copy verification is required. Keep temporary rollback protection only until the migration is verified; existing Git history is not rewritten.");
        return Ok(());
    }
    let original = ProjectManifest::load(&plan.root.join(MANIFEST_PATH))?;
    opdev_project::EvidenceLedger::load_optional(&plan.root, &original.catalog()?)
        .map_err(|_| anyhow::anyhow!("Original ledger is malformed or unsupported; preserve it for explicit recovery. No record contents echoed"))?;
    let Some(locator) = &plan.history else {
        plan.finding("history", Outcome::Failed, "Legacy history needs an independently selected retained archive before migration. Original ledger remains active; no cleanup performed");
        return Ok(());
    };
    let storage = candidate
        .assurance
        .review_storage
        .as_ref()
        .context("Selected storage missing")?;
    ensure!(
        storage.provider == locator.provider && storage.repository_id == locator.repository_id,
        "Historical archive must belong to the selected storage policy"
    );
    let observed = opdev_remote::retrieve_archive(locator).map_err(anyhow::Error::msg)?;
    ensure!(
        observed.bytes() == before.as_bytes(),
        "Retained history does not match every original ledger byte; nothing changed"
    );
    plan.changes.push(Change {path: EVIDENCE_PATH.into(), before: Some(before), after: None,
        reason: "Retire active path only after exact authenticated archive retrieval and independently reread recovery snapshot; historical semantics remain unchanged".into()});
    plan.finding("history", Outcome::Unverified, "Original bytes retrieved exactly; retention owner/access/lifetime/protection are reviewed project obligations, not proven by successful retrieval. Apply also requires independently reread recovery bytes");
    Ok(())
}

pub(super) fn run(
    root: &Path,
    request: Option<&Path>,
    resume: Option<&Path>,
    apply: Option<&str>,
    recovery: Option<&Path>,
    plugin: Option<&Path>,
) -> Result<ExitCode> {
    let plan = if let Some(snapshot) = resume {
        let bytes = crate::local_state::read(&std::path::absolute(snapshot)?)?
            .context("Recovery snapshot is missing")?;
        let plan: MigrationPlan = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("Recovery snapshot is malformed or unsupported"))?;
        ensure!(
            plan.schema == 1
                && plan.kind == "coordinated_migration"
                && plan.identity()? == plan.plan_id,
            "Recovery snapshot changed or is unsupported"
        );
        ensure!(
            opdev_project::discover(root)?.root == plan.root,
            "Recovery belongs to another checkout"
        );
        plan
    } else {
        assess(root, request.context("Migration request required")?, plugin)?
    };
    if let Some(expected) = apply {
        ensure!(
            expected == plan.plan_id,
            "Migration inputs changed since preview; nothing written. Inspect the current proposal"
        );
        ensure!(
            !plan.blocked(),
            "Migration has unresolved ownership or compatibility blockers; nothing written"
        );
        ensure!(
            plan.cli_sha256 == sha(&fs::read(std::env::current_exe()?)?),
            "Migration runtime changed; inspect with the reviewed runtime"
        );
        if resume.is_none() && plan.changed() {
            let output = recovery.context(
                "Migration apply needs --recovery-output outside the source; nothing written",
            )?;
            let output = crate::evidence_bundle::export_destination(&plan.root, output)?;
            let bytes = serde_json::to_vec_pretty(&plan)?;
            ensure!(
                bytes.len() <= 8 * 1024 * 1024,
                "Recovery exceeds supported size; preserve history and choose a reviewed migration strategy"
            );
            crate::local_state::write_new(&output, &bytes)?;
            ensure!(
                crate::local_state::read(&output)?.as_ref() == Some(&bytes),
                "Recovery snapshot could not be independently reread; nothing changed"
            );
        }
        apply_plan(&plan, usize::MAX)?;
    }
    let blocked = plan.blocked();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({"schema":1,"plan":plan,
        "applied":apply.is_some(),"qualification":"unverified",
        "limits":"Exact migration preview/apply, not approval or qualification. No installation, provider write or release. Writes are atomic per file, not a transaction; retain recovery and resume only its original reviewed identity after interruption."}))?
    );
    Ok(ExitCode::from(u8::from(blocked)))
}

fn apply_plan(plan: &MigrationPlan, stop_after: usize) -> Result<()> {
    cleanup::validate_plan(plan)?;
    let mut paths = std::collections::BTreeSet::new();
    for change in &plan.changes {
        ensure!(
            paths.insert(&change.path)
                && (matches!(
                    change.path.as_str(),
                    MANIFEST_PATH
                        | ADOPTION_PATH
                        | EVIDENCE_PATH
                        | ".opdev/guidance.md"
                        | "AGENTS.md"
                        | "CLAUDE.md"
                ) || inventory::ci_path(&change.path)
                    || cleanup::file_target(plan, &change.path)),
            "Unsupported or repeated migration target; no arbitrary recovery instructions executed"
        );
        ensure!(
            change.after.is_some()
                || change.path == EVIDENCE_PATH
                || cleanup::retired_file(plan, &change.path),
            "Unsupported migration deletion"
        );
    }
    inventory::unchanged(plan)?;
    if let Some(change) = plan
        .changes
        .iter()
        .find(|c| c.path == EVIDENCE_PATH && c.after.is_none())
    {
        let policy = plan
            .changes
            .iter()
            .find(|c| c.path == MANIFEST_PATH)
            .and_then(|c| c.after.as_deref())
            .context("Migration policy missing")?;
        if ProjectManifest::from_yaml(policy)?
            .assurance
            .review_storage
            .is_some_and(|p| p.version == 2)
        {
            ensure!(
                plan.history.is_none(),
                "MR/PR migration does not require a separate evidence archive"
            );
        } else {
            let observed = opdev_remote::retrieve_archive(
                plan.history.as_ref().context("History locator missing")?,
            )
            .map_err(anyhow::Error::msg)?;
            ensure!(
                Some(observed.bytes()) == change.before.as_deref().map(str::as_bytes),
                "Historical archive changed or unavailable; migration stopped"
            );
        }
    }
    for (count, change) in plan.changes.iter().enumerate() {
        ensure!(
            count < stop_after,
            "Migration interrupted after prior writes; preserve recovery, inspect current files and resume the original reviewed plan"
        );
        inventory::unchanged(plan)?;
        let path = plan.root.join(&change.path);
        let current = inventory::text(&path)?;
        if current == change.after {
            continue;
        }
        if let Some(after) = &change.after {
            inventory::target(&plan.root, &change.path)?;
            fs::create_dir_all(path.parent().context("Migration target parent missing")?)?;
            opdev_project::apply_agent_preview(&[AgentFilePreview {
                file: ManagedFile {
                    path,
                    change: if current.is_some() {
                        FileChange::Updated
                    } else {
                        FileChange::Created
                    },
                },
                before: current,
                after: after.clone(),
            }])?;
        } else {
            ensure!(
                change.path == EVIDENCE_PATH || cleanup::retired_file(plan, &change.path),
                "Only explicitly reviewed retained-history or obsolete files may be retired"
            );
            fs::remove_file(path).context("Migration partially applied; retain recovery and resume after inspecting current files")?;
        }
    }
    cleanup::remove_empty_directories(plan)?;
    inventory::unchanged(plan)?;
    Ok(())
}
