//! Adversarial catalog behavior from independent, requirement-derived fixtures.
use anyhow::Result;
use opdev_core::Outcome;
use opdev_project::{
    CommandSpec, TestStage, TestSuite, TrackedEvidence, discover, requirements::*,
};
use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> Result<()> {
    let result = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}

fn fixture() -> Result<(
    tempfile::TempDir,
    opdev_project::ProjectManifest,
    CatalogSnapshot,
)> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    git(root, &["init", "-q"])?;
    fs::create_dir_all(root.join(DIRECTORY))?;
    fs::write(
        root.join("test.txt"),
        "assert rejected_input_leaves_data_unchanged\n",
    )?;
    fs::write(root.join("helper.txt"), "invalid_input = null\n")?;
    git(root, &["add", "."])?;
    let target = TrackedEvidence::bind(
        root,
        "test.txt".into(),
        "assert rejected_input_leaves_data_unchanged".into(),
    )?;
    let input = TrackedEvidence::bind(root, "helper.txt".into(), "invalid_input = null".into())?;
    let mut manifest = discover(root)?.manifest;
    manifest.assurance.requirements = Some(RequirementsPolicy {
        version: 1,
        review_reference: "fixture decision".into(),
        configurations: BTreeMap::from([(
            "default".into(),
            vec![TestStage::PreMerge, TestStage::PostMerge],
        )]),
    });
    manifest.commands.insert(
        "test".into(),
        CommandSpec {
            argv: vec!["tester".into()],
            working_directory: None,
            timeout_seconds: Some(30),
        },
    );
    manifest.testing.suites = vec![TestSuite {
        id: "test".into(),
        command: "test".into(),
        stages: vec![TestStage::PreMerge, TestStage::PostMerge],
    }];
    let bytes = serde_json::to_vec(&json!({"schema":1,
        "requirements":[{"id":"R1","title":"Safe rejection","statement":{"kind":"inline","text":"Malformed input must not erase data"},"origin":"fixture accepted behavior","rationale":"Data integrity","configurations":["default"],"criteria":[{"id":"C1","expected":"Reject null and preserve existing bytes"}]}],
        "verifications":[{"id":"V1","target":target,"inputs":[input],"method":{"kind":"automated","suite":"test","assurance":"suite"}}],
        "plans":(["pre_merge","post_merge"].map(|stage| json!({"id":format!("P-{stage}"),"criterion":"C1","configuration":"default","stage":stage,"members":[{"verification":"V1","assertion":"Exact byte comparison after rejection","discriminating_case":"Preexisting nonempty data must survive null input"}],"review":{"outcome":"unverified","reviewer":"fixture agent","reference":"fixture review","rationale":"Negative path must preserve bytes","subject_sha256":""}})))
    }))?;
    let document = CatalogDocument::parse(&bytes)?;
    let mut snapshot = CatalogSnapshot {
        tree: String::new(),
        documents: BTreeMap::from([(".opdev/requirements/storage.json".into(), document)]),
    };
    let subjects: Vec<_> = snapshot
        .plans()
        .map(|p| snapshot.plan_digest(p, &manifest))
        .collect::<Result<_>>()?;
    let doc = snapshot
        .documents
        .values_mut()
        .next()
        .ok_or_else(|| anyhow::anyhow!("fixture"))?;
    for (p, subject) in doc.plans.iter_mut().zip(subjects) {
        p.review.subject_sha256 = subject;
        p.review.outcome = Outcome::Passed;
    }
    save(root, &snapshot)?;
    let loaded = load_index(root)?;
    Ok((temp, manifest, loaded))
}

fn save(root: &Path, snapshot: &CatalogSnapshot) -> Result<()> {
    for (path, doc) in &snapshot.documents {
        fs::write(root.join(path), serde_json::to_vec_pretty(doc)?)?;
    }
    git(root, &["add", "."])
}

#[test]
fn complete_inputs_are_not_execution_or_consent() -> Result<()> {
    let (temp, manifest, snapshot) = fixture()?;
    let report = snapshot.inspect(temp.path(), &manifest)?;
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    assert_eq!(report.qualification, Outcome::Unverified);
    assert_eq!(report.review_subjects.len(), 2);
    Ok(())
}

#[test]
fn empty_criteria_and_empty_members_never_pass_vacuously() -> Result<()> {
    let (temp, manifest, mut snapshot) = fixture()?;
    let doc = snapshot
        .documents
        .values_mut()
        .next()
        .ok_or_else(|| anyhow::anyhow!("fixture"))?;
    doc.requirements[0].criteria.clear();
    doc.plans[0].members.clear();
    let report = snapshot.inspect(temp.path(), &manifest)?;
    assert!(report.findings.iter().any(|f| f.code == "missing_criteria"));
    assert!(report.findings.iter().any(|f| f.code == "missing_members"));
    assert!(report.findings.iter().any(|f| f.code == "unresolved_plan"));
    Ok(())
}

