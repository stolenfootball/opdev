//! Behavioral coverage for explicit practical contracts; no provider credentials.
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn local_probe_accounting_is_offline_and_keeps_failed_attempts() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(root.join("tests/practical_trials_test.py"))
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn practical_adoption_routes_inspect_without_restarting_legacy_assessment() -> TestResult {
    let dir = fixture()?;
    for args in [
        vec!["adoption", "next"],
        vec!["adoption", "plan"],
        vec!["adoption", "start"],
    ] {
        let output = run(dir.path(), &args)?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!dir.path().join(".opdev/adoption.yaml").exists());
        assert!(!dir.path().join("ran.txt").exists());
    }
    let preflight = run(
        dir.path(),
        &["adoption", "check", "--preflight", "--format", "json"],
    )?;
    assert_eq!(preflight.status.code(), Some(1));
    let preflight: Value = serde_json::from_slice(&preflight.stdout)?;
    assert_eq!(
        preflight["design_documents"]["recorded_decisions_complete"],
        false
    );
    assert!(!dir.path().join("ran.txt").exists());
    let output = run(dir.path(), &["adoption", "check"])?;
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("No checks ran"));
    Ok(())
}

#[test]
fn legacy_migration_cannot_drop_original_obligations_or_guess_unknown_policy() -> TestResult {
    let dir = fixture()?;
    let outside = tempfile::tempdir()?;
    let original = include_str!("../../../.opdev/project.yaml");
    fs::write(dir.path().join(".opdev/project.yaml"), original)?;
    let request = outside.path().join("request.json");
    let mut value = json!({"schema":2,"project":contract(),"assurance_change_reference":"synthetic authorized migration"});
    fs::write(&request, serde_json::to_vec(&value)?)?;
    let path = request.to_str().ok_or("path")?;
    let rejected = run(dir.path(), &["upgrade", "--migration", path])?;
    assert_eq!(rejected.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("Original obligation"));
    assert_eq!(
        fs::read_to_string(dir.path().join(".opdev/project.yaml"))?,
        original
    );
    value["retired_obligations"] = json!({"minimumcd":"synthetic original authority retirement", "profiles:nist-ssdf-derived":"synthetic original authority retirement"});
    fs::write(&request, serde_json::to_vec(&value)?)?;
    let accepted = run(dir.path(), &["upgrade", "--migration", path])?;
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    fs::write(dir.path().join(".opdev/project.yaml"), "schema: 99\n")?;
    assert_eq!(
        run(dir.path(), &["upgrade", "--migration", path])?
            .status
            .code(),
        Some(2)
    );
    Ok(())
}

#[test]
fn practical_migration_schema_is_offline_and_rejects_unknown_choices() -> TestResult {
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["upgrade", "--request-schema", "--request-version", "2"])
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let schema: Value = serde_json::from_slice(&output.stdout)?;
    let validator = jsonschema::validator_for(&schema)?;
    let mut request =
        json!({"schema":2,"project":contract(),"assurance_change_reference":"reviewed"});
    assert!(validator.is_valid(&request));
    request["project"]["preset"] = json!("invented");
    assert!(!validator.is_valid(&request));
    Ok(())
}

