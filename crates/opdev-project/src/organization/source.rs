//! Read bounded immutable Git definitions and verify the actual checkout separately.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
    process::Command,
};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{MAX_PACK_BYTES, PolicyPack, PolicySelection, ResolvedPolicyPack, valid_id};

/// Current selected data, never an execution report or authenticated policy decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicySnapshot {
    /// Actual staged tree, including project policy and relevant source.
    pub tree: String,
    /// Validated exact definitions and explicit values in stable selection order.
    pub policies: Vec<ResolvedPolicyPack>,
    /// Identity bound into the existing source/stage acceptance review.
    pub resolution_sha256: String,
}

/// Only portable, lowercase, immediate definition files belong in this namespace.
#[must_use]
pub fn portable_path(path: &str) -> bool {
    path.strip_prefix(".opdev/policies/")
        .and_then(|s| s.strip_suffix(".json"))
        .is_some_and(valid_id)
}

fn regular(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    !meta.file_type().is_symlink()
}

fn safe_directory(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            ensure!(
                regular(&meta) && meta.is_dir(),
                "Policy directory must be a regular directory, not a link or special file."
            );
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e).context("Cannot inspect policy directory"),
    }
}

/// Inspect a single definition without loading a project or running any command.
/// # Errors
/// Unsafe or missing directories/files, overlarge data and invalid identities fail closed.
pub fn inspect_file(root: &Path, id: &str) -> Result<PolicyPack> {
    ensure!(
        valid_id(id),
        "Policy ID must be a portable lowercase filename stem."
    );
    ensure!(
        safe_directory(&root.join(".opdev"))? && safe_directory(&root.join(".opdev/policies"))?,
        "Policy directory is missing; nothing created."
    );
    let path = root.join(format!(".opdev/policies/{id}.json"));
    let meta = fs::symlink_metadata(&path).context("Selected policy file is unavailable")?;
    ensure!(
        regular(&meta) && meta.is_file(),
        "Policy definition must be a regular file, not a link or special file."
    );
    ensure!(
        meta.len() <= MAX_PACK_BYTES as u64,
        "Policy definition exceeds 256 KiB."
    );
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(MAX_PACK_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let definition = PolicyPack::parse(&bytes)?;
    ensure!(
        definition.id == id,
        "Policy filename and definition ID disagree."
    );
    Ok(definition)
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("--no-replace-objects")
        .args(args)
        .output()?;
    ensure!(
        output.status.success(),
        "Cannot inspect policy Git snapshot; resolve repository/index errors. No project commands ran."
    );
    Ok(output.stdout)
}

fn checkout_inventory(root: &Path, selected: &BTreeSet<String>) -> Result<()> {
    if !safe_directory(&root.join(".opdev"))? || !safe_directory(&root.join(".opdev/policies"))? {
        ensure!(selected.is_empty(), "Selected policy directory is missing.");
        return Ok(());
    }
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(root.join(".opdev/policies"))? {
        ensure!(
            actual.len() < 8,
            "Policy namespace exceeds eight current selected definitions."
        );
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("Policy filename must be UTF-8"))?;
        let path = format!(".opdev/policies/{name}");
        ensure!(
            portable_path(&path),
            "Policy namespace contains a non-definition path; classify and review it, no cleanup performed."
        );
        actual.insert(path);
    }
    ensure!(
        &actual == selected,
        "Policy namespace and explicit selections differ; review missing or retired definitions. No automatic cleanup."
    );
    Ok(())
}

fn staged_definitions(root: &Path, tree: &str) -> Result<BTreeMap<String, PolicyPack>> {
    let entries = git(
        root,
        &["ls-tree", "-r", "-z", tree, "--", ".opdev/policies"],
    )?;
    ensure!(
        entries.len() <= 64 * 1024,
        "Policy inventory exceeds its safety bound."
    );
    let mut result = BTreeMap::new();
    for entry in entries.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        ensure!(
            result.len() < 8,
            "Policy namespace exceeds eight definitions."
        );
        let (header, path) = std::str::from_utf8(entry)?
            .split_once('\t')
            .context("Invalid policy Git entry")?;
        let fields: Vec<_> = header.split_whitespace().collect();
        ensure!(
            fields.len() == 3
                && fields[0] == "100644"
                && fields[1] == "blob"
                && portable_path(path),
            "Policy definitions must be regular non-executable staged JSON files, never links, submodules or nested data."
        );
        let size = String::from_utf8(git(root, &["cat-file", "-s", fields[2]])?)?
            .trim()
            .parse::<usize>()?;
        ensure!(size <= MAX_PACK_BYTES, "Staged policy exceeds 256 KiB.");
        let definition = PolicyPack::parse(&git(root, &["cat-file", "blob", fields[2]])?)?;
        ensure!(
            path == format!(".opdev/policies/{}.json", definition.id),
            "Staged policy filename and identity differ."
        );
        ensure!(
            result.insert(path.into(), definition).is_none(),
            "Duplicate policy definition path."
        );
    }
    Ok(result)
}

