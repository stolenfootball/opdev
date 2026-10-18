//! Explicit practical migration using the same bounded path checks and atomic writer.
use super::{Change, inventory, sha};
use anyhow::{Context, Result, ensure};
use opdev_project::{
    AgentFilePreview, FileChange, ManagedFile,
    practical::{Contract, Requirements, read_regular},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: u32,
    project: Contract,
    assurance_change_reference: String,
    /// Exact original obligation -> actual decision removing it. No implicit waivers.
    #[serde(default)]
    retired_obligations: BTreeMap<String, String>,
    #[serde(default)]
    retired_requirements: BTreeMap<String, String>,
    /// Complete explicit replacements; current files not named here are preserved.
    #[serde(default)]
    files: BTreeMap<String, String>,
    /// Exact obsolete files with reviewed reasons. Never recursive.
    #[serde(default)]
    retire: BTreeMap<String, String>,
    #[serde(default)]
    retire_directories: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: u32,
    kind: String,
    root: PathBuf,
    runtime: String,
    identity: String,
    assurance_change_reference: String,
    retired_obligations: BTreeMap<String, String>,
    inputs: BTreeMap<String, Option<String>>,
    retired_requirements: BTreeMap<String, String>,
    changes: Vec<Change>,
    retire_directories: Vec<String>,
}

impl Plan {
    fn digest(&self) -> Result<String> {
        let mut value = serde_json::to_value(self)?;
        value["identity"] = serde_json::json!("");
        Ok(sha(&serde_json::to_vec(&value)?))
    }
}

pub(super) fn run(
    root: &Path,
    bytes: &[u8],
    resume: bool,
    apply: Option<&str>,
    recovery: Option<&Path>,
) -> Result<ExitCode> {
    let root =
        PathBuf::from(crate::practical::git(root, &["rev-parse", "--show-toplevel"])?.trim())
            .canonicalize()?;
    let plan = if resume {
        serde_json::from_slice::<Plan>(bytes)?
    } else {
        prepare(&root, bytes)?
    };
    ensure!(
        plan.schema == 1
            && plan.kind == "practical_migration"
            && plan.root == root
            && plan.identity == plan.digest()?
            && plan.runtime == sha(&fs::read(std::env::current_exe()?)?),
        "Migration recovery identity, checkout or runtime changed; nothing written"
    );
    validate_targets(&plan)?;
    unchanged(&plan)?;
    if let Some(expected) = apply {
        ensure!(
            expected == plan.identity,
            "Migration inputs changed since preview; nothing written"
        );
        if !resume {
            let destination = crate::evidence_bundle::export_destination(
                &root,
                recovery.context("Apply needs --recovery-output outside source")?,
            )?;
            let bytes = serde_json::to_vec_pretty(&plan)?;
            ensure!(
                bytes.len() <= 8 * 1024 * 1024,
                "Migration recovery exceeds 8 MiB; use a smaller reviewed change"
            );
            crate::local_state::write_new(&destination, &bytes)?;
            ensure!(
                crate::local_state::read(&destination)?.as_ref() == Some(&bytes),
                "Recovery readback failed; nothing changed"
            );
        }
        apply_plan(&plan)?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema":1,"plan":plan,"applied":apply.is_some(),"verification":"not_run",
            "limits":"Exact reviewed file migration, not authenticated consent, product verification or release. Writes are atomic per file. Keep temporary recovery until verification, then remove it; no permanent history archive is required."
        }))?
    );
    Ok(ExitCode::SUCCESS)
}

