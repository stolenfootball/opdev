//! Offline upgrade assessment and snapshot-bound guidance application.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use opdev_core::Outcome;
use opdev_project::{
    ADOPTION_PATH, AdoptionRecord, AgentFilePreview, EVIDENCE_PATH, FileChange, MANIFEST_PATH,
    apply_agent_preview, preview_agent_files,
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Args)]
pub(super) struct UpgradeArgs {
    /// Directory inside the initialized Git repository.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Installed or candidate plugin directory to inspect (never executed).
    #[arg(long)]
    plugin_root: Option<PathBuf>,
    /// Explicit read-only preview (also the default).
    #[arg(long, conflicts_with = "apply")]
    dry_run: bool,
    /// Apply only the managed guidance in this exact reviewed plan ID.
    #[arg(long, value_name = "PLAN_ID")]
    apply: Option<String>,
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Human,
    Json,
}

#[derive(Serialize)]
struct Finding {
    component: String,
    outcome: Outcome,
    detail: String,
}

#[derive(Serialize)]
struct Change {
    path: String,
    action: String,
    before: Option<String>,
    after: String,
}

#[derive(Serialize)]
struct UpgradeReport {
    schema: u32,
    root: PathBuf,
    cli_path: PathBuf,
    target_cli_version: &'static str,
    plan_id: String,
    applied: bool,
    project_verification: Outcome,
    findings: Vec<Finding>,
    changes: Vec<Change>,
    /// Digests also bind reviewed read-only state; no on-disk approval file needed.
    inputs: BTreeMap<String, Option<String>>,
    untouched: Vec<&'static str>,
    next_steps: Vec<&'static str>,
}

impl UpgradeReport {
    fn finding(&mut self, component: &str, outcome: Outcome, detail: impl Into<String>) {
        self.findings.push(Finding {
            component: component.into(),
            outcome,
            detail: detail.into(),
        });
    }

    fn read(&mut self, key: &str, path: &Path) -> Result<Option<String>> {
        let bytes = match fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(error).with_context(|| format!("could not inspect {}", path.display()));
            }
        };
        self.inputs.insert(
            key.into(),
            bytes
                .as_ref()
                .map(|bytes| format!("{:x}", Sha256::digest(bytes))),
        );
        bytes
            .map(String::from_utf8)
            .transpose()
            .with_context(|| format!("{} is not UTF-8", path.display()))
    }
}

