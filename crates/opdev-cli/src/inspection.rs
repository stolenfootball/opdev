//! Shared read-only observations; callers own reporting and input binding.
use anyhow::{Context, Result};
use opdev_core::Outcome;
use serde_json::Value;
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
            match serde_saphyr::from_str::<Value>(&source) {
                Ok(value) => {
                    let mut pins = Vec::new();
                    collect_pins(&value, &mut pins);
                    plan.source_finding("ci_pins", &key, Outcome::Unverified, format!("{key}: OPDEV_VERSION declarations {pins:?}. Preserved. Declarations do not prove the effective runtime; review scripts, includes, variables and download/signature configuration together."));
                }
                Err(error) => plan.source_finding(
                    "ci_pins",
                    &key,
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