fn prepare(root: &Path, bytes: &[u8]) -> Result<Plan> {
    let request: Request = serde_json::from_slice(bytes)?;
    ensure!(
        request.schema == 2 && !request.assurance_change_reference.trim().is_empty(),
        "Migration requires schema 2 and actual assurance-change review"
    );
    request.project.validate()?;
    ensure!(
        request.files.len() <= 256
            && request.retire.len() <= 256
            && request.retire_directories.len() <= 64,
        "Migration exceeds bounded change count"
    );
    let mut plan = Plan {
        schema: 1,
        kind: "practical_migration".into(),
        root: root.into(),
        runtime: sha(&fs::read(std::env::current_exe()?)?),
        identity: String::new(),
        assurance_change_reference: request.assurance_change_reference,
        retired_obligations: request.retired_obligations,
        retired_requirements: request.retired_requirements,
        inputs: inventory::namespace(root)?,
        changes: vec![],
        retire_directories: request.retire_directories,
    };
    for file in opdev_project::preview_practical_agent_files(root)? {
        replacement(
            &mut plan,
            &file
                .file
                .path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/"),
            Some(file.after),
            "Updated routing; unrelated instructions preserved",
        )?;
    }
    replacement(
        &mut plan,
        ".opdev/project.yaml",
        Some(serde_saphyr::to_string(&request.project)?),
        "Explicit policy selection; review loss of default provenance and traceability",
    )?;
    for (path, content) in request.files {
        ensure!(
            request.project.requirements.contains(&path)
                || request.project.policies.contains(&path)
                || inventory::ci_path(&path),
            "Replacement is not a selected definition or CI file"
        );
        if inventory::ci_path(&path) {
            serde_saphyr::from_str::<serde_json::Value>(&content)?;
        }
        replacement(
            &mut plan,
            &path,
            Some(content),
            "Explicit reviewed replacement",
        )?;
    }
    for (path, reason) in request.retire {
        ensure!(
            !reason.trim().is_empty()
                && path.starts_with(".opdev/")
                && !matches!(path.as_str(), ".opdev/project.yaml" | ".opdev/guidance.md")
                && !request.project.requirements.contains(&path)
                && !request.project.policies.contains(&path),
            "Retirement needs an obsolete unselected .opdev file and reason"
        );
        replacement(&mut plan, &path, None, &reason)?;
    }
    validate_targets(&plan)?;
    validate_candidate(&mut plan, &request.project)?;
    plan.identity = plan.digest()?;
    Ok(plan)
}

fn replacement(plan: &mut Plan, path: &str, after: Option<String>, reason: &str) -> Result<()> {
    inventory::target(&plan.root, path)?;
    let before = inventory::text(&plan.root.join(path))?;
    ensure!(
        after.is_some() || before.is_some(),
        "Retirement target {path} is absent"
    );
    plan.inputs
        .insert(path.into(), before.as_ref().map(|s| sha(s.as_bytes())));
    plan.changes.push(Change {
        path: path.into(),
        before,
        after,
        reason: reason.into(),
    });
    Ok(())
}

fn validate_targets(plan: &Plan) -> Result<()> {
    let mut seen = BTreeSet::new();
    for change in &plan.changes {
        ensure!(
            seen.insert(&change.path)
                && (change.path.starts_with(".opdev/")
                    || matches!(change.path.as_str(), "AGENTS.md" | "CLAUDE.md")
                    || inventory::ci_path(&change.path)),
            "Unsupported or repeated migration target"
        );
        inventory::target(&plan.root, &change.path)?;
        if change.after.is_none() {
            reject_executable_retirement(&plan.root, &change.path)?;
        }
        ensure!(
            change.after.is_some()
                || (change.path.starts_with(".opdev/")
                    && !matches!(
                        change.path.as_str(),
                        ".opdev/project.yaml" | ".opdev/guidance.md"
                    )),
            "Protected file retirement refused"
        );
    }
    for directory in &plan.retire_directories {
        ensure!(
            directory.starts_with(".opdev/")
                && plan.inputs.contains_key(&format!("@directory:{directory}")),
            "Directory retirement must select an existing inventoried .opdev directory"
        );
        inventory::target(&plan.root, &format!("{directory}/.migration-probe"))?;
    }
    Ok(())
}

fn candidate_file(plan: &Plan, path: &str) -> Result<Option<String>> {
    if let Some(change) = plan.changes.iter().find(|c| c.path == path) {
        return Ok(change.after.clone());
    }
    inventory::text(&plan.root.join(path))
}

fn reject_executable_retirement(root: &Path, path: &str) -> Result<()> {
    let output = crate::practical::git(
        root,
        &["--literal-pathspecs", "ls-files", "--stage", "--", path],
    )?;
    ensure!(
        !output.lines().any(|line| line.starts_with("100755 ")),
        "Executable retirement needs mode-preserving recovery; this text migration will not remove it"
    );
    #[cfg(unix)]
    if root.join(path).exists() {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            fs::symlink_metadata(root.join(path))?.permissions().mode() & 0o111 == 0,
            "Executable retirement needs mode-preserving recovery"
        );
    }
    Ok(())
}

