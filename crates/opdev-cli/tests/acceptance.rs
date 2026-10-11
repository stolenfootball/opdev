//! Requirement-to-assertion evidence is independent of a green suite or policy.

use opdev_core::{Outcome, embedded_catalog};
use opdev_project::{
    AcceptanceCondition, AcceptanceEvidence, AcceptanceMethod, AcceptanceReview, AcceptanceScope,
    AcceptanceVerification, ChangeEvidence, CommandSpec, EVIDENCE_PATH, EvidenceLedger,
    MANIFEST_PATH, TestStage, TestSuite, TrackedEvidence, discover, staged_fingerprint,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn git(root: &Path, args: &[&str]) -> Result {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn reference(root: &Path, path: &str, excerpt: &str) -> Result<TrackedEvidence> {
    let blob = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "blob", &format!(":{path}")])
        .output()?;
    assert!(blob.status.success());
    Ok(TrackedEvidence {
        path: path.into(),
        sha256: format!("{:x}", Sha256::digest(blob.stdout)),
        excerpt: excerpt.into(),
    })
}

fn project() -> Result<(tempfile::TempDir, EvidenceLedger)> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    git(root, &["init", "--quiet"])?;
    fs::create_dir(root.join(".opdev"))?;
    fs::write(
        root.join("requirements.md"),
        "R1: Preserve caller order and return at most the requested count.\n",
    )?;
    fs::write(
        root.join("tests.py"),
        "items = ['c', 'a', 'b']\nassert items[:2] == ['c', 'a']\n",
    )?;
    let mut manifest = discover(root)?.manifest;
    manifest.commands.insert(
        "acceptance".into(),
        CommandSpec {
            argv: vec![
                if cfg!(windows) { "python" } else { "python3" }.into(),
                "tests.py".into(),
            ],
            working_directory: None,
            timeout_seconds: Some(30),
        },
    );
    manifest.testing.suites = vec![TestSuite {
        id: "acceptance".into(),
        command: "acceptance".into(),
        stages: vec![
            TestStage::Local,
            TestStage::PreMerge,
            TestStage::PostMerge,
            TestStage::Delivery,
        ],
    }];
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    git(root, &["add", "."])?;
    let acceptance = AcceptanceEvidence {
        organization_controls: None,
        policy_controls: None,
        requirements: None,
        safeguards: None,
        scope: AcceptanceScope::Behavioral,
        rationale: "Only the agreed order/count behavior changes; invalid inputs are excluded.".into(),
        conditions: vec![AcceptanceCondition { id: "R1".into(),
            statement: "Preserve caller order and return no more than the requested count".into(),
            authority: "requirements.md".into(), source: opdev_project::RequirementSource::Tracked(reference(root, "requirements.md", "R1: Preserve caller order")?) }],
        verifications: vec![AcceptanceVerification { condition: "R1".into(), stages: None, method: AcceptanceMethod::Automated,
            target: reference(root, "tests.py", "assert items[:2] == ['c', 'a']")?,
            assertion: "Exact sequence checks order and count together".into(),
            discriminating_case: "Three items c,a,b with limit 2 must produce c,a, not a,b,c".into(),
            suite: Some("acceptance".into()), automation_limitation: None, outcome: Outcome::Passed }],
        review: AcceptanceReview { outcome: Outcome::Passed, reviewer: "synthetic reviewer".into(),
            reference: "fixture review R1".into(), rationale: "R1 is the complete fixture scope; exact sequence is derived from the requirement, not code.".into(), subject_sha256: String::new() },
    };
    let mut ledger = EvidenceLedger {
        schema: 2,
        project: vec![],
        changes: vec![ChangeEvidence {
            fingerprint: staged_fingerprint(root)?,
            work: "synthetic acceptance fixture".into(),
            assertions: vec![],
            acceptance: Some(acceptance),
        }],
    };
    bind(&mut ledger)?;
    save(root, &ledger)?;
    Ok((temp, ledger))
}

fn bind(ledger: &mut EvidenceLedger) -> Result {
    let change = &mut ledger.changes[0];
    let acceptance = change.acceptance.as_mut().ok_or("acceptance")?;
    acceptance.review.subject_sha256 = acceptance.digest(&change.fingerprint, &change.work)?;
    Ok(())
}

fn save(root: &Path, ledger: &EvidenceLedger) -> Result {
    fs::write(root.join(EVIDENCE_PATH), ledger.to_yaml()?)?;
    Ok(())
}

fn check(root: &Path, extra: &[&str]) -> Result<Value> {
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["check", "--ci", "--format", "json", "--root"])
        .arg(root)
        .args(extra)
        .output()?;
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn outcome(report: &Value, id: &str) -> Result<String> {
    Ok(report["rules"]
        .as_array()
        .ok_or("rules")?
        .iter()
        .find(|rule| rule["rule_id"] == id)
        .ok_or("rule")?["outcome"]
        .as_str()
        .ok_or("outcome")?
        .into())
}

fn recovery_capabilities() -> opdev_project::SafeguardPolicy {
    use opdev_project::{Capability, CapabilityFact, CapabilityState, SafeguardPolicy};
    SafeguardPolicy {
        version: 1,
        review_reference: "fixture".into(),
        capabilities: Capability::ALL
            .into_iter()
            .map(|c| {
                (
                    c,
                    CapabilityFact {
                        state: if c == Capability::Distribution {
                            CapabilityState::Present
                        } else {
                            CapabilityState::Absent
                        },
                        rationale: "Isolated control-plumbing fixture; not real product adoption"
                            .into(),
                        authority: "implementation".into(),
                    },
                )
            })
            .collect(),
    }
}

