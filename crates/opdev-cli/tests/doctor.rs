//! Readiness is scoped; diagnostic success never qualifies project behavior.
use opdev_project::{AuthorityKind, AuthorityRef, CommandSpec, MANIFEST_PATH, ProjectManifest};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Output},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture() -> Result<(tempfile::TempDir, ProjectManifest), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let mut manifest = ProjectManifest::from_yaml(include_str!("../../../.opdev/project.yaml"))?;
    manifest.commands.clear();
    manifest.testing.suites.clear();
    manifest.authorities.values_mut().for_each(|authority| {
        if authority.kind == AuthorityKind::Path {
            authority.location = ".".into();
        }
    });
    manifest.commands.insert(
        "probe".into(),
        CommandSpec {
            argv: vec![env!("CARGO_BIN_EXE_opdev").into(), "--version".into()],
            working_directory: None,
            timeout_seconds: None,
        },
    );
    save(root.path(), &manifest)?;
    Ok((root, manifest))
}

fn save(root: &Path, manifest: &ProjectManifest) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(root.join(".opdev"))?;
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    Ok(())
}

fn cli(root: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .arg("doctor")
        .arg("--root")
        .arg(root)
        .args(args)
        .env("OPDEV_GITHUB_TOKEN", "NEVER_ECHO_THIS_TOKEN")
        .env("OPDEV_GITLAB_TOKEN", "NEVER_ECHO_THIS_TOKEN")
        .env("NO_COLOR", "1")
        .output()
}

fn json(output: &Output, exit: i32) -> Result<Value, Box<dyn std::error::Error>> {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["exit_code"], exit);
    assert_eq!(report["project_verification"], "unverified");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("NEVER_ECHO_THIS_TOKEN"));
    let schema: Value = serde_json::from_str(include_str!("../../../schema/doctor.schema.json"))?;
    let validator = jsonschema::validator_for(&schema)?;
    let errors: Vec<_> = validator
        .iter_errors(&report)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    Ok(report)
}

fn finding<'a>(
    report: &'a Value,
    id: &str,
    subject: Option<&str>,
) -> Result<&'a Value, Box<dyn std::error::Error>> {
    report["findings"]
        .as_array()
        .ok_or("findings")?
        .iter()
        .find(|f| f["id"] == id && subject.is_none_or(|s| f["subject"] == s))
        .ok_or_else(|| format!("missing {id}").into())
}

