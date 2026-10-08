//! Proposed layout inspection never selects policy, executes commands or cleans files.
use opdev_project::{ADOPTION_PATH, AdoptionRecord, MANIFEST_PATH, discover};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn init_strict(root: &Path, extra: &[&str]) -> Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args([
            "init",
            "--engineering-policy",
            "1",
            "--minimumcd-assessment",
            "none",
            "--policy-review-reference",
            "synthetic developer decision",
            "--layout-version",
            "1",
        ])
        .args(extra)
        .output()?)
}

fn policy_result(root: &Path) -> Result<Value> {
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(["check", "--ci", "--no-exec", "--format", "json"])
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let schema: Value = serde_json::from_str(include_str!("../../../schema/report.schema.json"))?;
    assert!(jsonschema::validator_for(&schema)?.is_valid(&report));
    let check = report["checks"]
        .as_array()
        .ok_or("missing checks")?
        .iter()
        .find(|c| c["kind"] == "policy")
        .ok_or("missing policy check")?;
    assert_eq!(check["id"], "opdev-layout");
    assert_eq!(check["blocking"], true);
    assert!(
        check["gates"]
            .as_array()
            .ok_or("missing gates")?
            .iter()
            .any(|g| g == "integration")
    );
    // Layout conformance is not adoption, execution or a general gate pass.
    let integration = report["gates"]
        .as_array()
        .ok_or("missing gates")?
        .iter()
        .find(|g| g["gate"] == "integration")
        .ok_or("missing integration gate")?;
    assert_eq!(integration["verdict"], "blocked");
    let blocked = integration["blocking_checks"]
        .as_array()
        .ok_or("missing blockers")?;
    assert_eq!(
        blocked.iter().any(|id| id == "opdev-layout"),
        check["outcome"] != "passed"
    );
    Ok(check.clone())
}

