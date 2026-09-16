use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use thiserror::Error;

const START_MARKER: &str = "<!-- opdev:start -->";
const END_MARKER: &str = "<!-- opdev:end -->";

const AGENTS_BLOCK: &str = r#"<!-- opdev:start -->
## OpDev development protocol

This repository is initialized for OpDev. Apply this protocol seamlessly to every software-development task; do not ask whether to use it. `.opdev/project.yaml` is the machine-readable project contract and the starting point for finding project-specific commands, authorities, risks, tests, delivery requirements, assurance profiles, and context routes. Do not assume that design or operational material lives in a conventional `docs/` directory. Existing authority locations take precedence over folder defaults. Treat non-OpDev folders as project-owned; do not create, move, overwrite, or repurpose their contents merely to match a convention. Record reviewed locations in the project contract.

If the `opdev` CLI or required OpDev agent integration is unavailable, tell the user before substantive development begins and offer to install it. Do not silently replace OpDev with an improvised process. You may continue only when the user chooses to proceed without it or the current task does not require the unavailable capability.

For every planning objective, including roadmaps, task breakdowns, prioritization and "what is the next step?", use the OpDev planning guidance. Honor explicit user scope, constraints and requested order. Otherwise prefer the next thin, demonstrable consumer outcome over completing whole technical layers; keep later increments provisional. Bound enabling work by its supported outcome, uncertainty and exit evidence. Include acceptance tests, feedback and applicable delivery/recovery within each increment, and replan from evidence. Advice does not authorize implementation or tracker writes. Keep planning facts in existing authorities; no extra project document is required.

For each development task:

For initialization or conversion to OpDev, follow the adoption guidance: assess every supplied practice, preserve adequate existing choices, research only unresolved project-specific gaps, and record reviewed dispositions in `.opdev/adoption.yaml`. Pending work is not complete adoption; only optional practices may be explicitly ignored, and no disposition waives core requirements. Use `opdev adoption check` on a capable CLI before claiming completion. Existing projects without a record require explicit assessment adoption; ordinary tasks reuse decisions and do not restart setup or research.

Adoption requests authorize assessment, not policy selection. Present preserve/change/ignore/unresolved choices and wait for an actual developer response or explicit bounded delegation; never infer approval from an owner label or invoke approval commands to manufacture consent. Offer retaining a non-main trunk name or renaming it to main, while separately resolving multi-branch integration/release workflows. Assess applicability from capabilities, not missing configuration or a project-kind label. Report scaffolding, approval, implementation and verification separately; green integration CI does not establish completed adoption or delivery readiness. Use one adoption work item unless separate issues are approved or independently owned work requires them.

1. Read `.opdev/project.yaml` before planning or editing. Load the authorities listed by `context.always` and the authorities in every route relevant to the task. Treat those sources as project facts; resolve contradictions explicitly instead of guessing.
2. Identify the intended outcome, acceptance evidence, affected consumers, and applicable quality risks. Use the project work authority for active decisions and progress when one is declared.
3. Keep design effort proportional to risk, reversibility, novelty, and blast radius. Record durable decisions in the declared decision or architecture authority when the project contract routes the task there.
4. Implement in small, reviewable increments that preserve delivered behavior except where the accepted change intentionally modifies it. Integrate with the declared trunk frequently and keep branches short-lived.
5. Add or update automated tests for behavioral changes. Add regression protection for escaped defects unless a specific justification is recorded. Exercise the suites declared for the relevant stage both before and after integration. Keep retries visible; quarantines require an owner, tracked remediation, and an expiry. Use coverage as risk evidence, not as a substitute for meaningful assertions.
6. Run canonical commands from `commands` as argument vectors in their declared working directories. Do not reinterpret them through a shell or substitute a different command without reporting the difference. Project extensions may add checks but may not weaken, replace, or mark core requirements satisfied.
7. When a rule needs evidence that automation cannot infer, record only reviewable facts in `.opdev/evidence.yaml`. For a new ledger, stage all material files and use the schema-backed `opdev evidence bootstrap` review flow: every decision begins as `review_required`, durable project facts stay separate from fingerprint-bound change facts, and the CLI writes only after explicit review. For direct maintenance, obtain the fingerprint with `opdev evidence fingerprint` and bind change assertions to that exact staged state. Never copy forward an assertion whose evidence was not rechecked; future repository states intentionally invalidate it.
8. Report outcomes using OpDev semantics: `passed`, `failed`, `unverified`, `not_applicable`, `error`, or `migration_required`. Only `passed` and justified `not_applicable` satisfy a required rule. Missing evidence is `unverified`, not a pass. Tooling failure is `error`, not a product failure. Do not claim a gate passed when a required rule has another outcome.

