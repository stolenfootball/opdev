//! Read-only CLI behavior and published schemas, not fabricated execution receipts.
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn git(root: &Path, args: &[&str]) -> Result<Output> {
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
    Ok(result)
}
fn cli(root: &Path, args: &[&str]) -> Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(["requirements"])
        .args(args)
        .output()?)
}

#[test]
fn catalog_cli_never_executes_or_writes_and_diff_exposes_removed_guarantees() -> Result {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    git(root, &["init", "-q"])?;
    fs::create_dir_all(root.join(".opdev/requirements"))?;
    let mut doc = json!({"schema":1,"requirements":[{"id":"R1","title":"No data loss","statement":{"kind":"inline","text":"Rejected input preserves data"},"origin":"fixture accepted promise","rationale":"Integrity","configurations":["default"],"criteria":[]}],"verifications":[],"plans":[]});
    let path = root.join(".opdev/requirements/storage.json");
    fs::write(&path, serde_json::to_vec(&doc)?)?;
    fs::write(root.join("assertion.txt"), "assert actual == expected\n")?;
    git(root, &["add", "."])?;
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "accepted baseline",
        ],
    )?;
    let show = cli(root, &["show", "R1"])?;
    assert!(show.status.success());
    let value: Value = serde_json::from_slice(&show.stdout)?;
    assert_eq!(value["qualification"], "unverified");
    assert_eq!(value["complete"], true);
    assert_eq!(value["requirement"]["id"], "R1");
    let bound = cli(
        root,
        &[
            "bind",
            "assertion.txt",
            "--excerpt",
            "assert actual == expected",
        ],
    )?;
    assert!(bound.status.success());
    let bound: Value = serde_json::from_slice(&bound.stdout)?;
    assert_eq!(bound["path"], "assertion.txt");
    assert_eq!(bound["sha256"].as_str().ok_or("hash")?.len(), 64);
    assert!(
        git(root, &["status", "--porcelain"])?.stdout.is_empty(),
        "readers leave source/index unchanged"
    );
    doc["requirements"] = json!([]);
    fs::write(&path, serde_json::to_vec(&doc)?)?;
    git(root, &["add", "."])?;
    let diff = cli(root, &["diff", "--base", "HEAD"])?;
    assert!(diff.status.success());
    let diff: Value = serde_json::from_slice(&diff.stdout)?;
    assert_eq!(diff["removed"], json!(["R1"]));
    assert_eq!(diff["qualification"], "unverified");
    assert_eq!(
        diff["base_catalog_sha256"]
            .as_str()
            .ok_or("baseline digest")?
            .len(),
        64
    );
    assert_eq!(
        diff["candidate_catalog_sha256"]
            .as_str()
            .ok_or("candidate digest")?
            .len(),
        64
    );
    assert_ne!(
        diff["base_catalog_sha256"],
        diff["candidate_catalog_sha256"]
    );
    assert!(!cli(root, &["show", "R1"])?.status.success());
    assert!(
        !cli(root, &["bind", "../outside", "--excerpt", "anything"])?
            .status
            .success()
    );
    Ok(())
}

#[test]
fn schema_is_offline_and_invalid_catalog_is_not_partial_success() -> Result {
    let temp = tempfile::tempdir()?;
    let schema = cli(temp.path(), &["schema"])?;
    assert!(schema.status.success());
    let value: Value = serde_json::from_slice(&schema.stdout)?;
    let validator = jsonschema::validator_for(&value)?;
    assert!(
        validator.is_valid(&json!({"schema":1,"requirements":[],"verifications":[],"plans":[]}))
    );
    assert!(!validator.is_valid(
        &json!({"schema":1,"requirements":[],"verifications":[],"plans":[],"execution":"passed"})
    ));
    git(temp.path(), &["init", "-q"])?;
    fs::create_dir_all(temp.path().join(".opdev/requirements"))?;
    fs::write(
        temp.path().join(".opdev/requirements/bad.json"),
        "{\"schema\":1,\"schema\":99}",
    )?;
    git(temp.path(), &["add", "."])?;
    assert!(!cli(temp.path(), &["show", "R1"])?.status.success());
    Ok(())
}

#[test]
fn published_authoring_example_is_valid_but_never_preapproved() -> Result {
    let guide = include_str!("../../../docs/requirements-and-verification.md");
    let lf = guide.replace("\r\n", "\n");
    for checkout in [&lf, &lf.replace('\n', "\r\n")] {
        check_authoring_example(checkout)?;
    }
    Ok(())
}

fn check_authoring_example(guide: &str) -> Result {
    let guide = guide.replace("\r\n", "\n");
    let example = guide
        .split_once("```json\n")
        .ok_or("example")?
        .1
        .split_once("\n```")
        .ok_or("example end")?
        .0;
    let doc = opdev_project::requirements::CatalogDocument::parse(example.as_bytes())?;
    assert_eq!(doc.requirements.len(), 1);
    assert_eq!(doc.plans.len(), 2);
    assert!(
        doc.plans
            .iter()
            .all(|p| p.review.outcome == opdev_core::Outcome::Unverified
                && p.review.subject_sha256.is_empty())
    );
    assert!(guide.contains("not a qualification record"));
    Ok(())
}
