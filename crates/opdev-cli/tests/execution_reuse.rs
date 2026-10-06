//! Process-level producer tests. Provider trust is deliberately not simulated by JSON input.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use opdev_engine::{
    ExecutionPolicy, ExecutionRecord, ProducerPolicy, ProgramLocation, inspect_program,
};
use opdev_project::{CiProvider, CommandSpec, TestStage, TestSuite, discover};
use sha2::{Digest, Sha256};

const WRITER: &str = "import pathlib, sys, time\np = pathlib.Path('target')\np.mkdir(exist_ok=True)\nwith (p / 'count').open('a') as f: f.write('run\\n')\nprint('OPDEV_EXECUTION_V1 fake child record')\nif sys.argv[1] == 'timeout': time.sleep(10)\nif sys.argv[1] == 'mutate': pathlib.Path('source.txt').write_text('changed')\nsys.exit(7 if sys.argv[1] == 'fail' else 0)\n";

fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
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
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn fixture(
    provider: CiProvider,
    mode: &str,
) -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    git(root.path(), &["init", "-q", "-b", "main"])?;
    git(root.path(), &["config", "user.name", "Neutral Test"])?;
    git(
        root.path(),
        &["config", "user.email", "test@example.invalid"],
    )?;
    fs::write(root.path().join("source.txt"), "original")?;
    fs::write(root.path().join("writer.py"), WRITER)?;
    fs::write(root.path().join(".gitignore"), "target/\n")?;
    let ProgramLocation::Located(python) =
        inspect_program(if cfg!(windows) { "python" } else { "python3" })
    else {
        return Err("test prerequisite: absolute Python executable".into());
    };
    let mut manifest = discover(root.path())?.manifest;
    manifest.project.trunk = "main".into();
    manifest.project.ci.provider = provider;
    manifest.commands.insert(
        "check".into(),
        CommandSpec {
            argv: vec![
                python.to_string_lossy().into(),
                "writer.py".into(),
                mode.into(),
            ],
            working_directory: None,
            timeout_seconds: Some(if mode == "timeout" { 1 } else { 20 }),
        },
    );
    manifest.testing.suites = vec![TestSuite {
        id: "unit".into(),
        command: "check".into(),
        stages: vec![TestStage::PreMerge, TestStage::PostMerge],
    }];
    manifest.write_new(&root.path().join(".opdev/project.yaml"))?;
    let policy = ExecutionPolicy {
        schema: 1,
        review_reference: "controlled neutral fixture, not human approval".into(),
        inputs_complete: true,
        ledger_is_input: true,
        environment: "controlled-python-fixture".into(),
        executor_sha256: format!(
            "{:x}",
            Sha256::digest(fs::read(env!("CARGO_BIN_EXE_opdev"))?)
        ),
        github_workflow_id: (provider == CiProvider::Github).then_some(9),
        producers: vec![ProducerPolicy {
            suite: "unit".into(),
            job: "unit-job".into(),
            executable_sha256: format!("{:x}", Sha256::digest(fs::read(python)?)),
        }],
    };
    let value = serde_json::to_value(&policy)?;
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schema/execution-policy.schema.json"))?;
    assert!(jsonschema::is_valid(&schema, &value));
    fs::write(
        root.path().join(".opdev/execution.json"),
        serde_json::to_vec(&policy)?,
    )?;
    git(root.path(), &["add", "."])?;
    git(root.path(), &["commit", "-q", "-m", "fixture"])?;
    let revision = git(root.path(), &["rev-parse", "HEAD"])?;
    Ok((root, revision))
}

fn producer(root: &Path, revision: &str, provider: CiProvider) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_opdev"));
    command.current_dir(root).args([
        "ci",
        "execute",
        "--policy",
        ".opdev/execution.json",
        "--suite",
        "unit",
        "--environment",
        "controlled-python-fixture",
    ]);
    if provider == CiProvider::Gitlab {
        command
            .env("GITLAB_CI", "true")
            .env("CI_PROJECT_PATH", "neutral/project")
            .env("CI_COMMIT_SHA", revision)
            .env("CI_PIPELINE_ID", "10")
            .env("CI_COMMIT_REF_NAME", "main")
            .env("CI_PIPELINE_SOURCE", "push")
            .env("CI_JOB_NAME", "unit-job");
    } else {
        command
            .env("GITHUB_ACTIONS", "true")
            .env("GITHUB_REPOSITORY", "neutral/project")
            .env("GITHUB_SHA", revision)
            .env("GITHUB_RUN_ID", "10")
            .env("GITHUB_REF_NAME", "main")
            .env("GITHUB_EVENT_NAME", "push")
            .env("GITHUB_RUN_ATTEMPT", "2");
    }
    command
}