#[test]
fn explicit_layout_has_thin_entries_shared_guide_and_index_bound_structural_check() -> Result {
    let repo = tempfile::tempdir()?;
    let root = repo.path();
    git(root, &["init", "--quiet"])?;
    fs::write(root.join("AGENTS.md"), "Project-owned rules\n")?;
    fs::write(root.join("CLAUDE.md"), "@AGENTS.md\nCustom host rules\n")?;
    let dry = init_strict(root, &["--dry-run"])?;
    assert!(
        dry.status.success(),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
    assert!(!root.join(".opdev").exists());
    let initialized = init_strict(root, &[])?;
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let guide = fs::read(root.join(".opdev/guidance.md"))?;
    for file in ["AGENTS.md", "CLAUDE.md"] {
        let content = fs::read_to_string(root.join(file))?;
        assert!(content.len() < 900);
        assert!(content.contains(".opdev/guidance.md"));
        assert!(content.contains("context"));
        assert!(!content.contains("MinimumCD requirements"));
    }
    assert!(fs::read_to_string(root.join("AGENTS.md"))?.starts_with("Project-owned rules\n"));
    assert!(
        fs::read_to_string(root.join("CLAUDE.md"))?.starts_with("@AGENTS.md\nCustom host rules\n")
    );
    assert!(guide.len() > 8000);
    assert!(!root.join(".opdev/docs").exists());
    assert!(init_strict(root, &[])?.status.success());
    assert_eq!(guide, fs::read(root.join(".opdev/guidance.md"))?);
    git(root, &["add", "."])?;
    assert_eq!(policy_result(root)?["outcome"], "passed");

    fs::write(
        root.join(".opdev/scratch.md"),
        "work tracking does not belong here\n",
    )?;
    assert_eq!(
        policy_result(root)?["outcome"],
        "passed",
        "untracked content is not committed layout"
    );
    git(root, &["add", ".opdev/scratch.md"])?;
    assert_eq!(policy_result(root)?["outcome"], "failed");
    fs::remove_file(root.join(".opdev/scratch.md"))?;
    assert_eq!(
        policy_result(root)?["outcome"],
        "failed",
        "unstaged deletion cannot hide committed junk"
    );
    git(root, &["add", "-u"])?;
    fs::write(
        root.join(".opdev/guidance.md"),
        "## OpDev\nShared guidance format: 1\n",
    )?;
    git(root, &["add", ".opdev/guidance.md"])?;
    let failed = policy_result(root)?;
    assert_eq!(failed["outcome"], "failed");
    assert!(
        failed["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("guidance")
    );
    fs::write(root.join(".opdev/guidance.md"), &guide)?;
    assert_eq!(
        policy_result(root)?["outcome"],
        "failed",
        "unstaged repair is not the selected source"
    );
    git(root, &["add", ".opdev/guidance.md"])?;
    assert_eq!(policy_result(root)?["outcome"], "passed");
    let entry = git(root, &["ls-files", "--stage", "--", "AGENTS.md"])?;
    let entry = String::from_utf8(entry.stdout)?;
    let object = entry.split_whitespace().nth(1).ok_or("missing root blob")?;
    git(
        root,
        &[
            "update-index",
            "--cacheinfo",
            &format!("120000,{object},AGENTS.md"),
        ],
    )?;
    assert_eq!(
        policy_result(root)?["outcome"],
        "failed",
        "regular Windows bytes cannot override the staged link mode"
    );
    // An unreviewed upgrade cannot follow that placeholder to another file.
    let blocked = init_strict(root, &[])?;
    assert!(!blocked.status.success());
    assert_eq!(guide, fs::read(root.join(".opdev/guidance.md"))?);
    Ok(())
}

#[test]
fn legacy_init_does_not_select_layout_and_cannot_be_used_as_migration() -> Result {
    let repo = fixture()?;
    let root = repo.path();
    let before = fs::read(root.join(MANIFEST_PATH))?;
    let output = init_strict(root, &[])?;
    assert!(!output.status.success());
    assert_eq!(before, fs::read(root.join(MANIFEST_PATH))?);
    assert!(!root.join("AGENTS.md").exists());
    let mut manifest = opdev_project::ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    manifest.layout = Some(opdev_project::LayoutPolicy {
        version: 1,
        review_reference: "synthetic".into(),
    });
    assert!(
        manifest.to_yaml().is_err(),
        "legacy schema cannot silently acquire layout enforcement"
    );
    Ok(())
}

#[test]
fn formatting_is_reviewed_deterministic_and_preserves_meaning_without_adoption_approval() -> Result
{
    let repo = fixture()?;
    let root = repo.path();
    let manifest = opdev_project::ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    let adoption = AdoptionRecord::load(root)?.ok_or("missing adoption")?;
    let original = format!(
        "# Deliberately retained until reviewed\n{}",
        manifest.to_yaml()?
    );
    fs::write(root.join(MANIFEST_PATH), &original)?;
    let invoke = |extra: &[&str]| -> Result<Output> {
        Ok(Command::new(env!("CARGO_BIN_EXE_opdev"))
            .current_dir(root)
            .args(["layout", "format"])
            .args(extra)
            .output()?)
    };
    let preview = invoke(&[])?;
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let preview: Value = serde_json::from_slice(&preview.stdout)?;
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schema/configuration-format.schema.json"
    ))?;
    assert!(jsonschema::validator_for(&schema)?.is_valid(&preview));
    assert_eq!(preview["applied"], false);
    assert_eq!(fs::read_to_string(root.join(MANIFEST_PATH))?, original);
    let id = preview["plan_id"].as_str().ok_or("missing plan id")?;
    let applied = invoke(&["--apply", id])?;
    assert!(applied.status.success());
    assert_eq!(
        manifest,
        opdev_project::ProjectManifest::load(&root.join(MANIFEST_PATH))?
    );
    assert_eq!(
        serde_json::to_value(&adoption)?,
        serde_json::to_value(AdoptionRecord::load(root)?)?
    );
    let stable: Value = serde_json::from_slice(&invoke(&[])?.stdout)?;
    assert!(
        stable["changes"]
            .as_array()
            .ok_or("missing changes")?
            .iter()
            .all(|c| c["changed"] == false)
    );
    assert!(
        !invoke(&["--apply", id])?.status.success(),
        "old pre-format bytes are no longer current"
    );
    let stable_id = stable["plan_id"].as_str().ok_or("missing stable id")?;
    assert!(invoke(&["--apply", stable_id])?.status.success());
    assert!(invoke(&["--apply", stable_id])?.status.success());
    fs::write(
        root.join(ADOPTION_PATH),
        format!("{}\nunknown: true\n", adoption.to_yaml()?),
    )?;
    assert!(!invoke(&[])?.status.success());
    assert!(!invoke(&["--apply", stable_id])?.status.success());
    assert_eq!(
        manifest.to_yaml()?,
        fs::read_to_string(root.join(MANIFEST_PATH))?
    );
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<Output> {
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
    Ok(output)
}

fn fixture() -> Result<tempfile::TempDir> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    git(root, &["init", "--quiet"])?;
    let mut project = discover(root)?.manifest;
    project.commands.insert(
        "check".into(),
        opdev_project::CommandSpec {
            argv: vec![
                "git".into(),
                "config".into(),
                "opdev.executed".into(),
                "true".into(),
            ],
            working_directory: None,
            timeout_seconds: Some(10),
        },
    );
    project.testing.suites = vec![opdev_project::TestSuite {
        id: "behavior".into(),
        command: "check".into(),
        stages: vec![opdev_project::TestStage::Local],
    }];
    project.write_new(&root.join(MANIFEST_PATH))?;
    AdoptionRecord::pending()?.write_new(root)?;
    fs::write(
        root.join(".opdev/guidance.md"),
        "Proposed shared guide; content not certified by layout inspection.\n",
    )?;
    fs::create_dir_all(root.join("outside-docs"))?;
    fs::write(
        root.join("outside-docs/design.md"),
        "Existing project authority\n",
    )?;
    git(root, &["add", "."])?;
    Ok(temp)
}

