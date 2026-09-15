//! Adoption must be explicit, resumable, and unable to waive verification.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use opdev_core::{Evidence, Outcome, VerificationMethod, embedded_catalog};
use opdev_project::{
    ADOPTION_PATH, AdoptionRecord, AdoptionState, ChangeEvidence, CiProvider, CommandSpec,
    DeliveryStatus, EVIDENCE_PATH, Environment, EvidenceAssertion, EvidenceLedger, MANIFEST_PATH,
    RecoveryStrategy, Requirement, TestStage, TestSuite, adoption_catalog, discover,
    staged_fingerprint,
};
use serde_json::Value;

fn cli(root: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
}

fn git(root: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new("git").arg("-C").arg(root).args(args).output()
}

fn repo() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    assert!(git(repo.path(), &["init", "--quiet"])?.status.success());
    Ok(repo)
}

#[test]
fn initialization_is_unresolved_read_only_on_preview_and_resumable()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let root = repo.path();
    fs::write(
        root.join("AGENTS.md"),
        "# Existing conventions\nKeep this.\n",
    )?;
    let preview = cli(root, &["init", "--dry-run"])?;
    assert!(preview.status.success());
    assert!(!root.join(".opdev").exists());
    assert!(String::from_utf8(preview.stderr)?.contains("formatting"));
    assert!(cli(root, &["init"])?.status.success());
    let mut record = AdoptionRecord::load(root)?.ok_or("missing record")?;
    assert!(
        record
            .practices
            .values()
            .all(|d| d.state == AdoptionState::Pending)
    );
    assert!(fs::read_to_string(root.join("AGENTS.md"))?.contains("Keep this."));
    assert!(!root.join("docs").exists());
    assert!(!root.join(".opdev/experiments").exists());
    record
        .practices
        .get_mut("formatting")
        .ok_or("missing formatting")?
        .reason = "Preserve the existing style; research a compatible check.".into();
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
    let before = fs::read(root.join(ADOPTION_PATH))?;
    assert!(cli(root, &["init"])?.status.success());
    assert!(cli(root, &["adoption", "start"])?.status.success());
    assert_eq!(fs::read(root.join(ADOPTION_PATH))?, before);
    let status: Value =
        serde_json::from_slice(&cli(root, &["adoption", "status", "--format", "json"])?.stdout)?;
    assert_eq!(status["status"], "pending");
    assert_eq!(cli(root, &["adoption", "check"])?.status.code(), Some(1));
    Ok(())
}

#[test]
fn legacy_projects_are_not_silently_migrated() -> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let root = repo.path();
    discover(root)?
        .manifest
        .write_new(&root.join(MANIFEST_PATH))?;
    let before = fs::read(root.join(MANIFEST_PATH))?;
    assert!(cli(root, &["init"])?.status.success());
    assert!(!root.join(ADOPTION_PATH).exists());
    let status: Value =
        serde_json::from_slice(&cli(root, &["adoption", "status", "--format", "json"])?.stdout)?;
    assert_eq!(status["status"], "legacy_unassessed");
    assert!(
        cli(root, &["adoption", "start", "--dry-run"])?
            .status
            .success()
    );
    assert!(!root.join(ADOPTION_PATH).exists());
    assert!(cli(root, &["adoption", "start"])?.status.success());
    assert_eq!(fs::read(root.join(MANIFEST_PATH))?, before);
    Ok(())
}

#[test]
fn interrupted_initialization_preserves_pending_record_for_retry()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let root = repo.path();
    fs::write(root.join("AGENTS.md"), "<!-- opdev:start -->\nmalformed")?;
    assert_eq!(cli(root, &["init"])?.status.code(), Some(2));
    assert!(root.join(MANIFEST_PATH).exists());
    let record = fs::read(root.join(ADOPTION_PATH))?;
    fs::write(root.join("AGENTS.md"), "# Repaired project instructions\n")?;
    assert!(cli(root, &["init"])?.status.success());
    assert_eq!(fs::read(root.join(ADOPTION_PATH))?, record);
    assert!(root.join("CLAUDE.md").exists());
    Ok(())
}