MinimumCD requirements are mandatory. Every change is version controlled and delivered through CI; CI is the exclusive delivery path. Use one integration trunk, stop delivery when it is red, and restore it as the highest priority. Build a deployable artifact once, identify it immutably, and promote that same artifact rather than rebuilding. Qualify in a production-like environment when the project has runtime environments. Keep configuration versioned and tested while externalizing environment-specific values. Delivery must have a single consumer-facing path and an automated, tested recovery strategy appropriate to the software. Preserve or deliberately migrate already-delivered behavior.

For experimental work, separate integration from activation and distinguish not enabled from not shipped. Use the OpDev experiment guidance and existing work authority to record ownership, review date, decision criteria, isolation, stable defaults, supported configuration tests, recovery, and cleanup. Test stable and supported experimental configurations before and after integration; qualify each distributed artifact variant independently. Review overdue experiments and remove temporary machinery or deliberately adopt a supported permanent option. Experiments never waive core gates; ordinary changes need no experiment record.

Before declaring work complete, reconcile implementation, tests, project authorities, delivery behavior, and tracked work. Run the applicable canonical checks and provide concise evidence, including anything not run or still requiring migration. Never hide a failing or unverified requirement behind a summary success statement.
<!-- opdev:end -->"#;

const CLAUDE_BLOCK: &str = r"<!-- opdev:start -->
@AGENTS.md
<!-- opdev:end -->";

/// How reconciliation affected one managed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChange {
    /// A new file was created.
    Created,
    /// An existing managed block was updated.
    Updated,
    /// The current file already contained the desired guidance.
    Unchanged,
}

/// Reconciliation result for one project-level agent instruction file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedFile {
    /// Absolute file path.
    pub path: PathBuf,
    /// Resulting change.
    pub change: FileChange,
}

/// Read-only candidate for a managed guidance file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentFilePreview {
    /// Reconciliation metadata.
    pub file: ManagedFile,
    /// Exact existing contents, or absence.
    pub before: Option<String>,
    /// Proposed complete contents, including preserved project text.
    pub after: String,
}

/// Errors produced while reconciling agent instructions.
#[derive(Debug, Error)]
pub enum BootstrapError {
    /// A linked/non-regular file is not a safe managed-write target.
    #[error("agent instructions `{path}` must be a regular file, not a link or directory")]
    UnsafeTarget {
        /// File path.
        path: PathBuf,
    },
    /// A file changed after inspection; nothing should overwrite the new state.
    #[error("agent instructions `{path}` changed after preview; preview again")]
    StalePreview {
        /// File path.
        path: PathBuf,
    },
    /// An instruction file could not be read.
    #[error("could not read agent instructions `{path}`: {source}")]
    Read {
        /// File path.
        path: PathBuf,
        /// Filesystem error.
        source: std::io::Error,
    },

    /// An instruction file could not be written.
    #[error("could not write agent instructions `{path}`: {source}")]
    Write {
        /// File path.
        path: PathBuf,
        /// Filesystem error.
        source: std::io::Error,
    },

    /// Existing marker structure is ambiguous and cannot be edited safely.
    #[error("agent instructions `{path}` contain malformed or duplicate OpDev markers")]
    MalformedMarkers {
        /// File path.
        path: PathBuf,
    },
}

/// Creates or updates the managed `OpDev` sections in `AGENTS.md` and
/// `CLAUDE.md` while preserving all project-owned content outside the markers.
///
/// # Errors
///
/// Returns [`BootstrapError`] for unreadable files, failed writes, or ambiguous
/// marker layouts. Ambiguous files are never modified.
pub fn reconcile_agent_files(root: &Path) -> Result<Vec<ManagedFile>, BootstrapError> {
    let preview = preview_agent_files(root)?;
    apply_agent_preview(&preview)
}

/// Plans both files before writing either. Does not create directories or files.
///
/// # Errors
/// Returns an error for unreadable, linked or ambiguous instruction files.
pub fn preview_agent_files(root: &Path) -> Result<Vec<AgentFilePreview>, BootstrapError> {
    Ok(vec![
        preview_file(&root.join("AGENTS.md"), AGENTS_BLOCK, false)?,
        preview_file(&root.join("CLAUDE.md"), CLAUDE_BLOCK, true)?,
    ])
}

