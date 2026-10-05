//! Real CLI side effects, attributed decisions and stale writer regressions.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn cli(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root)
        .args(args)
        .env("PATH", "")
        .output()
}

fn subject() -> Value {
    json!({"schema":1,"source_sha256":"a".repeat(64),"configuration_sha256":"b".repeat(64),
        "stage":"pre_merge","artifact_sha256":null})
}

#[test]
fn artifact_replacement_and_missing_bytes_never_reuse_old_qualification()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let artifact = b"candidate immutable bytes";
    let proof = b"qualification for that exact candidate";
    fs::write(root.join("artifact.bin"), artifact)?;
    fs::write(root.join("proof.json"), proof)?;
    let mut current = subject();
    current["artifact_sha256"] = json!(format!("{:x}", Sha256::digest(artifact)));
    fs::write(root.join("subject.json"), serde_json::to_vec(&current)?)?;
    let journal = json!({"schema":1,"protocol":"workflow.v1","work":"tracker:neutral/1","events":[{
        "event":"record","value":{"id":"artifact","kind":"artifact_qualification","subject":current,
        "scope":"candidate only","origin":{"reference":"ci:neutral/run/1","actor":"CI","human_attributed":false},
        "evidence":[{"path":"artifact.bin","sha256":format!("{:x}",Sha256::digest(artifact))},
        {"path":"proof.json","sha256":format!("{:x}",Sha256::digest(proof))}],
        "observed_at":1,"outcome":"passed"}}]});
    let schema: Value =
        serde_json::from_str(include_str!("../../../schema/workflow-journal.schema.json"))?;
    assert!(jsonschema::is_valid(&schema, &journal));
    fs::write(root.join("journal.json"), serde_json::to_vec(&journal)?)?;
    let inspect = || {
        cli(
            root,
            &[
                "workflow",
                "inspect",
                "--journal",
                "journal.json",
                "--subject",
                "subject.json",
                "--need",
                "artifact-qualification",
                "--json",
            ],
        )
    };
    assert_eq!(inspect()?.status.code(), Some(0));
    fs::write(root.join("artifact.bin"), b"replacement bytes")?;
    let changed = inspect()?;
    assert_eq!(changed.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&changed.stdout)?["findings"][0]["state"],
        "unresolved"
    );
    fs::remove_file(root.join("artifact.bin"))?;
    assert_eq!(inspect()?.status.code(), Some(1));
    fs::write(root.join("artifact.bin"), artifact)?;
    fs::remove_file(root.join("proof.json"))?;
    assert_eq!(inspect()?.status.code(), Some(1));
    assert_eq!(
        fs::read(root.join("journal.json"))?,
        serde_json::to_vec(&journal)?
    );
    Ok(())
}

fn fixture(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        root.join("original-decision.txt"),
        "Permission to run the bounded implementation, not product feedback or publication.",
    )?;
    let hash = format!(
        "{:x}",
        Sha256::digest(fs::read(root.join("original-decision.txt"))?)
    );
    fs::write(root.join("subject.json"), serde_json::to_vec(&subject())?)?;
    fs::write(
        root.join("event.json"),
        serde_json::to_vec(&json!({"event":"record","value":{
            "id":"execution-permission", "kind":"execution_permission", "subject":subject(),
            "scope":"neutral implementation", "origin":{"reference":"tracker:neutral/decision/1","actor":"Fixture developer","human_attributed":true},
            "evidence":[{"path":"original-decision.txt","sha256":hash}], "observed_at":1,
            "expires_at":null,"outcome":"passed","supersedes":null
        }}))?,
    )?;
    let output = cli(
        root,
        &[
            "workflow",
            "append",
            "--journal",
            "journal.json",
            "--event",
            "event.json",
            "--expected",
            "absent",
            "--work",
            "tracker:neutral/work/1",
        ],
    )?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let schema: Value =
        serde_json::from_str(include_str!("../../../schema/workflow-journal.schema.json"))?;
    assert!(jsonschema::is_valid(
        &schema,
        &serde_json::from_slice::<Value>(&fs::read(root.join("journal.json"))?)?
    ));
    Ok(())
}