fn recovery_project() -> Result<(tempfile::TempDir, EvidenceLedger)> {
    use opdev_project::{
        Capability, CapabilityImpact, CiProvider, DeliveryStatus, Environment, Impact,
        PolicyControlReview, ProjectManifest, RecoveryStrategy, SafeguardReview,
    };
    let (temp, mut ledger) = project()?;
    let root = temp.path();
    let mut manifest = ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    manifest.schema = 4;
    manifest.assurance.profiles.clear();
    manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "2".into(),
        minimumcd: Some("1".into()),
        review_reference: "synthetic fixture selection".into(),
        maintenance_branches: vec![],
    });
    manifest.assurance.safeguards = Some(recovery_capabilities());
    manifest.project.ci.provider = CiProvider::Gitlab;
    manifest.delivery.status = DeliveryStatus::Configured;
    manifest.delivery.recovery.strategy = RecoveryStrategy::Restore;
    manifest.delivery.environments = vec![Environment {
        name: "isolated local fixture".into(),
        production_like: true,
    }];
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    fs::write(
        root.join("requirements.md"),
        "R1: Interrupted replacement preserves the last committed file; no external effects.\n",
    )?;
    fs::write(
        root.join("tests.py"),
        "import tempfile\nfrom pathlib import Path\nwith tempfile.TemporaryDirectory() as d:\n    current = Path(d) / 'current'\n    candidate = Path(d) / 'candidate'\n    current.write_bytes(b'acknowledged-write')\n    candidate.write_bytes(b'incomplete-transition')\n    candidate.unlink()  # recovery after interruption before replacement\n    assert current.read_bytes() == b'acknowledged-write'\n    assert not candidate.exists()\n",
    )?;
    git(root, &["add", "."])?;
    ledger.changes[0].fingerprint = staged_fingerprint(root)?;
    ledger.changes[0].assertions = vec![opdev_project::EvidenceAssertion {
        rule_id: "OPDEV-RECOVERY-001".parse()?, outcome: Outcome::Passed,
        summary: "Synthetic exact-control review: interruption before replacement restores committed bytes by discarding incomplete candidate; no external effects or unacknowledged-write guarantee. Tests routing, not production recovery adequacy.".into(),
        evidence: vec![opdev_core::Evidence {
            kind: "synthetic_review".into(), summary: "Inspect the actual restore condition and assertion; execution is separate".into(),
            location: Some("requirements.md and tests.py".into()),
        }],
    }];
    let a = ledger.changes[0].acceptance.as_mut().ok_or("acceptance")?;
    a.rationale =
        "Isolated recovery/control fixture; real required CI and other controls remain unverified."
            .into();
    a.conditions[0].statement =
        "Interrupted replacement preserves acknowledged committed bytes".into();
    a.conditions[0].source = opdev_project::RequirementSource::Tracked(reference(
        root,
        "requirements.md",
        "R1: Interrupted replacement",
    )?);
    a.verifications[0].target = reference(
        root,
        "tests.py",
        "assert current.read_bytes() == b'acknowledged-write'",
    )?;
    a.verifications[0].assertion = "Exact committed bytes survive interrupted replacement".into();
    a.verifications[0].discriminating_case =
        "An incomplete candidate must not replace the acknowledged bytes".into();
    a.safeguards = Some(SafeguardReview {
        version: 1,
        impacts: [(
            Capability::Distribution,
            CapabilityImpact {
                impact: Impact::Unaffected,
                rationale: "Fixture control evaluation changes no installation/update path".into(),
            },
        )]
        .into(),
        objectives: std::collections::BTreeMap::default(),
    });
    a.policy_controls = Some(PolicyControlReview {
        version: "2".into(),
        definition_sha256: opdev_core::resolve_engineering_policy("2")?.definition_sha256,
        stage: TestStage::Delivery,
        bindings: [("OPDEV-RECOVERY-001".into(), vec!["R1".into()])].into(),
    });
    bind(&mut ledger)?;
    save(root, &ledger)?;
    Ok((temp, ledger))
}

#[test]
fn policy_two_recovery_uses_real_execution_not_a_strategy_or_another_stage() -> Result {
    let (temp, mut ledger) = recovery_project()?;
    let root = temp.path();
    let manifest = opdev_project::ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    let absent = serde_json::to_value(opdev_engine::evaluate(
        root,
        &manifest,
        opdev_engine::CheckOptions {
            test_stage: TestStage::Delivery,
            extension_stage: opdev_project::ExtensionStage::Deliver,
            execute_checks: false,
        },
    )?)?;
    assert_eq!(outcome(&absent, "OPDEV-RECOVERY-001")?, "unverified");
    let executed = check(root, &["--delivery"])?;
    assert_eq!(outcome(&executed, "OPDEV-RECOVERY-001")?, "passed");
    assert_ne!(
        outcome(&executed, "MCD-RECOVERY-002")?,
        "passed",
        "safe restoration is not literal rollback"
    );
    assert_eq!(
        outcome(&check(root, &[])?, "OPDEV-RECOVERY-001")?,
        "unverified",
        "delivery review cannot qualify pre-merge"
    );
    ledger.changes[0]
        .acceptance
        .as_mut()
        .ok_or("acceptance")?
        .policy_controls = None;
    bind(&mut ledger)?;
    save(root, &ledger)?;
    assert_eq!(
        outcome(&check(root, &["--delivery"])?, "OPDEV-RECOVERY-001")?,
        "unverified",
        "green execution still needs reviewed control links"
    );
    Ok(())
}