#[test]
fn migration_previews_retires_and_resumes_without_a_permanent_ledger() -> TestResult {
    let dir = fixture()?;
    let external = tempfile::tempdir()?;
    fs::write(dir.path().join(".opdev/evidence.yaml"), "obsolete history")?;
    fs::write(dir.path().join(".opdev/adoption.yaml"), "obsolete adoption")?;
    fs::write(dir.path().join("AGENTS.md"), "Unrelated instruction\n")?;
    let request = external.path().join("request.json");
    let recovery = external.path().join("recovery.json");
    fs::write(
        &request,
        serde_json::to_vec(&json!({"schema":2,"project":contract(),
        "assurance_change_reference":"actual synthetic approval",
        "retire":{".opdev/evidence.yaml":"replace permanent history", ".opdev/adoption.yaml":"workflow configuration replaces log"}}))?,
    )?;
    let request = request.to_str().ok_or("request path")?;
    let recovery_path = recovery.to_str().ok_or("recovery path")?;
    let preview = run(dir.path(), &["upgrade", "--migration", request])?;
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    assert!(dir.path().join(".opdev/evidence.yaml").exists());
    let identity = report(&preview)?["plan"]["identity"]
        .as_str()
        .ok_or("identity")?
        .to_owned();
    assert_eq!(
        run(
            dir.path(),
            &["upgrade", "--migration", request, "--apply", &identity]
        )?
        .status
        .code(),
        Some(2)
    );
    let applied = run(
        dir.path(),
        &[
            "upgrade",
            "--migration",
            request,
            "--apply",
            &identity,
            "--recovery-output",
            recovery_path,
        ],
    )?;
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert_eq!(report(&applied)?["verification"], "not_run");
    assert!(!dir.path().join(".opdev/evidence.yaml").exists());
    assert!(!dir.path().join(".opdev/adoption.yaml").exists());
    assert!(fs::read_to_string(dir.path().join("AGENTS.md"))?.contains("Unrelated instruction"));
    assert!(fs::read_to_string(&recovery)?.contains("obsolete history"));
    // Recreate an interrupted per-file apply: some exact before states remain,
    // while the already-written files retain their exact after states.
    let saved: Value = serde_json::from_slice(&fs::read(&recovery)?)?;
    for change in saved["changes"].as_array().ok_or("changes")?.iter().skip(2) {
        let path = dir.path().join(change["path"].as_str().ok_or("path")?);
        if let Some(before) = change["before"].as_str() {
            fs::write(path, before)?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
    }
    assert!(
        run(
            dir.path(),
            &["upgrade", "--resume", recovery_path, "--apply", &identity]
        )?
        .status
        .success()
    );
    fs::write(dir.path().join(".opdev/guidance.md"), "later user edit")?;
    assert_eq!(
        run(
            dir.path(),
            &["upgrade", "--resume", recovery_path, "--apply", &identity]
        )?
        .status
        .code(),
        Some(2)
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(".opdev/guidance.md"))?,
        "later user edit"
    );
    Ok(())
}

#[test]
fn migration_rejects_stale_unknown_and_unselected_content_before_writing() -> TestResult {
    let dir = fixture()?;
    let external = tempfile::tempdir()?;
    let request = external.path().join("request.json");
    let recovery = external.path().join("recovery.json");
    fs::write(
        &request,
        serde_json::to_vec(
            &json!({"schema":2,"project":contract(),"assurance_change_reference":"actual choice"}),
        )?,
    )?;
    let request = request.to_str().ok_or("path")?;
    let preview = run(dir.path(), &["upgrade", "--migration", request])?;
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let identity = report(&preview)?["plan"]["identity"]
        .as_str()
        .ok_or("identity")?
        .to_owned();
    fs::write(dir.path().join(".opdev/junk.txt"), "user content")?;
    let result = run(
        dir.path(),
        &[
            "upgrade",
            "--migration",
            request,
            "--apply",
            &identity,
            "--recovery-output",
            recovery.to_str().ok_or("path")?,
        ],
    )?;
    assert_eq!(result.status.code(), Some(2));
    assert!(!recovery.exists());
    assert_eq!(
        fs::read_to_string(dir.path().join(".opdev/junk.txt"))?,
        "user content"
    );
    Ok(())
}