#[test]
fn inspection_has_no_command_or_write_side_effects_and_never_infers_release()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    fixture(root.path())?;
    let before = fs::read(root.path().join("journal.json"))?;
    let entries = fs::read_dir(root.path())?.count();
    let output = cli(
        root.path(),
        &[
            "workflow",
            "inspect",
            "--journal",
            "journal.json",
            "--subject",
            "subject.json",
            "--need",
            "execution-permission",
            "--json",
        ],
    )?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let view: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(view["qualification"], "unverified");
    assert_eq!(view["findings"][0]["state"], "recorded");
    assert!(
        view["missing"]
            .as_array()
            .ok_or("missing array")?
            .is_empty()
    );
    let output = cli(
        root.path(),
        &[
            "workflow",
            "inspect",
            "--journal",
            "journal.json",
            "--subject",
            "subject.json",
            "--need",
            "user-feedback",
            "--need",
            "release-authorization",
            "--json",
        ],
    )?;
    assert_eq!(output.status.code(), Some(1));
    let view: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(view["missing"].as_array().ok_or("missing array")?.len(), 2);
    assert_eq!(view["missing"][0]["kind"], "user_feedback");
    assert_eq!(view["missing"][1]["kind"], "release_authorization");
    assert_eq!(fs::read(root.path().join("journal.json"))?, before);
    assert_eq!(fs::read_dir(root.path())?.count(), entries);
    assert!(!root.path().join(".git").exists());
    Ok(())
}

#[test]
fn changed_source_missing_original_and_stale_update_remain_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    fixture(root.path())?;
    let before = fs::read(root.path().join("journal.json"))?;
    let output = cli(
        root.path(),
        &[
            "workflow",
            "append",
            "--journal",
            "journal.json",
            "--event",
            "event.json",
            "--expected",
            "absent",
            "--work",
            "tracker:neutral/work/1",
        ],
    )?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read(root.path().join("journal.json"))?, before);
    let mut changed = subject();
    changed["source_sha256"] = "d".repeat(64).into();
    fs::write(
        root.path().join("subject.json"),
        serde_json::to_vec(&changed)?,
    )?;
    let output = cli(
        root.path(),
        &[
            "workflow",
            "inspect",
            "--journal",
            "journal.json",
            "--subject",
            "subject.json",
            "--json",
        ],
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout)?["findings"][0]["state"],
        "stale"
    );
    fs::write(
        root.path().join("subject.json"),
        serde_json::to_vec(&subject())?,
    )?;
    fs::remove_file(root.path().join("original-decision.txt"))?;
    let output = cli(
        root.path(),
        &[
            "workflow",
            "inspect",
            "--journal",
            "journal.json",
            "--subject",
            "subject.json",
            "--json",
        ],
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout)?["findings"][0]["state"],
        "unresolved"
    );
    assert_eq!(fs::read(root.path().join("journal.json"))?, before);
    Ok(())
}

#[test]
fn subject_helper_generates_real_bindings_without_running_checks()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .arg(root.path())
            .status()?
            .success()
    );
    let mut manifest = opdev_project::discover(root.path())?.manifest;
    manifest.commands.insert(
        "check".into(),
        opdev_project::CommandSpec {
            argv: vec!["must-not-launch-for-subject".into()],
            working_directory: None,
            timeout_seconds: Some(1),
        },
    );
    manifest.write_new(&root.path().join(".opdev/project.yaml"))?;
    fs::write(root.path().join("source.txt"), "neutral source")?;
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(root.path())
            .args(["add", "."])
            .status()?
            .success()
    );
    let index_before = fs::read(root.path().join(".git/index"))?;
    let artifact = tempfile::NamedTempFile::new()?;
    let bytes = vec![42u8; 9 * 1024 * 1024];
    fs::write(artifact.path(), &bytes)?;
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root.path())
        .args(["workflow", "subject", "--stage", "pre_merge", "--artifact"])
        .arg(artifact.path())
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["schema"], 1);
    assert_eq!(
        value["source_sha256"],
        opdev_project::staged_fingerprint(root.path())?
    );
    assert_eq!(
        value["configuration_sha256"],
        format!("{:x}", Sha256::digest(serde_json::to_vec(&manifest)?))
    );
    assert_eq!(
        value["artifact_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(fs::read(root.path().join(".git/index"))?, index_before);
    fs::write(root.path().join("source.txt"), "unstaged edit")?;
    let output = Command::new(env!("CARGO_BIN_EXE_opdev"))
        .current_dir(root.path())
        .args(["workflow", "subject", "--stage", "pre_merge"])
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read(root.path().join(".git/index"))?, index_before);
    Ok(())
}
