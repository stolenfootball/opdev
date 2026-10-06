//! Bounded worker protocol validation, not a scheduler or permission grant.
use crate::workflow_records::{Subject, digest};
use anyhow::{Context, Result, ensure};
use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

#[derive(Debug, Args)]
pub struct DelegationArgs {
    #[arg(long)]
    assignment: PathBuf,
    /// Explicit current subject; no Git or provider calls are made here.
    #[arg(long)]
    subject: PathBuf,
    /// Current complete acceptance inventory/review subject, not a worker's chosen subset.
    #[arg(long)]
    acceptance_sha256: String,
    #[arg(long)]
    result: Option<PathBuf>,
    /// JSON array of concurrently active assignments, excluding this assignment.
    #[arg(long)]
    active: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Role {
    Controller,
    Investigator,
    Implementer,
    AcceptanceReviewer,
    CiAnalyst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Read,
    Edit,
    FocusedCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Assignment {
    schema: u32,
    protocol: String,
    id: String,
    parent_work: String,
    role: Role,
    outcome: String,
    original_authorities: Vec<String>,
    subject: Subject,
    acceptance_sha256: String,
    owned_paths: Vec<String>,
    allowed_actions: Vec<Action>,
    checks: Vec<String>,
    model: String,
    effort: String,
    max_seconds: u64,
    max_tokens: Option<u64>,
    stop_condition: String,
    return_contract: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Completed,
    Partial,
    Failed,
    Interrupted,
    Unavailable,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerResult {
    schema: u32,
    assignment_id: String,
    subject: Subject,
    acceptance_sha256: String,
    status: Status,
    observed_model: Option<String>,
    observed_effort: Option<String>,
    elapsed_seconds: Option<u64>,
    input_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    changed_paths: Vec<String>,
    performed_actions: Vec<Action>,
    checks_executed: Vec<String>,
    evidence: Vec<String>,
    findings: Vec<String>,
    limitations: Vec<String>,
    unresolved_decisions: Vec<String>,
}

fn path(value: &str) -> Result<String> {
    ensure!(
        !value.is_empty()
            && !value.contains(['\\', ':', '*', '?'])
            && Path::new(value)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
            && value
                .split('/')
                .all(|p| !p.ends_with(['.', ' ']) && !p.is_empty()),
        "owned/changed paths must be explicit project-relative paths without traversal or globs"
    );
    Ok(value.to_lowercase())
}

fn overlaps(first: &str, second: &str) -> bool {
    first == second
        || first.starts_with(&format!("{second}/"))
        || second.starts_with(&format!("{first}/"))
}

impl Assignment {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == 1 && self.protocol == "delegation.v1",
            "unsupported delegation version; no worker was dispatched"
        );
        self.subject.validate()?;
        ensure!(
            digest(&self.acceptance_sha256),
            "the original acceptance inventory needs an exact digest"
        );
        ensure!(
            [
                &self.id,
                &self.parent_work,
                &self.outcome,
                &self.model,
                &self.effort,
                &self.stop_condition,
                &self.return_contract
            ]
            .iter()
            .all(|v| !v.trim().is_empty()),
            "assignment identity, scope, model, effort, stopping and return contract must be explicit"
        );
        ensure!(
            !self.original_authorities.is_empty()
                && self
                    .original_authorities
                    .iter()
                    .all(|v| !v.trim().is_empty()),
            "supply original authorities, not only an implementation summary"
        );
        ensure!(
            self.max_seconds > 0 && self.max_tokens != Some(0),
            "assignment needs a positive bounded budget"
        );
        ensure!(
            !self.allowed_actions.is_empty()
                && self.allowed_actions.iter().collect::<BTreeSet<_>>().len()
                    == self.allowed_actions.len(),
            "allowed actions must be explicit and unique"
        );
        if matches!(
            self.role,
            Role::Investigator | Role::AcceptanceReviewer | Role::CiAnalyst
        ) {
            ensure!(
                !self.allowed_actions.contains(&Action::Edit) && self.owned_paths.is_empty(),
                "review/investigation workers are read-only; the controller owns changes"
            );
        }
        ensure!(
            !self.allowed_actions.contains(&Action::Edit) || !self.owned_paths.is_empty(),
            "a writer needs explicit file ownership before dispatch"
        );
        for owned in &self.owned_paths {
            path(owned)?;
        }
        ensure!(
            self.checks.is_empty() || self.allowed_actions.contains(&Action::FocusedCheck),
            "declared checks require explicit focused-check permission"
        );
        ensure!(
            self.checks.iter().all(|c| !c.trim().is_empty())
                && self.checks.iter().collect::<BTreeSet<_>>().len() == self.checks.len(),
            "focused check identities must be nonempty and unique"
        );
        Ok(())
    }
}

fn validate_assignments(
    assignment: &Assignment,
    active: &[Assignment],
    subject: &Subject,
    acceptance: &str,
) -> Result<()> {
    assignment.validate()?;
    ensure!(
        assignment.subject == *subject && assignment.acceptance_sha256 == acceptance,
        "assignment source or acceptance changed; refresh the bounded assignment before dispatch"
    );
    let mut ids = BTreeSet::from([assignment.id.as_str()]);
    let all: Vec<_> = std::iter::once(assignment).chain(active).collect();
    for other in active {
        other.validate()?;
        ensure!(
            ids.insert(other.id.as_str()),
            "assignment is already active; do not dispatch it twice"
        );
    }
    for (index, writer) in all.iter().enumerate() {
        for other in &all[index + 1..] {
            for first in &writer.owned_paths {
                for second in &other.owned_paths {
                    ensure!(
                        !overlaps(&path(first)?, &path(second)?),
                        "another writer owns an overlapping path; serialize the writes or use separately verified worktrees"
                    );
                }
            }
        }
    }
    Ok(())
}

fn inspect_result(assignment: &Assignment, result: &WorkerResult) -> Result<Vec<String>> {
    ensure!(
        result.schema == 1 && result.assignment_id == assignment.id,
        "worker result has an unsupported version or belongs to another assignment"
    );
    ensure!(
        result.subject == assignment.subject
            && result.acceptance_sha256 == assignment.acceptance_sha256,
        "worker inspected an old source or acceptance subject; do not apply its review to the current change"
    );
    ensure!(
        result
            .performed_actions
            .iter()
            .all(|action| assignment.allowed_actions.contains(action)),
        "worker reports an action outside its assignment; inspect the actual side effects"
    );
    ensure!(
        result.changed_paths.is_empty()
            || (assignment.allowed_actions.contains(&Action::Edit)
                && result.performed_actions.contains(&Action::Edit)),
        "read-only worker reports changed files; stop and reconcile the actual diff"
    );
    for changed in &result.changed_paths {
        let changed = path(changed)?;
        ensure!(
            assignment.owned_paths.iter().any(|owned| path(owned)
                .is_ok_and(|owned| changed == owned || changed.starts_with(&format!("{owned}/")))),
            "worker changed a path it did not own; do not blindly apply the result"
        );
    }
    ensure!(
        result.checks_executed.is_empty()
            || result.performed_actions.contains(&Action::FocusedCheck),
        "reported checks require a reported focused-check action"
    );
    ensure!(
        result
            .checks_executed
            .iter()
            .all(|id| assignment.checks.contains(id))
            && result.checks_executed.iter().collect::<BTreeSet<_>>().len()
                == result.checks_executed.len(),
        "worker ran an unassigned or repeated check; retain the attempt and inspect why"
    );
    let mut limits = Vec::new();
    if result.status != Status::Completed {
        limits.push("Worker did not complete its bounded assignment; retain this attempt without calling it a successful review".into());
    }
    if result.observed_model.as_deref() != Some(&assignment.model)
        || result.observed_effort.as_deref() != Some(&assignment.effort)
    {
        limits.push("Observed model/effort is missing or differs from the approved assignment; do not claim a matched-model result or silently substitute".into());
    }
    if result
        .elapsed_seconds
        .is_none_or(|seconds| seconds > assignment.max_seconds)
    {
        limits.push("Worker elapsed time is unknown or exceeds its budget".into());
    }
    if let Some(limit) = assignment.max_tokens
        && result
            .input_tokens
            .zip(result.output_tokens)
            .is_none_or(|(input, output)| input.saturating_add(output) > limit)
    {
        limits.push("Worker token usage is unknown or exceeds its budget; cached input is still part of the recorded total".into());
    }
    if result.evidence.is_empty() {
        limits.push(
            "No inspectable evidence was returned; a completion claim alone is insufficient".into(),
        );
    }
    Ok(limits)
}

fn read<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T> {
    let meta = fs::symlink_metadata(path)?;
    ensure!(
        meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= 1024 * 1024,
        "delegation input must be a regular JSON file of at most 1 MiB"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 1024 * 1024,
        "delegation input grew beyond 1 MiB"
    );
    serde_json::from_slice(&bytes)
        .context("invalid delegation JSON; unknown fields are not ignored")
}

pub fn run(args: &DelegationArgs) -> Result<ExitCode> {
    let assignment: Assignment = read(&args.assignment)?;
    let subject: Subject = read(&args.subject)?;
    let active: Vec<Assignment> = args
        .active
        .as_deref()
        .map(read)
        .transpose()?
        .unwrap_or_default();
    validate_assignments(&assignment, &active, &subject, &args.acceptance_sha256)?;
    let findings = args
        .result
        .as_deref()
        .map(|path| inspect_result(&assignment, &read(path)?))
        .transpose()?
        .unwrap_or_default();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"schema":1,"qualification":"unverified",
        "assignment":assignment.id,"structurally_consistent":findings.is_empty(),"findings":findings,
        "limits":"Validation does not dispatch a worker, constrain OS permissions, authenticate reported actions, prove review adequacy or grant approval/merge/publication. Controller must compare actual diff, commands, original authorities and all worker usage. Single-agent work remains supported."}))?
    );
    Ok(if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
