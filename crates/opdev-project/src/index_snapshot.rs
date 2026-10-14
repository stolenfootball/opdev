//! Inspect staged blobs without asking Git to write an index or tree object.
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

struct Entry {
    mode: String,
    object: String,
    path: Vec<u8>,
}

pub(crate) struct IndexSnapshot {
    /// Exact Git tree identity, computed but not materialized in the object store.
    pub tree: String,
    entries: Vec<Entry>,
}

fn git(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).arg("--no-replace-objects");
    command.env("GIT_NO_LAZY_FETCH", "1");
    command.env("GIT_OPTIONAL_LOCKS", "0");
    command.env("GIT_ALLOW_PROTOCOL", "");
    command
}

pub(crate) fn ensure_read_only_objects(root: &Path) -> Result<()> {
    let supported = git(root)
        .args(["--no-lazy-fetch", "--version"])
        .stderr(Stdio::null())
        .output()?;
    if supported.status.success() {
        return Ok(());
    }
    reject_legacy_promisor(root)
}

fn reject_legacy_promisor(root: &Path) -> Result<()> {
    // Older Git ignores GIT_NO_LAZY_FETCH. Inspect configuration before any object
    // read; ordinary complete repositories remain supported without an upgrade.
    let output = git(root)
        .args([
            "config",
            "--get-regexp",
            r"^(extensions\.partialclone|remote\..*\.(promisor|partialclonefilter))$",
        ])
        .stderr(Stdio::null())
        .output()?;
    ensure!(
        output.status.code() == Some(1),
        "Read-only partial-clone inspection needs Git with --no-lazy-fetch support, or a complete repository without promisor configuration. Inspect Git configuration and use the normal authorized setup workflow; nothing fetched or repaired"
    );
    Ok(())
}

impl IndexSnapshot {
    pub fn read(root: &Path) -> Result<Self> {
        ensure_read_only_objects(root)?;
        let intent = intent_to_add_paths(root)?;
        let output = git(root).args(["ls-files", "--stage", "-z"]).output()?;
        ensure!(
            output.status.success(),
            "Cannot read staged Git entries; inspect repository access and index state. No writes attempted"
        );
        ensure!(
            output.stdout.len() <= 64 * 1024 * 1024,
            "Staged path inventory exceeds 64 MiB"
        );
        let mut entries = Vec::new();
        for record in output.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
            let split = record
                .iter()
                .position(|b| *b == b'\t')
                .context("Invalid staged Git entry")?;
            let fields: Vec<_> = std::str::from_utf8(&record[..split])?
                .split_whitespace()
                .collect();
            ensure!(
                fields.len() == 3 && fields[2] == "0",
                "Staged Git entries are unmerged; resolve the actual index before inspection"
            );
            ensure!(
                matches!(fields[0], "100644" | "100755" | "120000" | "160000"),
                "Unsupported staged Git mode"
            );
            ensure!(
                matches!(fields[1].len(), 40 | 64)
                    && fields[1].bytes().all(|b| b.is_ascii_hexdigit()),
                "Invalid staged object identity"
            );
            let path = record[split + 1..].to_vec();
            if intent.contains(&path) {
                continue;
            }
            ensure!(
                path.split(|b| *b == b'/').count() <= 128,
                "Staged path exceeds 128 components"
            );
            ensure!(
                path.split(|b| *b == b'/')
                    .all(|p| !p.is_empty() && p != b"." && p != b".."),
                "Invalid staged path"
            );
            entries.push(Entry {
                mode: fields[0].into(),
                object: fields[1].into(),
                path,
            });
        }
        validate_objects(root, &entries)?;
        let tree = tree_identity(root, &entries.iter().collect::<Vec<_>>(), 0)?;
        Ok(Self { tree, entries })
    }

    /// The same entry encoding as ls-tree, using the original staged object IDs.
    pub fn listing(&self, directory: &str) -> Vec<u8> {
        let prefix = format!("{directory}/");
        let mut output = Vec::new();
        for entry in &self.entries {
            if entry.path.starts_with(prefix.as_bytes()) || entry.path == directory.as_bytes() {
                let kind = if entry.mode == "160000" {
                    "commit"
                } else {
                    "blob"
                };
                output.extend_from_slice(
                    format!("{} {kind} {}\t", entry.mode, entry.object).as_bytes(),
                );
                output.extend_from_slice(&entry.path);
                output.push(0);
            }
        }
        output
    }
}