/// Applies a previously reviewed candidate, checking all inputs before writing.
/// Each changed file is staged and atomically replaced; this is not a multi-file
/// transaction. On an I/O failure inspect the files and preview again.
///
/// # Errors
/// Returns an error for changed inputs, unsafe targets, staging or commit errors.
pub fn apply_agent_preview(
    preview: &[AgentFilePreview],
) -> Result<Vec<ManagedFile>, BootstrapError> {
    for item in preview {
        if read_target(&item.file.path)? != item.before {
            return Err(BootstrapError::StalePreview {
                path: item.file.path.clone(),
            });
        }
    }
    let mut staged = Vec::new();
    for item in preview
        .iter()
        .filter(|item| item.file.change != FileChange::Unchanged)
    {
        let path = &item.file.path;
        let mut temporary = tempfile::NamedTempFile::new_in(
            path.parent().unwrap_or(Path::new(".")),
        )
        .map_err(|source| BootstrapError::Write {
            path: path.clone(),
            source,
        })?;
        temporary
            .write_all(item.after.as_bytes())
            .map_err(|source| BootstrapError::Write {
                path: path.clone(),
                source,
            })?;
        if let Ok(metadata) = fs::metadata(path) {
            temporary
                .as_file()
                .set_permissions(metadata.permissions())
                .map_err(|source| BootstrapError::Write {
                    path: path.clone(),
                    source,
                })?;
        }
        staged.push((item, temporary));
    }
    for (item, temporary) in staged {
        if read_target(&item.file.path)? != item.before {
            return Err(BootstrapError::StalePreview {
                path: item.file.path.clone(),
            });
        }
        if item.before.is_none() {
            temporary.persist_noclobber(&item.file.path)
        } else {
            temporary.persist(&item.file.path)
        }
        .map_err(|error| BootstrapError::Write {
            path: item.file.path.clone(),
            source: error.error,
        })?;
    }
    Ok(preview.iter().map(|item| item.file.clone()).collect())
}

fn read_target(path: &Path) -> Result<Option<String>, BootstrapError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            return Err(BootstrapError::UnsafeTarget {
                path: path.to_path_buf(),
            });
        }
        Ok(_) => {}
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(BootstrapError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
    }
    fs::read_to_string(path)
        .map(Some)
        .map_err(|source| BootstrapError::Read {
            path: path.to_path_buf(),
            source,
        })
}

fn preview_file(
    path: &Path,
    desired_block: &str,
    accept_existing_agents_import: bool,
) -> Result<AgentFilePreview, BootstrapError> {
    let existing = read_target(path)?;

    let (next, change) = match existing {
        None => (format!("{desired_block}\n"), FileChange::Created),
        Some(ref content)
            if accept_existing_agents_import
                && !content.contains(START_MARKER)
                && !content.contains(END_MARKER)
                && content.lines().any(|line| line.trim() == "@AGENTS.md") =>
        {
            (content.clone(), FileChange::Unchanged)
        }
        Some(ref content) => {
            let next = replace_or_append_block(path, content, desired_block)?;
            let change = if next == *content {
                FileChange::Unchanged
            } else {
                FileChange::Updated
            };
            (next, change)
        }
    };

    Ok(AgentFilePreview {
        file: ManagedFile {
            path: path.to_path_buf(),
            change,
        },
        before: existing,
        after: next,
    })
}

fn replace_or_append_block(
    path: &Path,
    content: &str,
    desired_block: &str,
) -> Result<String, BootstrapError> {
    let starts: Vec<_> = content.match_indices(START_MARKER).collect();
    let ends: Vec<_> = content.match_indices(END_MARKER).collect();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => {
            let newline = newline_style(content);
            let block = desired_block.replace('\n', newline);
            let trimmed = content.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                Ok(format!("{block}{newline}"))
            } else {
                Ok(format!("{trimmed}{newline}{newline}{block}{newline}"))
            }
        }
        ([(start, _)], [(end, _)]) if start < end => {
            let end = end + END_MARKER.len();
            let newline = newline_style(content);
            let block = desired_block.replace('\n', newline);
            let mut next = String::with_capacity(content.len() + block.len());
            next.push_str(&content[..*start]);
            next.push_str(&block);
            next.push_str(&content[end..]);
            Ok(next)
        }
        _ => Err(BootstrapError::MalformedMarkers {
            path: path.to_path_buf(),
        }),
    }
}