fn snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, std::io::Error> {
    fn visit(
        root: &Path,
        dir: &Path,
        result: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                visit(root, &entry.path(), result)?;
            } else {
                result.insert(
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
    let mut result = BTreeMap::new();
    visit(root, root, &mut result)?;
    Ok(result)
}

#[test]
fn runtime_only_is_not_adoption_and_same_version_is_not_build_identity() -> TestResult {
    let root = tempfile::tempdir()?;
    let before = snapshot(root.path())?;
    let report = json(&cli(root.path(), &["--format", "json"])?, 0)?;
    assert_eq!(
        finding(&report, "project.contract", None)?["outcome"],
        "not_applicable"
    );
    assert_eq!(report["runtime"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        report["runtime"]["sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(env!("CARGO_BIN_EXE_opdev"))?)
        )
    );
    assert_eq!(report["runtime"]["path"], env!("CARGO_BIN_EXE_opdev"));
    assert!(
        report["runtime"]["capabilities"]
            .as_array()
            .ok_or("capabilities")?
            .contains(&Value::from("evidence.acceptance-digest"))
    );
    assert_eq!(
        report["runtime"]["project_schemas"],
        serde_json::json!([1, 2])
    );
    assert_eq!(
        report["runtime"]["evidence_schemas"],
        serde_json::json!([1, 2])
    );
    assert_eq!(snapshot(root.path())?, before);
    Ok(())
}

#[test]
fn valid_standalone_preserves_gaps_without_claiming_qualification() -> TestResult {
    let (root, _) = fixture()?;
    let before = snapshot(root.path())?;
    let report = json(&cli(root.path(), &["--format", "json"])?, 0)?;
    assert_eq!(
        finding(&report, "command.executable", None)?["outcome"],
        "passed"
    );
    assert_eq!(
        finding(&report, "gap.coverage", None)?["outcome"],
        "migration_required"
    );
    assert_eq!(finding(&report, "gap.coverage", None)?["required"], false);
    assert_eq!(
        finding(&report, "inventory.plugin", None)?["outcome"],
        "unverified"
    );
    assert_eq!(
        finding(&report, "command.execution", None)?["outcome"],
        "unverified"
    );
    assert_eq!(
        finding(&report, "remote.not_requested", None)?["required"],
        false
    );
    assert_eq!(snapshot(root.path())?, before);
    Ok(())
}

#[test]
fn missing_executable_directory_and_authority_have_specific_next_actions() -> TestResult {
    let (root, mut manifest) = fixture()?;
    manifest.commands.get_mut("probe").ok_or("command")?.argv[0] = root
        .path()
        .join("missing-tool.exe")
        .to_string_lossy()
        .into_owned();
    manifest.commands.insert(
        "bad-cwd".into(),
        CommandSpec {
            argv: vec!["also-missing".into()],
            working_directory: Some("missing-dir".into()),
            timeout_seconds: None,
        },
    );
    manifest.authorities.insert(
        "custom".into(),
        AuthorityRef {
            kind: AuthorityKind::Path,
            location: "handbook/decisions.md".into(),
        },
    );
    save(root.path(), &manifest)?;
    let report = json(&cli(root.path(), &["--format", "json"])?, 1)?;
    assert_eq!(
        finding(&report, "command.executable", Some("commands.probe"))?["outcome"],
        "failed"
    );
    assert_eq!(
        finding(&report, "command.directory", Some("commands.bad-cwd"))?["outcome"],
        "failed"
    );
    assert_eq!(
        finding(&report, "command.executable", Some("commands.bad-cwd"))?["outcome"],
        "unverified"
    );
    assert_eq!(
        finding(&report, "authority.path", Some("authorities.custom"))?["outcome"],
        "failed"
    );
    for f in report["findings"].as_array().ok_or("findings")? {
        assert!(!f["next_step"].as_str().ok_or("next_step")?.is_empty());
    }
    let human = cli(root.path(), &[])?;
    assert_eq!(human.status.code(), Some(1));
    let text = String::from_utf8(human.stdout)?;
    assert!(text.contains("command.executable: failed"));
    assert!(text.contains("Project verification: unverified"));
    assert!(!text.contains('\u{1b}'));
    Ok(())
}

#[test]
fn invalid_and_unsupported_contracts_keep_independent_observations() -> TestResult {
    let (root, _) = fixture()?;
    for source in [
        "not: [valid yaml SECRET_INPUT",
        "schema: 987\nprivate: SECRET_INPUT\n",
    ] {
        fs::write(root.path().join(MANIFEST_PATH), source)?;
        let output = cli(root.path(), &["--format", "json"])?;
        let report = json(&output, 2)?;
        assert_eq!(
            finding(&report, "runtime.identity", None)?["outcome"],
            "passed"
        );
        assert_eq!(
            finding(&report, "project.contract", None)?["outcome"],
            "error"
        );
        assert_eq!(
            finding(&report, "project.dependents", None)?["outcome"],
            "unverified"
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("SECRET_INPUT"));
        assert!(
            !report["findings"]
                .as_array()
                .ok_or("findings")?
                .iter()
                .any(|f| f["id"] == "command.executable")
        );
        assert_eq!(fs::read_to_string(root.path().join(MANIFEST_PATH))?, source);
    }
    Ok(())
}

#[test]
fn compatible_different_pin_and_incompatible_plugin_are_distinct() -> TestResult {
    let root = tempfile::tempdir()?;
    let plugin = tempfile::tempdir()?;
    fs::write(plugin.path().join("runtime.lock"), "version 0.2.1\n")?;
    let contract = |requirement: &str| {
        serde_json::json!({"schema":1,"plugin":{"name":"opdev","version":"0.2.5"},"requires":{"cli":requirement}}).to_string()
    };
    let plugin_arg = plugin.path().to_str().ok_or("plugin path")?;
    fs::write(
        plugin.path().join("opdev-compatibility.json"),
        contract(">=0.2.0, <0.3.0"),
    )?;
    let before = snapshot(plugin.path())?;
    let report = json(
        &cli(
            root.path(),
            &["--plugin-root", plugin_arg, "--format", "json"],
        )?,
        0,
    )?;
    assert_eq!(
        finding(&report, "inventory.plugin", None)?["outcome"],
        "passed"
    );
    assert_eq!(
        finding(&report, "inventory.runtime_pin", None)?["outcome"],
        "unverified"
    );
    assert_eq!(snapshot(plugin.path())?, before);
    fs::write(plugin.path().join("runtime.lock"), "version 9.0.0\n")?;
    let bad_pin = json(
        &cli(
            root.path(),
            &["--plugin-root", plugin_arg, "--format", "json"],
        )?,
        1,
    )?;
    assert_eq!(
        finding(&bad_pin, "inventory.runtime_pin", None)?["outcome"],
        "failed"
    );
    fs::write(
        plugin.path().join("opdev-compatibility.json"),
        contract(">=9.0.0"),
    )?;
    let incompatible = json(
        &cli(
            root.path(),
            &["--plugin-root", plugin_arg, "--format", "json"],
        )?,
        1,
    )?;
    assert_eq!(
        finding(&incompatible, "inventory.plugin", None)?["outcome"],
        "failed"
    );
    Ok(())
}

#[test]
fn ci_pins_are_observations_and_malformed_inventory_is_an_error() -> TestResult {
    let (root, _) = fixture()?;
    fs::write(
        root.path().join(".gitlab-ci.yml"),
        "variables:\n  OPDEV_VERSION: '0.2.2'\ninclude: 'elsewhere.yml'\n",
    )?;
    fs::create_dir_all(root.path().join(".github/workflows"))?;
    fs::write(
        root.path().join(".github/workflows/build.yml"),
        "env:\n  OPDEV_VERSION: '${{ vars.OPDEV_VERSION }}'\njobs:\n  build:\n    env:\n      OPDEV_VERSION: NEVER_ECHO_THIS_TOKEN\n",
    )?;
    let before = snapshot(root.path())?;
    let report = json(&cli(root.path(), &["--format", "json"])?, 0)?;
    let pins: Vec<_> = report["findings"]
        .as_array()
        .ok_or("findings")?
        .iter()
        .filter(|f| f["id"] == "inventory.ci_pins")
        .collect();
    assert_eq!(pins.len(), 2);
    assert!(
        pins.iter()
            .all(|f| f["outcome"] == "unverified" && f["required"] == false)
    );
    assert_eq!(snapshot(root.path())?, before);
    fs::write(root.path().join(".gitlab-ci.yml"), "secret: [SECRET_INPUT")?;
    let output = cli(root.path(), &["--format", "json"])?;
    let error = json(&output, 2)?;
    assert_eq!(
        finding(&error, "inventory.ci_pins", Some(".gitlab-ci.yml"))?["outcome"],
        "error"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("SECRET_INPUT"));
    Ok(())
}

#[test]
fn relative_executable_and_nested_environments_are_not_guessed() -> TestResult {
    let (root, mut manifest) = fixture()?;
    manifest.commands.get_mut("probe").ok_or("probe")?.argv = vec![
        "./tools/runner".into(),
        "wsl".into(),
        "cargo".into(),
        "test".into(),
    ];
    save(root.path(), &manifest)?;
    let report = json(&cli(root.path(), &["--format", "json"])?, 1)?;
    assert_eq!(
        finding(&report, "command.executable", None)?["outcome"],
        "unverified"
    );
    assert!(
        finding(&report, "command.execution", None)?["next_step"]
            .as_str()
            .ok_or("next")?
            .contains("WSL")
    );
    assert_eq!(report["runtime"]["os"], std::env::consts::OS);
    Ok(())
}

#[test]
fn project_commands_version_probes_and_local_helpers_are_never_invoked() -> TestResult {
    let (root, mut manifest) = fixture()?;
    let bin = root.path().join("tools with spaces");
    fs::create_dir(&bin)?;
    let marker = root.path().join("EXECUTED");
    for name in ["pnpm", "git", "glab", "gh", "wsl", "docker"] {
        #[cfg(windows)]
        fs::write(
            bin.join(format!("{name}.cmd")),
            format!("@echo off\r\necho executed>\"{}\"\r\n", marker.display()),
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = bin.join(name);
            fs::write(
                &path,
                format!("#!/bin/sh\necho executed > '{}'\n", marker.display()),
            )?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
        }
    }
    manifest.commands.get_mut("probe").ok_or("probe")?.argv =
        vec!["pnpm".into(), "--version".into()];
    save(root.path(), &manifest)?;
    let before = snapshot(root.path())?;
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["doctor", "--format", "json", "--root"])
        .arg(root.path())
        .env("PATH", &bin)
        .env("PATHEXT", ".EXE;.CMD")
        .output()?;
    let report = json(&output, 0)?;
    assert_eq!(
        finding(&report, "command.executable", None)?["outcome"],
        "passed"
    );
    assert!(!marker.exists());
    assert_eq!(snapshot(root.path())?, before);
    Ok(())
}

#[test]
fn remote_is_explicit_and_missing_configuration_is_unverified() -> TestResult {
    let (root, mut manifest) = fixture()?;
    manifest.project.ci.remote = None;
    save(root.path(), &manifest)?;
    let default = json(&cli(root.path(), &["--format", "json"])?, 0)?;
    assert_eq!(default["remote_requested"], false);
    let requested = json(&cli(root.path(), &["--remote", "--format", "json"])?, 1)?;
    assert_eq!(requested["remote_requested"], true);
    assert_eq!(
        finding(&requested, "remote.configuration", None)?["outcome"],
        "unverified"
    );
    let blank = tempfile::tempdir()?;
    assert_eq!(
        finding(
            &json(&cli(blank.path(), &["--remote", "--format", "json"])?, 1)?,
            "remote.blocked",
            None
        )?["outcome"],
        "unverified"
    );
    Ok(())
}

#[test]
fn nested_roots_resolve_contract_but_do_not_cross_nested_git_boundaries() -> TestResult {
    let (root, _) = fixture()?;
    let child = root.path().join("nested/child");
    fs::create_dir_all(&child)?;
    let report = json(&cli(&child, &["--format", "json"])?, 0)?;
    assert_eq!(
        Path::new(report["root"].as_str().ok_or("root")?),
        root.path().canonicalize()?
    );
    fs::create_dir(child.join(".git"))?;
    let nested = json(&cli(&child, &["--format", "json"])?, 0)?;
    assert_eq!(
        finding(&nested, "project.contract", None)?["outcome"],
        "not_applicable"
    );
    Ok(())
}

#[test]
fn missing_root_and_oversized_input_retain_runtime_details() -> TestResult {
    let root = tempfile::tempdir()?;
    let missing = json(
        &cli(&root.path().join("missing"), &["--format", "json"])?,
        2,
    )?;
    assert_eq!(
        finding(&missing, "runtime.identity", None)?["outcome"],
        "passed"
    );
    fs::create_dir(root.path().join(".opdev"))?;
    fs::write(root.path().join(MANIFEST_PATH), " ".repeat(1024 * 1024 + 1))?;
    let oversized = json(&cli(root.path(), &["--format", "json"])?, 2)?;
    assert_eq!(
        finding(&oversized, "project.contract", None)?["outcome"],
        "error"
    );
    Ok(())
}

#[test]
#[ignore = "explicit historical CLI required for the doctor exit migration demonstration"]
fn historical_doctor_misses_missing_tool_but_new_doctor_blocks_it() -> TestResult {
    let baseline =
        std::env::var_os("OPDEV_TEST_DOCTOR_BASELINE").ok_or("explicit baseline required")?;
    let (root, mut manifest) = fixture()?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(root.path())
            .status()?
            .success()
    );
    manifest.commands.get_mut("probe").ok_or("probe")?.argv[0] = root
        .path()
        .join("required-but-missing.exe")
        .to_string_lossy()
        .into_owned();
    save(root.path(), &manifest)?;
    let before = Command::new(baseline)
        .arg("doctor")
        .arg("--root")
        .arg(root.path())
        .output()?;
    assert_eq!(
        before.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&before.stderr)
    );
    let after = json(&cli(root.path(), &["--format", "json"])?, 1)?;
    assert_eq!(
        finding(&after, "command.executable", None)?["outcome"],
        "failed"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn linked_inputs_are_not_followed() -> TestResult {
    use std::os::unix::fs::symlink;
    let (root, mut manifest) = fixture()?;
    let outside = tempfile::tempdir()?;
    fs::write(outside.path().join("secret"), "DO_NOT_READ")?;
    symlink(outside.path(), root.path().join("linked"))?;
    manifest.authorities.insert(
        "link".into(),
        AuthorityRef {
            kind: AuthorityKind::Path,
            location: "linked/secret".into(),
        },
    );
    save(root.path(), &manifest)?;
    let report = json(&cli(root.path(), &["--format", "json"])?, 1)?;
    assert_eq!(
        finding(&report, "authority.path", Some("authorities.link"))?["outcome"],
        "unverified"
    );
    symlink(
        outside.path().join("secret"),
        root.path().join(".gitlab-ci.yml"),
    )?;
    json(&cli(root.path(), &["--format", "json"])?, 2)?;
    assert_eq!(
        fs::read_to_string(outside.path().join("secret"))?,
        "DO_NOT_READ"
    );
    Ok(())
}
