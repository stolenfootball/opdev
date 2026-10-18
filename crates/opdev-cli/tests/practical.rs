//! Behavioral coverage for explicit practical contracts; no provider credentials.
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn published_practical_schemas_reject_unknown_fields_and_versions() -> TestResult {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schema/practical-project.schema.json"
    ))?;
    let validator = jsonschema::validator_for(&schema)?;
    assert!(validator.is_valid(&contract()));
    for (field, value) in [
        ("preset", json!("fast")),
        ("policy_version", json!(99)),
        ("unexpected", json!(true)),
        ("decision_reference", json!(null)),
    ] {
        let mut invalid = contract();
        invalid[field] = value;
        assert!(!validator.is_valid(&invalid));
        assert!(
            opdev_project::practical::Contract::from_yaml(&serde_json::to_string(&invalid)?)
                .is_err()
        );
    }
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["requirements", "schema", "--version", "2"])
        .output()?;
    assert!(output.status.success());
    let schema: Value = serde_json::from_slice(&output.stdout)?;
    let validator = jsonschema::validator_for(&schema)?;
    let mut document = json!({"schema":2,"requirements":[{"id":"R1","statement":"Observable result",
        "verification":[{"check":"unit","path":"tests.rs","assertion":"expected_error"}]}]});
    assert!(validator.is_valid(&document));
    document["requirements"][0]["stages"] = json!(["local"]);
    assert!(!validator.is_valid(&document));
    assert!(serde_json::from_value::<opdev_project::practical::Requirements>(document).is_err());
    Ok(())
}

#[test]
fn strict_namespace_preserves_unclassified_files() -> TestResult {
    let dir = fixture()?;
    let path = dir.path().join(".opdev/notes.md");
    fs::write(&path, "unclassified user content")?;
    let output = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("outside the selected namespace"));
    assert_eq!(fs::read_to_string(path)?, "unclassified user content");
    Ok(())
}

#[test]
fn configured_workflow_status_does_not_claim_product_verification() -> TestResult {
    let dir = fixture()?;
    let output = run(dir.path(), &["adoption", "status", "--format", "json"])?;
    assert!(output.status.success());
    let result = report(&output)?;
    assert_eq!(result["status"], "workflow_configured");
    assert_eq!(result["product_verification"], "not_run");
    assert_eq!(result["remote_qualification"], "not_run");
    assert!(
        result["findings"]
            .as_array()
            .ok_or("findings")?
            .iter()
            .any(|f| f.as_str().is_some_and(|f| f.contains("PostMerge")))
    );
    assert!(!dir.path().join(".opdev/adoption.yaml").exists());
    assert_eq!(
        run(dir.path(), &["adoption", "status", "--remote"])?
            .status
            .code(),
        Some(2)
    );
    Ok(())
}

#[test]
fn policy_mutation_retains_execution_but_cannot_qualify() -> TestResult {
    let dir = fixture()?;
    let mut config = contract();
    config["commands"]["test"]["argv"] = json!([
        if cfg!(windows) { "python" } else { "python3" },
        "-c",
        "import pathlib; p=pathlib.Path('.opdev/project.yaml'); p.write_text(p.read_text()+'\\n')"
    ]);
    write(dir.path(), &config)?;
    let output = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    let result = report(&output)?;
    assert_eq!(result["checks"][0]["status"], "passed");
    assert!(
        result["mapping_findings"]
            .as_array()
            .ok_or("findings")?
            .iter()
            .any(|v| v.as_str().is_some_and(|v| v.contains("policy changed")))
    );
    Ok(())
}

#[test]
fn preset_preview_preserves_existing_requirement_selection() -> TestResult {
    let dir = fixture()?;
    let mut config = contract();
    config["requirements"] = json!([".opdev/requirements/product.json"]);
    write(dir.path(), &config)?;
    let output = run(dir.path(), &["adoption", "plan", "--preset", "assured"])?;
    assert_eq!(output.status.code(), Some(0));
    let proposal = report(&output)?;
    assert_eq!(proposal["project"]["requirements"], config["requirements"]);
    assert_eq!(proposal["prior_obligations"]["preset"], "standard");
    assert_eq!(proposal["project"]["decision_reference"], Value::Null);
    assert_eq!(
        fs::read_to_string(dir.path().join(".opdev/project.yaml"))?,
        serde_json::to_string(&config)?
    );
    Ok(())
}

