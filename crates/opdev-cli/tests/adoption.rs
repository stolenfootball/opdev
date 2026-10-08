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
fn engineering_initialization_requires_choices_and_preserves_existing_projects()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = repo()?;
    let root = repo.path();
    fs::write(root.join("AGENTS.md"), "Keep project instructions.\n")?;
    assert!(
        !cli(root, &["init", "--engineering-policy", "1"])?
            .status
            .success()
    );
    assert!(!root.join(".opdev").exists());
    let mut args = vec![
        "init",
        "--engineering-policy",
        "1",
        "--policy-review-reference",
        "synthetic-decision",
        "--minimumcd-assessment",
        "none",
        "--dry-run",
    ];
    assert!(cli(root, &args)?.status.success());
    assert!(!root.join(".opdev").exists());
    args.pop();
    assert!(cli(root, &args)?.status.success());
    let manifest = discover(root)?.manifest;
    assert_eq!(manifest.schema, 3);
    assert!(
        manifest
            .assurance
            .engineering
            .as_ref()
            .ok_or("policy")?
            .minimumcd
            .is_none()
    );
    let record = AdoptionRecord::load(root)?.ok_or("record")?;
    assert_eq!(record.catalog_version, 2);
    assert_eq!(record.practices.len(), 20);
    assert!(record.review.is_none());
    assert!(
        record
            .practices
            .values()
            .all(|d| d.state == AdoptionState::Pending)
    );
    assert!(
        manifest.commands.is_empty(),
        "no invented tool stack in an empty repo"
    );
    assert!(fs::read_to_string(root.join("AGENTS.md"))?.contains("Keep project instructions."));
    let before = fs::read(root.join(MANIFEST_PATH))?;
    let record_before = fs::read(root.join(ADOPTION_PATH))?;
    assert!(cli(root, &args)?.status.success(), "idempotent retry");
    args[4] = "changed-decision";
    assert!(!cli(root, &args)?.status.success());
    assert_eq!(before, fs::read(root.join(MANIFEST_PATH))?);
    assert_eq!(record_before, fs::read(root.join(ADOPTION_PATH))?);
    // Simulate interruption after the inventory write but before the contract.
    fs::remove_file(root.join(MANIFEST_PATH))?;
    assert!(!cli(root, &["init"])?.status.success());
    assert!(!root.join(MANIFEST_PATH).exists());
    assert_eq!(record_before, fs::read(root.join(ADOPTION_PATH))?);
    args[4] = "synthetic-decision";
    assert!(cli(root, &args)?.status.success());
    Ok(())
}

