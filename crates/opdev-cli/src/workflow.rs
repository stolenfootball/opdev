//! Explicit journal updates and a read-only continuation projection.
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use clap::{Args, Subcommand};
use serde::de::DeserializeOwned;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::workflow_records::{
    ContentReference, Event, Journal, Kind, Subject, project_acceptance,
};

const LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Debug, Args)]
pub struct WorkflowArgs {
    #[command(subcommand)]
    command: WorkflowCommand,
}

#[derive(Debug, Subcommand)]
enum WorkflowCommand {
    /// Observe staged source/configuration identities using read-only Git queries; runs no project checks.
    Subject {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        stage: String,
        /// Optional exact artifact to hash without loading it into memory.
        #[arg(long)]
        artifact: Option<PathBuf>,
    },
    /// Inspect retained references against an explicitly supplied current subject; no commands or network.
    Inspect {
        /// Current complete acceptance inventory; missing identity leaves review stale.
        #[arg(long)]
        acceptance_sha256: Option<String>,
        #[arg(long)]
        journal: PathBuf,
        /// JSON subject observation. This command does not authenticate or refresh that observation.
        #[arg(long)]
        subject: PathBuf,
        /// Root of retained evidence references, not a source-discovery command.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Required kinds for this requested outcome; publication is never implied.
        #[arg(long, value_enum)]
        need: Vec<Kind>,
        #[arg(long)]
        json: bool,
    },
    /// Append one explicitly attributed event with an expected previous journal digest.
    Append {
        #[arg(long)]
        journal: PathBuf,
        #[arg(long)]
        event: PathBuf,
        /// SHA-256 returned by inspection, or 'absent' for a new journal.
        #[arg(long)]
        expected: String,
        /// Original work authority; required for a new journal, must match on updates.
        #[arg(long)]
        work: String,
    },
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= LIMIT,
        "workflow input must be a regular file of at most 8 MiB"
    );
    let mut bytes = Vec::new();
    File::open(path)?.take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= LIMIT,
        "workflow input grew beyond 8 MiB"
    );
    Ok(bytes)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes)
        .context("invalid or unsupported workflow JSON; nothing was repaired")
}

fn evidence_matches(root: &Path, evidence: &ContentReference) -> bool {
    let relative = Path::new(&evidence.path);
    if relative.as_os_str().is_empty()
        || evidence.path.contains('\\')
        || evidence.path.contains(':')
        || !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return false;
    }
    let mut path = root.to_path_buf();
    for part in relative.components() {
        path.push(part);
        if !fs::symlink_metadata(&path).is_ok_and(|m| !m.file_type().is_symlink()) {
            return false;
        }
    }
    content_digest(&path).is_ok_and(|digest| digest == evidence.sha256)
}

fn content_digest(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "evidence must be a regular file"
    );
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8 * 1024];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        hasher.update(&buffer[..length]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn append(path: &Path, event: Event, expected: &str, work: &str) -> Result<String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    ensure!(
        fs::symlink_metadata(parent)?.is_dir()
            && !fs::symlink_metadata(parent)?.file_type().is_symlink(),
        "journal parent must be an existing real directory; no folders were created"
    );
    let name = path
        .file_name()
        .context("journal needs a filename")?
        .to_string_lossy();
    let lock_path = parent.join(format!(".{name}.opdev-lock"));
    crate::state_io::with_lock(&lock_path, || {
        append_locked(path, parent, event, expected, work)
    })
}

fn append_locked(
    path: &Path,
    parent: &Path,
    event: Event,
    expected: &str,
    work: &str,
) -> Result<String> {
    let before = match read(path) {
        Ok(bytes) => Some(bytes),
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            None
        }
        Err(error) => return Err(error),
    };
    ensure!(
        before
            .as_ref()
            .map_or(expected == "absent", |bytes| sha(bytes) == expected),
        "this journal changed since it was read; inspect the newer facts and do not overwrite them"
    );
    let mut journal: Journal = match &before {
        Some(bytes) => decode(bytes)?,
        None => Journal {
            schema: 1,
            protocol: "workflow.v1".into(),
            work: work.into(),
            events: vec![],
        },
    };
    journal.validate()?;
    ensure!(
        journal.work == work,
        "work authority differs; do not move decisions to another task implicitly"
    );
    journal.events.push(event);
    journal.validate()?;
    let bytes = serde_json::to_vec_pretty(&journal)?;
    ensure!(
        bytes.len() as u64 <= LIMIT,
        "journal exceeds 8 MiB; no history was discarded"
    );
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    if before.is_some() {
        ensure!(
            read(path)?.as_slice() == before.as_deref().unwrap_or_default(),
            "journal changed outside the writer protocol; nothing replaced"
        );
        temporary.persist(path)?;
    } else {
        temporary.persist_noclobber(path)?;
    }
    Ok(sha(&bytes))
}

fn missing_kinds(
    need: &[Kind],
    findings: &[crate::workflow_records::Finding],
) -> Vec<serde_json::Value> {
    need.iter()
        .filter(|kind| {
            !findings
                .iter()
                .any(|f| f.kind == **kind && f.state == "recorded")
        })
        .map(|kind| json!({"kind":kind,"next_action":kind.next_action()}))
        .collect()
}

