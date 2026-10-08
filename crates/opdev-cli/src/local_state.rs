//! Private continuation aids, never a second policy authority or qualification cache.
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use clap::{Args, Subcommand};
use opdev_engine::{CheckOptions, CheckReport, plan_checks};
use opdev_project::ProjectManifest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::workflow_records::Subject;

const LIMIT: u64 = 8 * 1024 * 1024;
const LIMITS: &str = "Local continuation only. References are not authenticated decisions, saved reports cannot qualify another check, and no release is authorized. Read the original work and decision sources before reuse. No cleanup or upload was performed.";

#[derive(Debug, Args)]
pub(super) struct StateArgs {
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    #[command(subcommand)]
    command: StateCommand,
}

#[derive(Debug, Subcommand)]
enum StateCommand {
    /// Resolve platform storage and local clone/worktree identities without creating files.
    Resolve,
    /// Inspect derived context against current source; no project commands or network.
    Inspect,
    /// Inspect one immutable attempt; missing completion is unfinished/unknown, never success.
    Attempt { id: String },
    /// Store explicit derived references, not decisions or a transcript, under an expected head.
    Context {
        #[arg(long)]
        input: PathBuf,
        /// Current context SHA-256, or 'absent' for a new pointer.
        #[arg(long)]
        expected: String,
    },
}

#[derive(Debug, Serialize)]
pub(super) struct Locations {
    schema: u32,
    repository_id: String,
    worktree_id: String,
    state: PathBuf,
    cache: PathBuf,
    pub(super) worktree: PathBuf,
    context: PathBuf,
    drafts: PathBuf,
    runs: PathBuf,
}

/// Only derived references: deliberately no approval, outcome, transcript or policy fields.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextPointer {
    schema: u32,
    subject: Subject,
    work: String,
    references: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandIdentity {
    id: String,
    command_sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Environment {
    os: String,
    arch: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    schema: u32,
    kind: String,
    subject: Subject,
    started_at_ms: u128,
    runtime_sha256: String,
    runtime_version: String,
    environment: Environment,
    execute_checks: bool,
    commands: Vec<CommandIdentity>,
    qualification: opdev_core::Outcome,
    limits: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    schema: u32,
    completed_at_ms: u128,
    report_sha256: String,
    source_unchanged: bool,
    observed_subject_after: Option<Subject>,
    qualification: opdev_core::Outcome,
    limits: String,
}

fn inspect_attempt(paths: &Locations, id: &str) -> Result<()> {
    ensure!(
        id.starts_with("attempt-")
            && id.len() < 128
            && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        "Use one attempt directory name, not a path"
    );
    let directory = paths.runs.join(id);
    let start: Start = serde_json::from_slice(
        &read(&directory.join("start.json"))?
            .context("Attempt start is missing; do not infer an execution result")?,
    )?;
    ensure!(
        start.schema == 1
            && start.kind == "check_attempt"
            && start.qualification == opdev_core::Outcome::Unverified,
        "Unsupported attempt schema or qualification claim"
    );
    start.subject.validate()?;
    let bytes = read(&directory.join("completion.json"))?;
    let completion: Option<Completion> =
        bytes.as_deref().map(serde_json::from_slice).transpose()?;
    let mut report: Option<CheckReport> = None;
    if let Some(ref finish) = completion {
        ensure!(
            finish.schema == 1
                && finish.qualification == opdev_core::Outcome::Unverified
                && finish.completed_at_ms >= start.started_at_ms,
            "Unsupported or inconsistent completion record"
        );
        ensure!(
            finish.source_unchanged
                == (finish.observed_subject_after.as_ref() == Some(&start.subject)),
            "Completion subject contradicts its unchanged-source claim"
        );
        let bytes =
            read(&directory.join("report.json"))?.context("Completed attempt report is missing")?;
        ensure!(
            sha(&bytes) == finish.report_sha256,
            "Attempt report changed; do not reuse it"
        );
        report = Some(serde_json::from_slice(&bytes)?);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"schema":1, "state":if completion.is_some() {"completed_observation"} else {"unfinished_or_interrupted"},
        "start":start,"completion":completion,"report":report,"qualification":"unverified","limits":LIMITS})
        )?
    );
    Ok(())
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn now() -> Result<u128> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

