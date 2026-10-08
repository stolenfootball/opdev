//! Existing-ledger updates calculate bindings, not approval or qualification.
use opdev_project::{EVIDENCE_PATH, MANIFEST_PATH, discover};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn git(root: &Path, args: &[&str]) -> Result {
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()?
            .status
            .success()
    );
    Ok(())
}

fn cli(root: &Path, args: &[&str]) -> Result<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_opdev"))
        .args(["evidence", "prepare", "--root"])
        .arg(root)
        .args(args)
        .output()?)
}

fn parse(output: &Output) -> Result<Value> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_saphyr::from_slice(&output.stdout)?)
}

fn fixture() -> Result<(tempfile::TempDir, tempfile::TempDir)> {
    let repo = tempfile::tempdir()?;
    let scratch = tempfile::tempdir()?;
    let root = repo.path();
    git(root, &["init", "--quiet"])?;
    discover(root)?
        .manifest
        .write_new(&root.join(MANIFEST_PATH))?;
    fs::write(root.join("contract.md"), "Preserve the supplied order.\n")?;
    fs::write(
        root.join("test.py"),
        "assert ['b', 'a'] != sorted(['b', 'a'])\n",
    )?;
    fs::write(
        root.join(EVIDENCE_PATH),
        serde_json::to_vec(&json!({"schema":2, "project":[], "changes":[
            {"fingerprint":"a".repeat(64),"work":"history", "assertions":[]}
        ]}))?,
    )?;
    git(root, &["add", "."])?;
    let input = json!({"scope":"behavioral", "rationale":"Order only", "conditions":[{
        "id":"order", "statement":"Preserve order", "authority":"contract.md",
        "source":{"path":"contract.md","excerpt":"Preserve the supplied order."}
    }], "verifications":[{
        "condition":"order", "method":"review", "target":{"path":"test.py","excerpt":"assert ['b', 'a'] != sorted(['b', 'a'])"},
        "assertion":"Counterexample distinguishes sorting", "discriminating_case":"b,a differs from a,b",
        "automation_limitation":"Synthetic preparation fixture, not a behavior qualification"
    }]});
    fs::write(
        scratch.path().join("input.json"),
        serde_json::to_vec(&input)?,
    )?;
    Ok((repo, scratch))
}

fn draft(root: &Path, scratch: &Path) -> Result<Value> {
    parse(&cli(
        root,
        &[
            "--input",
            scratch.join("input.json").to_str().ok_or("path")?,
            "--work",
            "fixture",
        ],
    )?)
}

fn preview(root: &Path, path: &Path, draft: &Value) -> Result<Value> {
    fs::write(path, serde_json::to_vec(draft)?)?;
    parse(&cli(root, &["--draft", path.to_str().ok_or("path")?])?)
}

#[test]
fn unresolved_draft_needs_current_review_and_preserves_history() -> Result {
    let (repo, scratch) = fixture()?;
    let root = repo.path();
    let path = scratch.path().join("draft.json");
    let mut draft = draft(root, scratch.path())?;
    assert_eq!(
        draft["acceptance"]["verifications"][0]["outcome"],
        "unverified"
    );
    assert_eq!(draft["acceptance"]["review"]["outcome"], "unverified");
    assert_eq!(
        draft["acceptance"]["conditions"][0]["source"]["sha256"]
            .as_str()
            .ok_or("sha")?
            .len(),
        64
    );
    let before = fs::read(root.join(EVIDENCE_PATH))?;
    preview(root, &path, &draft)?;
    assert!(
        !cli(root, &["--draft", path.to_str().ok_or("path")?, "--write"])?
            .status
            .success()
    );
    assert_eq!(fs::read(root.join(EVIDENCE_PATH))?, before);
    draft["acceptance"]["verifications"][0]["outcome"] = json!("passed");
    let plan = preview(root, &path, &draft)?;
    draft["acceptance"]["review"] = json!({"outcome":"passed","reviewer":"synthetic test reviewer", "reference":"fixture review", "rationale":"Prepared counterexample inspected", "subject_sha256":plan["subject_sha256"]});
    let mut changed = draft.clone();
    changed["acceptance"]["verifications"][0]["assertion"] = json!("Changed mapping");
    assert_eq!(preview(root, &path, &changed)?["review_current"], false);
    assert!(
        !cli(root, &["--draft", path.to_str().ok_or("path")?, "--write"])?
            .status
            .success()
    );
    assert_eq!(preview(root, &path, &draft)?["review_current"], true);
    assert!(
        cli(root, &["--draft", path.to_str().ok_or("path")?, "--write"])?
            .status
            .success()
    );
    let after: Value = serde_saphyr::from_slice(&fs::read(root.join(EVIDENCE_PATH))?)?;
    let old: Value = serde_saphyr::from_slice(&before)?;
    assert_eq!(after["project"], old["project"]);
    assert_eq!(after["changes"][0], old["changes"][0]);
    assert_eq!(after["changes"].as_array().ok_or("changes")?.len(), 2);
    assert!(
        !cli(root, &["--draft", path.to_str().ok_or("path")?, "--write"])?
            .status
            .success(),
        "old ledger snapshot accepted"
    );
    Ok(())
}