pub(super) fn run(args: &UpgradeArgs) -> Result<ExitCode> {
    let (mut plan, preview) = assess(args)?;
    let blocked = plan
        .findings
        .iter()
        .any(|finding| matches!(finding.outcome, Outcome::Failed | Outcome::Error));
    if let Some(approved) = &args.apply {
        if approved != &plan.plan_id {
            bail!(
                "upgrade plan is stale or belongs to another target; nothing written. Preview and review again"
            );
        }
        if blocked {
            bail!(
                "upgrade has compatibility or assessment errors; nothing written. Resolve the preview findings first"
            );
        }
        // Re-read the complete inventory immediately before the first write.
        if assess(args)?.0.plan_id != plan.plan_id {
            bail!("upgrade inputs changed during review; nothing written. Preview again");
        }
        apply_agent_preview(&preview).context("guidance apply failed; inspect both files and preview again (writes are atomic per file, not a multi-file transaction)")?;
        if preview_agent_files(&plan.root)?
            .iter()
            .any(|item| item.file.change != FileChange::Unchanged)
        {
            bail!("guidance changed during application; inspect files and preview again");
        }
        plan.applied = true;
        plan.finding(
            "guidance",
            Outcome::Passed,
            "Managed guidance updated and reread. Project checks have NOT run.",
        );
    }
    match args.format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&plan)?),
        Format::Human => print_plan(&plan),
    }
    Ok(if blocked {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn assess(args: &UpgradeArgs) -> Result<(UpgradeReport, Vec<AgentFilePreview>)> {
    let (root, manifest) = crate::load_project(&args.root)
        .context("upgrade assessment cannot read this project; unsupported schemas require migration with a compatible CLI, not rewriting")?;
    let preview = preview_agent_files(&root)?;
    let mut plan = UpgradeReport {
        schema: 1,
        root,
        cli_path: std::env::current_exe()?,
        target_cli_version: env!("CARGO_PKG_VERSION"),
        plan_id: String::new(),
        applied: false,
        project_verification: Outcome::Unverified,
        findings: Vec::new(),
        changes: Vec::new(),
        inputs: BTreeMap::new(),
        untouched: vec![
            "plugin installation and runtime storage",
            "project contract, commands, authorities and assurance profile pins",
            "adoption decisions, evidence and experiment opt-ins",
            "CI configurations and release pins",
            "project content outside managed guidance markers",
        ],
        next_steps: vec![
            "Review this preview; apply only its exact plan ID using the same CLI and --plugin-root, if supplied.",
            "Handle plugin/runtime installation and CI pin changes separately with explicit consent and verified release artifacts.",
            "Resolve reported migrations; do not restart legacy adoption or change selected profiles silently.",
            "Review the project contract, then run canonical checks and opdev check --ci; run adoption check only for assessed projects. Refresh change evidence only after review.",
            "Integrate through CI and verify trunk; runtime or guidance updates alone do not establish project upgrade completion.",
        ],
    };
    for relative in [MANIFEST_PATH, ADOPTION_PATH, EVIDENCE_PATH] {
        plan.read(relative, &plan.root.join(relative))?;
    }
    for item in &preview {
        let relative = item
            .file
            .path
            .strip_prefix(&plan.root)?
            .to_string_lossy()
            .into_owned();
        plan.read(&relative, &item.file.path)?;
        plan.changes.push(Change {
            path: relative,
            action: format!("{:?}", item.file.change).to_lowercase(),
            before: item.before.clone(),
            after: item.after.clone(),
        });
    }
    plan.finding(
        "project_schema",
        Outcome::Passed,
        format!(
            "Schema {} is supported; no schema migration is applied.",
            manifest.schema
        ),
    );
    match AdoptionRecord::load(&plan.root) {
        Ok(None) => plan.finding("adoption", Outcome::Unverified, "Legacy-unassessed project; no adoption record created. Assessment remains a separate explicit choice."),
        Ok(Some(record)) => match record.blockers(&manifest) {
            Ok(blockers) if blockers.is_empty() => plan.finding("adoption", Outcome::Unverified, "Decisions are structurally complete; adoption check and fresh evidence still required."),
            Ok(blockers) => plan.finding("adoption", Outcome::MigrationRequired, blockers.join("; ")),
            Err(error) => plan.finding("adoption", Outcome::Error, error.to_string()),
        },
        Err(error) => plan.finding("adoption", Outcome::Error, format!("{error}; no automatic adoption migration exists")),
    }
    for profile in &manifest.assurance.profiles {
        let detail = format!("{} {}", profile.name, profile.version);
        match opdev_core::resolve_profile(&profile.name, &profile.version, profile.level.as_deref())
        {
            Ok(_) => plan.finding(
                "assurance_profile",
                Outcome::Passed,
                format!("{detail}: available, pin unchanged (not a compliance verdict)"),
            ),
            Err(error) => plan.finding(
                "assurance_profile",
                Outcome::Error,
                format!("{detail}: {error}"),
            ),
        }
    }
    inspect_plugin(&mut plan, args.plugin_root.as_deref())?;
    inspect_ci(&mut plan)?;
    plan.finding("release_discovery", Outcome::Unverified, "Target is the running CLI, not a claim about the newest published release. No network or project commands executed.");
    // Includes target guidance, root, executable path, versions, inspection results
    // and exact input digests. Source builds with equal SemVer still differ.
    plan.plan_id = format!("{:x}", Sha256::digest(serde_json::to_vec(&plan)?));
    Ok((plan, preview))
}

fn inspect_plugin(plan: &mut UpgradeReport, directory: Option<&Path>) -> Result<()> {
    let Some(directory) = directory else {
        plan.finding("plugin", Outcome::Unverified, "No --plugin-root supplied; plugin version, runtime pin and compatibility not inspected. Standalone CLI use is supported.");
        return Ok(());
    };
    let directory = directory
        .canonicalize()
        .context("could not resolve plugin directory")?;
    let source = plan
        .read(
            &format!("plugin:{}", directory.display()),
            &directory.join("opdev-compatibility.json"),
        )?
        .context("plugin compatibility contract is missing")?;
    let contract: crate::PluginCompatibility =
        serde_json::from_str(&source).context("invalid plugin compatibility contract")?;
    let cli = semver::Version::parse(plan.target_cli_version)?;
    let compatible = contract.schema == 1
        && contract.plugin.name == "opdev"
        && contract.requires.cli.matches(&cli);
    plan.finding("plugin", if compatible { Outcome::Passed } else { Outcome::Failed }, format!("Plugin {} requires CLI {}; selected CLI {}. Package data inspected, installation/authenticity not established.", contract.plugin.version, contract.requires.cli, cli));
    let lock = plan.read("plugin:runtime.lock", &directory.join("runtime.lock"))?;
    let versions: Vec<_> = lock
        .as_deref()
        .unwrap_or_default()
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|words| words.first() == Some(&"version"))
        .collect();
    if let [words] = versions.as_slice()
        && let [_, pin] = words.as_slice()
        && let Ok(version) = semver::Version::parse(pin)
    {
        plan.finding("runtime_pin", if contract.requires.cli.matches(&version) { Outcome::Unverified } else { Outcome::Failed }, format!("Packaged pin {pin}; selected CLI {cli}. A different compatible pin is allowed. Use read-only runtime lookup to verify installation/selection; this command never executes package scripts."));
        return Ok(());
    }
    plan.finding(
        "runtime_pin",
        Outcome::Unverified,
        "No unambiguous runtime version pin; inspect package/setup before installing.",
    );
    Ok(())
}

fn inspect_ci(plan: &mut UpgradeReport) -> Result<()> {
    let mut files = vec![PathBuf::from(".gitlab-ci.yml")];
    let workflows = plan.root.join(".github/workflows");
    match fs::read_dir(&workflows) {
        Ok(entries) => {
            for entry in entries {
                let path = entry?.path();
                if matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("yml" | "yaml")
                ) {
                    files.push(path.strip_prefix(&plan.root)?.to_path_buf());
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("could not inspect GitHub workflow directory"),
    }
    files.sort();
    for relative in files {
        let key = relative.to_string_lossy().replace('\\', "/");
        if let Some(source) = plan.read(&key, &plan.root.join(&relative))? {
            match serde_saphyr::from_str::<Value>(&source) {
                Ok(value) => {
                    let mut pins = Vec::new();
                    collect_pins(&value, &mut pins);
                    plan.finding("ci_pins", Outcome::Unverified, format!("{key}: OPDEV_VERSION declarations {pins:?}. Preserved. Declarations do not prove the effective runtime; review scripts, includes, variables and download/signature configuration together."));
                }
                Err(error) => plan.finding(
                    "ci_pins",
                    Outcome::Error,
                    format!("{key}: {error}; left untouched"),
                ),
            }
        }
    }
    plan.finding("ci_qualification", Outcome::Unverified, "CI is never rewritten by upgrade. Included/remote/custom configuration is not resolved; matching version declarations alone do not qualify CI.");
    Ok(())
}

fn collect_pins(value: &Value, pins: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "OPDEV_VERSION" {
                    pins.push(child.clone());
                } else {
                    collect_pins(child, pins);
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_pins(child, pins);
            }
        }
        _ => {}
    }
}

fn print_plan(plan: &UpgradeReport) {
    println!(
        "OpDev upgrade {} for {}",
        if plan.applied {
            "applied (guidance only)"
        } else {
            "preview (no writes)"
        },
        plan.root.display()
    );
    println!(
        "Target CLI: {} at {}",
        plan.target_cli_version,
        plan.cli_path.display()
    );
    for finding in &plan.findings {
        println!(
            "{}: {:?} — {}",
            finding.component, finding.outcome, finding.detail
        );
    }
    for change in &plan.changes {
        println!("{} {}", change.action, change.path);
        if change.action != "unchanged" {
            println!(
                "--- {} (before)\n+++ {} (after)\n@@ complete file replacement @@",
                change.path, change.path
            );
            for line in change.before.as_deref().unwrap_or_default().lines() {
                println!("-{line}");
            }
            for line in change.after.lines() {
                println!("+{line}");
            }
        }
    }
    println!("Review token: {}", plan.plan_id);
    println!("Untouched: {}", plan.untouched.join("; "));
    for step in &plan.next_steps {
        println!("- {step}");
    }
    println!("Project verification: unverified (no project checks executed).");
}
