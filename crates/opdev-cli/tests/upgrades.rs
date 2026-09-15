//! Consumer upgrade flow, preservation and failure-path regression tests.

use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Output},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn cli(root: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
}

fn fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(repo.path())
            .status()?
            .success()
    );
    let output = cli(repo.path(), &["init"])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(
        repo.path().join("AGENTS.md"),
        "before\r\n<!-- opdev:start -->\r\nold guidance\r\n<!-- opdev:end -->\r\nafter\r\n",
    )?;
    fs::write(
        repo.path().join("CLAUDE.md"),
        "project-owned\r\n@AGENTS.md\r\n",
    )?;
    fs::write(
        repo.path().join(".opdev/experiment.yaml"),
        "compact: explicitly-disabled\n",
    )?;
    Ok(repo)
}

fn preview(root: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let output = cli(root, &["upgrade", "--format", "json"])?;
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, std::io::Error> {
    fn visit(
        root: &Path,
        directory: &Path,
        files: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_name() == ".git" {
                continue;
            }
            if entry.file_type()?.is_dir() {
                visit(root, &entry.path(), files)?;
            } else {
                files.insert(
                    entry
                        .path()
                        .strip_prefix(root)
                        .map_err(std::io::Error::other)?
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(entry.path())?,
                );
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(files)
}

#[test]
fn preview_apply_verify_preserves_project_state_and_is_idempotent() -> TestResult {
    let repo = fixture()?;
    let before = snapshot(repo.path())?;
    let plan = preview(repo.path())?;
    assert_eq!(snapshot(repo.path())?, before);
    assert_eq!(plan["project_verification"], "unverified");
    let token = plan["plan_id"].as_str().ok_or("token")?;
    assert_eq!(preview(repo.path())?["plan_id"], token);
    let applied = cli(
        repo.path(),
        &["upgrade", "--apply", token, "--format", "json"],
    )?;
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let result: Value = serde_json::from_slice(&applied.stdout)?;
    assert_eq!(result["applied"], true);
    assert_eq!(result["project_verification"], "unverified");
    let after = snapshot(repo.path())?;
    for (path, bytes) in before
        .iter()
        .filter(|(path, _)| path.as_str() != "AGENTS.md")
    {
        assert_eq!(after.get(path), Some(bytes), "{path}");
    }
    let agents = fs::read_to_string(repo.path().join("AGENTS.md"))?;
    assert!(agents.starts_with("before\r\n"));
    assert!(agents.ends_with("\r\nafter\r\n"));
    let current = preview(repo.path())?;
    assert!(
        current["changes"]
            .as_array()
            .ok_or("changes")?
            .iter()
            .all(|change| change["action"] == "unchanged")
    );
    assert!(
        cli(
            repo.path(),
            &[
                "upgrade",
                "--apply",
                current["plan_id"].as_str().ok_or("token")?
            ]
        )?
        .status
        .success()
    );
    assert_eq!(snapshot(repo.path())?, after);
    Ok(())
}

#[test]
fn reviewed_input_changes_reject_apply_without_writes() -> TestResult {
    for relative in [
        "AGENTS.md",
        "CLAUDE.md",
        ".opdev/project.yaml",
        ".opdev/adoption.yaml",
        ".opdev/evidence.yaml",
        ".gitlab-ci.yml",
        ".github/workflows/new.yaml",
    ] {
        let repo = fixture()?;
        let plan = preview(repo.path())?;
        let path = repo.path().join(relative);
        fs::create_dir_all(path.parent().ok_or("parent")?)?;
        let original = fs::read_to_string(&path).unwrap_or_default();
        fs::write(path, format!("{original}\n# concurrent edit\n"))?;
        let before = snapshot(repo.path())?;
        let result = cli(
            repo.path(),
            &[
                "upgrade",
                "--apply",
                plan["plan_id"].as_str().ok_or("token")?,
            ],
        )?;
        assert!(!result.status.success(), "{relative}");
        assert_eq!(snapshot(repo.path())?, before, "{relative}");
    }
    Ok(())
}

#[test]
fn malformed_second_file_does_not_update_first_and_future_schema_is_preserved() -> TestResult {
    let repo = fixture()?;
    fs::write(
        repo.path().join("CLAUDE.md"),
        "@AGENTS.md\n<!-- opdev:end -->\n",
    )?;
    let before = snapshot(repo.path())?;
    assert!(!cli(repo.path(), &["upgrade"])?.status.success());
    assert_eq!(snapshot(repo.path())?, before);
    fs::write(repo.path().join("CLAUDE.md"), "@AGENTS.md\n")?;
    let path = repo.path().join(".opdev/project.yaml");
    fs::write(
        &path,
        fs::read_to_string(&path)?.replace("schema: 1", "schema: 999"),
    )?;
    let before = snapshot(repo.path())?;
    assert!(!cli(repo.path(), &["upgrade"])?.status.success());
    assert_eq!(snapshot(repo.path())?, before);
    Ok(())
}

#[test]
fn detects_ci_pins_but_never_claims_qualification_or_rewrites_ci() -> TestResult {
    let repo = fixture()?;
    fs::write(
        repo.path().join(".gitlab-ci.yml"),
        "include: remote.yml\nvariables:\n  OPDEV_VERSION: '0.1.1'\ncustom:\n  script: echo untouched\n",
    )?;
    fs::create_dir_all(repo.path().join(".github/workflows"))?;
    fs::write(
        repo.path().join(".github/workflows/custom.yaml"),
        "env:\n  OPDEV_VERSION: '0.1.2'\njobs: {}\n",
    )?;
    let plan = preview(repo.path())?;
    let findings = plan["findings"].as_array().ok_or("findings")?;
    let pins: Vec<_> = findings
        .iter()
        .filter(|f| f["component"] == "ci_pins")
        .collect();
    assert_eq!(pins.len(), 2);
    assert!(pins.iter().all(|f| f["outcome"] == "unverified"));
    assert!(
        pins.iter()
            .any(|f| f["detail"].as_str().is_some_and(|s| s.contains("0.1.1")))
    );
    let before = fs::read(repo.path().join(".gitlab-ci.yml"))?;
    assert!(
        cli(
            repo.path(),
            &[
                "upgrade",
                "--apply",
                plan["plan_id"].as_str().ok_or("token")?
            ]
        )?
        .status
        .success()
    );
    assert_eq!(fs::read(repo.path().join(".gitlab-ci.yml"))?, before);
    Ok(())
}

#[test]
fn incompatible_plugin_blocks_apply_and_package_change_invalidates_review() -> TestResult {
    let repo = fixture()?;
    let plugin = tempfile::tempdir()?;
    let path = plugin.path().to_str().ok_or("path")?;
    let contract = plugin.path().join("opdev-compatibility.json");
    fs::write(plugin.path().join("runtime.lock"), "version 0.1.1\n")?;
    for range in [">=99.0.0", ">=0.1.1, <0.2.0"] {
        fs::write(
            &contract,
            serde_json::to_vec(
                &serde_json::json!({"schema":1,"plugin":{"name":"opdev","version":"0.1.2"},"requires":{"cli":range}}),
            )?,
        )?;
        let result = cli(
            repo.path(),
            &["upgrade", "--plugin-root", path, "--format", "json"],
        )?;
        let plan: Value = serde_json::from_slice(&result.stdout)?;
        let token = plan["plan_id"].as_str().ok_or("token")?;
        if range.starts_with(">=99") {
            assert_eq!(result.status.code(), Some(1));
        } else {
            assert!(result.status.success());
            fs::write(plugin.path().join("runtime.lock"), "version 0.1.2\n")?;
        }
        let before = snapshot(repo.path())?;
        assert!(
            !cli(
                repo.path(),
                &["upgrade", "--plugin-root", path, "--apply", token]
            )?
            .status
            .success()
        );
        assert_eq!(snapshot(repo.path())?, before);
    }
    Ok(())
}

#[test]
fn legacy_upgrade_does_not_start_adoption_and_cross_project_tokens_fail() -> TestResult {
    let repo = fixture()?;
    fs::remove_file(repo.path().join(".opdev/adoption.yaml"))?;
    let plan = preview(repo.path())?;
    assert!(
        cli(
            repo.path(),
            &[
                "upgrade",
                "--apply",
                plan["plan_id"].as_str().ok_or("token")?
            ]
        )?
        .status
        .success()
    );
    assert!(!repo.path().join(".opdev/adoption.yaml").exists());
    let other = fixture()?;
    let before = snapshot(other.path())?;
    assert!(
        !cli(
            other.path(),
            &[
                "upgrade",
                "--apply",
                plan["plan_id"].as_str().ok_or("token")?
            ]
        )?
        .status
        .success()
    );
    assert_eq!(snapshot(other.path())?, before);
    Ok(())
}

#[test]
fn invalid_adoption_record_is_reported_and_preserved() -> TestResult {
    let repo = fixture()?;
    let path = repo.path().join(".opdev/adoption.yaml");
    fs::write(
        &path,
        fs::read_to_string(&path)?.replace("catalog_version: 1", "catalog_version: 999"),
    )?;
    let before = snapshot(repo.path())?;
    let result = cli(repo.path(), &["upgrade", "--format", "json"])?;
    assert_eq!(result.status.code(), Some(1));
    let plan: Value = serde_json::from_slice(&result.stdout)?;
    assert!(
        !cli(
            repo.path(),
            &[
                "upgrade",
                "--apply",
                plan["plan_id"].as_str().ok_or("token")?
            ]
        )?
        .status
        .success()
    );
    assert_eq!(snapshot(repo.path())?, before);
    Ok(())
}