#[test]
fn doctor_recognizes_practical_contract_without_claiming_execution() -> TestResult {
    let dir = fixture()?;
    let output = run(dir.path(), &["doctor", "--format", "json"])?;
    let result = report(&output)?;
    assert!(
        result["runtime"]["project_schemas"]
            .as_array()
            .ok_or("schemas")?
            .contains(&json!(5))
    );
    assert!(
        result["findings"]
            .as_array()
            .ok_or("findings")?
            .iter()
            .any(|f| f["id"] == "project.contract" && f["outcome"] == "passed")
    );
    assert_ne!(result["project_verification"], "passed");
    let output = run(dir.path(), &["doctor", "--remote", "--format", "json"])?;
    assert!(!output.status.success());
    assert!(
        report(&output)?["findings"]
            .as_array()
            .ok_or("findings")?
            .iter()
            .any(|f| f["id"] == "remote.unsupported" && f["outcome"] == "unverified")
    );
    Ok(())
}

fn run(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
}

fn fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let output = Command::new("git")
        .args(["init", "--quiet"])
        .arg(dir.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::create_dir(dir.path().join(".opdev"))?;
    fs::write(
        dir.path().join(".opdev/guidance.md"),
        include_str!("../../opdev-project/src/practical-guidance.md"),
    )?;
    fs::write(
        dir.path().join(".opdev/project.yaml"),
        serde_json::to_string(&contract())?,
    )?;
    Ok(dir)
}

fn contract() -> Value {
    json!({"schema":5,"policy_version":1,"preset":"standard","trunk":"develop",
        "decision_reference":"synthetic developer choice",
        "commands":{"test":{"argv":["git","--version"],"timeout_seconds":10}},
        "checks":[{"id":"unit","command":"test","stages":["local","pre_merge"]}]})
}

fn write(root: &Path, contract: &Value) -> TestResult {
    fs::write(
        root.join(".opdev/project.yaml"),
        serde_json::to_string(contract)?,
    )?;
    Ok(())
}

