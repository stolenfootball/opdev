//! Process-level continuation tests: local records never supply gate evidence.
use opdev_project::{MANIFEST_PATH, discover};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
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
    Ok(result.stdout)
}

struct Fixture {
    temp: tempfile::TempDir,
    root: PathBuf,
    state: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("source");
        fs::create_dir(&root)?;
        git(&root, &["init", "--quiet"])?;
        git(&root, &["config", "user.name", "State fixture"])?;
        git(&root, &["config", "user.email", "fixture@example.invalid"])?;
        let mut manifest = discover(&root)?.manifest;
        manifest.commands.insert(
            "check".into(),
            opdev_project::CommandSpec {
                argv: vec![
                    "git".into(),
                    "config".into(),
                    "opdev.executed".into(),
                    "true".into(),
                ],
                working_directory: None,
                timeout_seconds: Some(30),
            },
        );
        manifest.testing.suites = vec![opdev_project::TestSuite {
            id: "behavior".into(),
            command: "check".into(),
            stages: vec![opdev_project::TestStage::Local],
        }];
        manifest.write_new(&root.join(MANIFEST_PATH))?;
        git(&root, &["add", "."])?;
        git(&root, &["commit", "-qm", "fixture"])?;
        let state = temp.path().join("private-state");
        Ok(Self { temp, root, state })
    }

    fn command(&self, root: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_opdev"));
        command
            .current_dir(root)
            .env("OPDEV_STATE_DIR", &self.state);
        command
    }

    fn cli(&self, args: &[&str]) -> Result<Output> {
        Ok(self.command(&self.root).args(args).output()?)
    }

    fn json(&self, args: &[&str], code: i32) -> Result<Value> {
        let result = self.cli(args)?;
        assert_eq!(
            result.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        Ok(serde_json::from_slice(&result.stdout)?)
    }

    fn locations(&self) -> Result<Value> {
        Ok(self.json(&["state", "resolve"], 0)?["locations"].clone())
    }
}

fn path(value: &Value, key: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(value[key].as_str().ok_or("missing path")?))
}

fn assert_schema(value: &Value) -> Result {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schema/local-state.schema.json"))?;
    let validator = jsonschema::validator_for(&schema)?;
    assert!(validator.is_valid(value), "schema rejected {value}");
    Ok(())
}

#[test]
fn readonly_resolution_is_stable_and_clones_worktrees_are_isolated() -> Result {
    let fixture = Fixture::new()?;
    let before = git(
        &fixture.root,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    let first = fixture.locations()?;
    let second = fixture.locations()?;
    assert_eq!(first, second);
    assert!(!fixture.state.exists());
    assert_eq!(fixture.json(&["state", "inspect"], 0)?["state"], "missing");
    assert!(!fixture.state.exists());
    let other = fixture.temp.path().join("worktree");
    git(
        &fixture.root,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            other.to_str().ok_or("path")?,
        ],
    )?;
    let out = fixture
        .command(&other)
        .args(["state", "resolve"])
        .output()?;
    assert!(out.status.success());
    let linked: Value = serde_json::from_slice(&out.stdout)?;
    assert_eq!(first["repository_id"], linked["locations"]["repository_id"]);
    assert_ne!(first["worktree_id"], linked["locations"]["worktree_id"]);
    let clone = fixture.temp.path().join("clone");
    git(
        fixture.temp.path(),
        &[
            "clone",
            "--quiet",
            fixture.root.to_str().ok_or("root")?,
            clone.to_str().ok_or("clone")?,
        ],
    )?;
    let out = fixture
        .command(&clone)
        .args(["state", "resolve"])
        .output()?;
    assert!(out.status.success());
    let cloned: Value = serde_json::from_slice(&out.stdout)?;
    assert_ne!(first["repository_id"], cloned["locations"]["repository_id"]);
    assert!(!fixture.state.exists());
    assert_eq!(
        git(
            &fixture.root,
            &["status", "--porcelain=v1", "--untracked-files=all"]
        )?,
        before
    );
    Ok(())
}