/// Resolve all current selected definitions before execution, without network or
/// working-tree edits. Git may materialize immutable objects with `write-tree`.
/// # Errors
/// Stale pins, unknown files, unstaged semantic changes, unsafe files or missing policy fail closed.
pub fn load_selected(root: &Path, selections: &[PolicySelection]) -> Result<PolicySnapshot> {
    ensure!(
        selections.len() <= 8,
        "At most eight organization policies may be selected."
    );
    let mut paths = BTreeSet::new();
    for selection in selections {
        selection.validate()?;
        ensure!(
            paths.insert(format!(".opdev/policies/{}.json", selection.id)),
            "Select each organization policy once."
        );
    }
    checkout_inventory(root, &paths)?;
    let tree = String::from_utf8(git(root, &["write-tree"])?)?
        .trim()
        .to_owned();
    ensure!(
        matches!(tree.len(), 40 | 64) && tree.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid policy source tree identity."
    );
    let definitions = staged_definitions(root, &tree)?;
    ensure!(
        definitions.keys().cloned().collect::<BTreeSet<_>>() == paths,
        "Staged policies differ from current selections; stage the reviewed definitions and retire obsolete ones."
    );
    let mut policies = Vec::new();
    for selection in selections {
        let definition = definitions
            .get(&format!(".opdev/policies/{}.json", selection.id))
            .context("Missing selected definition")?;
        ensure!(
            &inspect_file(root, &selection.id)? == definition,
            "Policy checkout changed from staged content; inspect and stage the actual reviewed definition."
        );
        policies.push(definition.resolve(selection)?);
    }
    policies.sort_by(|a, b| a.selection.id.cmp(&b.selection.id));
    let resolution_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json_canonicalizer::to_vec(&(1_u32, &policies))?)
    );
    Ok(PolicySnapshot {
        tree,
        policies,
        resolution_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestStage;
    use crate::organization::{Applicability, OrganizationControl, VerificationRequirement};

    fn fixture() -> Result<(tempfile::TempDir, PolicyPack, PolicySelection)> {
        let dir = tempfile::tempdir()?;
        git(dir.path(), &["init", "--quiet"])?;
        git(dir.path(), &["config", "core.autocrlf", "false"])?;
        fs::create_dir_all(dir.path().join(".opdev/policies"))?;
        let pack = PolicyPack {
            schema: 1,
            id: "example".into(),
            version: "1".into(),
            title: "Example policy".into(),
            source: "https://example.org/policy/1".into(),
            parameters: BTreeMap::new(),
            controls: vec![OrganizationControl {
                id: "ORG-EXAMPLE-001".into(),
                statement: "Verify the public contract".into(),
                source: "https://example.org/policy/1#contract".into(),
                stages: vec![TestStage::PreMerge],
                applicability: Applicability::Always,
                verification: VerificationRequirement::Automated,
            }],
        };
        let selected = PolicySelection {
            id: pack.id.clone(),
            version: pack.version.clone(),
            definition_sha256: pack.definition_sha256()?,
            parameters: BTreeMap::new(),
        };
        fs::write(
            dir.path().join(".opdev/policies/example.json"),
            serde_json::to_vec(&pack)?,
        )?;
        git(dir.path(), &["add", "--", ".opdev/policies/example.json"])?;
        Ok((dir, pack, selected))
    }

    #[test]
    fn snapshot_requires_selected_staged_and_checkout_definitions_to_agree() -> Result<()> {
        let (dir, mut pack, selected) = fixture()?;
        let before = load_selected(dir.path(), std::slice::from_ref(&selected))?;
        assert_eq!(before.policies.len(), 1);
        let path = dir.path().join(".opdev/policies/example.json");
        fs::write(
            &path,
            serde_json::to_string_pretty(&pack)?.replace('\n', "\r\n"),
        )?;
        assert_eq!(
            before,
            load_selected(dir.path(), std::slice::from_ref(&selected))?
        );
        pack.controls[0].statement = "A changed obligation".into();
        fs::write(&path, serde_json::to_vec(&pack)?)?;
        assert!(load_selected(dir.path(), std::slice::from_ref(&selected)).is_err());
        git(dir.path(), &["add", "--", ".opdev/policies/example.json"])?;
        assert!(load_selected(dir.path(), std::slice::from_ref(&selected)).is_err());
        let mut changed = selected;
        changed.definition_sha256 = pack.definition_sha256()?;
        let after = load_selected(dir.path(), &[changed])?;
        assert_ne!(before.tree, after.tree);
        assert_ne!(before.resolution_sha256, after.resolution_sha256);
        Ok(())
    }

    #[test]
    fn stale_or_unselected_namespace_is_not_silently_ignored() -> Result<()> {
        let (dir, _, selected) = fixture()?;
        assert!(load_selected(dir.path(), &[]).is_err());
        fs::write(
            dir.path().join(".opdev/policies/notes.md"),
            "not policy data",
        )?;
        assert!(load_selected(dir.path(), std::slice::from_ref(&selected)).is_err());
        fs::remove_file(dir.path().join(".opdev/policies/notes.md"))?;
        fs::remove_file(dir.path().join(".opdev/policies/example.json"))?;
        assert!(load_selected(dir.path(), &[selected]).is_err());
        Ok(())
    }

    #[test]
    fn executable_staged_definition_is_rejected_even_with_regular_checkout() -> Result<()> {
        let (dir, _, selected) = fixture()?;
        git(
            dir.path(),
            &[
                "update-index",
                "--chmod=+x",
                "--",
                ".opdev/policies/example.json",
            ],
        )?;
        assert!(load_selected(dir.path(), &[selected]).is_err());
        Ok(())
    }
}
