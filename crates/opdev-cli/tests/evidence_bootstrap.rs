//! Process-level acceptance tests for evidence bootstrap review.

use std::fs;
use std::process::Command;

use opdev_core::Evidence;
use opdev_project::{EVIDENCE_PATH, EvidenceBootstrap, ReviewDecision};

fn opdev() -> Command {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
}

#[test]
#[allow(clippy::too_many_lines)] // One connected prepare/resolve/assemble protocol with unchanged-source assertions.
fn complete_adoption_questionnaire_round_trips_without_hand_written_rule_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    let scratch = tempfile::tempdir()?;
    initialize_external(repo.path())?;
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(repo.path())
            .args(["add", "."])
            .output()?
            .status
            .success()
    );
    let before = fs::read(repo.path().join(opdev_project::MANIFEST_PATH))?;
    let output = opdev()
        .args(["adoption", "prepare-evidence", "--complete", "--root"])
        .arg(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut answers: EvidenceBootstrap = serde_saphyr::from_slice(&output.stdout)?;
    assert_eq!(
        answers.change.decisions["OPDEV-TEST-002"],
        ReviewDecision::ReviewRequired
    );
    assert_eq!(
        answers.change.decisions["OPDEV-WORK-001"],
        ReviewDecision::ReviewRequired
    );
    assert!(
        answers
            .change
            .decisions
            .values()
            .all(|v| *v == ReviewDecision::ReviewRequired)
    );
    assert!(
        answers
            .project
            .decisions
            .values()
            .all(|v| *v == ReviewDecision::ReviewRequired)
    );
    assert!(
        answers
            .change
            .evidence
            .iter()
            .any(|e| e.kind == "adoption_review"
                && e.location.as_deref() == Some(opdev_project::ADOPTION_PATH))
    );
    let input = scratch.path().join("answers.yaml");
    fs::write(&input, answers.to_yaml()?)?;
    let unresolved = opdev()
        .args(["evidence", "bootstrap", "--adoption", "--root"])
        .arg(repo.path())
        .arg("--answers")
        .arg(&input)
        .output()?;
    assert!(!unresolved.status.success());
    // These are attributed synthetic judgments, not real project adoption evidence.
    answers.change.work = "synthetic test of input assembly".into();
    for decision in answers.change.decisions.values_mut() {
        *decision = ReviewDecision::Passed;
    }
    for evidence in answers
        .change
        .evidence
        .iter_mut()
        .chain(&mut answers.project.evidence)
    {
        evidence.summary = "Synthetic review for the assembly test only".into();
        evidence
            .location
            .get_or_insert_with(|| "fixture pipeline".into());
    }
    fs::write(&input, answers.to_yaml()?)?;
    let candidate = opdev()
        .args(["evidence", "bootstrap", "--adoption", "--root"])
        .arg(repo.path())
        .arg("--answers")
        .arg(&input)
        .output()?;
    assert!(
        candidate.status.success(),
        "{}",
        String::from_utf8_lossy(&candidate.stderr)
    );
    let ledger: opdev_project::EvidenceLedger = serde_saphyr::from_slice(&candidate.stdout)?;
    let change = &ledger.changes[0];
    for id in ["OPDEV-WORK-001", "OPDEV-TEST-002"] {
        assert!(
            change.assertions.iter().any(|a| a.rule_id.as_str() == id
                && a.evidence.iter().any(|e| e.kind == "adoption_review"))
        );
    }
    assert!(!repo.path().join(EVIDENCE_PATH).exists());
    assert_eq!(
        fs::read(repo.path().join(opdev_project::MANIFEST_PATH))?,
        before
    );
    Ok(())
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
fn complete_preparation_binds_catalogs_but_leaves_capability_judgments_unresolved()
-> Result<(), Box<dyn std::error::Error>> {
    for (engineering, safeguards) in [("2", 1), ("3", 2)] {
        complete_preparation_for_policy(engineering, safeguards)?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Connected current-policy fixture and unresolved-judgment assertions.
fn complete_preparation_for_policy(
    engineering: &str,
    safeguards: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::{Value, json};
    let repo = tempfile::tempdir()?;
    let root = repo.path();
    initialize_external(root)?;
    let path = root.join(opdev_project::MANIFEST_PATH);
    let mut manifest: Value = serde_json::to_value(opdev_project::ProjectManifest::load(&path)?)?;
    manifest["schema"] = json!(4);
    manifest["assurance"]["engineering"]["version"] = json!(engineering);
    manifest["layout"] = json!({"version":2,"review_reference":"synthetic layout choice"});
    manifest["assurance"]["review_storage"]["version"] = json!(2);
    manifest["assurance"]["review_storage"]["report_retention_days"] = json!(1);
    manifest["assurance"]["review_storage"]["retention_authority"] = json!("contracts");
    manifest["project"]["ci"]["provider"] = json!("gitlab");
    manifest["project"]["ci"]["remote"] = json!("https://gitlab.com/fixture/review.git");
    manifest["assurance"]["requirements"] = json!({"version":1,"review_reference":"synthetic selected policy","configurations":{"default":["local","pre_merge","post_merge"]}});
    manifest["assurance"]["safeguards"] = json!({"version":safeguards,"review_reference":"synthetic capability inventory","capabilities":{
        "persistent_data":{"state":"present","rationale":"fixture persistence","authority":"contracts"},
        "user_interface":{"state":"absent","rationale":"fixture has no interface","authority":"contracts"}
    }});
    let manifest: opdev_project::ProjectManifest = serde_json::from_value(manifest)?;
    fs::write(path, manifest.to_yaml()?)?;
    fs::create_dir_all(root.join(".opdev/requirements"))?;
    fs::write(
        root.join(".opdev/requirements/fixture.json"),
        serde_json::to_vec(&json!({"schema":1,"requirements":[],"verifications":[],"plans":[]}))?,
    )?;
    for args in [
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-qm",
            "baseline",
        ],
    ] {
        assert!(
            Command::new("git")
                .current_dir(root)
                .args(args)
                .output()?
                .status
                .success()
        );
    }
    let base = String::from_utf8(
        Command::new("git")
            .current_dir(root)
            .args(["rev-parse", "HEAD"])
            .output()?
            .stdout,
    )?
    .trim()
    .to_owned();
    let invoke = |baseline: Option<&str>| {
        let mut command = opdev();
        command
            .current_dir(root)
            .args(["adoption", "prepare-evidence", "--complete"]);
        if let Some(base) = baseline {
            command.args(["--base", base]);
        }
        command.output()
    };
    for invalid in [
        None,
        Some("HEAD"),
        Some("1111111111111111111111111111111111111111"),
    ] {
        assert!(!invoke(invalid)?.status.success());
    }
    let generated = invoke(Some(&base))?;
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let value: Value = serde_saphyr::from_slice(&generated.stdout)?;
    for control in [
        "OPDEV-FLOW-001",
        "OPDEV-DELIVERY-001",
        "OPDEV-PIPELINE-001",
        "OPDEV-RECOVERY-001",
    ] {
        assert_eq!(value["change"]["decisions"][control], "review_required");
        assert!(value["project"]["decisions"].get(control).is_none());
    }
    let acceptance = &value["change"]["acceptance"];
    let catalog = opdev_project::requirements::load_index(root)?.digest()?;
    assert_eq!(acceptance["requirements"]["catalog_sha256"], catalog);
    assert_eq!(
        acceptance["requirements"]["baseline_catalog_sha256"],
        catalog
    );
    assert_eq!(acceptance["requirements"]["baseline_commit"], base);
    assert_eq!(acceptance["requirements"]["decision_reference"], "");
    let impacts = acceptance["safeguards"]["impacts"]
        .as_object()
        .ok_or("impact slots")?;
    assert!(impacts["persistent_data"]["impact"].is_null());
    assert!(
        impacts.contains_key("public_contract"),
        "missing capability is unknown, not absent"
    );
    assert!(
        !impacts.contains_key("user_interface"),
        "only explicitly absent capability omits its slot"
    );
    assert!(impacts.values().all(|impact| impact["impact"].is_null()));
    assert_eq!(impacts.contains_key("sensitive_data"), safeguards == 2);
    assert_eq!(impacts.contains_key("high_consequence"), safeguards == 2);
    assert_eq!(acceptance["safeguards"]["version"], safeguards);
    assert!(
        acceptance["safeguards"]["objectives"]
            .as_object()
            .ok_or("objectives")?
            .values()
            .all(|v| v == &json!([]))
    );
    round_trip_current_questionnaire(root, value, engineering)?;
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(["status", "--porcelain"])
            .output()?
            .stdout
            .is_empty()
    );
    Ok(())
}

fn round_trip_current_questionnaire(
    root: &std::path::Path,
    mut value: serde_json::Value,
    engineering: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::json;
    let scratch = tempfile::tempdir()?;
    let input = scratch.path().join("current-answers.json");
    // Synthetic schema/assembly judgments only, not project qualification.
    value["change"]["work"] = json!("synthetic current-policy assembly regression");
    value["change"]["decisions"]["OPDEV-WORK-001"] = json!("passed");
    let controls = [
        "OPDEV-FLOW-001",
        "OPDEV-DELIVERY-001",
        "OPDEV-PIPELINE-001",
        "OPDEV-RECOVERY-001",
    ];
    for control in controls {
        value["change"]["decisions"][control] = json!("passed");
    }
    for evidence in value["change"]["evidence"]
        .as_array_mut()
        .ok_or("evidence")?
    {
        evidence["summary"] = json!("Synthetic assembly review, no execution claim");
        evidence["location"] = json!("synthetic fixture authority");
    }
    let acceptance = &mut value["change"]["acceptance"];
    acceptance["rationale"] = json!("Current-policy input round trip, not a passing gate");
    acceptance["requirements"]["rationale"] = json!("Preserve the exact generated catalog IDs");
    acceptance["requirements"]["decision_reference"] = json!("synthetic selected catalog");
    for impact in acceptance["safeguards"]["impacts"]
        .as_object_mut()
        .ok_or("impacts")?
        .values_mut()
    {
        impact["impact"] = json!("unaffected");
        impact["rationale"] = json!("No product changes in this schema-only synthetic fixture");
    }
    acceptance["safeguards"]["objectives"] = json!({});
    acceptance["policy_controls"] = json!({
        "version": engineering,
        "definition_sha256": opdev_core::resolve_engineering_policy(engineering)?.definition_sha256,
        "stage": "pre_merge", "bindings": {}
    });
    fs::write(&input, serde_json::to_vec(&value)?)?;
    let loaded = EvidenceBootstrap::load(&input)?;
    let expected: opdev_project::AcceptanceEvidence =
        serde_json::from_value(value["change"]["acceptance"].clone())?;
    assert_eq!(loaded.change.acceptance.as_ref(), Some(&expected));
    let output = opdev()
        .current_dir(root)
        .args(["evidence", "bootstrap", "--adoption", "--answers"])
        .arg(&input)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ledger: opdev_project::EvidenceLedger = serde_saphyr::from_slice(&output.stdout)?;
    assert_eq!(ledger.changes[0].acceptance.as_ref(), Some(&expected));
    for control in controls {
        assert!(
            ledger.changes[0]
                .assertions
                .iter()
                .any(|assertion| assertion.rule_id.as_str() == control)
        );
        assert!(
            !ledger
                .project
                .iter()
                .any(|assertion| assertion.rule_id.as_str() == control)
        );
    }
    assert!(!root.join(EVIDENCE_PATH).exists());
    for (pointer, invalid) in [
        (
            "/change/acceptance/requirements/catalog_sha256",
            json!("bad"),
        ),
        ("/change/acceptance/policy_controls/version", json!("99")),
        (
            "/change/acceptance/safeguards/impacts/persistent_data/impact",
            json!(null),
        ),
    ] {
        let mut malformed = value.clone();
        *malformed.pointer_mut(pointer).ok_or("negative target")? = invalid;
        fs::write(&input, serde_json::to_vec(&malformed)?)?;
        assert!(EvidenceBootstrap::load(&input).is_err(), "{pointer}");
    }
    value["change"]["acceptance"]["requirements"]["invented"] = json!(true);
    fs::write(&input, serde_json::to_vec(&value)?)?;
    assert!(EvidenceBootstrap::load(&input).is_err());
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
