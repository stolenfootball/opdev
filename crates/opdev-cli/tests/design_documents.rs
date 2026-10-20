//! Real document inspection; successful metadata is deliberately not consent.
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture(legacy: bool) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(dir.path())
            .status()?
            .success()
    );
    fs::create_dir_all(dir.path().join("docs"))?;
    if legacy {
        let mut manifest = opdev_project::discover(dir.path())?.manifest;
        manifest.authorities.insert(
            "documentation".into(),
            opdev_project::AuthorityRef {
                kind: opdev_project::AuthorityKind::Path,
                location: "docs".into(),
            },
        );
        manifest.write_new(&dir.path().join(".opdev/project.yaml"))?;
    } else {
        fs::create_dir_all(dir.path().join(".opdev"))?;
        fs::write(
            dir.path().join(".opdev/project.yaml"),
            serde_json::to_vec(&json!({
                "schema":5,"policy_version":1,"preset":"standard","trunk":"main",
                "decision_reference":"synthetic workflow selection, not product approval",
                "commands":{},"checks":[],"authorities":{"documentation":"docs"},
                "requirements":[],"policies":[]
            }))?,
        )?;
        fs::write(
            dir.path().join(".opdev/guidance.md"),
            "Synthetic project guidance",
        )?;
    }
    Ok(dir)
}

fn document(root: &Path, role: &str, decision: &str, reference: &str) -> TestResult {
    fs::write(
        root.join(format!("docs/{role}.md")),
        format!(
            "---\nopdev_document:\n  schema: 1\n  role: {role}\n  decision: {decision}\n  reference: {reference}\n  scope: Current supported behavior only\n---\n# Contract\n\nA caller receives the documented result; unknown future capabilities are not approved.\n"
        ),
    )?;
    Ok(())
}

