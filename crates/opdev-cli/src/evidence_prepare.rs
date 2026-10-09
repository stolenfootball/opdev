//! Mechanical preparation of an existing schema-2 ledger; never approval.
use anyhow::{Context, Result, bail};
use clap::Args;
use opdev_core::Outcome;
use opdev_project::{
    AcceptanceEvidence, ChangeEvidence, EVIDENCE_PATH, EvidenceLedger, TrackedEvidence,
    staged_fingerprint,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Args)]
pub(super) struct PrepareArgs {
    /// Read an explicit retained draft ledger instead of the legacy project ledger.
    #[arg(long)]
    ledger_input: Option<PathBuf>,
    /// Create a new reviewed ledger outside source; never overwrite retained history.
    #[arg(long, requires_all = ["write", "ledger_input"])]
    ledger_output: Option<PathBuf>,
    /// Save a newly prepared, unreviewed draft in private CLI-owned state; emit its path as JSON.
    #[arg(long, requires = "input", conflicts_with_all = ["draft", "write"])]
    retain_draft: bool,
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Acceptance YAML: explicit conditions/mappings, with mechanical fields omitted.
    #[arg(long, required_unless_present = "draft", conflicts_with = "draft")]
    input: Option<PathBuf>,
    /// Work authority for a new draft; must agree with an existing current entry.
    #[arg(long, requires = "input", required_unless_present = "draft")]
    work: Option<String>,
    /// Previously prepared draft; preview by default, never silently refresh bindings.
    #[arg(long)]
    draft: Option<PathBuf>,
    /// Atomically replace the ledger after explicit review of this draft.
    #[arg(long, requires = "draft")]
    write: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    schema: u32,
    fingerprint: String,
    ledger_sha256: String,
    work: String,
    acceptance: AcceptanceEvidence,
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 8 * 1024 * 1024 {
        bail!("input must be a regular file of at most 8 MiB");
    }
    Ok(fs::read(path)?)
}

