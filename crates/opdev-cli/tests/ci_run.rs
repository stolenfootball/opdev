//! CLI validation is read-only and rejects incomplete identity before network access.
use opdev_project::{CiProvider, MANIFEST_PATH, discover};
use std::{fs, process::Command};

#[test]
fn standalone_qualification_does_not_run_tests_or_need_review_storage()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let git = |args: &[&str]| -> Result<(), Box<dyn std::error::Error>> {
        let result = Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .output()?;
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        Ok(())
    };
    git(&["init", "--quiet"])?;
    let mut manifest = discover(root.path())?.manifest;
    manifest.commands.insert(
        "never".into(),
        opdev_project::CommandSpec {
            argv: vec!["opdev-must-not-run-nonexistent-command".into()],
            working_directory: None,
            timeout_seconds: Some(1),
        },
    );
    manifest.write_new(&root.path().join(MANIFEST_PATH))?;
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_opdev"))
            .args(["ci", "qualify", "--format", "json", "--root"])
            .arg(root.path())
            .output()
    };
    let dirty = run()?;
    assert_eq!(dirty.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&dirty.stdout)?;
    assert_eq!(value["local_tests"], "not_run");
    assert_eq!(value["outcome"], "unverified");
    assert!(value["qualification"].is_null());
    git(&["add", "."])?;
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "-qm",
        "fixture",
    ])?;
    let missing = run()?;
    assert_eq!(missing.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&missing.stdout)?;
    assert_eq!(value["source_unchanged"], true);
    assert_eq!(value["outcome"], "unverified");
    assert_eq!(value["artifact_readiness"], "not_evaluated");
    assert!(
        value["qualification"]
            .to_string()
            .contains("policy is missing")
    );
    assert!(!root.path().join(".opdev/evidence.yaml").exists());
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(["status", "--porcelain"])
            .output()?
            .stdout
            .is_empty()
    );
    Ok(())
}

#[test]
fn invalid_run_expectations_do_not_execute_commands_or_write_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    for (provider, extra) in [
        (CiProvider::Github, vec![]),
        (CiProvider::Gitlab, vec!["--workflow", "456"]),
        (CiProvider::Gitlab, vec!["--run", "0"]),
        (CiProvider::Gitlab, vec!["--require-job", ""]),
        (
            CiProvider::Gitlab,
            vec!["--require-job", "test", "--require-job", "test"],
        ),
    ] {
        let root = tempfile::tempdir()?;
        assert!(
            Command::new("git")
                .args(["init", "--quiet"])
                .arg(root.path())
                .status()?
                .success()
        );
        let mut manifest = discover(root.path())?.manifest;
        manifest.project.ci.provider = provider;
        manifest.project.ci.remote = Some(
            if provider == CiProvider::Github {
                "https://github.com/team/project"
            } else {
                "https://gitlab.com/team/project"
            }
            .into(),
        );
        manifest.write_new(&root.path().join(MANIFEST_PATH))?;
        let before = fs::read(root.path().join(MANIFEST_PATH))?;
        let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
            .args(["ci", "verify-run", "--root"])
            .arg(root.path())
            .args([
                "--revision",
                &"a".repeat(40),
                "--run",
                "123",
                "--ref",
                "main",
                "--source",
                "push",
                "--format",
                "json",
            ])
            .args(extra)
            .output()?;
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.contains(&0x1b));
        assert_eq!(fs::read(root.path().join(MANIFEST_PATH))?, before);
        assert!(!root.path().join(".opdev/evidence.yaml").exists());
    }
    Ok(())
}