fn validate_candidate(plan: &mut Plan, contract: &Contract) -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let mut paths: BTreeSet<String> = plan
        .inputs
        .keys()
        .filter(|p| !p.starts_with('@'))
        .cloned()
        .collect();
    paths.extend(plan.changes.iter().map(|c| c.path.clone()));
    for path in paths {
        if let Some(content) = candidate_file(plan, &path)? {
            let target = temporary.path().join(&path);
            fs::create_dir_all(target.parent().context("Target parent")?)?;
            fs::write(target, content)?;
        }
    }
    // Keep undeclared empty directories visible to strict inspection.
    for path in plan
        .inputs
        .keys()
        .filter_map(|k| k.strip_prefix("@directory:"))
    {
        if !plan
            .retire_directories
            .iter()
            .any(|d| path == d || path.starts_with(&format!("{d}/")))
        {
            fs::create_dir_all(temporary.path().join(path))?;
        }
    }
    for path in &contract.requirements {
        let doc: Requirements = serde_json::from_str(&read_regular(temporary.path(), path)?)?;
        for requirement in doc.requirements {
            ensure!(
                requirement
                    .review
                    .as_ref()
                    .is_none_or(|r| r.outcome != opdev_project::practical::ReviewOutcome::Passed),
                "Migration cannot transfer mapping approval; review the new mapping after apply"
            );
            for link in requirement.verification {
                for input in std::iter::once(link.path).chain(link.inputs) {
                    let contents = read_regular(&plan.root, &input)?;
                    plan.inputs
                        .insert(input.clone(), Some(sha(contents.as_bytes())));
                    let target = temporary.path().join(&input);
                    ensure!(
                        !input.starts_with(".opdev/"),
                        "Assertions must be product verification inputs, not policy definitions"
                    );
                    fs::create_dir_all(target.parent().context("Input parent")?)?;
                    fs::write(target, contents)?;
                }
            }
        }
    }
    contract.inspect_namespace(temporary.path(), true)?;
    let requirements = contract.inspect_requirements(temporary.path())?;
    let policies = contract.inspect_policies(temporary.path(), &requirements)?;
    preserve_requirements(plan, &requirements)?;
    preserve_obligations(plan, &policies)?;
    Ok(())
}

fn preserve_requirements(
    plan: &Plan,
    requirements: &[opdev_project::practical::Requirement],
) -> Result<()> {
    let mut original = BTreeMap::new();
    for path in plan.inputs.keys().filter(|p| {
        p.starts_with(".opdev/requirements/")
            && Path::new(p).extension().is_some_and(|e| e == "json")
    }) {
        if let Some(text) = inventory::text(&plan.root.join(path))? {
            let doc: serde_json::Value = serde_json::from_str(&text)?;
            ensure!(
                matches!(doc["schema"].as_u64(), Some(1 | 2)),
                "Unsupported original requirements schema; preserve the inventory"
            );
            ensure!(
                !doc["plans"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|p| p["review"]["outcome"] == "failed"),
                "Resolve known failed legacy mapping reviews before migration"
            );
            for entry in doc["requirements"]
                .as_array()
                .context("Original requirement inventory is malformed")?
            {
                let id = entry["id"]
                    .as_str()
                    .context("Original requirement ID missing")?;
                ensure!(
                    original.insert(id.to_owned(), entry.clone()).is_none(),
                    "Original requirement identity is ambiguous"
                );
            }
        }
    }
    ensure!(
        plan.retired_requirements
            .keys()
            .all(|id| original.contains_key(id)),
        "Requirement retirement names an unknown guarantee"
    );
    for (id, before) in original {
        let after = requirements.iter().find(|r| r.id == id);
        ensure!(
            after.is_some()
                || plan
                    .retired_requirements
                    .get(&id)
                    .is_some_and(|r| !r.trim().is_empty()),
            "Requirement {id} would disappear without its explicit retirement decision"
        );
        if before["review"]["outcome"] == "failed" {
            ensure!(
                after
                    .and_then(|r| r.review.as_ref())
                    .is_some_and(|r| r.outcome == opdev_project::practical::ReviewOutcome::Failed),
                "Migration cannot erase known failed review {id}; resolve it before migrating"
            );
        }
    }
    Ok(())
}

