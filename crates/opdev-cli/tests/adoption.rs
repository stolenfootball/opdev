//! Adoption must be explicit, resumable, and unable to waive verification.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use opdev_core::{Evidence, Outcome, VerificationMethod, embedded_catalog};
use opdev_project::{
    ADOPTION_PATH, AdoptionRecord, AdoptionReview, AdoptionState, AdoptionWorkflow, ChangeEvidence,
    CiProvider, CommandSpec, DeliveryStatus, EVIDENCE_PATH, Environment, EvidenceAssertion,
    EvidenceLedger, MANIFEST_PATH, MainOption, RecoveryStrategy, Requirement, TestStage, TestSuite,
    adoption_catalog, discover, staged_fingerprint,
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
    assert_eq!(record.schema, 2);
    let project: Value = serde_saphyr::from_slice(&fs::read(root.join(MANIFEST_PATH))?)?;
    assert_eq!(project["schema"], 1);
    // New adoption is already current even though the project contract is v1.
    let before_migration = fs::read(root.join(ADOPTION_PATH))?;
    assert!(cli(root, &["adoption", "migrate"])?.status.success());
    assert_eq!(fs::read(root.join(ADOPTION_PATH))?, before_migration);
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
    // Synthetic reviewer authorization; never used as a real project attestation.
    let manifest = discover(root)?.manifest;
    let mut record = AdoptionRecord::load(root)?.ok_or("record")?;
    record.workflow = Some(AdoptionWorkflow {
        integration_branches: vec![manifest.project.trunk.clone()],
        release_source: manifest.project.trunk.clone(),
        main_option: if manifest.project.trunk == "main" {
            MainOption::AlreadyMain
        } else {
            MainOption::KeepName
        },
        references: vec!["fixture-review.md".into()],
    });
    record.review = Some(AdoptionReview {
        plan_id: record.plan_id(&manifest)?,
        reviewer: "synthetic reviewer".into(),
        reference: "fixture-review.md".into(),
        delegation: None,
    });
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
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
                        kind: if rule.id.as_str() == "MCD-PIPELINE-001" {
                            "delivery_gate".into()
                        } else {
                            "adoption_review".into()
                        },
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
fn approval_is_separate_stale_choices_fail_and_progress_preserves_approval()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let manifest = discover(root)?.manifest;
    let mut record = AdoptionRecord::load(root)?.ok_or("record")?;
    let original_id = record.plan_id(&manifest)?;
    record
        .practices
        .get_mut("integration")
        .ok_or("integration")?
        .state = AdoptionState::InProgress;
    assert_eq!(original_id, record.plan_id(&manifest)?);
    assert!(record.approval_blockers(&manifest)?.is_empty());
    assert!(
        record
            .blockers(&manifest)?
            .iter()
            .any(|b| b.contains("in progress"))
    );
    record
        .practices
        .get_mut("coverage")
        .ok_or("coverage")?
        .reason = "Different proposed policy".into();
    assert!(!record.approval_blockers(&manifest)?.is_empty());
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
    let before = fs::read(root.join(ADOPTION_PATH))?;
    let stale = cli(
        root,
        &[
            "adoption",
            "approve",
            "--plan",
            &original_id,
            "--reviewer",
            "test",
            "--reference",
            "fixture-review.md",
        ],
    )?;
    assert_eq!(stale.status.code(), Some(2));
    assert_eq!(before, fs::read(root.join(ADOPTION_PATH))?);
    let fresh = record.plan_id(&manifest)?;
    assert!(
        cli(
            root,
            &[
                "adoption",
                "approve",
                "--plan",
                &fresh,
                "--reviewer",
                "test",
                "--reference",
                "fixture-review.md",
                "--delegation",
                "Only the reviewed fixture choices; no publication"
            ]
        )?
        .status
        .success()
    );
    assert!(
        AdoptionRecord::load(root)?
            .ok_or("record")?
            .review
            .ok_or("review")?
            .delegation
            .is_some()
    );
    let status: Value =
        serde_json::from_slice(&cli(root, &["adoption", "status", "--format", "json"])?.stdout)?;
    assert_eq!(status["approval"], "approved");
    assert_eq!(status["complete"], false);
    assert_eq!(status["verification"], "not_run");
    Ok(())
}

