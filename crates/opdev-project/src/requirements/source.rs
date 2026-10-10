//! Bounded catalog reads from immutable Git blobs, not checkout-transformed files.
use super::{CatalogDocument, DIRECTORY};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
};

/// Source snapshot; no writable database or execution verdicts.
#[derive(Debug, Clone)]
pub struct CatalogSnapshot {
    /// Immutable Git tree used for this read.
    pub tree: String,
    /// Capability-sized documents.
    pub documents: BTreeMap<String, CatalogDocument>,
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("--no-replace-objects")
        .args(args)
        .output()?;
    ensure!(
        out.status.success(),
        "Could not read requirements Git snapshot; resolve repository/index conflicts"
    );
    Ok(out.stdout)
}

/// Read staged catalog without altering the index or working files.
/// # Errors
/// Git/index errors, invalid input and exceeded safety limits fail closed.
pub fn load_index(root: &Path) -> Result<CatalogSnapshot> {
    let tree = String::from_utf8(git(root, &["write-tree"])?)?;
    load_tree(root, tree.trim())
}

/// Read an explicit baseline without changing checkout state.
/// # Errors
/// Unknown revision, invalid input and exceeded safety limits fail closed.
pub fn load_revision(root: &Path, revision: &str) -> Result<CatalogSnapshot> {
    ensure!(
        !revision.is_empty()
            && !revision.starts_with('-')
            && !revision.contains(['\0', '\n', '\r']),
        "Invalid requirements baseline revision"
    );
    let tree = String::from_utf8(git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{revision}^{{commit}}^{{tree}}"),
        ],
    )?)?;
    load_tree(root, tree.trim())
}

fn load_tree(root: &Path, tree: &str) -> Result<CatalogSnapshot> {
    ensure!(
        matches!(tree.len(), 40 | 64) && tree.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid Git tree identity"
    );
    let entries = git(root, &["ls-tree", "-r", "-z", tree, "--", DIRECTORY])?;
    ensure!(
        entries.len() <= 2 * 1024 * 1024,
        "Requirements path inventory exceeds 2 MiB"
    );
    let mut documents = BTreeMap::new();
    let mut names = BTreeSet::new();
    let mut total = 0usize;
    for entry in entries.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        ensure!(
            documents.len() < 1024,
            "Requirements catalog exceeds 1024 files"
        );
        let (header, path) = std::str::from_utf8(entry)?
            .split_once('\t')
            .context("Invalid requirements Git entry")?;
        let fields: Vec<_> = header.split_whitespace().collect();
        ensure!(
            fields.len() == 3 && matches!(fields[0], "100644" | "100755") && fields[1] == "blob",
            "Requirements source must be a regular Git file, not a symlink or submodule"
        );
        ensure!(
            portable_catalog_path(path),
            "Requirements files must be portable .opdev/requirements/<capability>.json paths"
        );
        ensure!(
            names.insert(path.to_ascii_lowercase()),
            "Case-ambiguous requirements paths"
        );
        let size = String::from_utf8(git(root, &["cat-file", "-s", fields[2]])?)?
            .trim()
            .parse::<usize>()?;
        total = total
            .checked_add(size)
            .context("Requirements size overflow")?;
        ensure!(
            size <= 8 * 1024 * 1024 && total <= 32 * 1024 * 1024,
            "Requirements catalog exceeds file (8 MiB) or total (32 MiB) bound"
        );
        let bytes = git(root, &["cat-file", "blob", fields[2]])?;
        documents.insert(
            path.into(),
            CatalogDocument::parse(&bytes)
                .with_context(|| format!("Invalid catalog file {path}"))?,
        );
    }
    Ok(CatalogSnapshot {
        tree: tree.into(),
        documents,
    })
}

/// Only portable capability JSON files are allowed, never arbitrary scratch data.
#[must_use]
pub fn portable_catalog_path(path: &str) -> bool {
    let Some(name) = path
        .strip_prefix(".opdev/requirements/")
        .and_then(|s| s.strip_suffix(".json"))
    else {
        return false;
    };
    let stem = name.to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|p| {
            stem.strip_prefix(p)
                .is_some_and(|v| v.len() == 1 && matches!(v.as_bytes()[0], b'1'..=b'9'))
        });
    !reserved
        && !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_".contains(&b))
}