pub fn run(args: &WorkflowArgs) -> Result<ExitCode> {
    match &args.command {
        WorkflowCommand::Subject {
            root,
            stage,
            artifact,
        } => {
            let (root, manifest) = crate::load_project(root)?;
            let subject = Subject {
                schema: 1,
                source_sha256: opdev_project::staged_fingerprint(&root)?,
                configuration_sha256: sha(&serde_json::to_vec(&manifest)?),
                stage: stage.clone(),
                artifact_sha256: artifact.as_deref().map(content_digest).transpose()?,
            };
            subject.validate()?;
            println!("{}", serde_json::to_string_pretty(&subject)?);
            Ok(ExitCode::SUCCESS)
        }
        WorkflowCommand::Append {
            journal,
            event,
            expected,
            work,
        } => {
            let event = decode(&read(event)?)?;
            let digest = append(journal, event, expected, work)?;
            println!(
                "Record appended; previous history retained. Journal SHA-256: {digest}\nAttribution is not authenticated approval. No tests ran and no integration or publication was authorized."
            );
            Ok(ExitCode::SUCCESS)
        }
        WorkflowCommand::Inspect {
            acceptance_sha256,
            journal,
            subject,
            root,
            need,
            json: as_json,
        } => {
            let bytes = read(journal)?;
            let subject: Subject = decode(&read(subject)?)?;
            let journal: Journal = decode(&bytes)?;
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let findings =
                project_acceptance(&journal, &subject, now, acceptance_sha256.as_deref(), |r| {
                    evidence_matches(root, r)
                })?;
            let missing = missing_kinds(need, &findings);
            let unresolved = !missing.is_empty()
                || findings
                    .iter()
                    .any(|f| !matches!(f.state, "recorded" | "retired"));
            if *as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"schema":1,
                    "qualification":"unverified", "journal_sha256":sha(&bytes), "work":journal.work,
                    "subject":subject,"findings":findings,"missing":missing,
                    "limits":"Compared retained evidence with the supplied subject only. No commands, provider observations, repairs, authenticated decisions or gate qualification. Read original scope before reusing work; execution permission is not product feedback or publication authority."}))?
                );
            } else {
                println!(
                    "Recorded workflow for {:?}\nJournal SHA-256: {}",
                    journal.work,
                    sha(&bytes)
                );
                for finding in &findings {
                    println!(
                        "{:?}: {} — {}\n  Next: {}\n  Original: {:?}",
                        finding.id,
                        finding.state,
                        finding.explanation,
                        finding.next_action,
                        finding.original_reference
                    );
                }
                for kind in need.iter().filter(|kind| {
                    !findings
                        .iter()
                        .any(|f| f.kind == **kind && f.state == "recorded")
                }) {
                    println!("Missing {kind:?}: {}", kind.next_action());
                }
                println!(
                    "Inspection only: supplied subject not refreshed; no commands or network. Read original scope before reusing work. Permission to run work is not product feedback or permission to publish. No gate is qualified."
                );
            }
            Ok(if unresolved {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_records::tests::record;

    #[test]
    fn stale_and_concurrent_writers_cannot_replace_history() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("journal.json");
        let event = Event::Record(record("first", Kind::Execution));
        let first = append(&path, event.clone(), "absent", "tracker:work")?;
        let original = fs::read(&path)?;
        assert!(append(&path, event.clone(), "absent", "tracker:work").is_err());
        let lock = File::options()
            .read(true)
            .write(true)
            .open(directory.path().join(".journal.json.opdev-lock"))?;
        lock.try_lock()?;
        assert!(
            append(
                &path,
                Event::Record(record("next", Kind::AcceptanceReview)),
                &first,
                "tracker:work"
            )
            .is_err()
        );
        assert_eq!(fs::read(&path)?, original);
        lock.unlock()?;
        // An interrupted temporary write is not a new head or an authoritative record.
        let mut interrupted = tempfile::NamedTempFile::new_in(directory.path())?;
        interrupted.write_all(b"{partial candidate")?;
        let next = append(
            &path,
            Event::Record(record("next", Kind::AcceptanceReview)),
            &first,
            "tracker:work",
        )?;
        assert_ne!(next, first);
        let journal: Journal = decode(&read(&path)?)?;
        assert_eq!(journal.events.len(), 2);
        assert_eq!(journal.events[0], event);
        let current = fs::read(&path)?;
        assert!(
            append(
                &path,
                Event::Record(record("stale", Kind::AcceptanceReview)),
                &first,
                "tracker:work"
            )
            .is_err()
        );
        assert_eq!(fs::read(path)?, current);
        Ok(())
    }

    #[test]
    fn unsupported_journal_and_outside_evidence_are_not_repaired() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("journal.json");
        let original =
            br#"{"schema":99,"protocol":"workflow.v99","work":"tracker:work","events":[]}"#;
        fs::write(&path, original)?;
        assert!(
            append(
                &path,
                Event::Record(record("next", Kind::Execution)),
                &sha(original),
                "tracker:work"
            )
            .is_err()
        );
        assert_eq!(fs::read(&path)?, original);
        fs::write(directory.path().join("decision.txt"), b"neutral decision")?;
        let valid = ContentReference {
            path: "decision.txt".into(),
            sha256: sha(b"neutral decision"),
        };
        assert!(evidence_matches(directory.path(), &valid));
        for path in [
            "../decision.txt",
            "C:/decision.txt",
            "sub/../../decision.txt",
            "sub\\decision.txt",
        ] {
            assert!(!evidence_matches(
                directory.path(),
                &ContentReference {
                    path: path.into(),
                    ..valid.clone()
                }
            ));
        }
        fs::remove_file(directory.path().join("decision.txt"))?;
        assert!(!evidence_matches(directory.path(), &valid));
        Ok(())
    }
}