fn git_path(root: &Path, option: &str) -> Result<PathBuf> {
    let result = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--path-format=absolute", option])
        .output()?;
    ensure!(
        result.status.success(),
        "Cannot resolve the local Git directory"
    );
    let text = std::str::from_utf8(&result.stdout)?;
    Ok(PathBuf::from(text.trim_end_matches(['\r', '\n'])).canonicalize()?)
}

fn identity(path: &Path) -> Result<String> {
    let meta = fs::metadata(path)?;
    #[cfg(windows)]
    let physical = {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            meta.creation_time() != 0,
            "Filesystem has no usable directory identity"
        );
        meta.creation_time().to_string()
    };
    #[cfg(unix)]
    let physical = {
        use std::os::unix::fs::MetadataExt;
        format!("{}:{}:{:?}", meta.dev(), meta.ino(), meta.created().ok())
    };
    Ok(sha(&serde_json::to_vec(&(path, physical))?))
}

fn absolute_env(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

fn platform_roots() -> Result<(PathBuf, PathBuf)> {
    #[cfg(windows)]
    let (state, cache) = {
        let base = absolute_env("LOCALAPPDATA")
            .context("LOCALAPPDATA must be an absolute user directory")?
            .join("opdev");
        (base.join("state"), base.join("cache"))
    };
    #[cfg(target_os = "macos")]
    let (state, cache) = {
        let home = absolute_env("HOME").context("HOME must be an absolute user directory")?;
        (
            home.join("Library/Application Support/opdev/state"),
            home.join("Library/Caches/opdev"),
        )
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let (state, cache) = {
        let home = absolute_env("HOME").context("HOME must be an absolute user directory")?;
        (
            absolute_env("XDG_STATE_HOME")
                .unwrap_or_else(|| home.join(".local/state"))
                .join("opdev"),
            absolute_env("XDG_CACHE_HOME")
                .unwrap_or_else(|| home.join(".cache"))
                .join("opdev"),
        )
    };
    let state = match std::env::var_os("OPDEV_STATE_DIR") {
        Some(value) => {
            let path = PathBuf::from(value);
            ensure!(
                path.is_absolute(),
                "OPDEV_STATE_DIR must be absolute; nothing changed"
            );
            path
        }
        None => state,
    };
    Ok((state, cache))
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

/// Reject linked ancestors and traversal before every operation; no hostile-race guarantee.
fn inspect_path(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "State paths must be absolute");
    ensure!(
        !path
            .components()
            .any(|p| matches!(p, Component::ParentDir | Component::CurDir)),
        "State paths must not contain traversal"
    );
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => ensure!(
                regular(&meta),
                "State path contains a link or reparse point; nothing followed"
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn observed_absolute(path: &Path) -> Result<PathBuf> {
    inspect_path(path)?;
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let parent = path.parent().context("State directory needs a parent")?;
    Ok(observed_absolute(parent)?.join(path.file_name().context("State directory needs a name")?))
}

pub(super) fn resolve(root: &Path) -> Result<Locations> {
    let source = git_path(root, "--show-toplevel")?;
    let common = git_path(root, "--git-common-dir")?;
    let git = git_path(root, "--absolute-git-dir")?;
    let (state, cache) = platform_roots()?;
    let state = observed_absolute(&state)?;
    let cache = observed_absolute(&cache)?;
    for storage in [&state, &cache] {
        ensure!(
            ![&source, &common, &git]
                .iter()
                .any(|p| storage.starts_with(p) || p.starts_with(storage)),
            "State/cache must be separate from source and Git metadata; nothing changed"
        );
    }
    ensure!(
        !state.starts_with(&cache) && !cache.starts_with(&state),
        "Essential state and disposable cache must not overlap"
    );
    let repository_id = identity(&common)?;
    let worktree_id = identity(&git)?;
    let worktree = state
        .join("repositories")
        .join(&repository_id)
        .join("worktrees")
        .join(&worktree_id);
    inspect_path(&worktree)?;
    Ok(Locations {
        schema: 1,
        repository_id,
        worktree_id,
        context: worktree.join("context.json"),
        drafts: worktree.join("drafts"),
        runs: worktree.join("runs"),
        state,
        cache,
        worktree,
    })
}

fn private_directory(path: &Path) -> Result<()> {
    inspect_path(path)?;
    if path.exists() {
        ensure!(path.is_dir(), "State directory is not a directory");
        return Ok(());
    }
    private_directory(path.parent().context("State directory needs a parent")?)?;
    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let builder = {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = builder;
        builder.mode(0o700);
        builder
    };
    match builder.create(path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            inspect_path(path)?;
            ensure!(
                path.is_dir(),
                "Concurrent state creation did not create a directory"
            );
        }
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    inspect_path(path)?;
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    ensure!(
        meta.is_file() && meta.len() <= LIMIT,
        "State input must be a regular file of at most 8 MiB"
    );
    let mut bytes = Vec::new();
    File::open(path)?.take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= LIMIT,
        "State input grew beyond the read limit"
    );
    Ok(Some(bytes))
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    inspect_path(path)?;
    let parent = path.parent().context("State output needs a parent")?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path)?;
    Ok(())
}

pub(super) fn retain_draft(root: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let paths = resolve(root)?;
    private_directory(&paths.drafts)?;
    let directory = tempfile::Builder::new()
        .prefix("draft-")
        .tempdir_in(&paths.drafts)?
        .keep();
    let path = directory.join("acceptance.yaml");
    write_new(&path, bytes)?;
    Ok(path)
}

fn subject(root: &Path, manifest: &ProjectManifest, stage: &str) -> Result<Subject> {
    Ok(Subject {
        schema: 1,
        source_sha256: opdev_project::staged_fingerprint(root)?,
        configuration_sha256: sha(&serde_json::to_vec(manifest)?),
        stage: stage.into(),
        artifact_sha256: None,
    })
}

fn decode_context(bytes: &[u8]) -> Result<ContextPointer> {
    let context: ContextPointer = serde_json::from_slice(bytes)
        .context("Invalid or unsupported context; nothing repaired")?;
    ensure!(
        context.schema == 1,
        "Unsupported context schema; use a compatible reader"
    );
    context.subject.validate()?;
    ensure!(
        !context.work.trim().is_empty() && context.work.len() <= 4096,
        "Context needs a bounded original work reference"
    );
    ensure!(
        context.references.len() <= 32
            && context.references.iter().all(|r| !r.trim().is_empty()
                && r.len() <= 4096
                && !r.chars().any(char::is_control)),
        "Context accepts at most 32 bounded references, not transcripts or copied decisions"
    );
    Ok(context)
}

fn store_context(paths: &Locations, bytes: &[u8], expected: &str) -> Result<()> {
    decode_context(bytes)?;
    private_directory(&paths.worktree)?;
    let lock = paths.worktree.join("context.lock");
    inspect_path(&lock)?;
    crate::state_io::with_lock(&lock, || {
        let before = read(&paths.context)?;
        if let Some(ref old) = before {
            decode_context(old)?;
        }
        ensure!(
            before
                .as_ref()
                .map_or(expected == "absent", |old| sha(old) == expected),
            "Context changed since inspection; read the current references before retrying"
        );
        if before.as_deref() == Some(bytes) {
            return Ok(());
        }
        // The pointer is replaceable; immutable previous bytes remain available for recovery.
        let history = paths.worktree.join("context-history");
        private_directory(&history)?;
        if let Some(ref old) = before {
            let archived = history.join(format!("{}.json", sha(old)));
            match read(&archived)? {
                Some(existing) => ensure!(
                    existing == *old,
                    "Context history differs; nothing replaced"
                ),
                None => write_new(&archived, old)?,
            }
        }
        let mut temporary = tempfile::NamedTempFile::new_in(&paths.worktree)?;
        temporary.write_all(bytes)?;
        temporary.as_file().sync_all()?;
        ensure!(
            read(&paths.context)? == before,
            "Context changed outside the writer protocol; nothing replaced"
        );
        if before.is_some() {
            temporary.persist(&paths.context)?;
        } else {
            temporary.persist_noclobber(&paths.context)?;
        }
        Ok(())
    })
}

pub(super) fn run(args: &StateArgs) -> Result<ExitCode> {
    let paths = resolve(&args.root)?;
    match &args.command {
        StateCommand::Attempt { id } => inspect_attempt(&paths, id)?,
        StateCommand::Resolve => println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({"locations": paths, "qualification": "unverified", "limits": LIMITS})
            )?
        ),
        StateCommand::Inspect => {
            let bytes = read(&paths.context)?;
            let (root, manifest) = crate::load_project(&args.root)?;
            let context = bytes.as_deref().map(decode_context).transpose()?;
            let current = context
                .as_ref()
                .map(|c| subject(&root, &manifest, &c.subject.stage))
                .transpose();
            let state = match (&context, &current) {
                (None, _) => "missing",
                (Some(c), Ok(Some(s))) if c.subject == *s => "references_only",
                _ => "stale",
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"schema": 1, "state": state,
                "context_sha256": bytes.as_deref().map(sha), "context": context, "qualification": "unverified", "limits": LIMITS}))?
            );
        }
        StateCommand::Context { input, expected } => {
            let input = std::path::absolute(input)?;
            let bytes = read(&input)?.context("Context input is missing")?;
            let context = decode_context(&bytes)?;
            let (root, manifest) = crate::load_project(&args.root)?;
            ensure!(
                context.subject == subject(&root, &manifest, &context.subject.stage)?,
                "Context subject is not current; refresh the references, not past decisions"
            );
            store_context(&paths, &bytes, expected)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"schema":1, "context_sha256":sha(&bytes), "qualification":"unverified", "limits":LIMITS})
                )?
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Owns only paths. Drop intentionally leaves unfinished attempts visible after an error/panic.
pub(super) struct Attempt {
    directory: PathBuf,
    started: Subject,
}