fn ledger_bytes(root: &Path) -> Result<Vec<u8>> {
    let parent = fs::symlink_metadata(root.join(".opdev"))?;
    if !parent.is_dir() || parent.file_type().is_symlink() {
        bail!(
            ".opdev must be a real directory, not a symbolic link; choose the actual project directory with --root. Nothing written"
        );
    }
    read(&root.join(EVIDENCE_PATH))
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn bind(root: &Path, reference: &mut Value) -> Result<()> {
    let path = reference["path"]
        .as_str()
        .context("reference needs a path")?
        .to_owned();
    let excerpt = reference["excerpt"]
        .as_str()
        .context("reference needs an excerpt")?
        .to_owned();
    // Unknown fields are rejected, not silently discarded when filling the hash.
    reference["sha256"] = Value::String("0".repeat(64));
    let _: TrackedEvidence = serde_json::from_value(reference.clone())?;
    *reference = serde_json::to_value(TrackedEvidence::bind(root, path, excerpt)?)?;
    Ok(())
}

fn prepare(root: &Path, path: &Path) -> Result<AcceptanceEvidence> {
    let mut input: Value = serde_saphyr::from_slice(&read(path)?)?;
    input
        .as_object_mut()
        .context("acceptance input must be an object")?;
    for condition in input["conditions"]
        .as_array_mut()
        .context("conditions must be an array")?
    {
        condition
            .as_object_mut()
            .context("condition must be an object")?;
        if condition["source"].get("selector").is_some() {
            let source: opdev_project::RequirementSource =
                serde_json::from_value(condition["source"].clone())?;
            source.verify(root)?;
        } else {
            bind(root, &mut condition["source"])?;
        }
    }
    for mapping in input["verifications"]
        .as_array_mut()
        .context("verifications must be an array")?
    {
        mapping
            .as_object_mut()
            .context("mapping must be an object")?;
        bind(root, &mut mapping["target"])?;
        mapping["outcome"] = json!("unverified");
    }
    input["review"] = serde_json::to_value(AcceptanceEvidence::default().review)?;
    Ok(serde_json::from_value(input)?)
}

fn candidate(root: &Path, draft: &Draft, bytes: &[u8]) -> Result<EvidenceLedger> {
    if draft.schema != 1
        || draft.ledger_sha256 != sha(bytes)
        || draft.fingerprint != staged_fingerprint(root)?
        || draft.work.trim().is_empty()
    {
        bail!(
            "stale or invalid draft: the staged files, evidence record, draft version, or work reference no longer match. Nothing written. Inspect what changed, prepare a new draft with --input and --work, and review it before using --write"
        );
    }
    let mut ledger: EvidenceLedger = serde_saphyr::from_slice(bytes)?;
    let (_, manifest) = crate::load_project(root)?;
    let catalog = manifest.catalog()?;
    ledger.validate(&catalog)?;
    if ledger.schema != 2 {
        bail!(
            "schema-2 ledger required: .opdev/evidence.yaml uses an older record format. Review a migration to format 2 before preparing change evidence; do not just change the schema number. Nothing written"
        );
    }
    for condition in &draft.acceptance.conditions {
        condition.source.verify(root)?;
    }
    for mapping in &draft.acceptance.verifications {
        mapping.target.verify(root)?;
    }
    if let Some(change) = ledger
        .changes
        .iter_mut()
        .find(|c| c.fingerprint == draft.fingerprint)
    {
        if change.work != draft.work {
            bail!(
                "current entry has a different work authority: this change is already linked to another work item. Compare that link with --work and resolve which item owns the change; nothing written"
            );
        }
        change.acceptance = Some(draft.acceptance.clone());
    } else {
        ledger.changes.push(ChangeEvidence {
            fingerprint: draft.fingerprint.clone(),
            work: draft.work.clone(),
            assertions: vec![],
            acceptance: Some(draft.acceptance.clone()),
        });
    }
    ledger.validate(&catalog)?;
    Ok(ledger)
}

fn reviewed(draft: &Draft) -> Result<()> {
    let review = &draft.acceptance.review;
    if !matches!(review.outcome, Outcome::Passed | Outcome::Failed)
        || [&review.reviewer, &review.reference, &review.rationale]
            .iter()
            .any(|s| s.trim().is_empty())
        || review.subject_sha256 != draft.acceptance.digest(&draft.fingerprint, &draft.work)?
    {
        bail!(
            "explicit current review required: review this draft's expected results and test mappings, then record the reviewer, decision reference, rationale and current subject digest (the identifier of the reviewed contents). Previewing calculates the identifier but is not approval; nothing written"
        );
    }
    Ok(())
}

pub(super) fn run(args: &PrepareArgs) -> Result<()> {
    let (root, manifest) = crate::load_project(&args.root)?;
    if args.write
        && (args.ledger_input.is_some() || manifest.assurance.review_storage.is_some())
        && args.ledger_output.is_none()
    {
        bail!(
            "External review preparation needs a new --ledger-output outside source; the legacy ledger will not be created or replaced"
        );
    }
    let read_input = || match &args.ledger_input {
        Some(path) => read(path),
        None => ledger_bytes(&root),
    };
    let before = read_input()?;
    let draft = if let Some(path) = &args.draft {
        serde_saphyr::from_slice::<Draft>(&read(path)?)?
    } else {
        Draft {
            schema: 1,
            fingerprint: staged_fingerprint(&root)?,
            ledger_sha256: sha(&before),
            work: args.work.clone().context("work required")?,
            acceptance: prepare(&root, args.input.as_deref().context("input required")?)?,
        }
    };
    let ledger = candidate(&root, &draft, &before)?;
    if args.draft.is_none() {
        let yaml = serde_saphyr::to_string(&draft)?;
        if args.retain_draft {
            let path = crate::local_state::retain_draft(&root, yaml.as_bytes())?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"schema":1,"draft":path,"review":"unverified","qualification":"unverified"})
                )?
            );
        } else {
            print!("{yaml}");
        }
        eprintln!(
            "Draft only; mappings and review are unverified. No suites ran and ledger unchanged. This proposes links between expected results and tests; it does not verify them or approve the change."
        );
    } else if args.write {
        reviewed(&draft)?;
        if let Some(output) = &args.ledger_output {
            let output = crate::evidence_bundle::export_destination(&root, output)?;
            if read_input()? != before || staged_fingerprint(&root)? != draft.fingerprint {
                bail!("Inputs changed before review export; nothing written");
            }
            crate::local_state::write_new(&output, ledger.to_yaml()?.as_bytes())?;
            println!(
                "Reviewed candidate created outside source; original history retained. No upload, checks or approval inferred."
            );
            return Ok(());
        }
        // Git-owned temporary storage does not become unindexed product content.
        let git_dir = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["rev-parse", "--absolute-git-dir"])
            .output()?;
        if !git_dir.status.success() {
            bail!("cannot locate Git storage for atomic apply");
        }
        let git_dir = String::from_utf8(git_dir.stdout)?;
        let mut file = tempfile::NamedTempFile::new_in(git_dir.trim())?;
        file.write_all(ledger.to_yaml()?.as_bytes())?;
        file.as_file().sync_all()?;
        if ledger_bytes(&root)? != before || staged_fingerprint(&root)? != draft.fingerprint {
            bail!("inputs changed before apply; nothing written");
        }
        file.persist(root.join(EVIDENCE_PATH))?;
        println!(
            "Acceptance updated; history/assertions preserved. Qualification unverified: no checks ran."
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema": 1, "qualification": "unverified", "subject_sha256": draft.acceptance.digest(&draft.fingerprint, &draft.work)?,
                "review_current": reviewed(&draft).is_ok(), "candidate": ledger,
            }))?
        );
    }
    Ok(())
}
