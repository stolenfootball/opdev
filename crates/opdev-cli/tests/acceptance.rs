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
        scope: AcceptanceScope::Behavioral,
        rationale: "Only the agreed order/count behavior changes; invalid inputs are excluded.".into(),
        conditions: vec![AcceptanceCondition { id: "R1".into(),
            statement: "Preserve caller order and return no more than the requested count".into(),
            authority: "requirements.md".into(), source: reference(root, "requirements.md", "R1: Preserve caller order")? }],
        verifications: vec![AcceptanceVerification { condition: "R1".into(), method: AcceptanceMethod::Automated,
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
            _ => acceptance.conditions[0].source.sha256 = "0".repeat(64),
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