fn newline_style(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_is_read_only_and_stale_second_file_blocks_all_writes()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let preview = preview_agent_files(directory.path())?;
        assert_eq!(fs::read_dir(directory.path())?.count(), 0);
        fs::write(directory.path().join("CLAUDE.md"), "new user content\n")?;
        assert!(matches!(
            apply_agent_preview(&preview),
            Err(BootstrapError::StalePreview { .. })
        ));
        assert!(!directory.path().join("AGENTS.md").exists());
        assert_eq!(
            fs::read_to_string(directory.path().join("CLAUDE.md"))?,
            "new user content\n"
        );
        Ok(())
    }

    #[test]
    fn invalid_second_target_never_updates_first() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        fs::write(directory.path().join("AGENTS.md"), "existing\n")?;
        fs::create_dir(directory.path().join("CLAUDE.md"))?;
        assert!(matches!(
            reconcile_agent_files(directory.path()),
            Err(BootstrapError::UnsafeTarget { .. })
        ));
        assert_eq!(
            fs::read_to_string(directory.path().join("AGENTS.md"))?,
            "existing\n"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn linked_guidance_is_not_followed_and_atomic_write_preserves_other_hardlink()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let path = outside.path().join("owned.md");
        fs::write(&path, "unrelated\n")?;
        std::os::unix::fs::symlink(&path, directory.path().join("AGENTS.md"))?;
        assert!(matches!(
            preview_agent_files(directory.path()),
            Err(BootstrapError::UnsafeTarget { .. })
        ));
        fs::remove_file(directory.path().join("AGENTS.md"))?;
        fs::hard_link(&path, directory.path().join("AGENTS.md"))?;
        reconcile_agent_files(directory.path())?;
        assert_eq!(fs::read_to_string(path)?, "unrelated\n");
        Ok(())
    }

    #[test]
    fn creates_both_files_and_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let first = reconcile_agent_files(directory.path())?;
        assert!(
            first
                .iter()
                .all(|result| result.change == FileChange::Created)
        );
        let agents = fs::read_to_string(directory.path().join("AGENTS.md"))?;
        assert!(agents.contains("If the `opdev` CLI"));
        assert!(agents.contains("MinimumCD requirements are mandatory"));
        assert!(fs::read_to_string(directory.path().join("CLAUDE.md"))?.contains("@AGENTS.md"));

        let second = reconcile_agent_files(directory.path())?;
        assert!(
            second
                .iter()
                .all(|result| result.change == FileChange::Unchanged)
        );
        Ok(())
    }

    #[test]
    fn preserves_project_owned_content() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        fs::write(
            directory.path().join("AGENTS.md"),
            "# Project rules\n\nKeep me.\n",
        )?;
        reconcile_agent_files(directory.path())?;
        let agents = fs::read_to_string(directory.path().join("AGENTS.md"))?;
        assert!(agents.starts_with("# Project rules\n\nKeep me.\n"));
        assert!(agents.contains(START_MARKER));
        Ok(())
    }

    #[test]
    fn upgrades_only_the_managed_block() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        fs::write(
            directory.path().join("AGENTS.md"),
            "before\n<!-- opdev:start -->\nold\n<!-- opdev:end -->\nafter\n",
        )?;
        reconcile_agent_files(directory.path())?;
        let agents = fs::read_to_string(directory.path().join("AGENTS.md"))?;
        assert!(agents.starts_with("before\n"));
        assert!(agents.ends_with("\nafter\n"));
        assert!(!agents.contains("\nold\n"));
        Ok(())
    }

    #[test]
    fn refuses_ambiguous_markers_without_writing() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("AGENTS.md");
        let original = "<!-- opdev:start -->\nbroken\n";
        fs::write(&path, original)?;
        assert!(matches!(
            reconcile_agent_files(directory.path()),
            Err(BootstrapError::MalformedMarkers { .. })
        ));
        assert_eq!(fs::read_to_string(path)?, original);
        Ok(())
    }

    #[test]
    fn accepts_an_existing_claude_import_without_duplication()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        fs::write(
            directory.path().join("CLAUDE.md"),
            "# Claude\n\n@AGENTS.md\n",
        )?;
        reconcile_agent_files(directory.path())?;
        let claude = fs::read_to_string(directory.path().join("CLAUDE.md"))?;
        assert_eq!(claude.matches("@AGENTS.md").count(), 1);
        Ok(())
    }
}