#[test]
fn configured_ci_cannot_qualify_unexecuted_failed_or_unreviewed_tests() -> Result {
    for mode in ["passed", "not_run", "failed", "stale", "pending"] {
        let (temp, mut ledger) = project()?;
        let root = temp.path();
        let path = root.join(MANIFEST_PATH);
        let mut manifest = opdev_project::ProjectManifest::load(&path)?;
        manifest.project.ci.provider = opdev_project::CiProvider::Gitlab;
        fs::write(path, manifest.to_yaml()?)?;
        opdev_ci::write_new(
            opdev_ci::adapter_for(opdev_project::CiProvider::Gitlab)?,
            root,
            &opdev_ci::TemplateContext {
                opdev_version: "0.4.0".into(),
                trunk: "main".into(),
                job_image: Some("python:3.13".into()),
            },
        )?;
        if mode == "failed" {
            fs::write(
                root.join("tests.py"),
                "items = ['c', 'a', 'b']\nassert items[:2] == ['c', 'a']\nraise AssertionError('visible failure')\n",
            )?;
        }
        git(root, &["add", "."])?;
        ledger.changes[0].fingerprint = staged_fingerprint(root)?;
        let acceptance = ledger.changes[0].acceptance.as_mut().ok_or("acceptance")?;
        acceptance.verifications[0].target =
            reference(root, "tests.py", "assert items[:2] == ['c', 'a']")?;
        if mode == "pending" {
            acceptance.review.outcome = Outcome::Unverified;
        }
        bind(&mut ledger)?;
        save(root, &ledger)?;
        if mode == "stale" {
            fs::write(root.join("new-behavior.py"), "changed = True\n")?;
            git(root, &["add", "."])?;
        }
        let flags = if mode == "not_run" {
            vec!["--no-exec"]
        } else {
            vec![]
        };
        let report = check(root, &flags)?;
        let expected = match mode {
            "passed" => "passed",
            "failed" => "failed",
            _ => "unverified",
        };
        assert_eq!(
            outcome(&report, "MCD-TEST-001")?,
            expected,
            "{mode}: {report}"
        );
        if mode == "pending" {
            let rule = report["rules"]
                .as_array()
                .ok_or("rules")?
                .iter()
                .find(|rule| rule["rule_id"] == "MCD-TEST-001")
                .ok_or("rule")?;
            assert!(
                rule["diagnostic"]
                    .as_str()
                    .ok_or("diagnostic")?
                    .contains("Repeating tests alone cannot repair a review gap")
            );
        }
        assert_ne!(
            outcome(&report, "MCD-TEST-002")?,
            "passed",
            "pre-merge does not prove post-merge"
        );
        assert_ne!(
            outcome(&report, "MCD-CI-001")?,
            "passed",
            "a template does not prove CI-exclusive delivery"
        );
    }
    Ok(())
}

fn selected_organization_outcome(report: &Value) -> Result<String> {
    Ok(report["checks"]
        .as_array()
        .ok_or("checks")?
        .iter()
        .find(|c| c["id"] == "opdev-organization:ORG-EXAMPLE-001")
        .ok_or("control")?["outcome"]
        .as_str()
        .ok_or("outcome")?
        .into())
}

fn organization_project() -> Result<(
    tempfile::TempDir,
    EvidenceLedger,
    opdev_project::ProjectManifest,
)> {
    use opdev_project::organization::{
        Applicability, OrganizationControl, PolicyPack, PolicySelection, VerificationRequirement,
    };
    let (temp, mut ledger) = recovery_project()?;
    let root = temp.path();
    let path = root.join(MANIFEST_PATH);
    let mut manifest = opdev_project::ProjectManifest::load(&path)?;
    manifest.layout = Some(opdev_project::LayoutPolicy {
        version: 3,
        review_reference: "fixture".into(),
    });
    let pack = PolicyPack {
        schema: 1,
        id: "example".into(),
        version: "1".into(),
        title: "Recovery requirement".into(),
        source: "synthetic existing organization requirement".into(),
        parameters: std::collections::BTreeMap::new(),
        controls: vec![OrganizationControl {
            id: "ORG-EXAMPLE-001".into(),
            statement: "Recover committed writes".into(),
            source: "requirements.md".into(),
            stages: vec![TestStage::Delivery],
            applicability: Applicability::Always,
            verification: VerificationRequirement::Automated,
        }],
    };
    manifest.assurance.organization_policies = vec![PolicySelection {
        id: pack.id.clone(),
        version: pack.version.clone(),
        definition_sha256: pack.definition_sha256()?,
        parameters: std::collections::BTreeMap::new(),
    }];
    fs::create_dir(root.join(".opdev/policies"))?;
    fs::write(
        root.join(".opdev/policies/example.json"),
        serde_json::to_vec(&pack)?,
    )?;
    fs::write(&path, manifest.to_yaml()?)?;
    git(root, &["add", "."])?;
    let snapshot = opdev_project::organization::load_selected(
        root,
        &manifest.assurance.organization_policies,
    )?;
    ledger.changes[0].fingerprint = staged_fingerprint(root)?;
    ledger.changes[0]
        .acceptance
        .as_mut()
        .ok_or("acceptance")?
        .organization_controls = Some(opdev_project::OrganizationControlReview {
        resolution_sha256: snapshot.resolution_sha256,
        stage: TestStage::Delivery,
        bindings: vec![opdev_project::OrganizationControlBinding {
            control: "ORG-EXAMPLE-001".into(),
            conditions: vec!["R1".into()],
            extensions: vec![],
        }],
    });
    bind(&mut ledger)?;
    save(root, &ledger)?;
    Ok((temp, ledger, manifest))
}

#[test]
fn organization_controls_reuse_real_current_execution_and_reject_missing_links() -> Result {
    let (temp, mut ledger, manifest) = organization_project()?;
    let root = temp.path();
    let executed = check(root, &["--delivery"])?;
    assert_eq!(
        selected_organization_outcome(&executed)?,
        "passed",
        "{executed}"
    );
    assert_eq!(
        executed["checks"]
            .as_array()
            .ok_or("checks")?
            .iter()
            .filter(|c| c["kind"] == "suite")
            .count(),
        1
    );
    let not_run = serde_json::to_value(opdev_engine::evaluate(
        root,
        &manifest,
        opdev_engine::CheckOptions {
            test_stage: TestStage::Delivery,
            extension_stage: opdev_project::ExtensionStage::Deliver,
            execute_checks: false,
        },
    )?)?;
    assert_eq!(selected_organization_outcome(&not_run)?, "unverified");
    ledger.changes[0]
        .acceptance
        .as_mut()
        .ok_or("acceptance")?
        .organization_controls
        .as_mut()
        .ok_or("controls")?
        .bindings[0]
        .conditions
        .clear();
    bind(&mut ledger)?;
    save(root, &ledger)?;
    assert_eq!(
        selected_organization_outcome(&check(root, &["--delivery"])?)?,
        "unverified"
    );
    Ok(())
}