#[test]
fn engineering_inventory_preserves_legacy_choices_but_cannot_waive_baseline()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut manifest = discover(root)?.manifest;
    let mut record = AdoptionRecord::load(root)?.ok_or("record")?;
    let old = record.to_yaml()?;
    assert_eq!(record.practices["formatting"].state, AdoptionState::Ignored);
    assert_eq!(
        cli(root, &["adoption", "migrate", "--catalog-version", "2"])?
            .status
            .code(),
        Some(2)
    );
    assert_eq!(old, fs::read_to_string(root.join(ADOPTION_PATH))?);
    manifest.schema = 3;
    manifest
        .assurance
        .profiles
        .retain(|p| p.name != "opdev-core");
    manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "1".into(),
        minimumcd: None,
        review_reference: "synthetic-decision".into(),
        maintenance_branches: vec![],
    });
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    let blockers = record.blockers(&manifest)?.join("\n");
    assert!(blockers.contains("does not match"));
    assert!(blockers.contains("setup: missing assessment"));
    let preview = cli(root, &["adoption", "migrate", "--catalog-version", "2"])?;
    assert!(preview.status.success());
    assert_eq!(old, fs::read_to_string(root.join(ADOPTION_PATH))?);
    record = AdoptionRecord::from_yaml(&String::from_utf8(preview.stdout)?)?;
    assert_eq!(record.practices["formatting"].state, AdoptionState::Ignored);
    assert_eq!(record.practices["setup"].state, AdoptionState::Pending);
    assert_eq!(record.practices["review"].state, AdoptionState::Pending);
    assert!(record.review.is_none());
    for id in [
        "coding_style",
        "formatting",
        "linting",
        "dependencies",
        "setup",
        "review",
    ] {
        for state in [AdoptionState::Ignored, AdoptionState::NotApplicable] {
            record.practices.get_mut(id).ok_or("decision")?.state = state;
            assert!(
                record
                    .blockers(&manifest)?
                    .iter()
                    .any(|b| b.starts_with(&format!("{id}:")))
            );
        }
    }
    let decision = record.practices.get_mut("formatting").ok_or("format")?;
    decision.state = AdoptionState::Implemented;
    assert!(
        record
            .blockers(&manifest)?
            .iter()
            .any(|b| b.contains("formatting: implementation requires executable"))
    );
    decision_fill(&mut record, "formatting", AdoptionState::Implemented)?;
    assert!(
        !record
            .blockers(&manifest)?
            .iter()
            .any(|b| b.starts_with("formatting:"))
    );
    decision_fill(&mut record, "effectiveness", AdoptionState::NotApplicable)?;
    assert!(
        !record
            .blockers(&manifest)?
            .iter()
            .any(|b| b.starts_with("effectiveness:"))
    );
    let unchanged = record.to_yaml()?;
    assert!(record.migrate_catalog(1).is_err());
    assert!(record.migrate_catalog(99).is_err());
    assert_eq!(record.to_yaml()?, unchanged);
    record.migrate_catalog(2)?;
    assert_eq!(record.to_yaml()?, unchanged);
    assert!(
        cli(
            root,
            &["adoption", "migrate", "--catalog-version", "2", "--write"]
        )?
        .status
        .success()
    );
    assert_eq!(
        AdoptionRecord::load(root)?.ok_or("record")?.catalog_version,
        2
    );
    Ok(())
}

fn decision_fill(
    record: &mut AdoptionRecord,
    id: &str,
    state: AdoptionState,
) -> Result<(), Box<dyn std::error::Error>> {
    let decision = record.practices.get_mut(id).ok_or("decision")?;
    decision.state = state;
    decision.suites = if state == AdoptionState::Implemented {
        vec!["verify".into()]
    } else {
        vec![]
    };
    decision.owner = "synthetic reviewer".into();
    decision.reason = "Synthetic scoped decision; not real project qualification".into();
    decision.references = vec!["fixture-review.md".into()];
    Ok(())
}

