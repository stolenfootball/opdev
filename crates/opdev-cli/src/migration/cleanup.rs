//! Exact reviewed text retirement, not a recursive filesystem cleanup service.
use super::{Change, MigrationPlan, inventory, sha};
use anyhow::{Context, Result, ensure};
use opdev_project::{ADOPTION_PATH, AdoptionRecord, EVIDENCE_PATH};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Kind {
    Move,
    RetireFile,
    EmptyDirectory,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Action {
    pub(super) path: String,
    kind: Kind,
    destination: Option<String>,
    reason: String,
}

fn protected(path: &str) -> bool {
    matches!(
        path.to_ascii_lowercase().as_str(),
        ".opdev/project.yaml"
            | ".opdev/adoption.yaml"
            | ".opdev/guidance.md"
            | ".opdev/evidence.yaml"
            | "agents.md"
            | "claude.md"
    )
}

pub(super) fn validate_actions(actions: &[Action]) -> Result<()> {
    ensure!(
        actions.len() <= 256,
        "Cleanup exceeds 256 explicit operations; use bounded increments"
    );
    let mut sources = BTreeSet::new();
    let mut destinations = BTreeSet::new();
    for action in actions {
        ensure!(
            opdev_project::clean_adoption::valid_path(&action.path)
                && !protected(&action.path)
                && !action.reason.trim().is_empty()
                && sources.insert(action.path.to_ascii_lowercase()),
            "Cleanup needs distinct portable paths, reasons and no current managed or historical-ledger targets"
        );
        if action.kind == Kind::Move {
            let dest = action
                .destination
                .as_ref()
                .context("Move needs an exact destination")?;
            ensure!(
                opdev_project::clean_adoption::valid_path(dest)
                    && !protected(dest)
                    && destinations.insert(dest.to_ascii_lowercase()),
                "Move destination is unsafe, protected or repeated"
            );
        } else {
            ensure!(
                action.destination.is_none(),
                "Only a move may select a destination"
            );
        }
        if action.kind == Kind::EmptyDirectory {
            ensure!(
                action.path.starts_with(".opdev/")
                    || action.path.starts_with(".github/workflows/")
                    || action.path.starts_with(".gitlab/"),
                "Empty-directory cleanup is limited to inventoried OpDev/CI namespaces; review other directories separately"
            );
        }
    }
    for source in &sources {
        ensure!(
            !destinations.iter().any(|dest| dest == source
                || dest.starts_with(&format!("{source}/"))
                || source.starts_with(&format!("{dest}/"))),
            "Cleanup destinations overlap retired paths; no cyclic or case-only moves"
        );
    }
    Ok(())
}

fn record(plan: &MigrationPlan) -> Result<AdoptionRecord> {
    let after = plan
        .changes
        .iter()
        .find(|c| c.path == ADOPTION_PATH)
        .and_then(|c| c.after.as_deref())
        .context("Cleanup needs the reviewed adoption destination")?;
    Ok(AdoptionRecord::from_yaml(after)?)
}

pub(super) fn validate_plan(plan: &MigrationPlan) -> Result<()> {
    validate_actions(&plan.cleanup)?;
    if plan.cleanup.is_empty() {
        return Ok(());
    }
    let record = record(plan)?;
    let target = record
        .clean_target
        .context("Cleanup needs clean_target retirement decisions")?;
    ensure!(
        !target.inventory_reference.trim().is_empty(),
        "Cleanup needs the actual inventory review reference"
    );
    for action in &plan.cleanup {
        ensure!(
            target
                .retirements
                .iter()
                .any(|r| r.path == action.path || action.path.starts_with(&format!("{}/", r.path))),
            "Cleanup target was not selected for retirement in the adoption record"
        );
    }
    Ok(())
}

fn inspect_directory(plan: &MigrationPlan, path: &str) -> Result<()> {
    // Target's parent validation rejects symlinks/reparse points without following them.
    inventory::target(&plan.root, &format!("{path}/.opdev-directory-probe"))?;
    if plan.root.join(path).exists() {
        for entry in fs::read_dir(plan.root.join(path))? {
            let entry = entry?;
            let relative = entry
                .path()
                .strip_prefix(&plan.root)?
                .to_string_lossy()
                .replace('\\', "/");
            ensure!(
                plan.cleanup.iter().any(|a| a.path == relative),
                "Directory has content outside the reviewed cleanup; preserve it and inspect again"
            );
        }
    }
    Ok(())
}

pub(super) fn prepare(plan: &mut MigrationPlan) -> Result<()> {
    validate_plan(plan)?;
    validate_text_modes(plan)?;
    for action in plan.cleanup.clone() {
        if action.kind == Kind::EmptyDirectory {
            inspect_directory(plan, &action.path)?;
            continue;
        }
        inventory::target(&plan.root, &action.path)?;
        let before = inventory::text(&plan.root.join(&action.path))?;
        plan.inputs.insert(
            action.path.clone(),
            before.as_ref().map(|s| sha(s.as_bytes())),
        );
        if let Some(dest) = &action.destination {
            inventory::target(&plan.root, dest)?;
            let existing = inventory::text(&plan.root.join(dest))?;
            plan.inputs
                .insert(dest.clone(), existing.as_ref().map(|s| sha(s.as_bytes())));
            if let Some(original) = &before {
                ensure!(
                    existing.as_ref().is_none_or(|text| text == original),
                    "Move destination has different content; consolidate explicitly through its owner, then review retirement. Nothing overwritten"
                );
                plan.replacement(dest, original.clone(), &action.reason)?;
            } else {
                ensure!(
                    existing.is_some(),
                    "Both move source and destination are missing; migration cannot recover content"
                );
            }
        }
        if before.is_some() {
            plan.changes.push(Change {
                path: action.path,
                before,
                after: None,
                reason: action.reason,
            });
        }
    }
    // A manually consolidated replacement is a frozen input too; preserve its later edits.
    if !plan.cleanup.is_empty() {
        for retirement in record(plan)?.clean_target.context("target")?.retirements {
            if let Some(replacement) = retirement.replacement {
                inventory::target(&plan.root, &replacement)?;
                let content = inventory::text(&plan.root.join(&replacement))?;
                ensure!(
                    content.is_some()
                        || plan
                            .changes
                            .iter()
                            .any(|c| c.path == replacement && c.after.is_some()),
                    "Declared retained replacement is missing; preserve obsolete content until it is migrated"
                );
                plan.inputs
                    .insert(replacement, content.map(|s| sha(s.as_bytes())));
            }
        }
    }
    Ok(())
}

// Byte equality cannot detect chmod. Recheck before apply/resume writes as well
// as preview, including retirements whose text-only recovery cannot retain mode.
pub(super) fn validate_text_modes(plan: &MigrationPlan) -> Result<()> {
    let paths: Vec<_> = plan
        .cleanup
        .iter()
        .filter(|action| action.kind != Kind::EmptyDirectory)
        .flat_map(|action| std::iter::once(&action.path).chain(action.destination.iter()))
        .collect();
    if paths.is_empty() {
        return Ok(());
    }
    let modes = std::process::Command::new("git")
        .arg("-C")
        .arg(&plan.root)
        .args(["--literal-pathspecs", "ls-files", "--stage", "-z", "--"])
        .args(&paths)
        .output()?;
    ensure!(
        modes.status.success()
            && !modes
                .stdout
                .split(|byte| *byte == 0)
                .any(|entry| entry.starts_with(b"100755 ")),
        "Executable file migration needs a separate reviewed mode-preserving mechanism"
    );
    #[cfg(unix)]
    for path in paths {
        use std::os::unix::fs::PermissionsExt;
        match fs::symlink_metadata(plan.root.join(path)) {
            Ok(meta) => ensure!(
                meta.permissions().mode() & 0o111 == 0,
                "Executable file migration needs a separate reviewed mode-preserving mechanism"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(super) fn file_target(plan: &MigrationPlan, path: &str) -> bool {
    plan.cleanup.iter().any(|a| {
        a.kind != Kind::EmptyDirectory && (a.path == path || a.destination.as_deref() == Some(path))
    })
}
pub(super) fn retired_file(plan: &MigrationPlan, path: &str) -> bool {
    path != EVIDENCE_PATH
        && plan
            .cleanup
            .iter()
            .any(|a| a.kind != Kind::EmptyDirectory && a.path == path)
}
pub(super) fn retired_directory(plan: &MigrationPlan, path: &str) -> bool {
    plan.cleanup
        .iter()
        .any(|a| a.kind == Kind::EmptyDirectory && a.path == path)
}

pub(super) fn remove_empty_directories(plan: &MigrationPlan) -> Result<()> {
    let mut paths: Vec<_> = plan
        .cleanup
        .iter()
        .filter(|a| a.kind == Kind::EmptyDirectory)
        .map(|a| &a.path)
        .collect();
    paths.sort_by_key(|path| std::cmp::Reverse(path.split('/').count()));
    for path in paths {
        inventory::unchanged(plan)?;
        inventory::target(&plan.root, &format!("{path}/.opdev-directory-probe"))?;
        if plan.root.join(path).exists() {
            // Deliberately nonrecursive. Any new or unplanned child prevents removal.
            fs::remove_dir(plan.root.join(path)).context("Directory is not empty or could not be removed; preserve it and inspect. Earlier writes remain in recovery")?;
        }
    }
    Ok(())
}