fn report(output: &Output) -> Result<Value, Box<dyn std::error::Error>> {
    assert!(
        !output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
fn standard_executes_without_review_comment_or_provider_access() -> TestResult {
    let dir = fixture()?;
    let output = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = report(&output)?;
    assert_eq!(report["execution_status"], "passed");
    assert_eq!(report["preset"], "standard");
    assert_eq!(report["checks"].as_array().map(Vec::len), Some(1));
    assert!(
        report["checks"][0]["stdout"]
            .as_str()
            .is_some_and(|s| s.contains("git version"))
    );
    assert!(
        report["limits"]
            .as_str()
            .is_some_and(|s| s.contains("not proof"))
    );
    Ok(())
}

#[test]
fn shared_command_runs_once_and_mapping_has_no_stage_copies() -> TestResult {
    let dir = fixture()?;
    let count = tempfile::tempdir()?;
    let count_path = count.path().join("count");
    let mut config = contract();
    let python = if cfg!(windows) { "python" } else { "python3" };
    config["commands"]["test"]["argv"] = json!([
        python,
        "-c",
        "import pathlib,sys; p=pathlib.Path(sys.argv[1]); p.write_text(p.read_text()+'x' if p.exists() else 'x')",
        count_path
    ]);
    config["checks"]
        .as_array_mut()
        .ok_or("checks")?
        .push(json!({"id":"also_unit","command":"test","stages":["local"]}));
    config["requirements"] = json!([".opdev/requirements/parser.json"]);
    fs::create_dir(dir.path().join(".opdev/requirements"))?;
    fs::write(dir.path().join("test.txt"), "reject malformed input")?;
    fs::write(
        dir.path().join(".opdev/requirements/parser.json"),
        serde_json::to_string(&json!({
            "schema":2,"requirements":[{"id":"R1","statement":"Reject malformed input",
            "verification":[{"check":"unit","path":"test.txt","assertion":"reject malformed input"}]}]
        }))?,
    )?;
    write(dir.path(), &config)?;
    let shown = run(dir.path(), &["requirements", "show", "R1"])?;
    assert!(shown.status.success());
    assert_eq!(report(&shown)?["requirement"]["id"], "R1");
    assert_eq!(report(&shown)?["checks"][0]["id"], "unit");
    assert!(
        !count_path.exists(),
        "Read-only inspection must not execute checks"
    );
    let output = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(count_path)?, "x");
    assert_eq!(report(&output)?["checks"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[test]
fn plans_missing_stages_and_no_execution_never_claim_verification() -> TestResult {
    let dir = fixture()?;
    let output = run(dir.path(), &["check", "--plan", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(report(&output)?["kind"], "practical_plan");
    for args in [
        vec!["check", "--no-exec", "--format", "json"],
        vec![
            "check",
            "--ci",
            "--post-merge",
            "--no-exec",
            "--format",
            "json",
        ],
    ] {
        let output = run(dir.path(), &args)?;
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(report(&output)?["execution_status"], "not_run");
    }
    Ok(())
}

#[test]
fn failure_survives_fail_fast_and_unrun_checks_are_visible() -> TestResult {
    let dir = fixture()?;
    let mut config = contract();
    config["commands"]["fail"] =
        json!({"argv":["git","rev-parse","--verify","refs/heads/nonexistent"]});
    config["checks"]
        .as_array_mut()
        .ok_or("checks")?
        .insert(0, json!({"id":"fail","command":"fail","stages":["local"]}));
    write(dir.path(), &config)?;
    let output = run(dir.path(), &["check", "--fail-fast", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    let report = report(&output)?;
    assert_eq!(report["execution_status"], "failed");
    assert_eq!(report["checks"][0]["status"], "failed");
    assert_eq!(report["checks"][1]["status"], "not_run");
    Ok(())
}

#[test]
fn invalid_or_unsupported_policy_fails_before_execution() -> TestResult {
    let dir = fixture()?;
    for (key, value) in [
        ("preset", json!("unknown")),
        ("policy_version", json!(99)),
        ("standards", json!(["minimumcd"])),
    ] {
        let mut config = contract();
        config[key] = value;
        write(dir.path(), &config)?;
        let output = run(dir.path(), &["check", "--format", "json"])?;
        assert_eq!(
            output.status.code(),
            Some(2),
            "{key}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
    Ok(())
}

#[test]
fn assured_requires_current_mapping_review_and_never_reuses_execution() -> TestResult {
    let dir = fixture()?;
    let mut config = contract();
    config["preset"] = json!("assured");
    config["requirements"] = json!([".opdev/requirements/parser.json"]);
    fs::create_dir(dir.path().join(".opdev/requirements"))?;
    let path = dir.path().join(".opdev/requirements/parser.json");
    fs::write(
        dir.path().join("test.txt"),
        "assert invalid checksum is rejected",
    )?;
    let mut catalog = json!({"schema":2,"requirements":[{"id":"R1","statement":"Reject malformed checksums",
        "verification":[{"check":"unit","path":"test.txt","assertion":"invalid checksum"}]}]});
    fs::write(&path, serde_json::to_string(&catalog)?)?;
    write(dir.path(), &config)?;
    let blocked = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(blocked.status.code(), Some(1));
    assert_eq!(report(&blocked)?["checks"][0]["status"], "not_run");
    let inspected = run(dir.path(), &["requirements", "inspect"])?;
    assert_eq!(
        inspected.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    catalog["requirements"][0]["review"] = json!({"reviewer":"synthetic reviewer",
        "reference":"synthetic review","rationale":"Synthetic mapping used to exercise freshness, not semantic proof",
        "outcome":"passed","subject_sha256":report(&inspected)?["mapping_subjects"]["R1"]});
    fs::write(&path, serde_json::to_string(&catalog)?)?;
    let passed = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(
        passed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&passed.stderr)
    );
    assert!(
        report(&passed)?["checks"][0]["stdout"]
            .as_str()
            .is_some_and(|s| s.contains("git version"))
    );
    // A review does not supply an execution receipt.
    assert_eq!(
        run(dir.path(), &["check", "--no-exec"])?.status.code(),
        Some(1)
    );
    fs::write(dir.path().join("unrelated.txt"), "Unrelated implementation")?;
    assert_eq!(run(dir.path(), &["check"])?.status.code(), Some(0));
    fs::write(dir.path().join("test.txt"), "assert weaker behavior")?;
    let stale = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(stale.status.code(), Some(1));
    assert_eq!(report(&stale)?["checks"][0]["status"], "not_run");
    // Explicit review failures cannot be hidden by selecting Standard.
    config["preset"] = json!("standard");
    catalog["requirements"][0]["review"]["outcome"] = json!("failed");
    write(dir.path(), &config)?;
    fs::write(&path, serde_json::to_string(&catalog)?)?;
    assert_eq!(run(dir.path(), &["check"])?.status.code(), Some(1));
    Ok(())
}

#[test]
fn adoption_proposal_cannot_masquerade_as_developer_approval() -> TestResult {
    let dir = fixture()?;
    let before = fs::read(dir.path().join(".opdev/project.yaml"))?;
    let output = run(dir.path(), &["adoption", "plan", "--preset", "lightweight"])?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let proposal = report(&output)?;
    assert!(proposal["project"]["decision_reference"].is_null());
    assert_eq!(proposal["project"]["trunk"], "develop");
    assert_eq!(proposal["prior_obligations"]["preset"], "standard");
    assert_eq!(fs::read(dir.path().join(".opdev/project.yaml"))?, before);
    Ok(())
}

#[test]
fn dangling_mappings_and_traversal_are_not_passes() -> TestResult {
    let dir = fixture()?;
    let mut config = contract();
    config["commands"]["test"]["working_directory"] = json!("../outside");
    write(dir.path(), &config)?;
    assert_eq!(run(dir.path(), &["check"])?.status.code(), Some(2));
    config = contract();
    config["checks"][0]["command"] = json!("missing");
    write(dir.path(), &config)?;
    assert_eq!(run(dir.path(), &["check"])?.status.code(), Some(2));
    config = contract();
    config["requirements"] = json!([".opdev/requirements/missing.json"]);
    write(dir.path(), &config)?;
    assert_eq!(run(dir.path(), &["check"])?.status.code(), Some(2));
    Ok(())
}

#[test]
fn dirty_integration_is_rejected_but_local_feedback_is_available() -> TestResult {
    let dir = fixture()?;
    let output = run(dir.path(), &["check", "--ci"])?;
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("clean committed checkout"));
    assert_eq!(run(dir.path(), &["check"])?.status.code(), Some(0));
    Ok(())
}

#[test]
fn empty_project_initialization_is_previewable_and_not_product_verification() -> TestResult {
    let dir = fixture()?;
    fs::remove_file(dir.path().join(".opdev/project.yaml"))?;
    fs::write(
        dir.path().join("AGENTS.md"),
        "Keep this project instruction.\n",
    )?;
    let proposed = tempfile::tempdir()?;
    let path = proposed.path().join("project.yaml");
    let mut config = contract();
    config["commands"] = json!({});
    config["checks"] = json!([]);
    fs::write(&path, serde_json::to_string(&config)?)?;
    let path = path.to_str().ok_or("UTF-8 fixture path")?;
    let output = run(dir.path(), &["init", "--project", path, "--dry-run"])?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!dir.path().join(".opdev/project.yaml").exists());
    let output = run(dir.path(), &["init", "--project", path])?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("have not been verified"));
    assert!(!dir.path().join(".opdev/adoption.yaml").exists());
    assert!(!dir.path().join(".opdev/evidence.yaml").exists());
    assert!(
        fs::read_to_string(dir.path().join("AGENTS.md"))?
            .contains("Keep this project instruction.")
    );
    assert!(
        fs::read_to_string(dir.path().join(".opdev/guidance.md"))?
            .split_whitespace()
            .count()
            < 400
    );
    assert!(fs::read_to_string(dir.path().join("CLAUDE.md"))?.contains(".opdev/guidance.md"));
    assert_eq!(run(dir.path(), &["check"])?.status.code(), Some(1));
    let before = fs::read(dir.path().join(".opdev/project.yaml"))?;
    assert_eq!(
        run(dir.path(), &["init", "--project", path])?.status.code(),
        Some(2)
    );
    assert_eq!(fs::read(dir.path().join(".opdev/project.yaml"))?, before);
    Ok(())
}