#[test]
fn context_resumes_references_but_never_consent_and_stale_writers_preserve_history() -> Result {
    let fixture = Fixture::new()?;
    let locations = fixture.locations()?;
    let subject = fixture.json(&["workflow", "subject", "--stage", "local"], 0)?;
    let mut context = json!({"schema":1,"subject":subject,"work":"tracker:original-task","references":["tracker:original-decision"]});
    let input = fixture.temp.path().join("input.json");
    fs::write(&input, serde_json::to_vec(&context)?)?;
    let input_arg = input.to_str().ok_or("input")?;
    let saved = fixture.json(
        &[
            "state",
            "context",
            "--input",
            input_arg,
            "--expected",
            "absent",
        ],
        0,
    )?;
    let expected = saved["context_sha256"].as_str().ok_or("digest")?;
    let inspected = fixture.json(&["state", "inspect"], 0)?;
    assert_eq!(inspected["state"], "references_only");
    assert_eq!(inspected["qualification"], "unverified");
    assert_eq!(inspected["context"], context);
    assert_schema(&context)?;
    context["references"] = json!(["tracker:original-decision", "tracker:revocation"]);
    fs::write(&input, serde_json::to_vec(&context)?)?;
    fixture.json(
        &[
            "state",
            "context",
            "--input",
            input_arg,
            "--expected",
            expected,
        ],
        0,
    )?;
    let current = fs::read(path(&locations, "context")?)?;
    assert_eq!(
        fixture
            .cli(&[
                "state",
                "context",
                "--input",
                input_arg,
                "--expected",
                expected
            ])?
            .status
            .code(),
        Some(2)
    );
    assert_eq!(fs::read(path(&locations, "context")?)?, current);
    let history = path(&locations, "worktree")?
        .join("context-history")
        .join(format!("{expected}.json"));
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(history)?)?["references"],
        json!(["tracker:original-decision"])
    );
    // A context input cannot introduce a new approval field, even from an attributed author.
    context["approved"] = json!(true);
    fs::write(&input, serde_json::to_vec(&context)?)?;
    assert_eq!(
        fixture
            .cli(&[
                "state",
                "context",
                "--input",
                input_arg,
                "--expected",
                "absent"
            ])?
            .status
            .code(),
        Some(2)
    );
    fs::write(fixture.root.join("changed.txt"), "material change")?;
    git(&fixture.root, &["add", "."])?;
    assert_eq!(fixture.json(&["state", "inspect"], 0)?["state"], "stale");
    assert_eq!(fs::read(path(&locations, "context")?)?, current);
    // Unknown readers fail without rewriting or silently dropping future fields.
    let mut future: Value = serde_json::from_slice(&current)?;
    future["schema"] = json!(99);
    fs::write(path(&locations, "context")?, serde_json::to_vec(&future)?)?;
    assert_eq!(fixture.cli(&["state", "inspect"])?.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(path(&locations, "context")?)?)?,
        future
    );
    Ok(())
}

fn attempts(locations: &Value) -> Result<Vec<PathBuf>> {
    Ok(fs::read_dir(path(locations, "runs")?)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?)
}

#[test]
fn retained_checks_keep_distinct_failures_and_never_supply_a_later_gate() -> Result {
    let fixture = Fixture::new()?;
    let locations = fixture.locations()?;
    let mut project = opdev_project::ProjectManifest::load(&fixture.root.join(MANIFEST_PATH))?;
    project
        .commands
        .get_mut("check")
        .ok_or("check command")?
        .argv = vec![
        "git".into(),
        "rev-parse".into(),
        "--verify".into(),
        "refs/heads/definitely-absent".into(),
    ];
    fs::write(fixture.root.join(MANIFEST_PATH), project.to_yaml()?)?;
    git(&fixture.root, &["add", "."])?;
    let failed = fixture.json(&["check", "--retain-state", "--format", "json"], 1)?;
    assert!(
        failed["checks"]
            .as_array()
            .ok_or("checks")?
            .iter()
            .any(|c| c["outcome"] == "failed")
    );
    let first = attempts(&locations)?.pop().ok_or("attempt")?;
    let original = fs::read(first.join("report.json"))?;
    let id = first.file_name().and_then(|p| p.to_str()).ok_or("id")?;
    let retained = fixture.json(&["state", "attempt", id], 0)?;
    assert_eq!(retained["state"], "completed_observation");
    assert_eq!(retained["report"], failed);
    assert_eq!(retained["qualification"], "unverified");
    assert_schema(&retained["start"])?;
    assert_schema(&retained["completion"])?;
    let later = fixture.json(
        &["check", "--retain-state", "--no-exec", "--format", "json"],
        1,
    )?;
    let missing = later["checks"].as_array().ok_or("checks")?;
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0]["outcome"], "unverified");
    assert!(missing[0]["duration_ms"].is_null());
    assert_eq!(attempts(&locations)?.len(), 2);
    project.commands.get_mut("check").ok_or("check")?.argv = vec!["git".into(), "--version".into()];
    fs::write(fixture.root.join(MANIFEST_PATH), project.to_yaml()?)?;
    git(&fixture.root, &["add", "."])?;
    let repaired = fixture.json(&["check", "--retain-state", "--format", "json"], 1)?;
    assert_eq!(repaired["checks"][0]["outcome"], "passed");
    assert_eq!(attempts(&locations)?.len(), 3);
    assert_eq!(fs::read(first.join("report.json"))?, original);
    fs::write(first.join("report.json"), b"{}")?;
    assert_eq!(
        fixture.cli(&["state", "attempt", id])?.status.code(),
        Some(2)
    );
    assert_eq!(
        fixture
            .cli(&["state", "attempt", "../escape"])?
            .status
            .code(),
        Some(2)
    );
    assert!(
        git(
            &fixture.root,
            &["ls-files", "--others", "--exclude-standard"]
        )?
        .is_empty()
    );
    Ok(())
}

