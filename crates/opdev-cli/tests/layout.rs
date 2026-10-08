//! Proposed layout inspection never selects policy, executes commands or cleans files.
use opdev_project::{ADOPTION_PATH, AdoptionRecord, MANIFEST_PATH, discover};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

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