#[test]
fn missing_suite_diagnostics_distinguish_absence_from_no_execution() -> Result {
    let (temp, mut ledger) = project()?;
    let root = temp.path();
    let report = check(root, &["--no-exec"])?;
    let rule = report["rules"]
        .as_array()
        .ok_or("rules")?
        .iter()
        .find(|rule| rule["rule_id"] == "OPDEV-TEST-002")
        .ok_or("rule")?;
    assert!(
        rule["diagnostic"]
            .as_str()
            .ok_or("diagnostic")?
            .contains("suite did not execute")
    );
    ledger.changes[0]
        .acceptance
        .as_mut()
        .ok_or("acceptance")?
        .verifications[0]
        .suite = Some("undeclared".into());
    bind(&mut ledger)?;
    save(root, &ledger)?;
    let report = check(root, &[])?;
    let rule = report["rules"]
        .as_array()
        .ok_or("rules")?
        .iter()
        .find(|rule| rule["rule_id"] == "OPDEV-TEST-002")
        .ok_or("rule")?;
    let diagnostic = rule["diagnostic"].as_str().ok_or("diagnostic")?;
    assert!(diagnostic.contains("undeclared") && diagnostic.contains("suite is not declared"));
    assert_ne!(rule["outcome"], "passed");
    Ok(())
}

#[test]
fn source_and_stage_blockers_explain_the_required_repair() -> Result {
    let (temp, mut ledger) = project()?;
    let root = temp.path();
    fs::write(root.join("downloaded-package.zip"), "release bytes")?;
    let report = check(root, &["--delivery"])?;
    for id in ["OPDEV-WORK-001", "OPDEV-TEST-002", "OPDEV-TEST-003"] {
        let rule = report["rules"]
            .as_array()
            .ok_or("rules")?
            .iter()
            .find(|rule| rule["rule_id"] == id)
            .ok_or("rule")?;
        assert!(
            rule["diagnostic"]
                .as_str()
                .unwrap_or_default()
                .contains("downloaded-package.zip"),
            "{id}: {rule}"
        );
        assert_ne!(rule["outcome"], "passed");
    }
    fs::remove_file(root.join("downloaded-package.zip"))?;
    let mut manifest = opdev_project::ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    manifest.testing.suites[0]
        .stages
        .retain(|stage| *stage != TestStage::Delivery);
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    git(root, &["add", MANIFEST_PATH])?;
    ledger.changes[0].fingerprint = staged_fingerprint(root)?;
    bind(&mut ledger)?;
    save(root, &ledger)?;
    let report = check(root, &["--delivery"])?;
    let rule = report["rules"]
        .as_array()
        .ok_or("rules")?
        .iter()
        .find(|rule| rule["rule_id"] == "OPDEV-TEST-002")
        .ok_or("rule")?;
    let diagnostic = rule["diagnostic"].as_str().ok_or("diagnostic")?;
    assert!(
        diagnostic.contains("R1")
            && diagnostic.contains("acceptance")
            && diagnostic.contains("delivery"),
        "{diagnostic}"
    );
    assert_eq!(rule["outcome"], "unverified");
    assert!(report["checks"].as_array().ok_or("checks")?.is_empty());
    Ok(())
}

#[test]
fn current_review_and_actual_suite_execution_are_both_required() -> Result {
    let (temp, ledger) = project()?;
    let root = temp.path();
    let report = check(root, &[])?;
    assert_eq!(outcome(&report, "OPDEV-TEST-002")?, "passed");
    assert_eq!(outcome(&report, "OPDEV-TEST-003")?, "passed");
    assert_eq!(report["checks"][0]["outcome"], "passed");
    assert_eq!(
        outcome(&check(root, &["--no-exec"])?, "OPDEV-TEST-002")?,
        "unverified"
    );
    let digest = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["evidence", "acceptance-digest", "--root"])
        .arg(root)
        .output()?;
    assert!(digest.status.success());
    assert_eq!(
        String::from_utf8(digest.stdout)?.trim(),
        ledger.changes[0]
            .acceptance
            .as_ref()
            .ok_or("acceptance")?
            .review
            .subject_sha256
    );
    Ok(())
}

#[test]
fn green_suite_cannot_hide_missing_pending_or_contradicted_mappings() -> Result {
    let (temp, ledger) = project()?;
    for (case, expected) in [
        ("missing", "unverified"),
        ("pending", "unverified"),
        ("contradiction", "failed"),
        ("empty", "unverified"),
        ("unknown-suite", "unverified"),
        ("review-failed", "failed"),
        ("stale-source", "unverified"),
    ] {
        let mut candidate = ledger.clone();
        let acceptance = candidate.changes[0]
            .acceptance
            .as_mut()
            .ok_or("acceptance")?;
        match case {
            "missing" => acceptance.verifications.clear(),
            "pending" => acceptance.verifications[0].outcome = Outcome::Unverified,
            "contradiction" => acceptance.verifications[0].outcome = Outcome::Failed,
            "empty" => {
                acceptance.conditions.clear();
                acceptance.verifications.clear();
            }
            "unknown-suite" => acceptance.verifications[0].suite = Some("absent".into()),
            "review-failed" => acceptance.review.outcome = Outcome::Failed,
            _ => {
                if let opdev_project::RequirementSource::Tracked(source) =
                    &mut acceptance.conditions[0].source
                {
                    source.sha256 = "0".repeat(64);
                }
            }
        }
        bind(&mut candidate)?;
        save(temp.path(), &candidate)?;
        let report = check(temp.path(), &[])?;
        assert_eq!(report["checks"][0]["outcome"], "passed");
        assert_eq!(outcome(&report, "OPDEV-TEST-002")?, expected, "{case}");
        assert_eq!(
            report["gates"]
                .as_array()
                .ok_or("gates")?
                .iter()
                .find(|gate| gate["gate"] == "integration")
                .ok_or("integration")?["verdict"],
            "blocked"
        );
    }
    Ok(())
}

