//! Versioned destination, not a waiver list or an automatic cleanup authority.
use crate::{AdoptionRecord, Capability, CapabilityState, ProjectManifest};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path},
    process::Command,
};

/// Exact destination bundled with this CLI; never resolved through a moving network alias.
pub const TARGET: &str = "clean-3";

/// Permanent adoption decisions stay in the existing adoption record, not a new registry.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CleanTarget {
    /// Destination revision: 4 for policy 3; earlier explicit targets remain readable.
    pub version: u32,
    /// Existing work/review authority covering inventory scope, purpose and retained content.
    pub inventory_reference: String,
    /// Explicit obsolete active paths that must be absent at completion.
    pub retirements: Vec<Retirement>,
}

/// Retirement is a reviewed postcondition, not permission for this reader to delete files.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Retirement {
    /// Exact repository-relative obsolete path, never a glob or broad directory.
    pub path: String,
    /// Why obsolete, including the retained owner of any useful content/history.
    pub reason: String,
    /// Optional current repository-relative replacement whose existence is checked.
    pub replacement: Option<String>,
}

/// Validate portable bounded exact paths without following them.
#[must_use]
pub fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.contains('\\')
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
        && !path.split('/').any(|p| {
            let stem = p.split('.').next().unwrap_or_default().to_ascii_uppercase();
            p.is_empty()
                || p.eq_ignore_ascii_case(".git")
                || p == "."
                || p == ".."
                || p.ends_with('.')
                || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || ["COM", "LPT"].iter().any(|prefix| {
                    stem.strip_prefix(prefix).is_some_and(|n| {
                        matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                    })
                })
        })
        && Path::new(path)
            .components()
            .all(|p| matches!(p, Component::Normal(_)))
        && !matches!(
            path.to_ascii_lowercase().as_str(),
            ".opdev" | ".github" | ".gitlab"
        )
}

/// Validate the proposal without requiring retirement to have happened yet.
/// # Errors
/// Reject unsafe, overlapping or protected paths before binding plan approval.
pub fn validate_target(target: &CleanTarget) -> Result<(), String> {
    if !matches!(target.version, 1..=4) || target.retirements.len() > 256 {
        return Err(
            "Unsupported destination or retirement inventory exceeds 256 exact paths".into(),
        );
    }
    let mut paths = BTreeSet::new();
    for retirement in &target.retirements {
        let path = retirement.path.to_ascii_lowercase();
        if !valid_path(&retirement.path)
            || matches!(
                path.as_str(),
                ".opdev/project.yaml"
                    | ".opdev/adoption.yaml"
                    | ".opdev/guidance.md"
                    | "agents.md"
                    | "claude.md"
            )
            || retirement.reason.trim().is_empty()
            || !paths.insert(path)
        {
            return Err("Retirements need distinct portable exact paths outside current required files and a reviewed reason".into());
        }
    }
    for path in &paths {
        if paths
            .iter()
            .any(|other| other.starts_with(&format!("{path}/")))
        {
            return Err("Retired paths overlap; select each bounded destination once".into());
        }
    }
    for retirement in &target.retirements {
        if let Some(replacement) = &retirement.replacement {
            let replacement_lower = replacement.to_ascii_lowercase();
            if !valid_path(replacement)
                || paths.iter().any(|path| {
                    &replacement_lower == path
                        || replacement_lower.starts_with(&format!("{path}/"))
                        || path.starts_with(&format!("{replacement_lower}/"))
                })
            {
                return Err("Retirement replacement must be a separate retained exact file".into());
            }
        }
    }
    Ok(())
}

/// Missing target requirements; declarations alone never qualify behavior.
#[must_use]
pub fn policy_gaps(project: &ProjectManifest) -> Vec<String> {
    policy_gaps_for(project, target_version(project))
}

/// New policy has its own destination; installing a reader does not migrate old projects.
#[must_use]
pub fn target_version(project: &ProjectManifest) -> u32 {
    if project
        .assurance
        .engineering
        .as_ref()
        .is_some_and(|p| p.version == "3")
    {
        4
    } else {
        3
    }
}

/// Explicit selected policy's target, not a floating version alias.
#[must_use]
pub fn target_name(project: &ProjectManifest) -> &'static str {
    if target_version(project) == 4 {
        "clean-4"
    } else {
        TARGET
    }
}

