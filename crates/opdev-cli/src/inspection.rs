//! Shared read-only observations; callers own reporting and input binding.
use anyhow::{Context, Result};
use opdev_core::Outcome;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) trait Inventory {
    fn root(&self) -> &Path;
    fn version(&self) -> &str;
    fn read(&mut self, key: &str, path: &Path) -> Result<Option<String>>;
    fn finding(&mut self, component: &str, outcome: Outcome, detail: impl Into<String>);
    fn source_finding(
        &mut self,
        component: &str,
        _subject: &str,
        outcome: Outcome,
        detail: impl Into<String>,
    ) {
        self.finding(component, outcome, detail);
    }
}

pub(super) fn inspect_plugin(plan: &mut impl Inventory, directory: Option<&Path>) -> Result<()> {
    plan.finding("session_guidance", Outcome::Unverified,
        "Session-loaded instructions are unknown: disk/package inspection cannot prove host context. After a package change, use the host's reload or a fresh session and inspect host evidence; no restart is needed merely to inspect.");
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
    let skill = plan.read(
        "plugin:skills/opdev/SKILL.md",
        &directory.join("skills/opdev/SKILL.md"),
    )?;
    plan.finding("package_identity", Outcome::Unverified, format!(
        "Selected package {} at {}; skill SHA-256 {}. This is inspected disk content, not proof of installation or session activation.",
        contract.plugin.version, directory.display(), skill.as_ref().map_or_else(|| "unavailable".into(), |s| format!("{:x}", Sha256::digest(s.as_bytes())))
    ));
    let cli = semver::Version::parse(plan.version())?;
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
        plan.finding("runtime_alignment", if version == cli { Outcome::Passed } else { Outcome::Unverified },
            if version == cli { "Selected CLI version equals the package pin; equal versions do not prove identical bytes, capabilities or active guidance." }
            else { "Selected CLI version differs from the package pin. Compatibility is reported separately; a compatible difference need not be repaired. For a specific upgrade, select the intended executable and preview again." });
        return Ok(());
    }
    plan.finding(
        "runtime_pin",
        Outcome::Unverified,
        "No unambiguous runtime version pin; inspect package/setup before installing.",
    );
    Ok(())
}

pub(super) fn inspect_ci(plan: &mut impl Inventory) -> Result<()> {
    let mut files = vec![PathBuf::from(".gitlab-ci.yml")];
    let workflows = plan.root().join(".github/workflows");
    match fs::read_dir(&workflows) {
        Ok(entries) => {
            for entry in entries {
                let path = entry?.path();
                if matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("yml" | "yaml")
                ) {
                    files.push(path.strip_prefix(plan.root())?.to_path_buf());
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("could not inspect GitHub workflow directory"),
    }
    files.sort();
    for relative in files {
        let key = relative.to_string_lossy().replace('\\', "/");
        if let Some(source) = plan.read(&key, &plan.root().join(&relative))? {
            let parsed = if key == ".gitlab-ci.yml" {
                opdev_ci::gitlab::resolve(&source, |relative| {
                    let bounded = opdev_ci::gitlab::read_local(plan.root(), relative)?;
                    let recorded =
                        plan.read(relative, &plan.root().join(relative))
                            .map_err(|_| opdev_ci::gitlab::ConfigurationProblem {
                                outcome: Outcome::Error,
                                diagnostic: format!(
                                    "Could not record included CI file `{relative}`"
                                ),
                            })?;
                    if recorded.as_deref() != Some(bounded.as_str()) {
                        return Err(opdev_ci::gitlab::ConfigurationProblem {
                            outcome: Outcome::Unverified,
                            diagnostic: format!(
                                "Included CI file `{relative}` changed during inspection"
                            ),
                        });
                    }
                    Ok(bounded)
                })
            } else {
                serde_saphyr::from_str::<Value>(&source).map_err(|_| {
                    opdev_ci::gitlab::ConfigurationProblem {
                        outcome: Outcome::Error,
                        diagnostic: "Invalid GitHub workflow YAML".into(),
                    }
                })
            };
            match parsed {
                Ok(value) => {
                    let mut pins = Vec::new();
                    collect_pins(&value, &mut pins);
                    plan.source_finding("ci_pins", &key, Outcome::Unverified, format!("{key}: OPDEV_VERSION declarations {pins:?}. Preserved. Declarations do not prove the effective runtime; review scripts, includes, variables and download/signature configuration together."));
                }
                Err(error) => plan.source_finding(
                    "ci_pins",
                    &key,
                    error.outcome,
                    format!("{key}: {}; left untouched", error.diagnostic),
                ),
            }
        }
    }
    plan.finding("ci_qualification", Outcome::Unverified, "CI is never rewritten by upgrade. Explicit local GitLab includes are interpreted; unsupported dynamic/external configuration remains unresolved. Matching version declarations alone do not qualify CI.");
    Ok(())
}

pub(super) fn inspect_guidance(plan: &mut impl Inventory) -> Result<()> {
    for relative in ["AGENTS.md", "CLAUDE.md"] {
        plan.read(relative, &plan.root().join(relative))?;
    }
    for item in opdev_project::preview_agent_files(plan.root())? {
        let relative = item
            .file
            .path
            .strip_prefix(plan.root())?
            .to_string_lossy()
            .into_owned();
        let matches = item.file.change == opdev_project::FileChange::Unchanged;
        plan.source_finding("project_guidance", &relative, if matches { Outcome::Passed } else { Outcome::MigrationRequired },
            format!("{relative}: {} the running CLI's embedded guidance. This target is not inferred from package SemVer or session state. {}",
                if matches { "matches" } else { "differs from" },
                if matches { "No managed edit indicated." } else { "Review upgrade --dry-run with the intended executable before applying; custom content is preserved." }));
    }
    Ok(())
}

fn collect_pins(value: &Value, pins: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "OPDEV_VERSION" {
                    pins.push(
                        match child
                            .as_str()
                            .and_then(|pin| semver::Version::parse(pin).ok())
                        {
                            Some(version) => Value::String(version.to_string()),
                            None => Value::String("[dynamic/non-version declaration]".into()),
                        },
                    );
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