#[test]
fn changed_inventory_mapping_work_or_source_invalidates_review() -> Result {
    let (temp, ledger) = project()?;
    for case in ["condition", "mapping", "work", "review"] {
        let mut changed = ledger.clone();
        let change = &mut changed.changes[0];
        let acceptance = change.acceptance.as_mut().ok_or("acceptance")?;
        match case {
            "condition" => acceptance.conditions[0].statement.push_str(" changed"),
            "mapping" => acceptance.verifications[0].assertion.push_str(" changed"),
            "work" => change.work.push_str(" changed"),
            _ => acceptance.review.subject_sha256 = "0".repeat(64),
        }
        save(temp.path(), &changed)?;
        assert_eq!(
            outcome(&check(temp.path(), &[])?, "OPDEV-TEST-002")?,
            "unverified",
            "{case}"
        );
    }
    save(temp.path(), &ledger)?;
    fs::write(
        temp.path().join("requirements.md"),
        "New accepted requirement\n",
    )?;
    assert_eq!(
        outcome(&check(temp.path(), &[])?, "OPDEV-TEST-002")?,
        "unverified"
    );
    git(temp.path(), &["add", "requirements.md"])?;
    assert_eq!(
        outcome(&check(temp.path(), &[])?, "OPDEV-TEST-002")?,
        "unverified"
    );
    Ok(())
}

#[test]
fn legacy_and_generic_pass_assertions_do_not_satisfy_acceptance() -> Result {
    let (temp, mut ledger) = project()?;
    ledger.schema = 1;
    ledger.changes[0].acceptance = None;
    for id in ["OPDEV-TEST-002", "OPDEV-TEST-003"] {
        let assertion = opdev_project::EvidenceAssertion {
            rule_id: id.parse()?,
            outcome: Outcome::Passed,
            summary: "Policy requires tests".into(),
            evidence: vec![opdev_core::Evidence {
                kind: "policy".into(),
                summary: "Policy declares tests".into(),
                location: Some("requirements.md".into()),
            }],
        };
        ledger.project.push(assertion.clone());
        ledger.changes[0].assertions.push(assertion);
    }
    save(temp.path(), &ledger)?;
    let report = check(temp.path(), &[])?;
    assert_eq!(report["checks"][0]["outcome"], "passed");
    assert_eq!(outcome(&report, "OPDEV-TEST-002")?, "unverified");
    assert_eq!(outcome(&report, "OPDEV-TEST-003")?, "unverified");
    Ok(())
}

#[test]
fn non_code_review_requires_specific_evidence_and_limits() -> Result {
    let (temp, mut ledger) = project()?;
    let acceptance = ledger.changes[0].acceptance.as_mut().ok_or("acceptance")?;
    acceptance.scope = AcceptanceScope::NonBehavioral;
    acceptance.rationale = "Synthetic editorial-only evidence fixture".into();
    acceptance.verifications[0].method = AcceptanceMethod::Review;
    acceptance.verifications[0].suite = None;
    acceptance.verifications[0].automation_limitation =
        Some("Human wording review, not runtime qualification".into());
    bind(&mut ledger)?;
    save(temp.path(), &ledger)?;
    let report = check(temp.path(), &["--no-exec"])?;
    assert_eq!(outcome(&report, "OPDEV-TEST-002")?, "passed");
    assert_eq!(outcome(&report, "OPDEV-TEST-003")?, "not_applicable");
    Ok(())
}

#[test]
fn invalid_shapes_and_untrusted_paths_fail_before_suite_execution() -> Result {
    let (temp, ledger) = project()?;
    for case in [
        "duplicate",
        "unknown",
        "path",
        "ledger",
        "v1",
        "field",
        "ignored",
    ] {
        let mut candidate = serde_json::to_value(&ledger)?;
        let acceptance = &mut candidate["changes"][0]["acceptance"];
        match case {
            "duplicate" => {
                let duplicate = acceptance["conditions"][0].clone();
                acceptance["conditions"]
                    .as_array_mut()
                    .ok_or("conditions")?
                    .push(duplicate);
            }
            "unknown" => acceptance["verifications"][0]["condition"] = "unknown".into(),
            "path" => acceptance["verifications"][0]["target"]["path"] = "../outside".into(),
            "ledger" => acceptance["conditions"][0]["source"]["path"] = EVIDENCE_PATH.into(),
            "v1" => candidate["schema"] = 1.into(),
            "field" => acceptance["invented"] = true.into(),
            _ => acceptance["verifications"][0]["outcome"] = "not_applicable".into(),
        }
        fs::write(
            temp.path().join(EVIDENCE_PATH),
            serde_json::to_vec(&candidate)?,
        )?;
        assert!(
            EvidenceLedger::load_optional(temp.path(), &embedded_catalog()?).is_err(),
            "{case}"
        );
    }
    Ok(())
}

