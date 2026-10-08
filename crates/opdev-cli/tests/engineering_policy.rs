//! Real CLI policy preview, legacy preservation and honest assessment boundaries.
use opdev_project::{MANIFEST_PATH, ProjectManifest};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn maintained_release_line_is_not_a_second_development_trunk() -> TestResult {
    use opdev_project::{AdoptionRecord, AdoptionWorkflow, MainOption};
    let dir = fixture()?;
    let mut project = ProjectManifest::load(&dir.path().join(MANIFEST_PATH))?;
    let mut record = AdoptionRecord::pending()?;
    record.workflow = Some(AdoptionWorkflow {
        integration_branches: vec!["main".into()],
        release_source: "support/1.x".into(),
        main_option: MainOption::AlreadyMain,
        references: vec!["reviewed branch/CI policy".into()],
    });
    assert!(
        !record.workflow_blockers(&project).is_empty(),
        "legacy policy unchanged"
    );
    project.schema = 3;
    project.assurance.profiles.clear();
    project.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "1".into(),
        minimumcd: Some("1".into()),
        review_reference: "synthetic developer decision".into(),
        maintenance_branches: vec![opdev_core::MaintenanceBranch {
            name: "support/1.x".into(),
            supported_version: "1.x".into(),
            authority: "existing support policy".into(),
        }],
    });
    assert!(
        record.workflow_blockers(&project).is_empty(),
        "reviewed maintained line allowed"
    );
    let report = opdev_engine::evaluate(
        dir.path(),
        &project,
        opdev_engine::CheckOptions {
            execute_checks: false,
            ..opdev_engine::CheckOptions::pre_merge()
        },
    )?;
    assert_eq!(
        report
            .rules
            .iter()
            .find(|r| r.rule_id.as_str() == "OPDEV-BRANCH-001")
            .ok_or("branch rule")?
            .outcome,
        opdev_core::Outcome::Unverified,
        "a branch declaration does not qualify its behavior"
    );
    record
        .workflow
        .as_mut()
        .ok_or("workflow")?
        .integration_branches
        .push("support/1.x".into());
    assert!(
        !record.workflow_blockers(&project).is_empty(),
        "maintenance label cannot hide a competing integration stream"
    );
    Ok(())
}

fn cli(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(args)
        .output()
}

fn fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .arg(dir.path())
            .status()?
            .success()
    );
    assert!(
        cli(dir.path(), &["init", "--legacy-policy"])?
            .status
            .success()
    );
    Ok(dir)
}

#[test]
fn migration_preview_is_read_only_and_preserves_unrelated_policy() -> TestResult {
    let dir = fixture()?;
    let root = dir.path();
    let before = fs::read(root.join(MANIFEST_PATH))?;
    let args = [
        "upgrade",
        "--engineering-policy",
        "1",
        "--policy-review-reference",
        "synthetic developer approval",
        "--format",
        "json",
    ];
    let preview = cli(root, &args)?;
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let result: Value = serde_json::from_slice(&preview.stdout)?;
    assert_eq!(result["written"], false);
    assert_eq!(result["project_verification"], "unverified");
    assert_eq!(before, fs::read(root.join(MANIFEST_PATH))?);
    let mut old = ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    let mut next = ProjectManifest::from_yaml(result["candidate_yaml"].as_str().ok_or("yaml")?)?;
    assert_eq!(next.schema, 3);
    assert_eq!(
        next.assurance
            .engineering
            .as_ref()
            .ok_or("policy")?
            .minimumcd
            .as_deref(),
        Some("1"),
        "keep retains legacy assessment intent"
    );
    next.schema = old.schema;
    next.assurance.engineering = None;
    old.assurance.profiles.retain(|p| p.name != "opdev-core");
    assert_eq!(
        old, next,
        "commands, authorities, testing, delivery and other profile pins preserved"
    );
    let mut invalid = args.to_vec();
    invalid.extend(["--apply", "anything"]);
    assert!(!cli(root, &invalid)?.status.success());
    assert_eq!(before, fs::read(root.join(MANIFEST_PATH))?);
    Ok(())
}

#[test]
fn cli_reports_separate_assessment_without_claiming_verification() -> TestResult {
    for kind in ["library", "cli", "service"] {
        let dir = fixture()?;
        let root = dir.path();
        let legacy: Value = serde_json::from_slice(
            &cli(root, &["check", "--no-exec", "--format", "json"])?.stdout,
        )?;
        assert_eq!(legacy["schema"], 1);
        assert_eq!(legacy["catalog_version"], 2);
        assert!(legacy.get("engineering").is_none());
        assert!(
            !cli(root, &["check", "--no-exec", "--require-minimumcd"])?
                .status
                .success()
        );
        let preview: Value = serde_json::from_slice(
            &cli(
                root,
                &[
                    "upgrade",
                    "--engineering-policy",
                    "1",
                    "--minimumcd-assessment",
                    "none",
                    "--policy-review-reference",
                    "synthetic approval",
                    "--format",
                    "json",
                ],
            )?
            .stdout,
        )?;
        let mut manifest =
            ProjectManifest::from_yaml(preview["candidate_yaml"].as_str().ok_or("yaml")?)?;
        let mut value = serde_json::to_value(&manifest)?;
        value["project"]["kind"] = Value::String(kind.into());
        manifest = ProjectManifest::from_yaml(&serde_json::to_string(&value)?)?;
        fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
        let human = String::from_utf8(cli(root, &["check", "--no-exec"])?.stdout)?;
        assert!(human.contains("MinimumCD assessment: not requested; no compliance claim."));
        let report: Value = serde_json::from_slice(
            &cli(root, &["check", "--no-exec", "--format", "json"])?.stdout,
        )?;
        assert_eq!(report["schema"], 2);
        assert_eq!(report["catalog_version"], 3);
        assert!(report["engineering"]["minimumcd"].is_null());
        assert_eq!(report["rules"].as_array().ok_or("rules")?.len(), 42);
        manifest
            .assurance
            .engineering
            .as_mut()
            .ok_or("policy")?
            .minimumcd = Some("1".into());
        fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
        let output = cli(
            root,
            &[
                "check",
                "--no-exec",
                "--require-minimumcd",
                "--format",
                "json",
            ],
        )?;
        assert_eq!(output.status.code(), Some(1));
        let report: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(report["engineering"]["minimumcd"]["verdict"], "blocked");
        assert_eq!(
            report["engineering"]["minimumcd"]["requirements"]
                .as_array()
                .ok_or("requirements")?
                .len(),
            19
        );
        assert!(jsonschema::is_valid(
            &serde_json::from_str(include_str!("../../../schema/report.schema.json"))?,
            &report
        ));
    }
    Ok(())
}
