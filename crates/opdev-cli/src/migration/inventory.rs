//! Bounded ownership/input inventory shared by migration preview and continuation.
use super::{MigrationPlan as Plan, sha};
use anyhow::{Context, Result, ensure};
use opdev_core::Outcome;
use opdev_project::ProjectManifest;
use std::{collections::BTreeMap, fs, io::Read, path::Path, process::Command};

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

fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    ensure!(
        regular(&meta) && meta.is_file() && meta.len() <= 8 * 1024 * 1024,
        "Migration input is linked, nonregular or exceeds 8 MiB; preserve it and resolve the migration explicitly"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "Migration input grew beyond its bound"
    );
    Ok(Some(bytes))
}

pub(super) fn text(path: &Path) -> Result<Option<String>> {
    read(path)?
        .map(String::from_utf8)
        .transpose()
        .context("Migration text input must be UTF-8; nothing normalized")
}

pub(super) fn target(root: &Path, relative: &str) -> Result<()> {
    ensure!(
        opdev_project::clean_adoption::valid_path(relative),
        "Migration paths must be contained relative paths"
    );
    let mut path = root.to_owned();
    let parts: Vec<_> = relative.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        path.push(part);
        if let Ok(meta) = fs::symlink_metadata(&path) {
            ensure!(
                regular(&meta)
                    && (if index + 1 == parts.len() {
                        meta.is_file()
                    } else {
                        meta.is_dir()
                    }),
                "Migration target or parent is linked/nonregular; review it explicitly, no implicit conversion"
            );
        }
    }
    let result = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "--literal-pathspecs",
            "ls-files",
            "--stage",
            "-z",
            "--",
            relative,
        ])
        .output()?;
    ensure!(
        result.status.success(),
        "Cannot inspect migration target Git mode"
    );
    for entry in result.stdout.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let header = entry
            .split(|b| *b == b'\t')
            .next()
            .context("Git mode missing")?;
        ensure!(
            (header.starts_with(b"100644 ") || header.starts_with(b"100755 "))
                && header.ends_with(b" 0"),
            "Migration target is a Git link or unresolved entry; no implicit conversion"
        );
    }
    Ok(())
}

fn walk(
    root: &Path,
    relative: &str,
    files: &mut BTreeMap<String, Option<String>>,
    total: &mut usize,
) -> Result<()> {
    ensure!(
        files.len() < 4096 && relative.split('/').count() <= 32,
        "Migration inventory reached its bounds; no partial inventory accepted"
    );
    let path = root.join(relative);
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    ensure!(
        regular(&meta),
        "Linked migration namespace requires an explicit ownership resolution"
    );
    if meta.is_dir() {
        // Directory entries are included too: new empty junk directories matter.
        files.insert(format!("@directory:{relative}"), Some("directory".into()));
        for entry in fs::read_dir(path)? {
            let name = entry?
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("Migration names must be UTF-8"))?;
            walk(root, &format!("{relative}/{name}"), files, total)?;
        }
    } else {
        let bytes = read(&path)?.context("Inventory file disappeared")?;
        *total += bytes.len();
        ensure!(
            *total <= 32 * 1024 * 1024,
            "Migration inventory exceeds 32 MiB; preserve originals and review a bounded strategy"
        );
        files.insert(relative.into(), Some(sha(&bytes)));
    }
    Ok(())
}

fn namespace(root: &Path) -> Result<BTreeMap<String, Option<String>>> {
    let mut files = BTreeMap::new();
    walk(root, ".opdev", &mut files, &mut 0)?;
    Ok(files)
}

pub(super) fn ci_path(path: &str) -> bool {
    let yaml = matches!(
        Path::new(path).extension().and_then(|s| s.to_str()),
        Some("yml" | "yaml")
    );
    path == ".gitlab-ci.yml"
        || yaml && (path.starts_with(".github/workflows/") || path.starts_with(".gitlab/"))
}

pub(super) fn validate_ci(
    root: &Path,
    path: &str,
    after: &str,
    replacements: &BTreeMap<String, String>,
) -> Result<()> {
    if path == ".gitlab-ci.yml" {
        let parsed = opdev_ci::gitlab::resolve(after, |relative| {
            replacements
                .get(relative)
                .cloned()
                .map_or_else(|| opdev_ci::gitlab::read_local(root, relative), Ok)
        });
        if let Err(problem) = parsed {
            ensure!(
                problem.outcome != Outcome::Error && problem.outcome != Outcome::Failed,
                "Proposed CI cannot be parsed safely; correct the explicit candidate before migration"
            );
        }
    } else {
        serde_saphyr::from_str::<serde_json::Value>(after)
            .map_err(|_| anyhow::anyhow!("Proposed CI YAML is malformed; nothing changed"))?;
    }
    Ok(())
}