#[test]
fn migration_preserves_decisions_without_inventing_consent()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut record = AdoptionRecord::load(root)?.ok_or("record")?;
    record.schema = 1;
    record.review = None;
    record.workflow = None;
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
    let before = fs::read(root.join(ADOPTION_PATH))?;
    let preview = cli(root, &["adoption", "migrate"])?;
    assert!(preview.status.success());
    assert_eq!(before, fs::read(root.join(ADOPTION_PATH))?);
    assert_eq!(cli(root, &["adoption", "check"])?.status.code(), Some(1));
    assert!(
        cli(root, &["adoption", "migrate", "--write"])?
            .status
            .success()
    );
    let migrated = AdoptionRecord::load(root)?.ok_or("record")?;
    assert_eq!(migrated.schema, 2);
    assert!(migrated.review.is_none());
    assert_eq!(
        serde_json::to_value(record.practices)?,
        serde_json::to_value(migrated.practices)?
    );
    Ok(())
}

#[test]
fn a_non_main_trunk_is_valid_but_gitflow_roles_are_not() -> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let mut manifest = discover(repo.path())?.manifest;
    manifest.project.trunk = "develop".into();
    let mut record = AdoptionRecord::load(repo.path())?.ok_or("record")?;
    let workflow = record.workflow.as_mut().ok_or("workflow")?;
    workflow.integration_branches = vec!["develop".into()];
    workflow.release_source = "develop".into();
    workflow.main_option = MainOption::KeepName;
    assert!(record.workflow_blockers(&manifest).is_empty());
    fs::write(repo.path().join(MANIFEST_PATH), manifest.to_yaml()?)?;
    fs::write(repo.path().join(ADOPTION_PATH), record.to_yaml()?)?;
    let before = fs::read(repo.path().join(ADOPTION_PATH))?;
    let proposal = cli(repo.path(), &["adoption", "plan"])?;
    assert!(proposal.status.success());
    let proposal: Value = serde_json::from_slice(&proposal.stdout)?;
    assert_eq!(
        proposal["branch_choices"]
            .as_array()
            .ok_or("choices")?
            .len(),
        2
    );
    assert!(
        proposal["branch_choices"][0]
            .as_str()
            .ok_or("keep")?
            .contains("develop")
    );
    assert!(
        proposal["branch_choices"][1]
            .as_str()
            .ok_or("rename")?
            .contains("main")
    );
    assert_eq!(before, fs::read(repo.path().join(ADOPTION_PATH))?);
    record.workflow.as_mut().ok_or("workflow")?.release_source = "main".into();
    assert!(
        record
            .workflow_blockers(&manifest)
            .iter()
            .any(|b| b.contains("migration_required"))
    );
    record
        .workflow
        .as_mut()
        .ok_or("workflow")?
        .integration_branches
        .push("main".into());
    assert!(!record.workflow_blockers(&manifest).is_empty());
    // Renaming is also valid, but only when both roles move together.
    manifest.project.trunk = "main".into();
    let workflow = record.workflow.as_mut().ok_or("workflow")?;
    workflow.integration_branches = vec!["main".into()];
    workflow.release_source = "main".into();
    workflow.main_option = MainOption::RenameToMain;
    assert!(record.workflow_blockers(&manifest).is_empty());
    Ok(())
}

#[test]
fn evidence_preparation_is_unresolved_read_only_and_correctly_scoped()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let before = fs::read(root.join(EVIDENCE_PATH))?;
    let output = cli(root, &["adoption", "prepare-evidence"])?;
    assert!(output.status.success());
    let review: opdev_project::EvidenceBootstrap = serde_saphyr::from_slice(&output.stdout)?;
    assert_eq!(review.change.fingerprint, staged_fingerprint(root)?);
    assert_eq!(review.change.evidence[0].kind, "adoption_review");
    assert_eq!(
        review.change.evidence[0].location.as_deref(),
        Some(ADOPTION_PATH)
    );
    assert!(
        review
            .change
            .decisions
            .values()
            .all(|d| *d == opdev_project::ReviewDecision::ReviewRequired)
    );
    assert_eq!(before, fs::read(root.join(EVIDENCE_PATH))?);
    Ok(())
}