#[test]
fn catalogs_are_read_only_and_invalid_contracts_do_not_fall_back()
-> Result<(), Box<dyn std::error::Error>> {
    let empty = tempfile::tempdir()?;
    for (args, expected) in [
        (vec!["adoption", "catalog"], 1),
        (vec!["adoption", "catalog", "--catalog-version", "2"], 2),
    ] {
        let output = cli(empty.path(), &args)?;
        assert!(output.status.success());
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout)?["version"],
            expected
        );
    }
    assert!(!empty.path().join(".opdev").exists());
    let repo = repo()?;
    fs::create_dir(repo.path().join(".opdev"))?;
    fs::write(repo.path().join(MANIFEST_PATH), "invalid: true\n")?;
    fs::create_dir(repo.path().join("nested"))?;
    for root in [repo.path().to_path_buf(), repo.path().join("nested")] {
        assert!(!cli(&root, &["adoption", "catalog"])?.status.success());
    }
    assert!(
        !cli(
            empty.path(),
            &["adoption", "catalog", "--catalog-version", "99"]
        )?
        .status
        .success()
    );
    Ok(())
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
fn remote_policy_gap_is_visible_before_commands_or_migration()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut manifest = discover(root)?.manifest;
    manifest.project.ci.remote = Some("git@gitlab.com:example/opdev-fixture.git".into());
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    bind_review(root)?;
    let before = fs::read(root.join(MANIFEST_PATH))?;
    let adoption_before = fs::read(root.join(ADOPTION_PATH))?;
    for args in [
        vec!["adoption", "plan"],
        vec!["adoption", "status", "--format", "json"],
    ] {
        let output = cli(root, &args)?;
        assert!(output.status.success());
        let value: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(value["remote_qualification"]["policy_ready"], false);
        assert_eq!(value["remote_qualification"]["verification"], "not_run");
        assert!(value["remote_qualification"]["worksheet"].is_null());
        let gap = value["remote_qualification"]["gap"].as_str().ok_or("gap")?;
        assert!(gap.contains("project schema 2") && gap.contains("adoption-record schema"));
        assert_eq!(value["delivery_readiness"], "not_run");
    }
    let checked = cli(root, &["adoption", "check", "--remote", "--format", "json"])?;
    assert_eq!(checked.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&checked.stdout)?;
    assert!(
        report["core_report"].is_null(),
        "project commands must not run before policy choice"
    );
    assert!(
        report["blockers"]
            .as_array()
            .ok_or("blockers")?
            .iter()
            .any(|b| b
                .as_str()
                .is_some_and(|s| s.starts_with("remote qualification:")))
    );
    assert_eq!(fs::read(root.join(MANIFEST_PATH))?, before);
    assert_eq!(fs::read(root.join(ADOPTION_PATH))?, adoption_before);
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
    let catalog = manifest.catalog()?;
    // Synthetic gate plumbing, not a behavioral product acceptance assessment.
    let fingerprint = staged_fingerprint(root)?;
    let mut acceptance = opdev_project::AcceptanceEvidence {
        scope: opdev_project::AcceptanceScope::NoMaterialConditions,
        rationale: "Synthetic adoption fixture with no product acceptance conditions".into(),
        ..Default::default()
    };
    acceptance.review.outcome = Outcome::Passed;
    acceptance.review.reviewer = "synthetic fixture reviewer".into();
    acceptance.review.reference = "fixture-review.md".into();
    acceptance.review.rationale = "Only unrelated gate prerequisites are modeled here".into();
    acceptance.review.subject_sha256 = acceptance.digest(&fingerprint, "synthetic fixture")?;
    let ledger = EvidenceLedger {
        schema: 2,
        project: vec![],
        changes: vec![ChangeEvidence {
            acceptance: Some(acceptance),
            fingerprint,
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
fn external_adoption_requires_selected_review_and_never_recreates_legacy_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut manifest = discover(root)?.manifest;
    manifest.schema = 3;
    manifest
        .assurance
        .profiles
        .retain(|p| p.name != "opdev-core");
    manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "1".into(),
        minimumcd: None,
        review_reference: "synthetic choice".into(),
        maintenance_branches: vec![],
    });
    manifest.layout = Some(opdev_project::LayoutPolicy {
        version: 1,
        review_reference: "synthetic layout".into(),
    });
    manifest.assurance.review_storage = Some(opdev_project::ReviewStorage {
        version: 1,
        provider: CiProvider::Gitlab,
        repository_id: 7,
        review_reference: "synthetic choice".into(),
        retention_authority: "synthetic retention".into(),
    });
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    opdev_project::reconcile_agent_files(root)?;
    let old = AdoptionRecord::load(root)?.ok_or("record")?;
    let mut record = AdoptionRecord::pending_for_catalog(2)?;
    record.scope = old.scope;
    let example = old.practices.get("integration").ok_or("example")?.clone();
    for value in record.practices.values_mut() {
        *value = example.clone();
    }
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
    bind_review(root)?;
    // Synthetic history only: the production migration must verify archive retention.
    fs::remove_file(root.join(EVIDENCE_PATH))?;
    assert!(git(root, &["add", "-A"])?.status.success());
    let before = fs::read(root.join(ADOPTION_PATH))?;
    let missing = cli(root, &["adoption", "check", "--format", "json"])?;
    assert_eq!(missing.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&missing.stdout)?;
    assert_eq!(value["complete"], false);
    assert!(
        value["core_report"].is_null(),
        "no commands without selected review"
    );
    assert!(
        value["blockers"]
            .as_array()
            .ok_or("blockers")?
            .iter()
            .any(|b| b.as_str().is_some_and(|s| s.contains("--review-locator")))
    );
    assert!(!root.join(EVIDENCE_PATH).exists());
    assert_eq!(fs::read(root.join(ADOPTION_PATH))?, before);

    let outside = tempfile::tempdir()?;
    let locator = outside.path().join("locator.json");
    fs::write(
        &locator,
        serde_json::to_vec(&serde_json::json!({
            "schema":1, "provider":"github", "repository_id":8, "commit":"a".repeat(40), "path":"review.json", "sha256":"b".repeat(64)
        }))?,
    )?;
    let wrong = cli(
        root,
        &[
            "adoption",
            "check",
            "--review-locator",
            locator.to_str().ok_or("path")?,
            "--review-acceptance-sha256",
            &"c".repeat(64),
        ],
    )?;
    assert_eq!(wrong.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("outside the selected storage policy"));
    assert!(!root.join(EVIDENCE_PATH).exists());

    fs::write(
        root.join(EVIDENCE_PATH),
        "schema: 2\nproject: []\nchanges: []\n",
    )?;
    let conflict = cli(root, &["adoption", "check", "--format", "json"])?;
    let conflict: Value = serde_json::from_slice(&conflict.stdout)?;
    assert!(
        conflict["blockers"]
            .as_array()
            .ok_or("blockers")?
            .iter()
            .any(|b| b
                .as_str()
                .is_some_and(|s| s.contains("both external storage and a legacy ledger")))
    );
    assert!(
        root.join(EVIDENCE_PATH).exists(),
        "verification never cleans up evidence"
    );
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
    let original_review = record.review.clone();
    for state in [
        AdoptionState::Pending,
        AdoptionState::InProgress,
        AdoptionState::Implemented,
    ] {
        record
            .practices
            .get_mut("integration")
            .ok_or("integration")?
            .state = state;
        fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
        let persisted = AdoptionRecord::load(root)?.ok_or("record")?;
        assert_eq!(original_id, persisted.plan_id(&manifest)?);
        assert_eq!(
            serde_json::to_value(&original_review)?,
            serde_json::to_value(&persisted.review)?
        );
        let status: Value = serde_json::from_slice(
            &cli(root, &["adoption", "status", "--format", "json"])?.stdout,
        )?;
        assert_eq!(status["approval"], "approved");
    }
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

fn engineering_fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let repo = ready_fixture()?;
    let root = repo.path();
    let mut manifest = discover(root)?.manifest;
    manifest.schema = 3;
    manifest
        .assurance
        .profiles
        .retain(|p| p.name != "opdev-core");
    manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "1".into(),
        minimumcd: None,
        review_reference: "synthetic-decision".into(),
        maintenance_branches: vec![],
    });
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    let mut record = AdoptionRecord::load(root)?.ok_or("record")?;
    record.migrate_catalog(2)?;
    for id in [
        "coding_style",
        "formatting",
        "linting",
        "dependencies",
        "setup",
        "review",
    ] {
        decision_fill(&mut record, id, AdoptionState::Implemented)?;
    }
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
    bind_review(root)?;
    Ok(repo)
}