#[test]
fn storage_collisions_and_relative_overrides_fail_without_writes() -> Result {
    let fixture = Fixture::new()?;
    for invalid in [
        PathBuf::from("relative"),
        fixture.root.join(".opdev/state"),
        fixture.root.join(".git/state"),
    ] {
        let output = fixture
            .command(&fixture.root)
            .env("OPDEV_STATE_DIR", &invalid)
            .args(["state", "resolve"])
            .output()?;
        assert_eq!(output.status.code(), Some(2));
        assert!(!invalid.exists());
    }
    assert!(!fixture.state.exists());
    Ok(())
}

#[test]
fn killed_check_preserves_unfinished_attempt_and_a_new_run_does_not_complete_it() -> Result {
    use std::{
        process::Stdio,
        thread,
        time::{Duration, Instant},
    };
    let fixture = Fixture::new()?;
    let locations = fixture.locations()?;
    let mut project = opdev_project::ProjectManifest::load(&fixture.root.join(MANIFEST_PATH))?;
    project.commands.get_mut("check").ok_or("check")?.argv = vec![
        if cfg!(windows) {"python"} else {"python3"}.into(), "-c".into(),
        "import pathlib,time; pathlib.Path('.git/attempt-running').write_text('started'); time.sleep(5)".into(),
    ];
    fs::write(fixture.root.join(MANIFEST_PATH), project.to_yaml()?)?;
    git(&fixture.root, &["add", "."])?;
    let mut child = fixture
        .command(&fixture.root)
        .args(["check", "--retain-state", "--format", "json"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !fixture.root.join(".git/attempt-running").exists() && Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let began = fixture.root.join(".git/attempt-running").exists();
    if child.try_wait()?.is_none() {
        child.kill()?;
    }
    child.wait()?;
    assert!(
        began,
        "the actual canonical command must start before interruption"
    );
    let original = attempts(&locations)?.pop().ok_or("attempt")?;
    let start = fs::read(original.join("start.json"))?;
    let id = original.file_name().and_then(|p| p.to_str()).ok_or("id")?;
    assert!(!original.join("completion.json").exists());
    assert_eq!(
        fixture.json(&["state", "attempt", id], 0)?["state"],
        "unfinished_or_interrupted"
    );
    fixture.json(
        &["check", "--retain-state", "--no-exec", "--format", "json"],
        1,
    )?;
    assert_eq!(attempts(&locations)?.len(), 2);
    assert_eq!(fs::read(original.join("start.json"))?, start);
    assert!(!original.join("completion.json").exists());
    assert!(
        git(
            &fixture.root,
            &["ls-files", "--others", "--exclude-standard"]
        )?
        .is_empty()
    );
    Ok(())
}

#[test]
fn simultaneous_context_writers_keep_one_head_without_overwriting_the_winner() -> Result {
    use std::process::Stdio;
    let fixture = Fixture::new()?;
    let subject = fixture.json(&["workflow", "subject", "--stage", "local"], 0)?;
    let mut children = Vec::new();
    for id in ["first", "second"] {
        let input = fixture.temp.path().join(format!("{id}.json"));
        fs::write(
            &input,
            serde_json::to_vec(&json!({"schema":1,"subject":subject,
            "work":"tracker:task", "references":[format!("tracker:{id}")]}))?,
        )?;
        children.push(
            fixture
                .command(&fixture.root)
                .args(["state", "context", "--expected", "absent", "--input"])
                .arg(input)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
        );
    }
    let mut exits = children
        .iter_mut()
        .map(|child| child.wait().map(|s| s.code()))
        .collect::<std::io::Result<Vec<_>>>()?;
    exits.sort();
    assert_eq!(exits, vec![Some(0), Some(2)]);
    let observed = fixture.json(&["state", "inspect"], 0)?;
    assert_eq!(observed["state"], "references_only");
    assert!(matches!(
        observed["context"]["references"][0].as_str(),
        Some("tracker:first" | "tracker:second")
    ));
    assert!(
        git(
            &fixture.root,
            &["status", "--porcelain=v1", "--untracked-files=all"]
        )?
        .is_empty()
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn linked_state_is_not_followed_and_created_directories_are_private() -> Result {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let fixture = Fixture::new()?;
    let target = fixture.temp.path().join("target");
    fs::create_dir(&target)?;
    symlink(&target, &fixture.state)?;
    assert_eq!(fixture.cli(&["state", "resolve"])?.status.code(), Some(2));
    assert!(fs::read_dir(&target)?.next().is_none());
    fs::remove_file(&fixture.state)?;
    fixture.json(
        &["check", "--retain-state", "--no-exec", "--format", "json"],
        1,
    )?;
    assert_eq!(
        fs::metadata(&fixture.state)?.permissions().mode() & 0o777,
        0o700
    );
    Ok(())
}