#[test]
fn migration_preserves_requirement_identity_and_rejects_unknown_or_failed_inventory() -> TestResult
{
    let dir = fixture()?;
    let external = tempfile::tempdir()?;
    fs::create_dir(dir.path().join(".opdev/requirements"))?;
    let legacy = dir.path().join(".opdev/requirements/old.json");
    fs::write(&legacy, r#"{"schema":1,"requirements":[{"id":"R-old"}]}"#)?;
    let request = external.path().join("request.json");
    let mut value = json!({"schema":2,"project":contract(),
        "assurance_change_reference":"synthetic approval",
        "retire":{".opdev/requirements/old.json":"explicit obsolete format retirement"},
        "retire_directories":[".opdev/requirements"]});
    fs::write(&request, serde_json::to_vec(&value)?)?;
    let path = request.to_str().ok_or("request path")?;
    let blocked = run(dir.path(), &["upgrade", "--migration", path])?;
    assert_eq!(blocked.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("Requirement R-old would disappear"));
    value["retired_requirements"] = json!({"R-old":"actual original authority decision"});
    fs::write(&request, serde_json::to_vec(&value)?)?;
    assert!(
        run(dir.path(), &["upgrade", "--migration", path])?
            .status
            .success()
    );
    for contents in [
        r#"{"schema":99,"requirements":[{"id":"R-old"}]}"#,
        r#"{"schema":1,"requirements":[{"id":"R-old"}],"plans":[{"review":{"outcome":"failed"}}]}"#,
    ] {
        fs::write(&legacy, contents)?;
        assert_eq!(
            run(dir.path(), &["upgrade", "--migration", path])?
                .status
                .code(),
            Some(2)
        );
        assert_eq!(fs::read_to_string(&legacy)?, contents);
    }
    Ok(())
}

#[test]
fn additive_obligations_block_standard_without_current_mapping_review() -> TestResult {
    let dir = fixture()?;
    let mut config = contract();
    config["requirements"] = json!([".opdev/requirements/security.json"]);
    config["policies"] = json!([".opdev/policies/security.json"]);
    fs::create_dir(dir.path().join(".opdev/requirements"))?;
    fs::create_dir(dir.path().join(".opdev/policies"))?;
    fs::write(dir.path().join("assertion.txt"), "reject an invalid token")?;
    fs::write(
        dir.path().join(".opdev/requirements/security.json"),
        serde_json::to_vec(&json!({"schema":2,"requirements":[{
        "id":"R1","statement":"Invalid token rejected", "verification":[{"check":"unit","path":"assertion.txt","assertion":"invalid token"}]}]}))?,
    )?;
    fs::write(
        dir.path().join(".opdev/policies/security.json"),
        serde_json::to_vec(
            &json!({"schema":1,"id":"security","source":"original organization policy",
        "origin":{"name":"organization-security","version":1},"decision_reference":"actual mapping review","requirements":["R1"],"stages":["local"]}),
        )?,
    )?;
    write(dir.path(), &config)?;
    let output = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report(&output)?["checks"][0]["status"], "not_run");
    assert!(
        report(&output)?["mapping_findings"]
            .as_array()
            .ok_or("findings")?
            .iter()
            .any(|f| f.as_str().is_some_and(|f| f.contains("needs review")))
    );
    Ok(())
}

#[test]
fn both_native_provider_templates_support_practical_contracts_without_review_transport()
-> TestResult {
    let dir = fixture()?;
    for provider in ["github", "gitlab"] {
        let mut args = vec!["ci", "generate", "--provider", provider];
        if provider == "gitlab" {
            args.extend(["--image", "ubuntu:24.04"]);
        }
        let result = run(dir.path(), &args)?;
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let text = String::from_utf8(result.stdout)?;
        assert!(text.contains("opdev check --ci"));
        assert!(text.contains("develop"));
        assert!(!text.contains("--review-ci"));
        assert!(text.contains("--post-merge"));
        assert!(text.contains("--expected-revision"));
        if provider == "github" {
            assert!(text.contains("$OPDEV_CHECKOUT_SHA"));
            assert!(text.contains("github.event.pull_request.head.sha || github.sha"));
        } else {
            assert!(text.contains("$CI_COMMIT_SHA"));
        }
        let _: Value = serde_saphyr::from_str(&text)?;
    }
    Ok(())
}