fn record(output: &Output) -> Result<ExecutionRecord, Box<dyn std::error::Error>> {
    let stdout = String::from_utf8(output.stdout.clone())?;
    let lines: Vec<_> = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("OPDEV_EXECUTION_V1 "))
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_str(lines[0])?;
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schema/execution-record.schema.json"))?;
    assert!(jsonschema::is_valid(&schema, &value));
    Ok(serde_json::from_value(value)?)
}

#[test]
fn both_provider_contexts_execute_once_and_escape_child_markers()
-> Result<(), Box<dyn std::error::Error>> {
    for provider in [CiProvider::Gitlab, CiProvider::Github] {
        let (root, revision) = fixture(provider, "pass")?;
        let output = producer(root.path(), &revision, provider).output()?;
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let record = record(&output)?;
        assert_eq!(record.result.outcome, opdev_core::Outcome::Passed);
        assert!(record.inputs_unchanged);
        assert_eq!(record.binding.provider, provider);
        assert_eq!(record.binding.revision, revision);
        assert!(
            record
                .result
                .stdout
                .as_deref()
                .is_some_and(|s| s.contains("fake child record"))
        );
        assert_eq!(
            fs::read_to_string(root.path().join("target/count"))?
                .lines()
                .collect::<Vec<_>>(),
            ["run"]
        );
    }
    Ok(())
}

#[test]
fn producer_preserves_failure_timeout_and_source_mutation() -> Result<(), Box<dyn std::error::Error>>
{
    for (mode, exit, expected) in [
        ("fail", 1, opdev_core::Outcome::Failed),
        ("timeout", 2, opdev_core::Outcome::Error),
        ("mutate", 1, opdev_core::Outcome::Passed),
    ] {
        let (root, revision) = fixture(CiProvider::Gitlab, mode)?;
        let output = producer(root.path(), &revision, CiProvider::Gitlab).output()?;
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let record = record(&output)?;
        assert_eq!(record.result.outcome, expected);
        assert_eq!(record.inputs_unchanged, mode != "mutate");
        assert_eq!(
            fs::read_to_string(root.path().join("target/count"))?
                .lines()
                .collect::<Vec<_>>(),
            ["run"]
        );
    }
    Ok(())
}

#[test]
fn invalid_policy_context_and_post_merge_subject_stop_before_execution()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, revision) = fixture(CiProvider::Gitlab, "pass")?;
    for (name, value) in [
        ("CI_JOB_NAME", "wrong"),
        ("CI_COMMIT_SHA", "wrong"),
        ("GITLAB_CI", "false"),
    ] {
        let output = producer(root.path(), &revision, CiProvider::Gitlab)
            .env(name, value)
            .output()?;
        assert_eq!(output.status.code(), Some(2));
        assert!(!root.path().join("target/count").exists());
    }
    let output = producer(root.path(), &revision, CiProvider::Gitlab)
        .arg("--post-merge")
        .env("CI_COMMIT_REF_NAME", "feature")
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(!root.path().join("target/count").exists());
    let path = root.path().join(".opdev/execution.json");
    let mut policy: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
    policy["schema"] = 99.into();
    fs::write(&path, serde_json::to_vec(&policy)?)?;
    let before = fs::read(&path)?;
    let output = producer(root.path(), &revision, CiProvider::Gitlab).output()?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read(path)?, before);
    assert!(!root.path().join("target/count").exists());
    Ok(())
}

#[test]
fn post_merge_produces_its_own_stage_and_rejects_pull_request_subjects()
-> Result<(), Box<dyn std::error::Error>> {
    for provider in [CiProvider::Gitlab, CiProvider::Github] {
        let (root, revision) = fixture(provider, "pass")?;
        let output = producer(root.path(), &revision, provider)
            .arg("--post-merge")
            .output()?;
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(record(&output)?.binding.stage, TestStage::PostMerge);
        assert_eq!(
            fs::read_to_string(root.path().join("target/count"))?
                .lines()
                .count(),
            1
        );
        let output = if provider == CiProvider::Github {
            producer(root.path(), &revision, provider)
                .env("GITHUB_EVENT_NAME", "pull_request")
                .output()?
        } else {
            producer(root.path(), &revision, provider)
                .arg("--post-merge")
                .env("CI_PIPELINE_SOURCE", "merge_request_event")
                .output()?
        };
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(
            fs::read_to_string(root.path().join("target/count"))?
                .lines()
                .count(),
            1
        );
    }
    Ok(())
}
