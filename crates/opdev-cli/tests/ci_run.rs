//! CLI validation is read-only and rejects incomplete identity before network access.
use opdev_project::{CiProvider, MANIFEST_PATH, discover};
use std::{fs, process::Command};

#[test]
fn invalid_run_expectations_do_not_execute_commands_or_write_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    for (provider, extra) in [
        (CiProvider::Github, vec![]),
        (CiProvider::Gitlab, vec!["--workflow", "456"]),
        (CiProvider::Gitlab, vec!["--run", "0"]),
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