/// Inspect the exact selected destination; legacy clean-1 keeps its archive policy.
#[must_use]
pub fn policy_gaps_for(project: &ProjectManifest, version: u32) -> Vec<String> {
    let mut gaps = Vec::new();
    if !matches!(version, 1..=4) {
        return vec![
            "Unsupported clean-adoption destination; no current completion can be claimed".into(),
        ];
    }
    let (schema, engineering, storage) = if version == 4 {
        (4, "3", 2)
    } else if version == 3 {
        (4, "2", 2)
    } else {
        (3, "1", version)
    };
    if project.schema != schema
        || project
            .assurance
            .engineering
            .as_ref()
            .is_none_or(|p| p.version != engineering)
    {
        gaps.push(format!(
            "Select reviewed engineering policy {engineering} in project schema {schema}"
        ));
    }
    if project.layout.as_ref().is_none_or(|p| {
        if version >= 3 {
            p.version != 3
        } else {
            !matches!(p.version, 1 | 2)
        }
    }) {
        gaps.push(if version >= 3 { "Select strict layout 3 with shared guidance, requirements and only current selected policy definitions".into() } else { "Migrate to strict layout 1 or 2 with shared guidance and thin host entries".into() });
    }
    if project
        .assurance
        .review_storage
        .as_ref()
        .is_none_or(|p| p.version != storage)
    {
        gaps.push(if storage == 2 {
            "Select MR/PR review storage 2 with a reviewed bounded CI report lifetime".into()
        } else {
            "Select external semantic-review storage with reviewed retention and recovery".into()
        });
    }
    if version >= 3
        && project
            .assurance
            .requirements
            .as_ref()
            .is_none_or(|p| p.version != 1)
    {
        gaps.push("Select requirements catalog 1 and review real guarantees and stage-specific verification; an empty scaffold is not adopted".into());
    }
    if project
        .assurance
        .safeguards
        .as_ref()
        .is_none_or(|p| p.version != if version == 4 { 2 } else { 1 })
    {
        gaps.push(
            format!("Select capability safeguards {}; assess actual capabilities, not missing configuration", if version == 4 { 2 } else { 1 }),
        );
    }
    gaps
}

/// Target completeness independent of adoption practice labels and gate execution.
#[must_use]
pub fn decision_gaps(project: &ProjectManifest, record: Option<&AdoptionRecord>) -> Vec<String> {
    let mut gaps = policy_gaps(project);
    if let Some(safeguards) = &project.assurance.safeguards {
        for &capability in Capability::for_version(safeguards.version) {
            if safeguards.capabilities.get(&capability).is_none_or(|fact| {
                fact.state == CapabilityState::Unknown
                    || fact.rationale.trim().is_empty()
                    || fact.authority.trim().is_empty()
            }) {
                gaps.push(format!(
                    "Assess capability {capability:?}; unknown is not an exemption"
                ));
            }
        }
    }
    match record.and_then(|r| r.clean_target.as_ref()) {
        None => gaps.push(format!(
            "Review the {} destination and repository inventory in the adoption record",
            target_name(project)
        )),
        Some(target) => {
            if target.version != target_version(project) {
                gaps.push(format!("Current adoption requires {}. Preview a reviewed migration; existing policy and historical completion are not changed automatically", target_name(project)));
            }
            if target.inventory_reference.trim().is_empty() {
                gaps.push("Repository inventory needs its actual review reference, including retained and retired content".into());
            }
        }
    }
    gaps
}

fn present(root: &Path, path: &str) -> Result<bool, String> {
    let mut current = root.to_path_buf();
    for part in path.split('/') {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err("linked path requires explicit review".into());
                    }
                }
                if meta.file_type().is_symlink() {
                    return Err("linked path requires explicit review".into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(_) => return Err("path could not be inspected".into()),
        }
    }
    Ok(true)
}

/// Verify reviewed absence in both the checkout and index, including ignored leftovers.
/// # Errors
/// Reject malformed/ambiguous paths and inaccessible Git state instead of partial success.
pub fn retirement_gaps(root: &Path, target: &CleanTarget) -> Result<Vec<String>, String> {
    validate_target(target)?;
    let mut gaps = Vec::new();
    for retirement in &target.retirements {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "--literal-pathspecs",
                "ls-files",
                "--stage",
                "-z",
                "--",
                &retirement.path,
            ])
            .output()
            .map_err(|_| "Could not inspect retired paths in Git")?;
        if !output.status.success() {
            return Err("Could not inspect retired paths in Git".into());
        }
        if present(root, &retirement.path)? || !output.stdout.is_empty() {
            gaps.push(format!("Retired path {} still exists in the checkout or staged source; finish the reviewed migration", retirement.path));
        }
        if let Some(replacement) = &retirement.replacement {
            let output = Command::new("git")
                .arg("-C")
                .arg(root)
                .args([
                    "--literal-pathspecs",
                    "ls-files",
                    "--stage",
                    "-z",
                    "--",
                    replacement,
                ])
                .output()
                .map_err(|_| "Could not inspect replacement in Git")?;
            if !output.status.success() {
                return Err("Could not inspect replacement in Git".into());
            }
            let entries: Vec<_> = output
                .stdout
                .split(|b| *b == 0)
                .filter(|e| !e.is_empty())
                .collect();
            let indexed_regular = entries.len() == 1
                && std::str::from_utf8(entries[0]).is_ok_and(|entry| {
                    (entry.starts_with("100644 ") || entry.starts_with("100755 "))
                        && entry.ends_with(&format!(" 0\t{replacement}"))
                });
            if !present(root, replacement)? || !root.join(replacement).is_file() || !indexed_regular
            {
                gaps.push(format!("Retirement replacement {replacement} must be a retained regular file in both checkout and staged source"));
            }
        }
    }
    Ok(gaps)
}