#[test]
fn retained_drafts_use_private_state_and_always_start_unreviewed() -> Result {
    let (repo, scratch) = fixture()?;
    let root = repo.path();
    let input = scratch.path().join("input.json");
    let state = scratch.path().join("state");
    let before = fs::read(root.join(EVIDENCE_PATH))?;
    let mut original: Value = serde_json::from_slice(&fs::read(&input)?)?;
    original["verifications"][0]["outcome"] = json!("passed");
    original["review"] = json!({"outcome":"passed","reviewer":"invented"});
    fs::write(&input, serde_json::to_vec(&original)?)?;
    let mut paths = Vec::new();
    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
            .current_dir(root)
            .env("OPDEV_STATE_DIR", &state)
            .args([
                "evidence",
                "prepare",
                "--retain-draft",
                "--work",
                "fixture",
                "--input",
            ])
            .arg(&input)
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let saved: Value = serde_json::from_slice(&output.stdout)?;
        let path = std::path::PathBuf::from(saved["draft"].as_str().ok_or("draft")?);
        assert!(path.starts_with(state.canonicalize()?));
        let draft: Value = serde_saphyr::from_slice(&fs::read(&path)?)?;
        assert_eq!(draft["acceptance"]["review"]["outcome"], "unverified");
        assert_eq!(
            draft["acceptance"]["verifications"][0]["outcome"],
            "unverified"
        );
        assert!(
            !cli(root, &["--draft", path.to_str().ok_or("path")?, "--write"])?
                .status
                .success()
        );
        paths.push(path);
    }
    assert_ne!(paths[0], paths[1]);
    assert_eq!(fs::read(root.join(EVIDENCE_PATH))?, before);
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["ls-files", "--others", "--exclude-standard"])
            .output()?
            .stdout
            .is_empty()
    );
    Ok(())
}

#[test]
fn source_and_ledger_changes_and_bad_excerpts_do_not_write() -> Result {
    let (repo, scratch) = fixture()?;
    let root = repo.path();
    let path = scratch.path().join("draft.json");
    let mut draft = draft(root, scratch.path())?;
    let before = fs::read(root.join(EVIDENCE_PATH))?;
    draft["acceptance"]["conditions"][0]["source"]["excerpt"] = json!("not in the source");
    fs::write(&path, serde_json::to_vec(&draft)?)?;
    assert!(
        !cli(root, &["--draft", path.to_str().ok_or("path")?, "--write"])?
            .status
            .success()
    );
    assert_eq!(fs::read(root.join(EVIDENCE_PATH))?, before);
    draft["acceptance"]["conditions"][0]["source"]["excerpt"] =
        json!("Preserve the supplied order.");
    fs::write(&path, serde_json::to_vec(&draft)?)?;
    fs::write(root.join("contract.md"), "Changed\n")?;
    git(root, &["add", "contract.md"])?;
    assert!(
        !cli(root, &["--draft", path.to_str().ok_or("path")?, "--write"])?
            .status
            .success()
    );
    assert_eq!(fs::read(root.join(EVIDENCE_PATH))?, before);
    Ok(())
}