#[test]
fn stage_configuration_policy_and_assertion_changes_invalidate_review() -> Result<()> {
    let (temp, manifest, snapshot) = fixture()?;
    let original = snapshot
        .plans()
        .next()
        .ok_or_else(|| anyhow::anyhow!("fixture"))?;
    let subject = snapshot.plan_digest(original, &manifest)?;
    for mutation in 0..6 {
        let mut changed = snapshot.clone();
        let doc = changed
            .documents
            .values_mut()
            .next()
            .ok_or_else(|| anyhow::anyhow!("fixture"))?;
        match mutation {
            0 => doc.plans[0].stage = TestStage::PostMerge,
            1 => doc.plans[0].configuration = "alternate".into(),
            2 => doc.plans[0].members[0].assertion = "Only checks an exception".into(),
            3 => doc.verifications[0].inputs.clear(),
            4 => doc.requirements[0].criteria[0].expected = "Different guarantee".into(),
            _ => doc.verifications[0].target.sha256 = "a".repeat(64),
        }
        let plan = changed
            .plans()
            .next()
            .ok_or_else(|| anyhow::anyhow!("fixture"))?;
        assert_ne!(subject, changed.plan_digest(plan, &manifest)?);
        assert!(
            changed
                .inspect(temp.path(), &manifest)?
                .findings
                .iter()
                .any(|f| f.code == "review_not_current")
        );
    }
    let mut policy = manifest.clone();
    policy
        .commands
        .get_mut("test")
        .ok_or_else(|| anyhow::anyhow!("fixture"))?
        .argv
        .push("--filtered".into());
    assert_ne!(subject, snapshot.plan_digest(original, &policy)?);
    Ok(())
}

#[test]
fn unrelated_implementation_preserves_mapping_but_changes_execution_snapshot() -> Result<()> {
    let (temp, manifest, snapshot) = fixture()?;
    fs::write(temp.path().join("implementation.txt"), "changed\n")?;
    git(temp.path(), &["add", "."])?;
    let current = load_index(temp.path())?;
    assert_ne!(snapshot.tree, current.tree);
    assert_eq!(snapshot.digest()?, current.digest()?);
    assert!(current.inspect(temp.path(), &manifest)?.findings.is_empty());
    fs::write(temp.path().join("helper.txt"), "changed fixture\n")?;
    git(temp.path(), &["add", "."])?;
    assert!(
        current
            .inspect(temp.path(), &manifest)?
            .findings
            .iter()
            .any(|f| f.code == "source_not_current")
    );
    Ok(())
}

#[test]
fn duplicate_ids_dangling_links_and_known_contradictions_accumulate() -> Result<()> {
    let (temp, manifest, mut snapshot) = fixture()?;
    let doc = snapshot
        .documents
        .values_mut()
        .next()
        .ok_or_else(|| anyhow::anyhow!("fixture"))?;
    doc.requirements.push(doc.requirements[0].clone());
    doc.plans[0].members[0].verification = "missing".into();
    doc.plans[0].review.outcome = Outcome::Failed;
    let report = snapshot.inspect(temp.path(), &manifest)?;
    for code in [
        "duplicate_identity",
        "unresolved_plan",
        "review_contradiction",
    ] {
        assert!(report.findings.iter().any(|f| f.code == code), "{code}");
    }
    Ok(())
}

#[test]
fn strict_json_and_portable_namespace() -> Result<()> {
    for bytes in [
        r#"{"schema":1,"schema":1,"requirements":[],"verifications":[],"plans":[]}"#,
        r#"{"schema":99,"requirements":[],"verifications":[],"plans":[]}"#,
        r#"{"schema":1,"requirements":[],"verifications":[],"plans":[],"passed":true}"#,
    ] {
        assert!(CatalogDocument::parse(bytes.as_bytes()).is_err());
    }
    for path in [
        ".opdev/requirements/../x.json",
        ".opdev/requirements/CON.json",
        ".opdev/requirements/x/y.json",
        ".opdev/requirements/cache.db",
        ".opdev/requirements/X.json",
    ] {
        assert!(!portable_catalog_path(path));
    }
    assert!(portable_catalog_path(
        ".opdev/requirements/data-integrity.json"
    ));
    let (temp, _, _) = fixture()?;
    fs::write(temp.path().join(DIRECTORY).join("junk.txt"), "junk")?;
    git(temp.path(), &["add", "."])?;
    assert!(load_index(temp.path()).is_err());
    Ok(())
}