#[test]
fn semantic_truth_is_not_established_by_a_mechanically_valid_record() -> Result {
    let (temp, mut ledger) = project()?;
    // Deliberately false interpretation, with real excerpts and an asserted review.
    // This must never be advertised as machine-verification of semantic truth.
    ledger.changes[0]
        .acceptance
        .as_mut()
        .ok_or("acceptance")?
        .conditions[0]
        .statement = "This also proves the entire application correct".into();
    bind(&mut ledger)?;
    save(temp.path(), &ledger)?;
    let report = check(temp.path(), &[])?;
    assert_eq!(outcome(&report, "OPDEV-TEST-002")?, "passed");
    let diagnostic = report["rules"]
        .as_array()
        .ok_or("rules")?
        .iter()
        .find(|rule| rule["rule_id"] == "OPDEV-TEST-002")
        .ok_or("rule")?["diagnostic"]
        .as_str()
        .ok_or("diagnostic")?;
    assert!(diagnostic.contains("reviewer claims"));
    Ok(())
}

#[test]
fn failing_unavailable_skipped_or_source_mutating_suites_do_not_qualify() -> Result {
    for (mode, expected) in [
        ("fail", "failed"),
        ("error", "error"),
        ("skipped", "unverified"),
        ("mutate", "unverified"),
    ] {
        let (temp, mut ledger) = project()?;
        let root = temp.path();
        let mut manifest = opdev_project::ProjectManifest::load(&root.join(MANIFEST_PATH))?;
        match mode {
            "fail" => fs::write(
                root.join("tests.py"),
                "items = ['c', 'a', 'b']\nassert items[:2] == ['c', 'a']\nassert False, 'known failure'\n",
            )?,
            "error" => {
                manifest
                    .commands
                    .get_mut("acceptance")
                    .ok_or("command")?
                    .argv[0] = "opdev-nonexistent-acceptance-runner".into();
            }
            "skipped" => manifest.testing.suites[0].stages = vec![TestStage::PostMerge],
            _ => fs::write(
                root.join("tests.py"),
                "items = ['c', 'a', 'b']\nassert items[:2] == ['c', 'a']\nfrom pathlib import Path\nPath('requirements.md').write_text('changed during verification')\n",
            )?,
        }
        fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
        git(root, &["add", "."])?;
        ledger.changes[0].fingerprint = staged_fingerprint(root)?;
        ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or("acceptance")?
            .verifications[0]
            .target = reference(root, "tests.py", "assert items[:2] == ['c', 'a']")?;
        bind(&mut ledger)?;
        save(root, &ledger)?;
        let report = check(root, &[])?;
        assert_eq!(outcome(&report, "OPDEV-TEST-002")?, expected, "{mode}");
        assert_eq!(outcome(&report, "OPDEV-TEST-003")?, expected, "{mode}");
    }
    Ok(())
}

#[test]
fn bootstrap_is_unresolved_and_compact_views_preserve_typed_acceptance() -> Result {
    let (temp, ledger) = project()?;
    let root = temp.path();
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args([
            "--experimental-compact",
            "evidence",
            "show",
            "--current",
            "--rule",
            "OPDEV-TEST-002",
            "--root",
        ])
        .arg(root)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let view: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(view["schema"], 2);
    assert!(view["missing_requested_rule"].is_null());
    assert_eq!(
        view["change"]["acceptance"],
        serde_json::to_value(&ledger.changes[0].acceptance)?
    );
    fs::remove_file(root.join(EVIDENCE_PATH))?;
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["evidence", "bootstrap", "--root"])
        .arg(root)
        .output()?;
    assert!(output.status.success());
    let answers: opdev_project::EvidenceBootstrap = serde_saphyr::from_slice(&output.stdout)?;
    assert_eq!(answers.schema, 2);
    assert_eq!(
        answers.change.acceptance.ok_or("template")?.review.outcome,
        Outcome::Unverified
    );
    assert!(!answers.change.decisions.contains_key("OPDEV-TEST-002"));
    assert!(!answers.change.decisions.contains_key("OPDEV-TEST-003"));
    assert!(!root.join(EVIDENCE_PATH).exists());
    Ok(())
}

#[test]
fn wrong_green_assertion_then_meaningful_red_green_is_observed() -> Result {
    let (temp, mut ledger) = project()?;
    let root = temp.path();
    fs::write(root.join(".gitignore"), "__pycache__/\n")?;
    fs::write(
        root.join("tasktray.py"),
        "def select(items, limit):\n    return sorted(items)[:limit + 1]\n",
    )?;
    // First the test agrees with incorrect code, not the accepted requirement.
    // Semantic review supplies the contradiction; the gate must preserve it.
    for (expected_list, mapping_outcome, check_outcome, rule_outcome) in [
        ("['a', 'b', 'c']", Outcome::Failed, "passed", "failed"),
        ("['c', 'a']", Outcome::Passed, "failed", "failed"),
        ("['c', 'a']", Outcome::Passed, "passed", "passed"),
    ] {
        if rule_outcome == "passed" {
            fs::write(
                root.join("tasktray.py"),
                "def select(items, limit):\n    return items[:limit]\n",
            )?;
        }
        let assertion = format!("assert select(['c', 'a', 'b'], 2) == {expected_list}");
        fs::write(
            root.join("tests.py"),
            format!("from tasktray import select\n{assertion}\n"),
        )?;
        git(root, &["add", "."])?;
        ledger.changes[0].fingerprint = staged_fingerprint(root)?;
        let mapping = &mut ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or("acceptance")?
            .verifications[0];
        mapping.target = reference(root, "tests.py", &assertion)?;
        mapping.outcome = mapping_outcome;
        mapping.assertion = if mapping_outcome == Outcome::Failed {
            "The actual assertion expects sorted three-item output, contradicting R1".into()
        } else {
            "The assertion calls the implementation and requires the first two items in caller order".into()
        };
        bind(&mut ledger)?;
        save(root, &ledger)?;
        let report = check(root, &[])?;
        assert_eq!(report["checks"][0]["outcome"], check_outcome);
        assert_eq!(outcome(&report, "OPDEV-TEST-002")?, rule_outcome);
        assert_eq!(outcome(&report, "OPDEV-TEST-003")?, rule_outcome);
    }
    Ok(())
}