fn report(root: &Path, args: &[&str], code: i32) -> Result<Value, Box<dyn std::error::Error>> {
    let config = fs::read(root.join(".git/config"))?;
    let manifest = fs::read(root.join(".opdev/project.yaml"))?;
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(args)
        .args(["--format", "json"])
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(code),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(config, fs::read(root.join(".git/config"))?);
    assert_eq!(manifest, fs::read(root.join(".opdev/project.yaml"))?);
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
fn distinct_boundaries_preserve_missing_technical_decision_after_product_and_interface_acceptance()
-> TestResult {
    for legacy in [false, true] {
        let dir = fixture(legacy)?;
        let root = dir.path();
        let missing = report(root, &["documentation", "status"], 1)?;
        assert_eq!(missing["documents"].as_array().ok_or("documents")?.len(), 3);
        assert!(!root.join("docs/design.md").exists());
        document(root, "design", "accepted", "'synthetic product decision'")?;
        document(
            root,
            "interface",
            "accepted",
            "'synthetic interface decision'",
        )?;
        document(
            root,
            "tech_stack",
            "inferred",
            "'observed config, not approval'",
        )?;
        let product = fs::read(root.join("docs/design.md"))?;
        let pending = report(root, &["documentation", "status"], 1)?;
        assert!(
            pending["next_step"]
                .as_str()
                .ok_or("next")?
                .contains("stack preferences")
        );
        assert_eq!(pending["documents"][0]["decision"]["decision"], "accepted");
        let status = report(root, &["adoption", "status"], 0)?;
        assert_eq!(
            status["design_documents"]["recorded_decisions_complete"],
            false
        );
        document(
            root,
            "tech_stack",
            "delegated",
            "'synthetic bounded tool selection'",
        )?;
        let ready = report(root, &["documentation", "status"], 0)?;
        assert_eq!(ready["recorded_decisions_complete"], true);
        assert!(
            ready["next_step"]
                .as_str()
                .ok_or("next")?
                .contains("implementation authorization")
        );
        assert_eq!(product, fs::read(root.join("docs/design.md"))?);
    }
    Ok(())
}

#[test]
fn malformed_metadata_empty_bodies_and_missing_references_are_not_acceptance() -> TestResult {
    let dir = fixture(false)?;
    for (decision, reference) in [
        ("accepted", "null"),
        ("delegated", "''"),
        ("proposed", "null"),
    ] {
        document(dir.path(), "design", decision, reference)?;
        assert!(
            report(dir.path(), &["documentation", "status"], 1)?["documents"][0]["finding"]
                .is_string()
        );
    }
    for content in [
        "# Design\nAlready done",
        "---\nopdev_document: {}\n---\nbody",
        "---\nopdev_document:\n  schema: 1\n  role: design\n  decision: accepted\n  reference: synthetic\n  scope: current\n---\n# Heading only\n",
    ] {
        fs::write(dir.path().join("docs/design.md"), content)?;
        let result = report(dir.path(), &["documentation", "status"], 1)?;
        assert!(result["documents"][0]["finding"].is_string());
        assert!(result["documents"][0]["decision"].is_null());
    }
    Ok(())
}

#[test]
fn custom_paths_reuse_existing_names_and_reject_unsafe_or_wrong_roles() -> TestResult {
    let dir = fixture(false)?;
    document(dir.path(), "design", "accepted", "synthetic")?;
    fs::rename(
        dir.path().join("docs/design.md"),
        dir.path().join("DESIGN.md"),
    )?;
    let config = dir.path().join(".opdev/project.yaml");
    let mut value: Value = serde_json::from_slice(&fs::read(&config)?)?;
    value["authorities"]["design"] = json!("DESIGN.md");
    fs::write(&config, serde_json::to_vec(&value)?)?;
    assert_eq!(
        report(dir.path(), &["documentation", "status"], 1)?["documents"][0]["finding"],
        Value::Null
    );
    for path in [
        "../design.md",
        ".opdev/design.md",
        "docs/interface.md",
        "C:/design.md",
    ] {
        value["authorities"]["design"] = json!(path);
        fs::write(&config, serde_json::to_vec(&value)?)?;
        // Contract validation can reject an unsafe route before document inspection.
        let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
            .current_dir(dir.path())
            .args(["documentation", "status", "--format", "json"])
            .output()?;
        assert!(!output.status.success());
    }
    assert!(dir.path().join("DESIGN.md").exists());
    Ok(())
}

#[test]
fn copied_templates_remain_unaccepted_and_adoption_preflight_executes_nothing() -> TestResult {
    let dir = fixture(false)?;
    let assets = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/opdev/skills/opdev/assets/project-documents");
    for role in ["design", "interface", "tech_stack"] {
        fs::copy(
            assets.join(format!("{role}.md")),
            dir.path().join(format!("docs/{role}.md")),
        )?;
    }
    let result = report(dir.path(), &["adoption", "check", "--preflight"], 1)?;
    assert_eq!(result["product_verification"], "not_run");
    assert_eq!(
        result["design_documents"]["recorded_decisions_complete"],
        false
    );
    Ok(())
}

#[test]
fn document_file_owner_is_not_silently_treated_as_a_directory() -> TestResult {
    let dir = fixture(false)?;
    let config = dir.path().join(".opdev/project.yaml");
    let mut value: Value = serde_json::from_slice(&fs::read(&config)?)?;
    value["authorities"]["documentation"] = json!("DESIGN.MD");
    fs::write(&config, serde_json::to_vec(&value)?)?;
    let result = report(dir.path(), &["documentation", "status"], 1)?;
    assert!(
        result["documents"][0]["finding"]
            .as_str()
            .ok_or("finding")?
            .contains("declare a separate")
    );
    assert!(!dir.path().join("DESIGN.MD").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn linked_document_is_not_followed() -> TestResult {
    let dir = fixture(false)?;
    let external = tempfile::tempdir()?;
    fs::write(
        external.path().join("design.md"),
        "private external content",
    )?;
    std::os::unix::fs::symlink(
        external.path().join("design.md"),
        dir.path().join("docs/design.md"),
    )?;
    let result = report(dir.path(), &["documentation", "status"], 1)?;
    let finding = result["documents"][0]["finding"]
        .as_str()
        .ok_or("finding")?;
    assert!(finding.contains("Linked path"));
    assert!(!finding.contains("private external content"));
    Ok(())
}

#[test]
fn both_host_routes_and_adoption_reach_real_packaged_documents() -> TestResult {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/opdev/skills/opdev");
    for route in [
        "planning",
        "work-boundaries",
        "adoption",
        "practical-workflow",
    ] {
        let text = fs::read_to_string(source.join(format!("references/{route}.md")))?;
        assert!(text.contains("](project-design.md)"));
    }
    assert!(source.join("references/project-design.md").is_file());
    for role in ["design", "interface", "tech_stack"] {
        assert!(
            source
                .join(format!("assets/project-documents/{role}.md"))
                .is_file()
        );
    }
    Ok(())
}