fn preserve_obligations(plan: &Plan, policies: &[opdev_project::practical::Policy]) -> Result<()> {
    let Some(before) = plan
        .changes
        .iter()
        .find(|c| c.path == ".opdev/project.yaml")
        .and_then(|c| c.before.as_deref())
    else {
        return Ok(());
    };
    let prior: serde_json::Value = serde_saphyr::from_str(before)?;
    let mut obligations = BTreeMap::new();
    if prior["schema"] == 5 {
        let old = Contract::from_yaml(before)?;
        for path in old.policies {
            let policy: opdev_project::practical::Policy =
                serde_json::from_str(&read_regular(&plan.root, &path)?)?;
            obligations.insert(policy.id, policy.origin);
        }
    } else {
        opdev_project::ProjectManifest::from_yaml(before)?;
        for kind in ["standards", "organization_policies", "profiles"] {
            for value in prior["assurance"][kind].as_array().into_iter().flatten() {
                let id = value["id"]
                    .as_str()
                    .or_else(|| value["name"].as_str())
                    .context("Original obligation identity missing")?;
                obligations.insert(format!("{kind}:{id}"), value.clone());
            }
        }
        if let Some(version) = prior["assurance"]["engineering"]["minimumcd"]
            .as_str()
            .filter(|s| *s != "none")
        {
            obligations.insert("minimumcd".into(), serde_json::json!({"minimumcd":version}));
        }
    }
    ensure!(
        plan.retired_obligations
            .keys()
            .all(|id| obligations.contains_key(id)),
        "Retirement names an unknown original obligation"
    );
    for (id, origin) in obligations {
        ensure!(
            policies.iter().any(|p| p.origin == origin)
                || plan
                    .retired_obligations
                    .get(&id)
                    .is_some_and(|r| !r.trim().is_empty()),
            "Original obligation {id} is not preserved. Map it to a selected policy or record the actual original authority's retirement decision; migration cannot waive it"
        );
    }
    Ok(())
}

fn unchanged(plan: &Plan) -> Result<()> {
    let current = inventory::namespace(&plan.root)?;
    for key in current.keys() {
        ensure!(
            plan.inputs.contains_key(key)
                || plan.changes.iter().any(|c| c.path == *key)
                || key.strip_prefix("@directory:").is_some_and(|d| plan
                    .changes
                    .iter()
                    .any(|c| c.after.is_some() && c.path.starts_with(&format!("{d}/")))),
            "New .opdev content appeared; preserve it and review migration again"
        );
    }
    for (path, digest) in &plan.inputs {
        if let Some(directory) = path.strip_prefix("@directory:") {
            ensure!(
                current.get(path) == Some(digest)
                    || plan
                        .retire_directories
                        .iter()
                        .any(|d| directory == d || directory.starts_with(&format!("{d}/"))),
                "Migration directory changed"
            );
        } else {
            inventory::target(&plan.root, path)?;
            let actual = inventory::text(&plan.root.join(path))?;
            if let Some(change) = plan.changes.iter().find(|c| c.path == *path) {
                ensure!(
                    actual == change.before || actual == change.after,
                    "Migration target changed outside reviewed states; preserve the later edit"
                );
            } else {
                ensure!(
                    actual.as_ref().map(|s| sha(s.as_bytes())) == *digest,
                    "Migration input changed; nothing further written"
                );
            }
        }
    }
    Ok(())
}

fn apply_plan(plan: &Plan) -> Result<()> {
    for change in &plan.changes {
        unchanged(plan)?;
        let path = plan.root.join(&change.path);
        let current = inventory::text(&path)?;
        if current == change.after {
            continue;
        }
        if let Some(after) = &change.after {
            fs::create_dir_all(path.parent().context("Target parent")?)?;
            opdev_project::apply_agent_preview(&[AgentFilePreview {
                file: ManagedFile {
                    path,
                    change: if current.is_some() {
                        FileChange::Updated
                    } else {
                        FileChange::Created
                    },
                },
                before: current,
                after: after.clone(),
            }])?;
        } else {
            fs::remove_file(path)?;
        }
    }
    let mut directories = plan.retire_directories.clone();
    directories.sort_by_key(|d| std::cmp::Reverse(d.len()));
    for directory in directories {
        unchanged(plan)?;
        let path = plan.root.join(directory);
        if path.exists() {
            fs::remove_dir(path)
                .context("Retired directory is not empty; preserve recovery and inspect")?;
        }
    }
    unchanged(plan)?;
    Ok(())
}