#[test]
fn existing_current_assertions_survive_preparation_and_bad_input_is_rejected() -> Result {
    let (repo, scratch) = fixture()?;
    let root = repo.path();
    let path = scratch.path().join("draft.json");
    let initial = draft(root, scratch.path())?;
    let assertion = json!({"rule_id":"OPDEV-WORK-001", "outcome":"passed", "summary":"Synthetic work assertion",
        "evidence":[{"kind":"review", "summary":"Synthetic retained fact", "location":"fixture"}]});
    let mut ledger: Value = serde_saphyr::from_slice(&fs::read(root.join(EVIDENCE_PATH))?)?;
    ledger["changes"].as_array_mut().ok_or("changes")?.push(json!({"fingerprint":initial["fingerprint"],"work":"fixture","assertions":[assertion.clone()]}));
    fs::write(root.join(EVIDENCE_PATH), serde_json::to_vec(&ledger)?)?;
    let mut prepared = draft(root, scratch.path())?;
    let plan = preview(root, &path, &prepared)?;
    prepared["acceptance"]["review"] = json!({"outcome":"failed", "reviewer":"synthetic test reviewer", "reference":"fixture", "rationale":"Mapping unresolved, record failure without approval", "subject_sha256":plan["subject_sha256"]});
    preview(root, &path, &prepared)?;
    assert!(
        cli(root, &["--draft", path.to_str().ok_or("path")?, "--write"])?
            .status
            .success()
    );
    let after: Value = serde_saphyr::from_slice(&fs::read(root.join(EVIDENCE_PATH))?)?;
    assert_eq!(after["changes"][1]["assertions"], json!([assertion]));
    assert_eq!(
        after["changes"][1]["acceptance"]["review"]["outcome"],
        "failed"
    );
    let before = fs::read(root.join(EVIDENCE_PATH))?;
    let input_path = scratch.path().join("input.json");
    let mut input: Value = serde_json::from_slice(&fs::read(&input_path)?)?;
    input["conditions"][0]["source"]["excerpt"] = json!("not present");
    fs::write(&input_path, serde_json::to_vec(&input)?)?;
    assert!(
        !cli(
            root,
            &[
                "--input",
                input_path.to_str().ok_or("path")?,
                "--work",
                "fixture"
            ]
        )?
        .status
        .success()
    );
    assert_eq!(fs::read(root.join(EVIDENCE_PATH))?, before);
    Ok(())
}

#[test]
fn malformed_preparation_input_reports_error_instead_of_panicking() -> Result {
    let (repo, scratch) = fixture()?;
    let input = scratch.path().join("input.json");
    for value in [
        json!("scalar"),
        json!({"conditions":["scalar"]}),
        json!({"conditions":[],"verifications":["scalar"]}),
    ] {
        fs::write(&input, serde_json::to_vec(&value)?)?;
        let output = cli(
            repo.path(),
            &[
                "--input",
                input.to_str().ok_or("path")?,
                "--work",
                "fixture",
            ],
        )?;
        assert_eq!(output.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
    }
    Ok(())
}

#[test]
fn preparation_preserves_disjoint_stages_without_approving_or_migrating() -> Result {
    let (repo, scratch) = fixture()?;
    let root = repo.path();
    let input_path = scratch.path().join("input.json");
    let mut input: Value = serde_json::from_slice(&fs::read(&input_path)?)?;
    input["verifications"][0]["stages"] = json!(["pre_merge"]);
    let mut post = input["verifications"][0].clone();
    post["stages"] = json!(["post_merge"]);
    input["verifications"]
        .as_array_mut()
        .ok_or("mappings")?
        .push(post);
    fs::write(&input_path, serde_json::to_vec(&input)?)?;
    let manifest_before = fs::read(root.join(MANIFEST_PATH))?;
    let ledger_before = fs::read(root.join(EVIDENCE_PATH))?;
    let rejected = cli(
        root,
        &[
            "--input",
            input_path.to_str().ok_or("path")?,
            "--work",
            "fixture",
        ],
    )?;
    assert_eq!(rejected.status.code(), Some(2));
    assert_eq!(fs::read(root.join(MANIFEST_PATH))?, manifest_before);
    assert_eq!(fs::read(root.join(EVIDENCE_PATH))?, ledger_before);
    let mut manifest = discover(root)?.manifest;
    manifest.schema = 3;
    manifest
        .assurance
        .profiles
        .retain(|profile| profile.name != "opdev-core");
    manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
        version: "1".into(),
        minimumcd: None,
        review_reference: "synthetic-policy-decision".into(),
        maintenance_branches: vec![],
    });
    fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    git(root, &["add", "."])?;
    let prepared = draft(root, scratch.path())?;
    assert_eq!(
        prepared["acceptance"]["verifications"][0]["stages"],
        json!(["pre_merge"])
    );
    assert_eq!(
        prepared["acceptance"]["verifications"][1]["stages"],
        json!(["post_merge"])
    );
    assert_eq!(prepared["acceptance"]["review"]["outcome"], "unverified");
    assert_eq!(
        prepared["acceptance"]["verifications"][1]["outcome"],
        "unverified"
    );
    assert_eq!(fs::read(root.join(EVIDENCE_PATH))?, ledger_before);
    let plan = preview(root, &scratch.path().join("draft.json"), &prepared)?;
    assert_eq!(plan["review_current"], false);
    Ok(())
}