#[test]
fn inventory_and_decisions_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let manifest = discover(repo.path())?.manifest;
    let record = AdoptionRecord::pending()?;
    let mut value = serde_json::to_value(&record)?;
    value["practices"]
        .as_object_mut()
        .ok_or("object")?
        .remove("formatting");
    assert!(AdoptionRecord::from_yaml(&value.to_string()).is_err());
    for field in ["schema", "catalog_version"] {
        let mut value = serde_json::to_value(&record)?;
        value[field] = serde_json::json!(999);
        assert!(AdoptionRecord::from_yaml(&value.to_string()).is_err());
    }
    let mut value = serde_json::to_value(&record)?;
    value["practices"]["invented"] = value["practices"]["formatting"].clone();
    assert!(AdoptionRecord::from_yaml(&value.to_string()).is_err());
    let mut value = serde_json::to_value(&record)?;
    value["practices"]["formatting"]["waive_core"] = serde_json::json!(true);
    assert!(AdoptionRecord::from_yaml(&value.to_string()).is_err());

    let mut record = record;
    record.scope = "Whole fixture".into();
    for decision in record.practices.values_mut() {
        decision.state = AdoptionState::Ignored;
        decision.owner = "fixture reviewer".into();
        decision.reason = "Explicit test disposition".into();
        decision.references = vec!["fixture-review.md".into()];
    }
    let blockers = record.blockers(&manifest)?;
    assert!(blockers.iter().any(|b| b.starts_with("integration:")));
    assert!(!blockers.iter().any(|b| b.starts_with("formatting:")));
    record
        .practices
        .get_mut("formatting")
        .ok_or("formatting")?
        .owner
        .clear();
    assert!(
        record
            .blockers(&manifest)?
            .iter()
            .any(|b| b.starts_with("formatting:"))
    );
    record
        .practices
        .get_mut("integration")
        .ok_or("integration")?
        .state = AdoptionState::NotApplicable;
    assert!(
        record
            .blockers(&manifest)?
            .iter()
            .any(|b| b.contains("required practice cannot"))
    );
    Ok(())
}

// Artificial reviewed fixture: it tests the verifier, not a real project's
// delivery qualification. No external CI or publication is performed here.
fn ready_fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let repo = repo()?;
    let root = repo.path();
    assert!(cli(root, &["init"])?.status.success());
    let mut manifest = discover(root)?.manifest;
    manifest.project.ci.provider = CiProvider::Gitlab;
    manifest.delivery.status = DeliveryStatus::Configured;
    manifest.delivery.recovery.strategy = RecoveryStrategy::RollForward;
    manifest.delivery.environments = vec![Environment {
        name: "fixture".into(),
        production_like: true,
    }];
    manifest.commands.insert(
        "verify".into(),
        CommandSpec {
            argv: vec!["git".into(), "status".into(), "--porcelain".into()],
            working_directory: None,
            timeout_seconds: Some(30),
        },
    );
    manifest.testing.suites = vec![TestSuite {
        id: "verify".into(),
        command: "verify".into(),
        stages: vec![TestStage::PreMerge, TestStage::PostMerge],
    }];
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    let generated = cli(
        root,
        &[
            "ci",
            "generate",
            "--provider",
            "gitlab",
            "--image",
            "rust:1.97.0-bookworm",
        ],
    )?;
    assert!(generated.status.success());
    fs::write(root.join(".gitlab-ci.yml"), generated.stdout)?;
    let mut record = AdoptionRecord::pending()?;
    record.scope = "Entire synthetic fixture including all components".into();
    for practice in adoption_catalog()?.practices {
        let decision = record.practices.get_mut(&practice.id).ok_or("practice")?;
        decision.state = if practice.requirement == Requirement::Optional {
            AdoptionState::Ignored
        } else {
            AdoptionState::Implemented
        };
        decision.owner = "test reviewer".into();
        decision.reason = "Synthetic completion test, not real qualification".into();
        decision.references = vec!["fixture-review.md".into()];
        if practice.automated && decision.state == AdoptionState::Implemented {
            decision.suites = vec!["verify".into()];
        }
    }
    fs::write(
        root.join("fixture-review.md"),
        "Synthetic review fixture. No real deployment claim.\n",
    )?;
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
    bind_review(root)?;
    Ok(repo)
}