#[test]
fn engineering_adoption_executes_checks_and_does_not_hide_violations()
-> Result<(), Box<dyn std::error::Error>> {
    for probe in ["format", "lint", "behavior", "missing_ci"] {
        let repo = engineering_fixture()?;
        let root = repo.path();
        let output = cli(root, &["adoption", "check", "--format", "json"])?;
        assert!(
            output.status.success(),
            "{probe}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let mut manifest = discover(root)?.manifest;
        let compiler_output = tempfile::tempdir()?;
        let argv = match probe {
            "format" => {
                fs::write(root.join("format.txt"), "trailing whitespace  \n")?;
                vec![
                    "git".into(),
                    "diff".into(),
                    "--cached".into(),
                    "--check".into(),
                ]
            }
            "lint" => {
                fs::write(
                    root.join("type_error.rs"),
                    "pub fn broken() -> bool { 42 }\n",
                )?;
                vec![
                    "rustc".into(),
                    "--crate-type=lib".into(),
                    "--emit=metadata".into(),
                    "--out-dir".into(),
                    compiler_output.path().to_string_lossy().into_owned(),
                    "type_error.rs".into(),
                ]
            }
            "behavior" => {
                fs::write(
                    root.join("behavior.rs"),
                    "#[test] fn expected_result() { assert_eq!(2 + 2, 5); }\n",
                )?;
                let binary = compiler_output
                    .path()
                    .join(format!("behavior{}", std::env::consts::EXE_SUFFIX));
                let compiled = Command::new("rustc")
                    .arg("--test")
                    .arg(root.join("behavior.rs"))
                    .arg("-o")
                    .arg(&binary)
                    .output()?;
                assert!(
                    compiled.status.success(),
                    "{}",
                    String::from_utf8_lossy(&compiled.stderr)
                );
                vec![binary.to_string_lossy().into_owned()]
            }
            _ => {
                fs::remove_file(root.join(".gitlab-ci.yml"))?;
                vec!["git".into(), "status".into(), "--porcelain".into()]
            }
        };
        manifest.commands.get_mut("verify").ok_or("command")?.argv = argv;
        fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
        bind_review(root)?;
        let output = cli(root, &["adoption", "check", "--format", "json"])?;
        assert_eq!(
            output.status.code(),
            Some(1),
            "{probe}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(report["complete"], false);
        if probe == "missing_ci" {
            let rules = report["core_report"]["rules"].as_array().ok_or("rules")?;
            assert!(
                rules
                    .iter()
                    .any(|r| r["rule_id"] == "MCD-CI-001" && r["outcome"] != "passed")
            );
        } else {
            assert_eq!(
                report["core_report"]["checks"][0]["outcome"], "failed",
                "{probe}: {report}"
            );
        }
    }
    Ok(())
}

#[test]
fn engineering_inapplicable_pipeline_is_distinct_from_missing_delivery_review()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = engineering_fixture()?;
    let root = repo.path();
    let manifest = discover(root)?.manifest;
    let mut ledger = EvidenceLedger::load_optional(root, &manifest.catalog()?)?.ok_or("ledger")?;
    let pipeline = ledger.changes[0]
        .assertions
        .iter_mut()
        .find(|a| a.rule_id.as_str() == "MCD-PIPELINE-001")
        .ok_or("pipeline")?;
    pipeline.outcome = Outcome::NotApplicable;
    pipeline.summary = "Synthetic capability-absence plumbing, not a real qualification".into();
    pipeline.evidence[0].kind = "applicability_review".into();
    fs::write(root.join(EVIDENCE_PATH), ledger.to_yaml()?)?;
    let output = cli(root, &["adoption", "check", "--format", "json"])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let pipeline = ledger.changes[0]
        .assertions
        .iter_mut()
        .find(|a| a.rule_id.as_str() == "MCD-PIPELINE-001")
        .ok_or("pipeline")?;
    pipeline.outcome = Outcome::Passed;
    fs::write(root.join(EVIDENCE_PATH), ledger.to_yaml()?)?;
    let output = cli(root, &["adoption", "check", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stdout)?.contains("delivery_gate"));
    Ok(())
}

#[test]
fn engineering_adoption_accepts_distinct_pre_and_post_checks_but_not_a_missing_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = engineering_fixture()?;
    let root = repo.path();
    let mut manifest = discover(root)?.manifest;
    manifest.testing.suites[0].stages = vec![TestStage::PreMerge];
    manifest.testing.suites.push(TestSuite {
        id: "integrated".into(),
        command: "verify".into(),
        stages: vec![TestStage::PostMerge],
    });
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    let mut record = AdoptionRecord::load(root)?.ok_or("record")?;
    for decision in record
        .practices
        .values_mut()
        .filter(|decision| !decision.suites.is_empty())
    {
        decision.suites = vec!["verify".into(), "integrated".into()];
    }
    fs::write(root.join(ADOPTION_PATH), record.to_yaml()?)?;
    bind_review(root)?;
    let output = cli(root, &["adoption", "check", "--format", "json"])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let checks = report["core_report"]["checks"].as_array().ok_or("checks")?;
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0]["id"], "verify");
    let post = cli(root, &["check", "--ci", "--post-merge", "--format", "json"])?;
    assert!(
        post.status.success(),
        "{}",
        String::from_utf8_lossy(&post.stdout)
    );
    let report: Value = serde_json::from_slice(&post.stdout)?;
    assert_eq!(report["checks"][0]["id"], "integrated");
    manifest.testing.suites.pop();
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    bind_review(root)?;
    let output = cli(root, &["adoption", "check", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert!(report["core_report"].is_null());
    assert!(
        report["blockers"]
            .to_string()
            .contains("No referenced test suite covers stage \\\"post_merge\\\"")
    );
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
