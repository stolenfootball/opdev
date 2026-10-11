//! Exact additional standards reuse commands and retain independent claim boundaries.
use opdev_project::{
    Capability, CapabilityFact, CapabilityState, CommandSpec, MANIFEST_PATH, ProjectManifest,
    SafeguardPolicy, StandardMode, StandardSelection, TestStage, TestSuite,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn cli(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(args)
        .output()
}

fn fixture() -> Result<(tempfile::TempDir, ProjectManifest), Box<dyn std::error::Error>> {
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
    let mut project = ProjectManifest::load(&dir.path().join(MANIFEST_PATH))?;
    project.schema = 4;
    project.assurance.profiles.clear();
    project.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "2".into(),
        minimumcd: None,
        review_reference: "synthetic fixture".into(),
        maintenance_branches: vec![],
    });
    project.assurance.safeguards = Some(SafeguardPolicy {
        version: 1,
        review_reference: "synthetic no-product capabilities".into(),
        capabilities: Capability::ALL
            .into_iter()
            .map(|c| {
                (
                    c,
                    CapabilityFact {
                        state: CapabilityState::Absent,
                        rationale: "isolated fixture".into(),
                        authority: "contracts".into(),
                    },
                )
            })
            .collect(),
    });
    project.commands.clear();
    project.commands.insert("count".into(), CommandSpec {
        argv: vec!["python".into(), "-c".into(), "from pathlib import Path; p=Path('counter'); p.write_text(str(int(p.read_text())+1) if p.exists() else '1')".into()],
        working_directory: None, timeout_seconds: Some(30),
    });
    project.testing.suites = vec![TestSuite {
        id: "count".into(),
        command: "count".into(),
        stages: vec![TestStage::Local],
    }];
    Ok((dir, project))
}

fn selection(name: &str, version: &str, mode: StandardMode) -> StandardSelection {
    StandardSelection {
        name: name.into(),
        version: version.into(),
        mode,
        stages: if mode == StandardMode::Guidance {
            vec![]
        } else {
            vec![TestStage::Local]
        },
        level: None,
    }
}

#[test]
fn multiple_standards_do_not_repeat_execution_or_invent_conformance() -> TestResult {
    let (dir, mut project) = fixture()?;
    project.assurance.standards = vec![
        selection("minimumcd", "1", StandardMode::Assess),
        selection("nist-ssdf-derived", "1.1", StandardMode::Guidance),
        selection("slsa-build-provenance", "1.2", StandardMode::Require),
    ];
    let path = dir.path().join(MANIFEST_PATH);
    fs::write(&path, project.to_yaml()?)?;
    let before = fs::read(&path)?;
    let preview = cli(
        dir.path(),
        &[
            "policy",
            "preview",
            "--engineering",
            "2",
            "--format",
            "json",
        ],
    )?;
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let preview: Value = serde_json::from_slice(&preview.stdout)?;
    assert_eq!(
        preview["current_standards"]
            .as_array()
            .ok_or("standards")?
            .len(),
        3
    );
    assert!(!dir.path().join("counter").exists());
    assert_eq!(fs::read(&path)?, before);
    let output = cli(
        dir.path(),
        &["check", "--require-minimumcd", "--format", "json"],
    )?;
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(dir.path().join("counter"))?, "1");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let standards = report["engineering"]["standards"]
        .as_array()
        .ok_or("standards")?;
    assert_eq!(standards[0]["assessment"]["verdict"], "blocked");
    assert!(standards[1].get("assessment").is_none());
    assert_eq!(standards[2]["complete_mapping"], false);
    assert_eq!(standards[2]["assessment"]["verdict"], "blocked");
    assert_eq!(
        report["checks"]
            .as_array()
            .ok_or("checks")?
            .iter()
            .filter(|c| c["kind"] == "suite")
            .count(),
        1
    );
    assert!(jsonschema::is_valid(
        &serde_json::from_str(include_str!("../../../schema/report.schema.json"))?,
        &report
    ));
    let skipped = cli(dir.path(), &["check", "--no-exec", "--format", "json"])?;
    let skipped: Value = serde_json::from_slice(&skipped.stdout)?;
    assert_eq!(fs::read_to_string(dir.path().join("counter"))?, "1");
    assert!(
        skipped["checks"]
            .as_array()
            .ok_or("checks")?
            .iter()
            .any(|c| c["id"] == "count" && c["outcome"] == "unverified")
    );
    Ok(())
}

#[test]
fn invalid_selections_and_unselected_minimumcd_fail_before_execution() -> TestResult {
    let (dir, mut project) = fixture()?;
    let path = dir.path().join(MANIFEST_PATH);
    let standard = selection("minimumcd", "1", StandardMode::Assess);
    project.assurance.standards = vec![standard.clone()];
    let valid = serde_json::to_value(&project)?;
    let mut invalids = vec![];
    let mut candidate = valid.clone();
    candidate["schema"] = json!(3);
    candidate["assurance"]["engineering"]["version"] = json!("1");
    invalids.push(candidate);
    let mut candidate = valid.clone();
    candidate["assurance"]["standards"] = json!([standard, standard]);
    invalids.push(candidate);
    let mut candidate = valid.clone();
    candidate["assurance"]["engineering"]["minimumcd"] = json!("1");
    invalids.push(candidate);
    for (field, value) in [
        ("version", json!("latest")),
        ("stages", json!([])),
        ("mode", json!("ignore")),
    ] {
        let mut candidate = valid.clone();
        candidate["assurance"]["standards"][0][field] = value;
        invalids.push(candidate);
    }
    for invalid in invalids {
        fs::write(&path, serde_json::to_vec(&invalid)?)?;
        assert!(ProjectManifest::load(&path).is_err());
        let before = fs::read(&path)?;
        assert!(!cli(dir.path(), &["check"])?.status.success());
        assert!(!dir.path().join("counter").exists());
        assert_eq!(fs::read(&path)?, before);
    }
    for mut selected in [
        selection("minimumcd", "1", StandardMode::Guidance),
        standard,
    ] {
        if selected.mode != StandardMode::Guidance {
            selected.stages = vec![TestStage::Delivery];
        }
        project.assurance.standards = vec![selected];
        fs::write(&path, project.to_yaml()?)?;
        let output = cli(dir.path(), &["check", "--require-minimumcd"])?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("MinimumCD"));
        assert!(!dir.path().join("counter").exists());
    }
    Ok(())
}