#[test]
fn published_practical_schemas_reject_unknown_fields_and_versions() -> TestResult {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schema/practical-project.schema.json"
    ))?;
    let validator = jsonschema::validator_for(&schema)?;
    assert!(validator.is_valid(&contract()));
    let mut manual = contract();
    manual["checks"] = json!([{"id":"review", "manual":"docs/review.md", "stages":["evaluation"]}]);
    assert!(validator.is_valid(&manual));
    manual["checks"][0]["command"] = json!("test");
    assert!(!validator.is_valid(&manual));
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
fn invalidated_inputs_preserve_the_observed_command_failure() -> TestResult {
    for mutation in [
        "pathlib.Path('.opdev/project.yaml').unlink()",
        "pathlib.Path('.opdev/requirements/product.json').write_text('{bad json')",
        "pathlib.Path('.opdev/junk.txt').write_text('new content')",
    ] {
        let dir = fixture()?;
        let mut config = contract();
        config["requirements"] = json!([".opdev/requirements/product.json"]);
        fs::create_dir(dir.path().join(".opdev/requirements"))?;
        fs::write(dir.path().join("assertion.txt"), "test assertion")?;
        fs::write(
            dir.path().join(".opdev/requirements/product.json"),
            serde_json::to_vec(&json!({
                "schema":2,"requirements":[{"id":"R1","statement":"Observed behavior", "verification":[{"check":"unit","path":"assertion.txt","assertion":"test assertion"}]}]
            }))?,
        )?;
        config["commands"]["test"]["argv"] = json!([
            if cfg!(windows) { "python" } else { "python3" },
            "-c",
            format!(
                "import pathlib,sys; {mutation}; print('observed product failure'); sys.exit(7)"
            )
        ]);
        write(dir.path(), &config)?;
        let output = run(dir.path(), &["check", "--format", "json"])?;
        assert_eq!(output.status.code(), Some(1));
        let result = report(&output)?;
        assert_eq!(result["execution_status"], "failed");
        assert_eq!(result["checks"][0]["exit_code"], 7);
        assert!(
            result["checks"][0]["stdout"]
                .as_str()
                .is_some_and(|s| s.contains("observed product failure"))
        );
        assert!(
            result["mapping_findings"]
                .as_array()
                .is_some_and(|f| !f.is_empty())
        );
    }
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
    opdev_project::apply_agent_preview(&opdev_project::preview_practical_agent_files(dir.path())?)?;
    let output = run(dir.path(), &["doctor", "--format", "json"])?;
    let result = report(&output)?;
    assert_eq!(output.status.code(), Some(0), "{result}");
    let guidance: Vec<_> = result["findings"]
        .as_array()
        .ok_or("findings")?
        .iter()
        .filter(|f| f["id"] == "inventory.project_guidance")
        .collect();
    assert_eq!(guidance.len(), 3, "{result}");
    assert!(guidance.iter().all(|f| f["outcome"] == "passed"));
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

#[test]
fn practical_doctor_preserves_malformed_guidance_and_reports_error() -> TestResult {
    let dir = fixture()?;
    let path = dir.path().join(".opdev/guidance.md");
    let malformed = "<!-- opdev:start -->\nmissing end marker\n";
    fs::write(&path, malformed)?;
    let output = run(dir.path(), &["doctor", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(2));
    assert!(
        report(&output)?["findings"]
            .as_array()
            .ok_or("findings")?
            .iter()
            .any(|f| f["id"] == "inventory.project_guidance" && f["outcome"] == "error")
    );
    assert_eq!(fs::read_to_string(path)?, malformed);
    Ok(())
}

#[test]
fn practical_doctor_reports_outdated_guidance_without_rewriting_it() -> TestResult {
    let dir = fixture()?;
    opdev_project::apply_agent_preview(&opdev_project::preview_practical_agent_files(dir.path())?)?;
    let path = dir.path().join(".opdev/guidance.md");
    let old = "<!-- opdev:start -->\nOld guidance\n<!-- opdev:end -->\n";
    fs::write(&path, old)?;
    let output = run(dir.path(), &["doctor", "--format", "json"])?;
    assert_ne!(output.status.code(), Some(2));
    assert!(
        report(&output)?["findings"]
            .as_array()
            .ok_or("findings")?
            .iter()
            .any(
                |f| f["id"] == "inventory.project_guidance" && f["outcome"] == "migration_required"
            )
    );
    assert_eq!(fs::read_to_string(path)?, old);
    Ok(())
}

fn run(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    // These existing cases exercise qualification, not implicit purpose selection.
    // The separate check_purpose suite covers the no-purpose refusal directly.
    let mut command = Command::new(env!("CARGO_BIN_EXE_opdev"));
    command.args(args);
    if args.first() == Some(&"check") && !args.contains(&"--evaluation") {
        command.args(["--purpose", "integration"]);
    }
    command.arg("--root").arg(root).output()
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

#[test]
fn manual_checks_are_not_run_and_never_satisfy_a_required_boundary() -> TestResult {
    let dir = fixture()?;
    fs::write(
        dir.path().join("procedure.md"),
        "Observe keyboard navigation.",
    )?;
    let mut policy = contract();
    policy["checks"]
        .as_array_mut()
        .ok_or("checks")?
        .push(json!({
            "id":"usability", "manual":"procedure.md", "stages":["evaluation"]
        }));
    write(dir.path(), &policy)?;
    assert!(run(dir.path(), &["check"])?.status.success());
    let output = run(dir.path(), &["check", "--evaluation", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    let result = report(&output)?;
    assert_eq!(result["stage"], "evaluation");
    assert_eq!(result["execution_status"], "not_run");
    assert_eq!(result["checks"], json!([]));
    assert_eq!(result["manual_checks"][0]["status"], "not_run");
    assert_eq!(result["manual_checks"][0]["procedure"], "procedure.md");
    policy["checks"][1]["stages"] = json!(["local"]);
    write(dir.path(), &policy)?;
    let output = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report(&output)?["checks"][0]["status"], "passed");
    assert_eq!(report(&output)?["execution_status"], "not_run");
    policy["commands"]["test"]["argv"] = json!(["git", "not-a-git-command"]);
    write(dir.path(), &policy)?;
    let output = run(dir.path(), &["check", "--format", "json"])?;
    assert_eq!(report(&output)?["execution_status"], "failed");
    Ok(())
}

#[test]
fn manual_check_rejects_missing_unsafe_and_contradictory_procedures() -> TestResult {
    let dir = fixture()?;
    for check in [
        json!({"id":"manual", "stages":["local"]}),
        json!({"id":"manual", "manual":"missing.md", "stages":["local"]}),
        json!({"id":"manual", "manual":"../outside.md", "stages":["local"]}),
        json!({"id":"manual", "command":"test", "manual":"procedure.md", "stages":["local"]}),
    ] {
        let mut policy = contract();
        policy["checks"] = json!([check]);
        write(dir.path(), &policy)?;
        assert_eq!(run(dir.path(), &["check"])?.status.code(), Some(2));
    }
    Ok(())
}

#[test]
fn manual_mapping_subject_binds_procedure_content_without_claiming_execution() -> TestResult {
    let dir = fixture()?;
    fs::write(
        dir.path().join("procedure.md"),
        "Observe keyboard navigation.",
    )?;
    let mut policy = contract();
    policy["checks"] = json!([{"id":"manual", "manual":"procedure.md", "stages":["evaluation"]}]);
    let contract = opdev_project::practical::Contract::from_yaml(&serde_json::to_string(&policy)?)?;
    let requirement = serde_json::from_value(json!({"id":"R1", "statement":"Keyboard works",
        "verification":[{"check":"manual","path":"procedure.md","assertion":"Observe"}]}))?;
    let before = contract.mapping_subject(dir.path(), &requirement)?;
    fs::write(
        dir.path().join("procedure.md"),
        "Observe keyboard navigation and error recovery.",
    )?;
    assert_ne!(before, contract.mapping_subject(dir.path(), &requirement)?);
    Ok(())
}

#[test]
fn preview_binary_reuses_the_existing_qualified_producer() -> TestResult {
    let ci: Value = serde_saphyr::from_str(include_str!("../../../.gitlab-ci.yml"))?;
    let job = &ci["package-linux-x86_64"];
    assert!(
        job["needs"]
            .as_array()
            .ok_or("needs")?
            .contains(&json!("quality"))
    );
    assert!(
        job["artifacts"]["paths"]
            .as_array()
            .ok_or("artifacts")?
            .contains(&json!("target/x86_64-unknown-linux-gnu/release/opdev"))
    );
    assert_eq!(job["artifacts"]["expire_in"], "7 days");
    let script = job["script"].as_array().ok_or("script")?;
    assert_eq!(
        script
            .iter()
            .filter(|s| s.as_str().is_some_and(|s| s.starts_with("cargo build")))
            .count(),
        1
    );
    for required in [
        "plugin verify",
        "preview-source.txt",
        "preview-toolchain.txt",
        "preview-binary.sha256",
    ] {
        assert!(
            script
                .iter()
                .any(|s| s.as_str().is_some_and(|s| s.contains(required)))
        );
    }
    Ok(())
}

#[test]
fn native_ci_checks_bind_exact_revision_and_report_actual_stage() -> TestResult {
    let dir = fixture()?;
    let mut policy = contract();
    policy["checks"][0]["stages"] = json!(["pre_merge", "post_merge", "delivery"]);
    write(dir.path(), &policy)?;
    for args in [
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
    ] {
        assert!(
            Command::new("git")
                .current_dir(dir.path())
                .args(args)
                .status()?
                .success()
        );
    }
    let head = Command::new("git")
        .current_dir(dir.path())
        .args(["rev-parse", "HEAD"])
        .output()?;
    let head = String::from_utf8(head.stdout)?;
    for (flag, stage) in [
        (None, "pre_merge"),
        (Some("--post-merge"), "post_merge"),
        (Some("--delivery"), "delivery"),
    ] {
        let mut args = vec![
            "check",
            "--ci",
            "--expected-revision",
            head.trim(),
            "--format",
            "json",
        ];
        if let Some(flag) = flag {
            args.push(flag);
        }
        let output = run(dir.path(), &args)?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(report(&output)?["stage"], stage);
    }
    for wrong in ["HEAD", "0000000000000000000000000000000000000000"] {
        let output = run(dir.path(), &["check", "--ci", "--expected-revision", wrong])?;
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("No checks ran"));
    }
    fs::write(dir.path().join("dirty.txt"), "not committed")?;
    assert_eq!(
        run(
            dir.path(),
            &["check", "--ci", "--expected-revision", head.trim()]
        )?
        .status
        .code(),
        Some(2)
    );
    Ok(())
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