pub(super) fn collect(plan: &mut Plan, plugin: Option<&Path>) -> Result<()> {
    plan.inputs = namespace(&plan.root)?;
    for path in [
        "AGENTS.md",
        "CLAUDE.md",
        ".opdev/project.yaml",
        ".opdev/adoption.yaml",
        ".opdev/evidence.yaml",
        ".opdev/guidance.md",
    ] {
        target(&plan.root, path)?;
        plan.inputs
            .insert(path.into(), read(&plan.root.join(path))?.map(|b| sha(&b)));
    }
    for finding in
        opdev_project::layout::inspect(&plan.root, 1, opdev_project::layout::Scope::WorkingTree)?
            .findings
    {
        // These exact owned targets are replaced/retired by the coordinated plan.
        // Every other structural/content-ownership finding remains blocking.
        if !plan.cleanup.iter().any(|a| a.path == finding.path)
            && !matches!(
                finding.path.as_str(),
                ".opdev/project.yaml"
                    | ".opdev/adoption.yaml"
                    | ".opdev/guidance.md"
                    | ".opdev/evidence.yaml"
            )
        {
            plan.finding(
                "layout",
                Outcome::Failed,
                format!(
                    "{}: {}. {}",
                    finding.path, finding.problem, finding.next_step
                ),
            );
        }
    }
    crate::inspection::inspect_plugin(plan, plugin)?;
    crate::inspection::inspect_ci(plan)?;
    // Recheck later directory additions as well as the files visible today.
    for directory in [".github/workflows", ".gitlab"] {
        walk(&plan.root, directory, &mut plan.inputs, &mut 0)?;
    }
    Ok(())
}

pub(super) fn owners(plan: &mut Plan, original: &ProjectManifest, candidate: &ProjectManifest) {
    for (name, authority) in &original.authorities {
        if candidate.authorities.get(name) != Some(authority)
            && plan
                .authority_review_reference
                .as_ref()
                .is_none_or(|r| r.trim().is_empty())
        {
            plan.finding("authority", Outcome::Failed, format!("Authority '{name}' changes ownership. Resolve its explicit content migration separately; coordinated policy migration cannot move or relabel it"));
        }
    }
    for (name, authority) in &candidate.authorities {
        if authority.location.starts_with(".opdev/")
            && !authority.location.starts_with(".opdev/docs/")
        {
            plan.finding("authority", Outcome::Failed, format!("Authority '{name}' still names a legacy internal location. Classify and migrate its real content before selecting strict layout"));
        }
    }
}

pub(super) fn unchanged(plan: &Plan) -> Result<()> {
    let mut current_namespace = namespace(&plan.root)?;
    for directory in [".github/workflows", ".gitlab"] {
        walk(&plan.root, directory, &mut current_namespace, &mut 0)?;
    }
    for key in current_namespace.keys() {
        ensure!(
            plan.inputs.contains_key(key)
                || plan.changes.iter().any(|c| c.path == *key)
                || key.strip_prefix("@directory:").is_some_and(|dir| plan
                    .changes
                    .iter()
                    .any(|c| c.after.is_some() && c.path.starts_with(&format!("{dir}/")))),
            "New namespace content appeared after migration review; preserve it and inspect before continuing"
        );
    }
    for (key, expected) in &plan.inputs {
        if key.starts_with("@directory:") {
            ensure!(
                current_namespace.get(key) == Some(expected)
                    || (!current_namespace.contains_key(key)
                        && super::cleanup::retired_directory(
                            plan,
                            key.trim_start_matches("@directory:")
                        )),
                "Migration directory inventory changed"
            );
            continue;
        }
        if let Some(change) = plan.changes.iter().find(|c| c.path == *key) {
            target(&plan.root, key)?;
            let current = text(&plan.root.join(key))?;
            ensure!(
                current == change.before || current == change.after,
                "Migration target changed outside the reviewed before/after states; preserve the later edit"
            );
        } else {
            let path = key
                .strip_prefix("@external:")
                .map_or_else(|| plan.root.join(key), std::path::PathBuf::from);
            ensure!(
                &read(&path)?.map(|b| sha(&b)) == expected,
                "Inspected migration input changed; no earlier snapshot substituted"
            );
        }
    }
    for change in &plan.changes {
        target(&plan.root, &change.path)?;
        let current = text(&plan.root.join(&change.path))?;
        ensure!(
            current == change.before || current == change.after,
            "Migration target changed; original recovery cannot overwrite later edits"
        );
    }
    Ok(())
}

impl crate::inspection::Inventory for Plan {
    fn root(&self) -> &Path {
        &self.root
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn read(&mut self, _key: &str, path: &Path) -> Result<Option<String>> {
        let value = text(path)?;
        let key = path.strip_prefix(&self.root).map_or_else(
            |_| format!("@external:{}", path.display()),
            |p| p.to_string_lossy().replace('\\', "/"),
        );
        self.inputs
            .insert(key, value.as_ref().map(|s| sha(s.as_bytes())));
        Ok(value)
    }
    fn finding(&mut self, component: &str, outcome: Outcome, detail: impl Into<String>) {
        let detail = if component == "ci_qualification" {
            "CI inventory is read-only. Only separately supplied exact CI replacements may be applied by this migration; declared versions and review references do not prove runtime capability or pipeline qualification.".into()
        } else {
            detail.into()
        };
        Self::finding(self, component, outcome, detail);
    }
}
