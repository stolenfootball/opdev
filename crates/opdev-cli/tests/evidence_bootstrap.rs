//! Process-level acceptance tests for evidence bootstrap review.

use std::fs;
use std::process::Command;

use opdev_core::Evidence;
use opdev_project::{EVIDENCE_PATH, EvidenceBootstrap, ReviewDecision};

fn opdev() -> Command {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
}

fn initialize_external(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(root)
            .status()?
            .success()
    );
    assert!(
        opdev()
            .args(["init", "--root"])
            .arg(root)
            .args([
                "--engineering-policy",
                "1",
                "--minimumcd-assessment",
                "none",
                "--policy-review-reference",
                "synthetic fixture policy",
            ])
            .output()?
            .status
            .success()
    );
    let manifest_path = root.join(opdev_project::MANIFEST_PATH);
    let mut manifest = opdev_project::ProjectManifest::load(&manifest_path)?;
    manifest.assurance.review_storage = Some(opdev_project::ReviewStorage {
        report_retention_days: None,
        version: 1,
        provider: opdev_project::CiProvider::Gitlab,
        repository_id: 7,
        review_reference: "fixture only".into(),
        retention_authority: "fixture recovery".into(),
    });
    fs::write(&manifest_path, manifest.to_yaml()?)?;
    Ok(())
}

#[test]
fn external_bootstrap_requires_new_output_and_never_creates_source_ledger()
-> Result<(), Box<dyn std::error::Error>> {
    let project = tempfile::tempdir()?;
    let root = project.path();
    initialize_external(root)?;
    let manifest_path = root.join(opdev_project::MANIFEST_PATH);
    let original = fs::read(&manifest_path)?;
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["add", "."])
            .status()?
            .success()
    );
    let generated = opdev()
        .args(["evidence", "bootstrap", "--root"])
        .arg(root)
        .output()?;
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let external = tempfile::tempdir()?;
    let answers_path = external.path().join("answers.yaml");
    fs::write(&answers_path, generated.stdout)?;
    let mut answers = EvidenceBootstrap::load(&answers_path)?;
    *answers
        .project
        .decisions
        .values_mut()
        .next()
        .ok_or("fixture project decision")? = ReviewDecision::Passed;
    answers.project.evidence.push(Evidence {
        kind: "fixture".into(),
        summary: "Synthetic assertion only; not actual qualification".into(),
        location: Some("fixture:review".into()),
    });
    answers.change.work = "synthetic incomplete review".into();
    fs::write(&answers_path, answers.to_yaml()?)?;
    let invoke = |output: Option<&std::path::Path>| {
        let mut command = opdev();
        command
            .args(["evidence", "bootstrap", "--root"])
            .arg(root)
            .arg("--answers")
            .arg(&answers_path)
            .arg("--write");
        if let Some(path) = output {
            command.arg("--output").arg(path);
        }
        command.output()
    };
    let denied = invoke(None)?;
    assert_eq!(denied.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&denied.stderr).contains("no legacy ledger created"));
    let inside = root.join("candidate.yaml");
    assert_eq!(invoke(Some(&inside))?.status.code(), Some(2));
    assert!(!inside.exists());
    let output = external.path().join("candidate.yaml");
    let created = invoke(Some(&output))?;
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let ledger: opdev_project::EvidenceLedger = serde_saphyr::from_slice(&fs::read(&output)?)?;
    assert_eq!(
        ledger.project.len(),
        1,
        "unreviewed decisions must stay absent"
    );
    assert!(
        ledger.changes.is_empty(),
        "no change or test success manufactured"
    );
    let bytes = fs::read(&output)?;
    assert_eq!(invoke(Some(&output))?.status.code(), Some(2));
    assert_eq!(fs::read(&output)?, bytes);
    assert_eq!(fs::read(&manifest_path)?, original);
    assert!(!root.join(EVIDENCE_PATH).exists());
    Ok(())
}

#[test]
fn bootstrap_cli_requires_review_previews_and_writes_once() -> Result<(), Box<dyn std::error::Error>>
{
    let project = tempfile::tempdir()?;
    let root = project.path();
    assert!(
        Command::new("git")
            .arg("init")
            .arg(root)
            .status()?
            .success()
    );
    fs::write(
        root.join("package.json"),
        r#"{"name":"cli-bootstrap","scripts":{"test":"node --test"}}"#,
    )?;
    assert!(
        opdev()
            .args(["init", "--legacy-policy", "--root"])
            .arg(root)
            .status()?
            .success()
    );
    fs::write(
        root.join("OPDEV_ADOPTION.md"),
        "Reviewed project policy, applicability, change scope, tests, and integration behavior.\n",
    )?;
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["add", "."])
            .status()?
            .success()
    );

    let generated = opdev()
        .args(["evidence", "bootstrap", "--root"])
        .arg(root)
        .output()?;
    assert!(generated.status.success());
    let yaml = String::from_utf8(generated.stdout)?;
    assert!(yaml.contains("review_required"));
    assert!(yaml.contains("# "));

    let answers_directory = tempfile::tempdir()?;
    let answers_path = answers_directory.path().join("review.yaml");
    fs::write(&answers_path, &yaml)?;
    let mut answers = EvidenceBootstrap::load(&answers_path)?;
    for decision in answers.project.decisions.values_mut() {
        *decision = ReviewDecision::Passed;
    }
    for decision in answers.change.decisions.values_mut() {
        *decision = ReviewDecision::Passed;
    }
    let evidence = Evidence {
        kind: "review".into(),
        summary: "The adoption review records the facts supporting these decisions.".into(),
        location: Some("OPDEV_ADOPTION.md".into()),
    };
    answers.project.evidence.push(evidence.clone());
    answers.change.evidence.push(evidence);
    answers.change.work = "OPDEV-15 CLI bootstrap acceptance".into();
    fs::write(&answers_path, answers.to_yaml()?)?;

    let preview = opdev()
        .args(["evidence", "bootstrap", "--root"])
        .arg(root)
        .arg("--answers")
        .arg(&answers_path)
        .output()?;
    assert!(preview.status.success());
    assert!(String::from_utf8(preview.stdout)?.contains("rule_id:"));
    assert!(!root.join(EVIDENCE_PATH).exists());

    assert!(
        opdev()
            .args(["evidence", "bootstrap", "--root"])
            .arg(root)
            .arg("--answers")
            .arg(&answers_path)
            .arg("--write")
            .status()?
            .success()
    );
    assert!(root.join(EVIDENCE_PATH).exists());
    assert!(
        !opdev()
            .args(["evidence", "bootstrap", "--root"])
            .arg(root)
            .status()?
            .success()
    );
    Ok(())
}