fn bind_review(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    assert!(git(root, &["add", "."])?.status.success());
    let catalog = embedded_catalog()?;
    let ledger = EvidenceLedger {
        schema: 1,
        project: vec![],
        changes: vec![ChangeEvidence {
            fingerprint: staged_fingerprint(root)?,
            work: "synthetic fixture".into(),
            assertions: catalog
                .rules
                .iter()
                .filter(|rule| {
                    rule.verification.contains(&VerificationMethod::Evidence)
                        || rule.verification.contains(&VerificationMethod::Agent)
                })
                .map(|rule| EvidenceAssertion {
                    rule_id: rule.id.clone(),
                    outcome: Outcome::Passed,
                    summary: "Synthetic reviewed test evidence".into(),
                    evidence: vec![Evidence {
                        kind: "adoption_review".into(),
                        summary:
                            "All fixture decisions and acceptance evidence reviewed for this state"
                                .into(),
                        location: Some(ADOPTION_PATH.into()),
                    }],
                })
                .collect(),
        }],
    };
    fs::write(root.join(EVIDENCE_PATH), ledger.to_yaml()?)?;
    assert!(git(root, &["add", "."])?.status.success());
    Ok(())
}

#[test]
fn completion_requires_current_review_and_all_core_gates() -> Result<(), Box<dyn std::error::Error>>
{
    let repo = ready_fixture()?;
    let root = repo.path();
    let result = cli(root, &["adoption", "check", "--format", "json"])?;
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let result: Value = serde_json::from_slice(&result.stdout)?;
    assert_eq!(result["complete"], true);
    assert_eq!(result["core_report"]["checks"][0]["outcome"], "passed");
    fs::write(root.join("new-component.txt"), "Unreviewed component")?;
    assert_eq!(cli(root, &["adoption", "check"])?.status.code(), Some(1));
    assert!(git(root, &["add", "."])?.status.success());
    assert_eq!(cli(root, &["adoption", "check"])?.status.code(), Some(1));
    bind_review(root)?;
    let mut manifest = discover(root)?.manifest;
    manifest.delivery.status = DeliveryStatus::MigrationRequired;
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    bind_review(root)?;
    let result = cli(root, &["adoption", "check", "--format", "json"])?;
    assert_eq!(result.status.code(), Some(1));
    let result: Value = serde_json::from_slice(&result.stdout)?;
    assert_eq!(result["complete"], false);
    assert!(result["blockers"].to_string().contains("core gate"));
    Ok(())
}

#[test]
fn implemented_without_review_or_with_failing_checks_cannot_complete()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut ledger = EvidenceLedger::load_optional(root, &embedded_catalog()?)?.ok_or("ledger")?;
    for assertion in &mut ledger.changes[0].assertions {
        assertion.evidence[0].kind = "generic_review".into();
    }
    fs::write(root.join(EVIDENCE_PATH), ledger.to_yaml()?)?;
    let result: Value =
        serde_json::from_slice(&cli(root, &["adoption", "check", "--format", "json"])?.stdout)?;
    assert_eq!(result["complete"], false);
    assert!(result["core_report"].is_null());
    let mut manifest = discover(root)?.manifest;
    manifest.commands.get_mut("verify").ok_or("command")?.argv = vec![
        "git".into(),
        "rev-parse".into(),
        "--verify".into(),
        "missing-ref".into(),
    ];
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    bind_review(root)?;
    let result = cli(root, &["adoption", "check", "--format", "json"])?;
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout)?["core_report"]["checks"][0]["outcome"],
        "failed"
    );
    Ok(())
}

#[test]
fn verification_cannot_change_the_reviewed_state() -> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut manifest = discover(root)?.manifest;
    manifest.commands.get_mut("verify").ok_or("command")?.argv = vec![
        "git".into(),
        "mv".into(),
        "fixture-review.md".into(),
        "renamed-review.md".into(),
    ];
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    bind_review(root)?;
    let result = cli(root, &["adoption", "check", "--format", "json"])?;
    assert_eq!(result.status.code(), Some(1));
    let result: Value = serde_json::from_slice(&result.stdout)?;
    assert_eq!(result["complete"], false);
    assert!(result["blockers"].to_string().contains("changed"));
    Ok(())
}