#[test]
fn strengthened_assertions_need_fresh_review_not_byte_identical_tests() -> Result {
    let (temp, mut ledger) = project()?;
    let root = temp.path();
    fs::write(root.join(".gitignore"), "__pycache__/\n")?;
    fs::write(
        root.join("tasktray.py"),
        "def select(items, limit):\n    return items[:limit]\n",
    )?;
    let original = "assert select(['c', 'a', 'b'], 2) == ['c', 'a']";
    // Same accepted order/count contract; additional boundary guarantees are
    // conjoined with the original assertion instead of preserving its bytes.
    let stronger = "assert (select(['c', 'a', 'b'], 2) == ['c', 'a'] and select(['c', 'a'], 0) == [] and select([], 3) == [] and select(['c', 'a'], 9) == ['c', 'a'])";
    for assertion in [original, stronger] {
        fs::write(
            root.join("tests.py"),
            format!("from tasktray import select\n{assertion}\n"),
        )?;
        git(root, &["add", "."])?;
        let stale = check(root, &[])?;
        assert_eq!(outcome(&stale, "OPDEV-TEST-002")?, "unverified");
        ledger.changes[0].fingerprint = staged_fingerprint(root)?;
        let acceptance = ledger.changes[0].acceptance.as_mut().ok_or("acceptance")?;
        acceptance.verifications[0].target = reference(root, "tests.py", assertion)?;
        acceptance.verifications[0].assertion = "Exact caller sequence and count; added empty/zero/large-limit boundaries retain the original guarantees".into();
        acceptance.review.rationale = "Synthetic review of actual conjoined assertions; no removed guarantee or changed accepted result".into();
        bind(&mut ledger)?;
        save(root, &ledger)?;
        let report = check(root, &[])?;
        assert_eq!(outcome(&report, "OPDEV-TEST-002")?, "passed");
        assert_eq!(outcome(&report, "OPDEV-TEST-003")?, "passed");
        assert_eq!(
            report["checks"].as_array().ok_or("checks")?.len(),
            1,
            "no compulsory extra test-strength tool"
        );
    }
    // A plausible wrong implementation must still fail the strengthened test.
    fs::write(
        root.join("tasktray.py"),
        "def select(items, limit):\n    return sorted(items)[:limit]\n",
    )?;
    git(root, &["add", "."])?;
    ledger.changes[0].fingerprint = staged_fingerprint(root)?;
    bind(&mut ledger)?;
    save(root, &ledger)?;
    let report = check(root, &[])?;
    assert_eq!(report["checks"][0]["outcome"], "failed");
    assert_eq!(outcome(&report, "OPDEV-TEST-003")?, "failed");
    Ok(())
}

fn staged_project() -> Result<(tempfile::TempDir, EvidenceLedger)> {
    let (temp, mut ledger) = project()?;
    let root = temp.path();
    fs::write(root.join(".gitignore"), "__pycache__/\n.counts\n")?;
    fs::write(
        root.join("tasktray.py"),
        "def select(items, limit):\n    return items[:limit]\n",
    )?;
    fs::write(
        root.join("tests.py"),
        "from tasktray import select\nwith open('.counts', 'a') as f: f.write('pre\\n')\nassert select(['c', 'a', 'b'], 2) == ['c', 'a']\n",
    )?;
    fs::write(
        root.join("consumer.py"),
        "from tasktray import select\nwith open('.counts', 'a') as f: f.write('post\\n')\nassert select(['c', 'a'], 1) == ['c']\n",
    )?;
    let mut manifest = discover(root)?.manifest;
    manifest.schema = 3;
    manifest
        .assurance
        .profiles
        .retain(|p| p.name != "opdev-core");
    manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "1".into(),
        minimumcd: None,
        review_reference: "synthetic-stage-review".into(),
        maintenance_branches: vec![],
    });
    manifest.testing.suites[0].stages = vec![TestStage::PreMerge];
    let mut command = manifest.commands["acceptance"].clone();
    command.argv[1] = "consumer.py".into();
    manifest.commands.insert("consumer".into(), command);
    manifest.testing.suites.push(TestSuite {
        id: "consumer".into(),
        command: "consumer".into(),
        stages: vec![TestStage::PostMerge],
    });
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    git(root, &["add", "."])?;
    ledger.changes[0].fingerprint = staged_fingerprint(root)?;
    let acceptance = ledger.changes[0].acceptance.as_mut().ok_or("acceptance")?;
    let pre = &mut acceptance.verifications[0];
    pre.stages = Some(vec![TestStage::PreMerge]);
    pre.target = reference(
        root,
        "tests.py",
        "assert select(['c', 'a', 'b'], 2) == ['c', 'a']",
    )?;
    let mut post = pre.clone();
    post.stages = Some(vec![TestStage::PostMerge]);
    post.suite = Some("consumer".into());
    post.target = reference(root, "consumer.py", "assert select(['c', 'a'], 1) == ['c']")?;
    post.assertion =
        "Actual consumer import preserves caller order and the single-item limit".into();
    post.discriminating_case = "c,a limited to one must yield c, not sorted a or two items".into();
    acceptance.verifications.push(post);
    bind(&mut ledger)?;
    save(root, &ledger)?;
    Ok((temp, ledger))
}