impl Attempt {
    pub(super) fn start(
        root: &Path,
        manifest: &ProjectManifest,
        options: CheckOptions,
    ) -> Result<Self> {
        let paths = resolve(root)?;
        let stage = serde_json::to_value(options.test_stage)?
            .as_str()
            .context("Invalid stage")?
            .to_owned();
        let started = subject(root, manifest, &stage)?;
        let plan = plan_checks(root, manifest, options);
        let commands: Vec<_> = plan
            .commands
            .iter()
            .map(|command| {
                Ok(CommandIdentity {
                    id: command.id.clone(),
                    command_sha256: sha(&serde_json::to_vec(command)?),
                })
            })
            .collect::<Result<_>>()?;
        let executable = std::env::current_exe()?;
        let runtime_sha256 = crate::workflow::content_digest(&executable)?;
        private_directory(&paths.runs)?;
        let directory = tempfile::Builder::new()
            .prefix("attempt-")
            .tempdir_in(&paths.runs)?
            .keep();
        write_new(
            &directory.join("start.json"),
            &serde_json::to_vec_pretty(&Start {
                schema: 1,
                kind: "check_attempt".into(),
                subject: started.clone(),
                started_at_ms: now()?,
                runtime_sha256,
                runtime_version: env!("CARGO_PKG_VERSION").into(),
                environment: Environment {
                    os: std::env::consts::OS.into(),
                    arch: std::env::consts::ARCH.into(),
                },
                execute_checks: options.execute_checks,
                commands,
                qualification: opdev_core::Outcome::Unverified,
                limits: LIMITS.into(),
            })?,
        )?;
        eprintln!("Retaining attempt at {}", directory.display());
        Ok(Self { directory, started })
    }

    pub(super) fn finish(
        self,
        root: &Path,
        manifest: &ProjectManifest,
        report: &CheckReport,
    ) -> Result<()> {
        let report = serde_json::to_vec_pretty(report)?;
        write_new(&self.directory.join("report.json"), &report)?;
        let after = subject(root, manifest, &self.started.stage).ok();
        write_new(
            &self.directory.join("completion.json"),
            &serde_json::to_vec_pretty(&Completion {
                schema: 1,
                completed_at_ms: now()?,
                report_sha256: sha(&report),
                source_unchanged: after.as_ref() == Some(&self.started),
                observed_subject_after: after,
                qualification: opdev_core::Outcome::Unverified,
                limits: LIMITS.into(),
            })?,
        )?;
        Ok(())
    }
}