fn cli(root: &Path, args: &[&str]) -> Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args([
            "layout",
            "inspect",
            "--layout-version",
            "1",
            "--format",
            "json",
        ])
        .args(args)
        .output()?)
}

fn report(root: &Path, args: &[&str], code: i32) -> Result<Value> {
    let output = cli(root, args)?;
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["qualification"], "unverified");
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schema/layout-inspection.schema.json"
    ))?;
    assert!(jsonschema::validator_for(&schema)?.is_valid(&value));
    Ok(value)
}

#[test]
fn minimal_and_optional_durable_layouts_remain_read_only_and_unqualified() -> Result {
    let repo = fixture()?;
    let root = repo.path();
    for directory in [
        ".opdev/docs/specs",
        ".opdev/docs/decisions",
        ".opdev/docs/assets",
    ] {
        fs::create_dir_all(root.join(directory))?;
    }
    for name in [
        "design.md",
        "development.md",
        "testing.md",
        "delivery.md",
        "specs/export.md",
        "decisions/001-storage.md",
    ] {
        fs::write(
            root.join(".opdev/docs").join(name),
            "Durable requirements, not certified from headings.\n",
        )?;
    }
    fs::write(root.join(".opdev/docs/assets/diagram.svg"), "<svg/>\n")?;
    let status = git(root, &["status", "--porcelain=v1", "-uall"])?.stdout;
    let config = fs::read(root.join(".git/config"))?;
    let manifest = fs::read(root.join(MANIFEST_PATH))?;
    let result = report(root, &[], 0)?;
    assert_eq!(result["findings"], serde_json::json!([]));
    assert_eq!(
        result["inspected_files"].as_array().ok_or("files")?.len(),
        10
    );
    assert_eq!(report(&root.join("outside-docs"), &[], 0)?, result);
    assert_eq!(fs::read(root.join(MANIFEST_PATH))?, manifest);
    assert_eq!(fs::read(root.join(".git/config"))?, config);
    assert_eq!(
        git(root, &["status", "--porcelain=v1", "-uall"])?.stdout,
        status
    );
    assert_eq!(
        fs::read_to_string(root.join("outside-docs/design.md"))?,
        "Existing project authority\n"
    );
    Ok(())
}

#[test]
fn local_inventory_sees_ignored_junk_while_index_uses_only_staged_files() -> Result {
    let repo = fixture()?;
    let root = repo.path();
    fs::write(root.join(".gitignore"), ".opdev/transcript.txt\n")?;
    fs::write(
        root.join(".opdev/transcript.txt"),
        "Do not expose these contents\n",
    )?;
    fs::write(
        root.join(".opdev/evidence.yaml"),
        "original historical proof\n",
    )?;
    fs::create_dir(root.join(".opdev/scratch"))?;
    fs::write(
        root.join(".opdev/scratch/private.txt"),
        "Do not read scratch contents\n",
    )?;
    let value = report(root, &[], 1)?;
    let findings = value["findings"].to_string();
    assert!(findings.contains(".opdev/transcript.txt"));
    assert!(findings.contains(".opdev/evidence.yaml"));
    assert!(findings.contains(".opdev/scratch"));
    assert!(!value.to_string().contains("original historical proof"));
    assert!(!value.to_string().contains("Do not expose"));
    assert_eq!(
        report(root, &["--scope", "index"], 0)?["findings"],
        serde_json::json!([])
    );
    assert_eq!(
        fs::read_to_string(root.join(".opdev/evidence.yaml"))?,
        "original historical proof\n"
    );
    Ok(())
}

