//! Destination checks exercise real contracts and filesystem effects, not prose meaning.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use opdev_project::{AuthorityKind, AuthorityRef, MANIFEST_PATH, discover};
use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture(work: AuthorityRef) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(dir.path())
            .status()?
            .success()
    );
    fs::create_dir_all(dir.path().join("knowledge"))?;
    fs::write(
        dir.path().join("knowledge/system.md"),
        "Original durable behavior\n",
    )?;
    let mut project = discover(dir.path())?.manifest;
    project.authorities.clear();
    project.context.always.clear();
    project.context.routes.clear();
    project.authorities.insert(
        "architecture".into(),
        AuthorityRef {
            kind: AuthorityKind::Path,
            location: "knowledge".into(),
        },
    );
    project.authorities.insert("work".into(), work);
    // A routing operation must never execute this observable local mutation.
    for command in project.commands.values_mut() {
        command.argv = vec![
            "git".into(),
            "config".into(),
            "--local".into(),
            "opdev.route-executed".into(),
            "true".into(),
        ];
    }
    project.write_new(&dir.path().join(MANIFEST_PATH))?;
    Ok(dir)
}

fn external() -> AuthorityRef {
    AuthorityRef {
        kind: AuthorityKind::Tracker,
        location: "tracker:project:123".into(),
    }
}

fn run(root: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(["documentation", "plan"])
        .args(args)
        .output()
}

fn json(root: &Path, args: &[&str], code: i32) -> Result<Value, Box<dyn std::error::Error>> {
    let mut arguments = args.to_vec();
    arguments.extend(["--format", "json"]);
    let output = run(root, &arguments)?;
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
fn mixed_routes_preserve_authorities_without_creating_a_local_backlog() -> TestResult {
    let dir = fixture(external())?;
    let contract = fs::read(dir.path().join(MANIFEST_PATH))?;
    let git_config = fs::read(dir.path().join(".git/config"))?;
    let report = json(
        dir.path(),
        &["--purpose", "mixed", "--authority", "architecture"],
        0,
    )?;
    assert_eq!(report["schema"], 1);
    assert_eq!(report["routes"][0]["declared"]["location"], "knowledge");
    assert_eq!(
        report["routes"][1]["declared"]["location"],
        "tracker:project:123"
    );
    assert_eq!(
        report["routes"].as_array().ok_or("routes missing")?.len(),
        2
    );
    assert_eq!(contract, fs::read(dir.path().join(MANIFEST_PATH))?);
    assert_eq!(git_config, fs::read(dir.path().join(".git/config"))?);
    assert_eq!(
        fs::read_to_string(dir.path().join("knowledge/system.md"))?,
        "Original durable behavior\n"
    );
    let mut names = fs::read_dir(dir.path())?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    assert_eq!(names, vec![".git", ".opdev", "knowledge"]);
    Ok(())
}

#[test]
fn custom_document_and_explicit_repository_tracker_are_valid() -> TestResult {
    let dir = fixture(AuthorityRef {
        kind: AuthorityKind::Path,
        location: "knowledge/system.md".into(),
    })?;
    let work = json(
        dir.path(),
        &["--purpose", "work", "--target", "knowledge/system.md"],
        0,
    )?;
    assert_eq!(work["routes"][0]["authority"], "work");
    for target in [
        "knowledge/system.md",
        "knowledge/new-guide.md",
        "knowledge/phase/plan.md",
        "knowledge\\new.md",
    ] {
        json(
            dir.path(),
            &[
                "--purpose",
                "durable",
                "--authority",
                "architecture",
                "--target",
                target,
            ],
            0,
        )?;
    }
    assert!(!dir.path().join("knowledge/phase").exists());
    assert!(!dir.path().join("knowledge/new-guide.md").exists());
    Ok(())
}

#[test]
fn outside_paths_traversal_and_file_children_are_unresolved() -> TestResult {
    let dir = fixture(external())?;
    for target in [
        "knowledge-other/plan.md",
        "docs/plan.md",
        "../knowledge/a.md",
        "C:\\knowledge\\a.md",
        "/knowledge/a.md",
        "knowledge/../a.md",
        "knowledge\\..\\a.md",
        "knowledge/a.md:stream",
        "knowledge/trailing./a.md",
        "knowledge/system.md/child",
    ] {
        let report = json(
            dir.path(),
            &[
                "--purpose",
                "durable",
                "--authority",
                "architecture",
                "--target",
                target,
            ],
            1,
        )?;
        assert_eq!(report["resolved"], false, "{target}");
    }
    Ok(())
}

#[test]
fn missing_owner_and_external_membership_are_not_inferred() -> TestResult {
    let dir = fixture(external())?;
    json(
        dir.path(),
        &["--purpose", "durable", "--authority", "unknown"],
        1,
    )?;
    json(
        dir.path(),
        &["--purpose", "work", "--target", "tracker:project:1234"],
        1,
    )?;
    json(
        dir.path(),
        &["--purpose", "work", "--target", "tracker:project:123"],
        0,
    )?;
    fs::remove_file(dir.path().join("knowledge/system.md"))?;
    fs::remove_dir(dir.path().join("knowledge"))?;
    json(
        dir.path(),
        &[
            "--purpose",
            "durable",
            "--authority",
            "architecture",
            "--target",
            "knowledge/new.md",
        ],
        1,
    )?;
    assert!(!dir.path().join("knowledge").exists());
    Ok(())
}

#[test]
fn invalid_classification_arguments_are_errors_and_temporary_needs_no_document() -> TestResult {
    let dir = fixture(external())?;
    for args in [
        vec!["--purpose", "durable"],
        vec![
            "--purpose",
            "mixed",
            "--authority",
            "architecture",
            "--target",
            "knowledge/system.md",
        ],
        vec!["--purpose", "temporary", "--target", "notes.md"],
        vec!["--purpose", "work", "--authority", "architecture"],
        vec!["--purpose", "durable", "--authority", "work"],
    ] {
        assert_eq!(run(dir.path(), &args)?.status.code(), Some(2));
    }
    let report = json(dir.path(), &["--purpose", "temporary"], 0)?;
    assert_eq!(report["routes"], serde_json::json!([]));
    let human = run(dir.path(), &["--purpose", "work"])?;
    let text = String::from_utf8(human.stdout)?;
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("not verification or approval"));
    let unsafe_label = run(
        dir.path(),
        &["--purpose", "durable", "--authority", "bad\u{1b}[31m"],
    )?;
    assert_eq!(unsafe_label.status.code(), Some(1));
    assert!(!String::from_utf8(unsafe_label.stdout)?.contains('\u{1b}'));
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_destinations_require_review_without_reading_or_mutating_targets() -> TestResult {
    let dir = fixture(external())?;
    let outside = tempfile::tempdir()?;
    fs::write(outside.path().join("private.md"), "private")?;
    std::os::unix::fs::symlink(outside.path(), dir.path().join("knowledge/link"))?;
    json(
        dir.path(),
        &[
            "--purpose",
            "durable",
            "--authority",
            "architecture",
            "--target",
            "knowledge/link/private.md",
        ],
        1,
    )?;
    assert_eq!(
        fs::read_to_string(outside.path().join("private.md"))?,
        "private"
    );
    Ok(())
}
