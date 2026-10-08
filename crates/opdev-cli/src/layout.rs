//! Read-only inventory against a proposed namespace; not migration or qualification.
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Subcommand, ValueEnum};
use opdev_project::{AdoptionRecord, ProjectManifest};
use serde::Serialize;

const LIMIT: u64 = 8 * 1024 * 1024;
const REQUIRED: [&str; 3] = [
    ".opdev/project.yaml",
    ".opdev/adoption.yaml",
    ".opdev/guidance.md",
];
const LIMITS: &str = "Proposed layout inspection only: no policy was selected, no file was changed, and no project checks ran. This is not an atomic filesystem snapshot. Content purpose, document adequacy, asset references and fresh-agent behavior still need review. Existing policy and evidence remain authoritative; do not delete the evidence ledger before verified retention and retrieval.";

#[derive(Debug, Args)]
pub(super) struct LayoutArgs {
    #[command(subcommand)]
    command: LayoutCommand,
}

#[derive(Debug, Subcommand)]
enum LayoutCommand {
    /// Inspect the proposed strict layout without migrating or enforcing it on this project.
    Inspect {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Exact proposed directory contract; unsupported versions are errors.
        #[arg(long)]
        layout_version: u32,
        /// Working tree includes untracked/ignored files; index reads staged Git blobs only.
        #[arg(long, value_enum, default_value_t = Scope::WorkingTree)]
        scope: Scope,
        #[arg(long, value_enum, default_value_t = crate::OutputFormat::Human)]
        format: crate::OutputFormat,
    },
}

#[derive(Debug, Clone, Copy, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Scope {
    WorkingTree,
    Index,
}

#[derive(Serialize)]
struct Finding {
    path: String,
    problem: &'static str,
    next_step: &'static str,
}

#[derive(Serialize)]
struct Report {
    schema: u32,
    kind: &'static str,
    layout_version: u32,
    scope: Scope,
    qualification: &'static str,
    findings: Vec<Finding>,
    inspected_files: Vec<String>,
    limits: &'static str,
}

impl Report {
    fn finding(&mut self, path: &str, problem: &'static str, next_step: &'static str) {
        self.findings.push(Finding {
            path: path.into(),
            problem,
            next_step,
        });
    }
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    ensure!(
        output.status.success(),
        "Could not inspect Git layout metadata; no files changed"
    );
    Ok(output.stdout)
}

fn index(root: &Path) -> Result<BTreeMap<String, String>> {
    let bytes = git(root, &["ls-files", "--stage", "-z", "--", ".opdev"])?;
    let mut files = BTreeMap::new();
    for entry in bytes.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let text = std::str::from_utf8(entry).context("Layout paths must be UTF-8")?;
        let (header, path) = text
            .split_once('\t')
            .context("Unexpected Git index entry")?;
        let fields: Vec<_> = header.split_whitespace().collect();
        ensure!(fields.len() == 3, "Unexpected Git index fields");
        let mode = if fields[2] == "0" {
            fields[0]
        } else {
            "unmerged"
        };
        files.insert(path.to_owned(), mode.to_owned());
    }
    Ok(files)
}

fn regular(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    !meta.file_type().is_symlink() && (meta.is_file() || meta.is_dir())
}

fn directory(path: &str) -> bool {
    matches!(
        path,
        ".opdev"
            | ".opdev/docs"
            | ".opdev/docs/specs"
            | ".opdev/docs/decisions"
            | ".opdev/docs/assets"
    ) || path.starts_with(".opdev/docs/assets/") && portable(path)
}

fn portable(path: &str) -> bool {
    path.split('/').all(|s| {
        !s.is_empty()
            && s != "."
            && s != ".."
            && !s.ends_with([' ', '.'])
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    })
}

fn permitted(path: &str) -> bool {
    if REQUIRED.contains(&path) {
        return true;
    }
    if !portable(path) {
        return false;
    }
    let Some(relative) = path.strip_prefix(".opdev/docs/") else {
        return false;
    };
    if matches!(
        relative,
        "design.md" | "development.md" | "testing.md" | "delivery.md"
    ) {
        return true;
    }
    if let Some(name) = relative.strip_prefix("specs/") {
        return !name.contains('/')
            && name
                .strip_suffix(".md")
                .is_some_and(|stem| !stem.is_empty());
    }
    if let Some(name) = relative.strip_prefix("decisions/") {
        return !name.contains('/')
            && name
                .strip_suffix(".md")
                .and_then(|s| s.split_once('-'))
                .is_some_and(|(id, label)| !id.is_empty() && !label.is_empty());
    }
    relative.starts_with("assets/")
        && Path::new(relative)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e, "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "pdf"))
}

fn walk(
    root: &Path,
    path: &str,
    files: &mut BTreeMap<String, String>,
    report: &mut Report,
    visited: &mut usize,
) -> Result<()> {
    *visited += 1;
    ensure!(
        *visited <= 4096 && path.split('/').count() <= 32,
        "Layout inspection reached its safety limit; inspect a smaller namespace before retrying. No files changed"
    );
    let absolute = root.join(path);
    let meta = fs::symlink_metadata(&absolute)?;
    if !regular(&meta) {
        files.insert(path.into(), "unsafe".into());
    } else if meta.is_dir() {
        if !directory(path) {
            report.finding(
                path,
                "Directory is outside the proposed namespace",
                "Review its purpose and owner; no directory was moved or deleted",
            );
            return Ok(());
        }
        for entry in fs::read_dir(absolute)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("Layout paths must be UTF-8"))?;
            walk(root, &format!("{path}/{name}"), files, report, visited)?;
        }
    } else {
        files.insert(path.into(), "regular".into());
    }
    Ok(())
}