#[test]
fn malformed_versions_duplicates_and_unknown_fields_are_not_repaired() -> Result {
    let repo = fixture()?;
    let root = repo.path();
    let original = fs::read_to_string(root.join(MANIFEST_PATH))?;
    for yaml in [
        format!("{original}\nschema: 1\n"),
        original.replacen("schema: 1", "schema: 99", 1),
        format!("{original}\ninvented: true\n"),
    ] {
        fs::write(root.join(MANIFEST_PATH), &yaml)?;
        let value = report(root, &[], 1)?;
        assert_eq!(value["findings"][0]["path"], MANIFEST_PATH);
        assert_eq!(fs::read_to_string(root.join(MANIFEST_PATH))?, yaml);
        report(root, &["--scope", "index"], 0)?;
    }
    fs::write(root.join(MANIFEST_PATH), original)?;
    fs::write(root.join(ADOPTION_PATH), "schema: 999\n")?;
    assert_eq!(report(root, &[], 1)?["findings"][0]["path"], ADOPTION_PATH);
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(["layout", "inspect", "--layout-version", "99"])
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Unsupported proposed layout version")
    );
    Ok(())
}

#[test]
fn indexed_link_is_rejected_even_when_windows_represents_it_as_regular_text() -> Result {
    let repo = fixture()?;
    let root = repo.path();
    let hash = String::from_utf8(git(root, &["rev-parse", ":.opdev/guidance.md"])?.stdout)?;
    git(
        root,
        &[
            "update-index",
            "--cacheinfo",
            &format!("120000,{},.opdev/guidance.md", hash.trim()),
        ],
    )?;
    for scope in ["working-tree", "index"] {
        let value = report(root, &["--scope", scope], 1)?;
        assert_eq!(value["findings"][0]["path"], ".opdev/guidance.md");
        assert_eq!(
            value["findings"][0]["problem"],
            "Not a regular resolved file"
        );
    }
    Ok(())
}

#[test]
fn missing_required_file_is_not_replaced_with_scaffolding() -> Result {
    let repo = fixture()?;
    let root = repo.path();
    fs::remove_file(root.join(".opdev/guidance.md"))?;
    let value = report(root, &[], 1)?;
    assert_eq!(value["findings"][0]["path"], ".opdev/guidance.md");
    assert!(!root.join(".opdev/guidance.md").exists());
    report(root, &["--scope", "index"], 0)?;
    Ok(())
}

#[test]
fn linked_worktree_uses_its_own_staged_namespace() -> Result {
    let repo = fixture()?;
    let worktree_parent = tempfile::tempdir()?;
    let linked = worktree_parent.path().join("linked");
    git(
        repo.path(),
        &[
            "-c",
            "user.name=Layout Test",
            "-c",
            "user.email=layout@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    )?;
    git(
        repo.path(),
        &[
            "worktree",
            "add",
            "--detach",
            linked.to_str().ok_or("path")?,
        ],
    )?;
    fs::write(
        linked.join(".opdev/backlog.md"),
        "Work belongs in its authority\n",
    )?;
    git(&linked, &["add", ".opdev/backlog.md"])?;
    assert_eq!(
        report(&linked, &["--scope", "index"], 1)?["findings"][0]["path"],
        ".opdev/backlog.md"
    );
    assert_eq!(
        report(repo.path(), &["--scope", "index"], 0)?["findings"],
        serde_json::json!([])
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn physical_directory_link_is_not_traversed() -> Result {
    let repo = fixture()?;
    let outside = tempfile::tempdir()?;
    fs::write(outside.path().join("private.md"), "outside content\n")?;
    std::os::unix::fs::symlink(outside.path(), repo.path().join(".opdev/docs"))?;
    let value = report(repo.path(), &[], 1)?;
    assert_eq!(value["findings"][0]["path"], ".opdev/docs");
    assert!(!value.to_string().contains("private.md"));
    assert!(!value.to_string().contains("outside content"));
    Ok(())
}