fn intent_to_add_paths(root: &Path) -> Result<BTreeSet<Vec<u8>>> {
    let mut sets = Vec::new();
    for option in ["--ita-visible-in-index", "--ita-invisible-in-index"] {
        let output = git(root)
            .args([
                "diff",
                "--cached",
                "--name-only",
                "-z",
                "--no-ext-diff",
                "--no-renames",
                option,
                "--",
            ])
            .output()?;
        ensure!(
            output.status.success(),
            "Cannot inspect intent-to-add entries"
        );
        ensure!(
            output.stdout.len() <= 64 * 1024 * 1024,
            "Staged path inventory exceeds 64 MiB"
        );
        sets.push(
            output
                .stdout
                .split(|b| *b == 0)
                .filter(|p| !p.is_empty())
                .map(<[u8]>::to_vec)
                .collect::<BTreeSet<_>>(),
        );
    }
    Ok(sets[0].difference(&sets[1]).cloned().collect())
}

fn validate_objects(root: &Path, entries: &[Entry]) -> Result<()> {
    // Gitlinks may name commits only present in the submodule repository.
    // Small batches keep both pipes bounded without a reader/writer deadlock.
    let blobs: Vec<_> = entries.iter().filter(|e| e.mode != "160000").collect();
    for batch in blobs.chunks(64) {
        let mut child = git(root)
            .args(["cat-file", "--batch-check=%(objectname) %(objecttype)"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut input = child.stdin.take().context("Git object input unavailable")?;
        for entry in batch {
            writeln!(input, "{}", entry.object)?;
        }
        drop(input);
        let output = child.wait_with_output()?;
        ensure!(
            output.status.success(),
            "Cannot inspect staged object types"
        );
        let text = std::str::from_utf8(&output.stdout)?;
        let lines: Vec<_> = text.lines().collect();
        ensure!(
            lines.len() == batch.len(),
            "Incomplete staged object inspection"
        );
        for (line, entry) in lines.into_iter().zip(batch) {
            ensure!(
                line == format!("{} blob", entry.object),
                "A staged file object is missing or is not a blob; inspection does not fetch or repair objects"
            );
        }
    }
    Ok(())
}

fn tree_identity(root: &Path, entries: &[&Entry], offset: usize) -> Result<String> {
    // Git orders a directory as though its name ends in '/'. Other names use NUL.
    let mut children: BTreeMap<Vec<u8>, Vec<&Entry>> = BTreeMap::new();
    for entry in entries {
        let relative = &entry.path[offset..];
        let end = relative.iter().position(|b| *b == b'/');
        let name = relative[..end.unwrap_or(relative.len())].to_vec();
        let mut key = name;
        key.push(if end.is_some() { b'/' } else { 0 });
        children.entry(key).or_default().push(entry);
    }
    let mut bytes = Vec::new();
    for key in children.keys().filter(|key| key.last() == Some(&0)) {
        let mut directory = key.clone();
        *directory.last_mut().context("Empty staged name")? = b'/';
        ensure!(
            !children.contains_key(&directory),
            "Staged file/directory collision"
        );
    }
    for (key, group) in children {
        let directory = key.last() == Some(&b'/');
        let name = &key[..key.len() - 1];
        let (mode, object) = if directory {
            (
                "40000",
                tree_identity(root, &group, offset + name.len() + 1)?,
            )
        } else {
            ensure!(group.len() == 1, "Ambiguous staged Git path");
            (group[0].mode.as_str(), group[0].object.clone())
        };
        bytes.extend_from_slice(mode.as_bytes());
        bytes.push(b' ');
        bytes.extend_from_slice(name);
        bytes.push(0);
        for pair in object.as_bytes().chunks_exact(2) {
            bytes.push(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?);
        }
    }
    // No -w: Git validates and hashes its native tree format but writes nothing.
    let mut child = git(root)
        .args(["hash-object", "-t", "tree", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .context("Git tree input unavailable")?
        .write_all(&bytes)?;
    let output = child.wait_with_output()?;
    ensure!(
        output.status.success(),
        "Cannot compute staged Git tree identity; no index or object writes attempted"
    );
    let tree = String::from_utf8(output.stdout)?.trim().to_owned();
    ensure!(
        matches!(tree.len(), 40 | 64) && tree.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid computed tree identity"
    );
    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn run(root: &Path, args: &[&str]) -> Result<String> {
        let output = git(root).args(args).output()?;
        ensure!(output.status.success(), "Git test setup failed: {args:?}");
        Ok(String::from_utf8(output.stdout)?.trim().into())
    }

    #[test]
    fn legacy_git_partial_clone_is_rejected_before_object_reads() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        run(root, &["init", "--quiet"])?;
        reject_legacy_promisor(root)?;
        run(root, &["config", "remote.origin.promisor", "true"])?;
        run(
            root,
            &[
                "config",
                "remote.origin.url",
                "https://example.invalid/repo",
            ],
        )?;
        let before = fs::read(root.join(".git/config"))?;
        assert!(reject_legacy_promisor(root).is_err());
        assert_eq!(fs::read(root.join(".git/config"))?, before);
        Ok(())
    }

    #[test]
    fn read_only_identity_matches_git_for_formats_modes_and_nested_names() -> Result<()> {
        for format in ["sha1", "sha256"] {
            let dir = tempfile::tempdir()?;
            let root = dir.path();
            run(
                root,
                &["init", "--quiet", &format!("--object-format={format}")],
            )?;
            assert_eq!(IndexSnapshot::read(root)?.tree, run(root, &["write-tree"])?);
            for path in ["a.b", "a0", "a/x", "a/sub/z", "space name", "é.txt"] {
                let file = root.join(path);
                fs::create_dir_all(file.parent().context("Test file parent")?)?;
                fs::write(file, path)?;
            }
            run(root, &["add", "."])?;
            fs::write(root.join("future"), "not staged content")?;
            run(root, &["add", "--intent-to-add", "future"])?;
            run(root, &["update-index", "--chmod=+x", "a.b"])?;
            let blob = run(root, &["rev-parse", ":a0"])?;
            run(
                root,
                &[
                    "update-index",
                    "--add",
                    "--cacheinfo",
                    "120000",
                    &blob,
                    "link",
                ],
            )?;
            // An unavailable submodule commit is valid, unlike a missing file blob.
            let missing = "1".repeat(blob.len());
            run(
                root,
                &[
                    "update-index",
                    "--add",
                    "--cacheinfo",
                    "160000",
                    &missing,
                    "submodule",
                ],
            )?;
            let index = fs::read(root.join(".git/index"))?;
            fs::write(root.join(".git/index.lock"), "owned test lock")?;
            let snapshot = IndexSnapshot::read(root)?;
            assert_eq!(fs::read(root.join(".git/index"))?, index);
            assert_eq!(
                fs::read_to_string(root.join(".git/index.lock"))?,
                "owned test lock"
            );
            fs::remove_file(root.join(".git/index.lock"))?;
            assert_eq!(snapshot.tree, run(root, &["write-tree"])?);
            run(
                root,
                &[
                    "update-index",
                    "--add",
                    "--cacheinfo",
                    "100644",
                    &missing,
                    "missing",
                ],
            )?;
            assert!(
                IndexSnapshot::read(root)
                    .err()
                    .context("missing blob must fail")?
                    .to_string()
                    .contains("missing")
            );
        }
        Ok(())
    }
}
