//! Release identity alignment without requiring plugin and CLI versions to match.

use std::{fs, path::Path};

use semver::{Version, VersionReq};
use serde_json::Value;

#[test]
fn release_versions_and_runtime_range_are_consistent() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cli = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let ci: Value = serde_saphyr::from_str(&fs::read_to_string(root.join(".gitlab-ci.yml"))?)?;
    assert_eq!(ci["variables"]["OPDEV_VERSION"], cli.to_string());
    let read_json = |path| -> Result<Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&fs::read_to_string(root.join(path))?)?)
    };
    let codex = read_json("plugins/opdev/.codex-plugin/plugin.json")?;
    let claude = read_json("plugins/opdev/.claude-plugin/plugin.json")?;
    let contract = read_json("plugins/opdev/opdev-compatibility.json")?;
    let marketplace = read_json(".claude-plugin/marketplace.json")?;
    assert_eq!(codex["version"], claude["version"]);
    assert_eq!(codex["version"], contract["plugin"]["version"]);
    let entry = marketplace["plugins"]
        .as_array()
        .ok_or("marketplace plugins")?
        .iter()
        .find(|entry| entry["name"] == "opdev")
        .ok_or("opdev marketplace entry")?;
    assert_eq!(codex["version"], entry["version"]);
    let range = VersionReq::parse(contract["requires"]["cli"].as_str().ok_or("CLI range")?)?;
    assert!(
        range.matches(&cli),
        "source CLI is outside the plugin range"
    );
    let lock = fs::read_to_string(root.join("plugins/opdev/runtime.lock"))?;
    let pin = Version::parse(
        lock.lines()
            .find_map(|line| line.strip_prefix("version "))
            .ok_or("runtime version")?,
    )?;
    assert!(
        range.matches(&pin),
        "managed pin is outside the plugin range"
    );
    Ok(())
}