#[test]
fn distinct_stage_checks_execute_once_and_missing_execution_never_reuses_another_stage() -> Result {
    let (temp, _) = staged_project()?;
    let root = temp.path();
    let pre = check(root, &[])?;
    assert_eq!(outcome(&pre, "OPDEV-TEST-002")?, "passed");
    assert_eq!(pre["checks"].as_array().ok_or("checks")?.len(), 1);
    assert_eq!(pre["checks"][0]["id"], "acceptance");
    assert_eq!(
        fs::read_to_string(root.join(".counts"))?
            .lines()
            .collect::<Vec<_>>(),
        ["pre"]
    );
    let unexecuted = check(root, &["--post-merge", "--no-exec"])?;
    assert_eq!(outcome(&unexecuted, "OPDEV-TEST-002")?, "unverified");
    assert_eq!(
        fs::read_to_string(root.join(".counts"))?
            .lines()
            .collect::<Vec<_>>(),
        ["pre"]
    );
    let post = check(root, &["--post-merge"])?;
    assert_eq!(outcome(&post, "OPDEV-TEST-002")?, "passed");
    assert_eq!(post["checks"].as_array().ok_or("checks")?.len(), 1);
    assert_eq!(post["checks"][0]["id"], "consumer");
    assert_eq!(
        fs::read_to_string(root.join(".counts"))?
            .lines()
            .collect::<Vec<_>>(),
        ["pre", "post"]
    );
    // Executable-looking guidance is not excluded just because it is Markdown.
    fs::write(
        root.join("shared-guidance.md"),
        "Run a changed shared setup command.\n",
    )?;
    git(root, &["add", "."])?;
    assert_eq!(
        outcome(&check(root, &["--post-merge"])?, "OPDEV-TEST-002")?,
        "unverified"
    );
    Ok(())
}

#[test]
fn stage_mapping_overlap_empty_unknown_and_legacy_policy_fail_closed() -> Result {
    let (temp, ledger) = staged_project()?;
    let catalog = discover(temp.path())?.manifest.catalog()?;
    for case in [
        "overlap",
        "all_overlap",
        "empty",
        "duplicate",
        "unknown",
        "legacy",
    ] {
        let mut value = serde_json::to_value(&ledger)?;
        let mappings = &mut value["changes"][0]["acceptance"]["verifications"];
        match case {
            "overlap" => mappings[1]["stages"] = serde_json::json!(["pre_merge"]),
            "all_overlap" => {
                mappings[0]
                    .as_object_mut()
                    .ok_or("mapping")?
                    .remove("stages");
            }
            "empty" => mappings[0]["stages"] = serde_json::json!([]),
            "duplicate" => mappings[0]["stages"] = serde_json::json!(["pre_merge", "pre_merge"]),
            "unknown" => mappings[0]["stages"] = serde_json::json!(["imagined_stage"]),
            _ => (),
        }
        fs::write(temp.path().join(EVIDENCE_PATH), serde_json::to_vec(&value)?)?;
        let selected = if case == "legacy" {
            embedded_catalog()?
        } else {
            catalog.clone()
        };
        assert!(
            EvidenceLedger::load_optional(temp.path(), &selected).is_err(),
            "{case}"
        );
        assert!(!temp.path().join(".counts").exists());
    }
    Ok(())
}

#[test]
fn omitted_stages_preserve_legacy_digest_and_scoped_stages_change_it() -> Result {
    let (_, ledger) = project()?;
    let change = &ledger.changes[0];
    let acceptance = change.acceptance.as_ref().ok_or("acceptance")?;
    let mappings = serde_json::to_value(&acceptance.verifications)?;
    assert!(mappings[0].get("stages").is_none());
    let legacy_payload = serde_json::json!({
        "protocol": 1, "fingerprint": change.fingerprint, "work": change.work,
        "scope": acceptance.scope, "rationale": acceptance.rationale,
        "conditions": acceptance.conditions, "verifications": mappings,
    });
    let digest = acceptance.digest(&change.fingerprint, &change.work)?;
    assert_eq!(
        digest,
        format!("{:x}", Sha256::digest(serde_json::to_vec(&legacy_payload)?))
    );
    let mut scoped = acceptance.clone();
    scoped.verifications[0].stages = Some(vec![TestStage::PreMerge]);
    assert_ne!(digest, scoped.digest(&change.fingerprint, &change.work)?);
    Ok(())
}

#[test]
fn missing_mapping_removed_check_and_known_contradictions_cannot_hide_at_another_stage() -> Result {
    for case in ["mapping", "suite", "failed", "other_stage_failed"] {
        let (temp, mut ledger) = staged_project()?;
        let root = temp.path();
        match case {
            "mapping" => {
                ledger.changes[0]
                    .acceptance
                    .as_mut()
                    .ok_or("acceptance")?
                    .verifications
                    .pop();
            }
            "suite" => {
                let mut manifest = discover(root)?.manifest;
                manifest
                    .testing
                    .suites
                    .retain(|suite| suite.id != "consumer");
                fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
            }
            "failed" => {
                let path = root.join("consumer.py");
                fs::write(
                    &path,
                    format!(
                        "{}\nassert False, 'consumer failure'\n",
                        fs::read_to_string(&path)?
                    ),
                )?;
            }
            _ => {
                ledger.changes[0]
                    .acceptance
                    .as_mut()
                    .ok_or("acceptance")?
                    .verifications[0]
                    .outcome = Outcome::Failed;
            }
        }
        git(root, &["add", "."])?;
        ledger.changes[0].fingerprint = staged_fingerprint(root)?;
        if case == "failed" {
            ledger.changes[0]
                .acceptance
                .as_mut()
                .ok_or("acceptance")?
                .verifications[1]
                .target = reference(root, "consumer.py", "assert select(['c', 'a'], 1) == ['c']")?;
        }
        bind(&mut ledger)?;
        save(root, &ledger)?;
        let report = check(root, &["--post-merge"])?;
        let expected = if case.contains("failed") {
            "failed"
        } else {
            "unverified"
        };
        assert_eq!(
            outcome(&report, "OPDEV-TEST-002")?,
            expected,
            "{case}: {report}"
        );
        if case == "mapping" {
            assert!(
                report
                    .to_string()
                    .contains("needs exactly one reviewed test or observation")
            );
        }
    }
    Ok(())
}