fn contents(root: &Path, path: &str, scope: Scope) -> Result<Vec<u8>> {
    match scope {
        Scope::WorkingTree => {
            ensure!(
                fs::metadata(root.join(path))?.len() <= LIMIT,
                "Layout input exceeds 8 MiB inspection limit"
            );
            let mut bytes = Vec::new();
            fs::File::open(root.join(path))?
                .take(LIMIT + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() as u64 <= LIMIT,
                "Layout input grew beyond 8 MiB inspection limit"
            );
            Ok(bytes)
        }
        Scope::Index => {
            let object = format!(":{path}");
            let size = git(root, &["cat-file", "-s", &object])?;
            ensure!(
                std::str::from_utf8(&size)?.trim().parse::<u64>()? <= LIMIT,
                "Layout input exceeds 8 MiB inspection limit"
            );
            git(root, &["cat-file", "blob", &object])
        }
    }
}

fn inspect_file(
    root: &Path,
    path: &str,
    mode: &str,
    scope: Scope,
    report: &mut Report,
) -> Result<()> {
    if !matches!(mode, "regular" | "100644" | "100755") {
        report.finding(path, "Not a regular resolved file", "Review the link, special file or merge conflict explicitly; its contents were not followed");
        return Ok(());
    }
    if path == ".opdev/evidence.yaml" {
        report.finding(path, "Legacy evidence needs a verified storage migration", "Preserve this ledger until exported evidence can be retained and retrieved; do not delete or archive it inside .opdev");
        return Ok(());
    }
    if !permitted(path) {
        report.finding(path, "File is outside the proposed namespace", "Classify its actual purpose and use the declared document, work or local-state owner; no automatic cleanup");
        return Ok(());
    }
    report.inspected_files.push(path.into());
    if !matches!(path, ".opdev/project.yaml" | ".opdev/adoption.yaml") {
        return Ok(());
    }
    let bytes = contents(root, path, scope)?;
    let yaml = std::str::from_utf8(&bytes).context("Configuration must be UTF-8")?;
    let valid = match path {
        ".opdev/project.yaml" => ProjectManifest::from_yaml(yaml).is_ok(),
        ".opdev/adoption.yaml" => AdoptionRecord::from_yaml(yaml).is_ok(),
        _ => false,
    };
    if !valid {
        report.finding(path, "Unsupported or invalid configuration", "Inspect its schema, duplicate keys, unknown fields and types with a capable CLI; nothing was rewritten");
    }
    Ok(())
}

pub(super) fn run(args: &LayoutArgs) -> Result<ExitCode> {
    let LayoutCommand::Inspect {
        root,
        layout_version,
        scope,
        format,
    } = &args.command;
    if *layout_version != 1 {
        bail!(
            "Unsupported proposed layout version {layout_version}; supported version is 1. Nothing changed"
        );
    }
    let bytes = git(root, &["rev-parse", "--show-toplevel"])?;
    let root = PathBuf::from(std::str::from_utf8(&bytes)?.trim());
    let tracked = index(&root)?;
    let mut report = Report {
        schema: 1,
        kind: "layout_inspection",
        layout_version: 1,
        scope: *scope,
        qualification: "unverified",
        findings: vec![],
        inspected_files: vec![],
        limits: LIMITS,
    };
    let mut files = match scope {
        Scope::Index => tracked,
        Scope::WorkingTree => {
            let mut files = BTreeMap::new();
            match fs::symlink_metadata(root.join(".opdev")) {
                Ok(_) => walk(&root, ".opdev", &mut files, &mut report, &mut 0)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => {
                    return Err(error).context("Could not inspect .opdev; no files changed");
                }
            }
            for (path, mode) in tracked {
                if !matches!(mode.as_str(), "100644" | "100755") {
                    files.insert(path, mode);
                }
            }
            files
        }
    };
    for path in REQUIRED {
        if !files.contains_key(path) {
            report.finding(path, "Required file is absent from the inspected scope", "Preview a coordinated layout migration; do not invent policy, evidence or placeholder documentation");
        }
    }
    for (path, mode) in &mut files {
        inspect_file(&root, path, mode, *scope, &mut report)?;
    }
    report
        .findings
        .sort_by(|a, b| a.path.cmp(&b.path).then(a.problem.cmp(b.problem)));
    match format {
        crate::OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        crate::OutputFormat::Human => {
            println!(
                "Proposed layout inspection: {} finding(s); qualification not run",
                report.findings.len()
            );
            for finding in &report.findings {
                println!(
                    "{}: {}. {}",
                    finding.path.escape_debug(),
                    finding.problem,
                    finding.next_step
                );
            }
            println!("{LIMITS}");
        }
    }
    Ok(ExitCode::from(u8::from(!report.findings.is_empty())))
}