#[test]
fn canonicalization_uses_utf16_sort_and_preserves_array_order() -> Result<()> {
    let value = json!({"\u{e000}":1,"\u{1f600}":2});
    assert_eq!(
        String::from_utf8(serde_json_canonicalizer::to_vec(&value)?)?,
        "{\"😀\":2,\"\u{e000}\":1}"
    );
    assert_eq!(
        digest(&json!({"b":2,"a":1}))?,
        digest(&json!({"a":1,"b":2}))?
    );
    assert_ne!(digest(&json!([1, 2]))?, digest(&json!([2, 1]))?);
    Ok(())
}

#[test]
fn ambiguous_excerpts_and_git_errors_never_become_current_sources() -> Result<()> {
    let (temp, manifest, mut snapshot) = fixture()?;
    fs::write(
        temp.path().join("test.txt"),
        "assert duplicate\nassert duplicate\n",
    )?;
    git(temp.path(), &["add", "."])?;
    let doc = snapshot
        .documents
        .values_mut()
        .next()
        .ok_or_else(|| anyhow::anyhow!("fixture"))?;
    doc.verifications[0].target =
        TrackedEvidence::bind(temp.path(), "test.txt".into(), "assert duplicate".into())?;
    let report = snapshot.inspect(temp.path(), &manifest)?;
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == "source_not_current"
                && f.outcome == Outcome::Unverified
                && f.message.contains("ambiguous"))
    );
    let non_repository = tempfile::tempdir()?;
    let report = snapshot.inspect(non_repository.path(), &manifest)?;
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == "source_not_current" && f.outcome == Outcome::Error)
    );
    Ok(())
}

#[test]
fn overlapping_excerpt_matches_are_ambiguous_too() -> Result<()> {
    let (temp, manifest, mut snapshot) = fixture()?;
    fs::write(temp.path().join("test.txt"), "aaa")?;
    git(temp.path(), &["add", "."])?;
    let doc = snapshot
        .documents
        .values_mut()
        .next()
        .ok_or_else(|| anyhow::anyhow!("fixture"))?;
    doc.verifications[0].target =
        TrackedEvidence::bind(temp.path(), "test.txt".into(), "aa".into())?;
    assert!(
        snapshot
            .inspect(temp.path(), &manifest)?
            .findings
            .iter()
            .any(|f| f.code == "source_not_current" && f.message.contains("ambiguous")),
        "Overlapping occurrences must not identify a unique fragment"
    );
    Ok(())
}

#[test]
fn change_review_binds_real_commit_and_rejects_ambiguous_or_wrong_manual_members() -> Result<()> {
    let (temp, _, snapshot) = fixture()?;
    git(
        temp.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "baseline",
        ],
    )?;
    let commit = Command::new("git")
        .arg("-C")
        .arg(temp.path())
        .args(["rev-parse", "HEAD"])
        .output()?;
    let mut review = ChangeReview {
        catalog_sha256: snapshot.digest()?,
        baseline_commit: String::from_utf8(commit.stdout)?.trim().into(),
        baseline_catalog_sha256: snapshot.digest()?,
        rationale: "Fixture comparison, no supported promise removed".into(),
        decision_reference: "Fixture decision, not developer consent".into(),
        manual_observations: vec![],
    };
    review.verify(temp.path(), &snapshot)?;
    let mut invalid = review.clone();
    invalid.baseline_commit = snapshot.tree.clone();
    assert!(
        invalid.verify(temp.path(), &snapshot).is_err(),
        "A tree is not an accepted commit"
    );
    invalid = review.clone();
    invalid.catalog_sha256 = "a".repeat(64);
    assert!(invalid.verify(temp.path(), &snapshot).is_err());
    let mut manual = snapshot.clone();
    let doc = manual
        .documents
        .values_mut()
        .next()
        .ok_or_else(|| anyhow::anyhow!("fixture"))?;
    doc.verifications[0].method = Method::Manual {
        technique: ManualTechnique::Inspection,
        automation_limitation: "Fixture manual boundary".into(),
        recheck: "Observe exact bytes".into(),
        max_age_seconds: 60,
    };
    review.catalog_sha256 = manual.digest()?;
    review.manual_observations.push(ManualObservation {
        plan: "P-pre_merge".into(),
        verification: "V1".into(),
        plan_subject_sha256: "a".repeat(64),
        observed_at: 100,
        outcome: Outcome::Passed,
        observer: "fixture".into(),
        observed_result: "unchanged bytes".into(),
        context_and_limits: "fixture only".into(),
        reference: "fixture observation".into(),
    });
    review.verify(temp.path(), &manual)?;
    let mut duplicate = review.clone();
    duplicate
        .manual_observations
        .push(duplicate.manual_observations[0].clone());
    assert!(duplicate.verify(temp.path(), &manual).is_err());
    review.manual_observations[0].verification = "unknown".into();
    assert!(review.verify(temp.path(), &manual).is_err());
    Ok(())
}
