//! Exercise provider-neutral candidate flow and its failure boundaries.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

#[test]
fn diagnostics_are_read_only_and_keep_failed_expectations_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::write(
        root.join("fixture.py"),
        include_str!("../../../examples/feedback/fixture.py"),
    )?;
    let output = run(root, "diagnose")?;
    assert!(
        !output.status.success(),
        "missing environment expectation must not pass"
    );
    let data: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(data["qualification"], "unverified");
    let observed = data["system"].as_str().ok_or("system")?;
    let success = Command::new(if cfg!(windows) { "python" } else { "python3" })
        .args(["fixture.py", "diagnose", observed])
        .current_dir(root)
        .output()?;
    assert!(success.status.success());
    assert!(!root.join("feedback-artifacts").exists());
    assert_eq!(
        fs::read_dir(root)?.count(),
        1,
        "diagnostic created resources"
    );
    Ok(())
}

fn run(root: &Path, action: &str) -> std::io::Result<Output> {
    Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg("fixture.py")
        .arg(action)
        .current_dir(root)
        .output()
}

#[test]
fn candidate_requires_current_verification_and_preserves_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let source = include_str!("../../../examples/feedback/fixture.py");
    fs::write(root.join("fixture.py"), source)?;
    assert!(!run(root, "candidate")?.status.success());
    assert!(run(root, "verify")?.status.success());
    assert!(
        !run(root, "verify")?.status.success(),
        "duplicate verification must be visible"
    );
    fs::write(root.join("fixture.py"), format!("{source}\n# changed\n"))?;
    assert!(
        !run(root, "candidate")?.status.success(),
        "stale verification accepted"
    );
    fs::write(root.join("fixture.py"), source)?;
    assert!(run(root, "candidate")?.status.success());
    assert_eq!(
        fs::read(root.join("feedback-artifacts/candidate.py"))?,
        source.as_bytes()
    );
    assert!(run(root, "consume")?.status.success());
    fs::write(root.join("feedback-artifacts/candidate.py"), "changed")?;
    assert!(!run(root, "consume")?.status.success());
    Ok(())
}