#[test]
fn implemented_labels_do_not_authorize_commands_or_complete_adoption()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut record = AdoptionRecord::load(root)?.ok_or("record")?;
    record.review = None;
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
    assert!(git(root, &["add", "."])?.status.success());
    let output = cli(root, &["adoption", "check", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert!(report["core_report"].is_null());
    assert_eq!(report["complete"], false);
    assert!(report["blockers"].to_string().contains("approval"));
    Ok(())
}

#[test]
fn integration_success_does_not_qualify_delivery_and_delivery_suites_execute()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut manifest = discover(root)?.manifest;
    manifest.delivery.status = DeliveryStatus::MigrationRequired;
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    bind_review(root)?;
    assert!(cli(root, &["check", "--ci"])?.status.success());
    assert_eq!(
        cli(root, &["check", "--ci", "--delivery"])?.status.code(),
        Some(1)
    );
    assert_eq!(
        cli(root, &["check", "--ci", "--delivery", "--no-exec"])?
            .status
            .code(),
        Some(2)
    );
    manifest.delivery.status = DeliveryStatus::Configured;
    manifest.commands.insert(
        "delivery-probe".into(),
        CommandSpec {
            argv: vec![
                "git".into(),
                "rev-parse".into(),
                "--verify".into(),
                "missing-ref".into(),
            ],
            working_directory: None,
            timeout_seconds: Some(30),
        },
    );
    manifest.testing.suites.push(TestSuite {
        id: "delivery-probe".into(),
        command: "delivery-probe".into(),
        stages: vec![TestStage::Delivery],
    });
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    bind_review(root)?;
    let output = cli(root, &["check", "--ci", "--delivery", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["checks"][0]["id"], "delivery-probe");
    assert_eq!(report["checks"][0]["outcome"], "failed");
    Ok(())
}

#[test]
fn adoption_requires_release_path_review_not_a_generic_pipeline_pass()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut ledger = EvidenceLedger::load_optional(root, &embedded_catalog()?)?.ok_or("ledger")?;
    let pipeline = ledger.changes[0]
        .assertions
        .iter_mut()
        .find(|a| a.rule_id.as_str() == "MCD-PIPELINE-001")
        .ok_or("pipeline")?;
    pipeline.evidence[0].kind = "generic_pipeline".into();
    fs::write(root.join(EVIDENCE_PATH), ledger.to_yaml()?)?;
    let output = cli(root, &["adoption", "check", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    let result: Value = serde_json::from_slice(&output.stdout)?;
    assert!(result["blockers"].to_string().contains("delivery_gate"));
    Ok(())
}

#[test]
fn known_gitflow_conflict_cannot_be_hidden_by_an_evidence_pass()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut record = AdoptionRecord::load(root)?.ok_or("record")?;
    record.workflow.as_mut().ok_or("workflow")?.release_source = "release-promotion".into();
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
    assert!(git(root, &["add", "."])?.status.success());
    let mut ledger = EvidenceLedger::load_optional(root, &embedded_catalog()?)?.ok_or("ledger")?;
    ledger.changes[0].fingerprint = staged_fingerprint(root)?;
    fs::write(root.join(EVIDENCE_PATH), ledger.to_yaml()?)?;
    let output = cli(root, &["check", "--ci", "--format", "json"])?;
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let trunk = report["rules"]
        .as_array()
        .ok_or("rules")?
        .iter()
        .find(|r| r["rule_id"] == "MCD-TRUNK-001")
        .ok_or("trunk")?;
    assert_eq!(trunk["outcome"], "migration_required");
    assert_eq!(output.status.code(), Some(1));
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
