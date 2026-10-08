//! Exact-source offline transport does not confer execution or decision authority.
use opdev_project::{MANIFEST_PATH, TrackedEvidence, discover, staged_fingerprint};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn assert_bundle_schema(value: &Value) -> Result {
    let registry = jsonschema::Registry::new()
        .extend([
            (
                "https://opdev.dev/schema/project.json",
                serde_json::from_str::<Value>(include_str!("../../../schema/project.schema.json"))?,
            ),
            (
                "https://opdev.dev/schema/evidence-v2.json",
                serde_json::from_str::<Value>(include_str!(
                    "../../../schema/evidence.schema.json"
                ))?,
            ),
            (
                "https://opdev.dev/schema/local-state-v1.json",
                serde_json::from_str::<Value>(include_str!(
                    "../../../schema/local-state.schema.json"
                ))?,
            ),
        ])?
        .prepare()?;
    let schema: Value =
        serde_json::from_str(include_str!("../../../schema/evidence-bundle.schema.json"))?;
    let validator = jsonschema::options()
        .with_registry(&registry)
        .build(&schema)?;
    assert!(validator.is_valid(value));
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(())
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
        git(&root, &["config", "user.name", "Bundle fixture"])?;
        git(&root, &["config", "user.email", "fixture@example.invalid"])?;
        let mut project = discover(&root)?.manifest;
        project.commands.insert(
            "check".into(),
            opdev_project::CommandSpec {
                argv: vec!["git".into(), "--version".into()],
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
        fs::write(root.join("contract.txt"), "Return declared caller order.\n")?;
        git(&root, &["add", "."])?;
        let reference = TrackedEvidence::bind(
            &root,
            "contract.txt".into(),
            "Return declared caller order.".into(),
        )?;
        let ledger = json!({"schema":2,"project":[],"changes":[{
            "fingerprint":staged_fingerprint(&root)?, "work":"fixture:accepted-work", "assertions":[],
            "acceptance": {"scope":"behavioral","rationale":"Synthetic review transport; not assertion adequacy or execution proof.",
                "conditions":[{"id":"order","statement":"Preserve caller order","authority":"fixture:accepted-work","source":reference}],
                "verifications":[{"condition":"order","method":"review","target":reference,"assertion":"Pending actual review",
                    "discriminating_case":"Sorting differs from caller order","automation_limitation":"Synthetic review fixture","outcome":"unverified"}],
                "review":{"outcome":"unverified","reviewer":"","reference":"","rationale":"","subject_sha256":""}
            }
        }]});
        fs::write(
            root.join(".opdev/evidence.yaml"),
            serde_json::to_vec_pretty(&ledger)?,
        )?;
        git(&root, &["add", "."])?;
        git(&root, &["commit", "-qm", "fixture"])?;
        let state = temp.path().join("state");
        Ok(Self { temp, root, state })
    }
    fn cli(&self, root: &Path, args: &[&str]) -> Result<Output> {
        Ok(Command::new(env!("CARGO_BIN_EXE_opdev"))
            .current_dir(root)
            .env("OPDEV_STATE_DIR", &self.state)
            .args(args)
            .output()?)
    }
    fn export(&self, output: &Path, attempt: Option<&str>) -> Result<Value> {
        let mut args = vec![
            "evidence",
            "bundle",
            "export",
            "--stage",
            "local",
            "--output",
            output.to_str().ok_or("output")?,
        ];
        if let Some(id) = attempt {
            args.extend(["--attempt", id]);
        }
        let out = self.cli(&self.root, &args)?;
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        Ok(serde_json::from_slice(&out.stdout)?)
    }
    fn inspect(&self, root: &Path, file: &Path, expected: &Value, stage: &str) -> Result<Output> {
        self.cli(
            root,
            &[
                "evidence",
                "bundle",
                "inspect",
                "--stage",
                stage,
                "--input",
                file.to_str().ok_or("path")?,
                "--sha256",
                expected["sha256"].as_str().ok_or("digest")?,
                "--acceptance-sha256",
                expected["acceptance_sha256"].as_str().ok_or("review")?,
            ],
        )
    }
}

#[test]
fn exact_review_retrieves_on_clean_clone_without_source_ledger_rewrite_or_state_cache() -> Result {
    let f = Fixture::new()?;
    let original = fs::read(f.root.join(".opdev/evidence.yaml"))?;
    let file = f.temp.path().join("bundle.json");
    let exported = f.export(&file, None)?;
    assert_bundle_schema(&serde_json::from_slice::<Value>(&fs::read(&file)?)?)?;
    assert_eq!(exported["qualification"], "unverified");
    assert!(!f.state.exists());
    let clone = f.temp.path().join("fresh");
    git(
        f.temp.path(),
        &[
            "clone",
            "--quiet",
            f.root.to_str().ok_or("root")?,
            clone.to_str().ok_or("clone")?,
        ],
    )?;
    fs::remove_file(clone.join(".opdev/evidence.yaml"))?;
    git(&clone, &["add", "-u"])?;
    let read = f.inspect(&clone, &file, &exported, "local")?;
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    let value: Value = serde_json::from_slice(&read.stdout)?;
    assert_eq!(value["integrity"], "matched");
    assert_eq!(value["origin"], "unverified");
    assert_eq!(value["qualification"], "unverified");
    assert_eq!(value["review_current"], false);
    assert_eq!(fs::read(f.root.join(".opdev/evidence.yaml"))?, original);
    assert!(!f.state.exists());
    assert!(!clone.join(".opdev/evidence.yaml").exists());
    let duplicate = f.cli(
        &f.root,
        &[
            "evidence",
            "bundle",
            "export",
            "--stage",
            "local",
            "--output",
            file.to_str().ok_or("file")?,
        ],
    )?;
    assert_eq!(duplicate.status.code(), Some(2));
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&file)?)),
        exported["sha256"]
    );
    Ok(())
}

#[test]
fn changed_bytes_versions_subject_and_review_fail_without_falling_back() -> Result {
    let f = Fixture::new()?;
    let file = f.temp.path().join("bundle.json");
    let mut expected = f.export(&file, None)?;
    assert_eq!(
        f.inspect(&f.root, &file, &expected, "post_merge")?
            .status
            .code(),
        Some(2)
    );
    let original = fs::read(&file)?;
    fs::write(&file, b"{}")?;
    assert_eq!(
        f.inspect(&f.root, &file, &expected, "local")?.status.code(),
        Some(2)
    );
    let mut future: Value = serde_json::from_slice(&original)?;
    future["schema"] = json!(99);
    fs::write(&file, serde_json::to_vec(&future)?)?;
    expected["sha256"] = json!(format!("{:x}", Sha256::digest(fs::read(&file)?)));
    assert_eq!(
        f.inspect(&f.root, &file, &expected, "local")?.status.code(),
        Some(2)
    );
    fs::write(&file, &original)?;
    expected["sha256"] = json!(format!("{:x}", Sha256::digest(&original)));
    let accepted = expected["acceptance_sha256"].clone();
    expected["acceptance_sha256"] = json!("a".repeat(64));
    assert_eq!(
        f.inspect(&f.root, &file, &expected, "local")?.status.code(),
        Some(2)
    );
    expected["acceptance_sha256"] = accepted;
    // Working-tree configuration is an independent input even before it is staged.
    let original_manifest = fs::read(f.root.join(MANIFEST_PATH))?;
    let mut manifest = opdev_project::ProjectManifest::load(&f.root.join(MANIFEST_PATH))?;
    manifest.project.trunk = "changed-trunk".into();
    fs::write(f.root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    assert_eq!(
        f.inspect(&f.root, &file, &expected, "local")?.status.code(),
        Some(2)
    );
    fs::write(f.root.join(MANIFEST_PATH), original_manifest)?;
    fs::write(f.root.join("contract.txt"), "Return sorted order.\n")?;
    git(&f.root, &["add", "."])?;
    assert_eq!(
        f.inspect(&f.root, &file, &expected, "local")?.status.code(),
        Some(2)
    );
    assert_eq!(fs::read(&file)?, original);
    Ok(())
}

#[test]
fn bundle_can_preserve_actual_attempt_without_reusing_execution_or_consent() -> Result {
    let f = Fixture::new()?;
    let run = f.cli(&f.root, &["check", "--retain-state", "--format", "json"])?;
    assert_eq!(run.status.code(), Some(1)); // Missing required reviewed evidence is still blocked.
    let paths: Value = serde_json::from_slice(&f.cli(&f.root, &["state", "resolve"])?.stdout)?;
    let runs = paths["locations"]["runs"].as_str().ok_or("runs")?;
    let attempt = fs::read_dir(runs)?.next().ok_or("attempt")??;
    let id = attempt
        .file_name()
        .into_string()
        .map_err(|_| "attempt name")?;
    let file = f.temp.path().join("with-attempt.json");
    let exported = f.export(&file, Some(&id))?;
    let bundle: Value = serde_json::from_slice(&fs::read(&file)?)?;
    assert_bundle_schema(&bundle)?;
    let start = &bundle["attempt"]["start"];
    assert!(
        start["runtime_sha256"]
            .as_str()
            .is_some_and(|s| s.len() == 64)
    );
    assert!(bundle["attempt"]["report_json"].as_str().is_some());
    let observed: Value =
        serde_json::from_slice(&f.inspect(&f.root, &file, &exported, "local")?.stdout)?;
    assert_eq!(observed["contains_attempt"], true);
    assert_eq!(observed["attempt_state"], "completed_observation");
    assert_eq!(observed["qualification"], "unverified");
    let no_run: Value = serde_json::from_slice(
        &f.cli(&f.root, &["check", "--no-exec", "--format", "json"])?
            .stdout,
    )?;
    assert_eq!(no_run["checks"][0]["outcome"], "unverified");
    for output in [f.root.join("bundle.json"), f.root.join(".git/bundle.json")] {
        let result = f.cli(
            &f.root,
            &[
                "evidence",
                "bundle",
                "export",
                "--stage",
                "local",
                "--output",
                output.to_str().ok_or("output")?,
            ],
        )?;
        assert_eq!(result.status.code(), Some(2));
        assert!(!output.exists());
    }
    Ok(())
}

#[test]
fn internally_inconsistent_records_cannot_hide_behind_a_matching_outer_digest() -> Result {
    let f = Fixture::new()?;
    f.cli(&f.root, &["check", "--retain-state", "--format", "json"])?;
    let paths: Value = serde_json::from_slice(&f.cli(&f.root, &["state", "resolve"])?.stdout)?;
    let first = fs::read_dir(paths["locations"]["runs"].as_str().ok_or("runs")?)?
        .next()
        .ok_or("attempt")??;
    let id = first.file_name().into_string().map_err(|_| "id")?;
    let file = f.temp.path().join("bundle.json");
    let original_expected = f.export(&file, Some(&id))?;
    let original: Value = serde_json::from_slice(&fs::read(&file)?)?;
    for (pointer, replacement) in [
        ("/qualification", json!("passed")),
        ("/configuration/project/trunk", json!("different-trunk")),
        (
            "/ledger/changes/0/acceptance/conditions/0/statement",
            json!("Different accepted behavior"),
        ),
        ("/attempt/start/schema", json!(99)),
        ("/attempt/start/runtime_sha256", json!("invalid")),
        ("/attempt/completion/source_unchanged", json!(false)),
        ("/attempt/report_json", Value::Null),
    ] {
        let mut altered = original.clone();
        *altered.pointer_mut(pointer).ok_or("field")? = replacement;
        let bytes = serde_json::to_vec(&altered)?;
        fs::write(&file, &bytes)?;
        let mut expected = original_expected.clone();
        expected["sha256"] = json!(format!("{:x}", Sha256::digest(&bytes)));
        assert_eq!(
            f.inspect(&f.root, &file, &expected, "local")?.status.code(),
            Some(2),
            "{pointer}"
        );
    }
    // Rehashing unknown report bytes must not make a future schema readable as today's result.
    let mut future_report = original.clone();
    let mut report: Value = serde_json::from_str(
        future_report["attempt"]["report_json"]
            .as_str()
            .ok_or("report")?,
    )?;
    report["schema"] = json!(99);
    let report_json = serde_json::to_string(&report)?;
    future_report["attempt"]["completion"]["report_sha256"] =
        json!(format!("{:x}", Sha256::digest(report_json.as_bytes())));
    future_report["attempt"]["report_json"] = json!(report_json);
    let bytes = serde_json::to_vec(&future_report)?;
    fs::write(&file, &bytes)?;
    let mut expected = original_expected.clone();
    expected["sha256"] = json!(format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(
        f.inspect(&f.root, &file, &expected, "local")?.status.code(),
        Some(2)
    );
    // An incomplete attempt is a legitimate observation, not a success or an error to erase.
    let mut incomplete = original;
    incomplete["attempt"]["completion"] = Value::Null;
    let bytes = serde_json::to_vec(&incomplete)?;
    fs::write(&file, &bytes)?;
    let mut expected = original_expected;
    expected["sha256"] = json!(format!("{:x}", Sha256::digest(&bytes)));
    let inspected: Value =
        serde_json::from_slice(&f.inspect(&f.root, &file, &expected, "local")?.stdout)?;
    assert_eq!(inspected["attempt_state"], "unfinished_or_interrupted");
    assert_eq!(inspected["qualification"], "unverified");
    let wrong_producer = f.cli(
        &f.root,
        &[
            "evidence",
            "bundle",
            "inspect",
            "--stage",
            "local",
            "--input",
            file.to_str().ok_or("file")?,
            "--sha256",
            expected["sha256"].as_str().ok_or("sha")?,
            "--acceptance-sha256",
            expected["acceptance_sha256"].as_str().ok_or("review")?,
            "--attempt-runtime-sha256",
            &"a".repeat(64),
        ],
    )?;
    assert_eq!(wrong_producer.status.code(), Some(2));
    Ok(())
}

#[test]
fn failed_then_successful_execution_exports_preserve_both_attempts_without_green_fallback() -> Result
{
    let f = Fixture::new()?;
    let mut manifest = opdev_project::ProjectManifest::load(&f.root.join(MANIFEST_PATH))?;
    manifest.commands.get_mut("check").ok_or("check")?.argv = vec![
        "git".into(),
        "rev-parse".into(),
        "--verify".into(),
        "refs/heads/fixture-ready".into(),
    ];
    fs::write(f.root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
    git(&f.root, &["add", "."])?;
    let mut ledger: Value =
        serde_json::from_slice(&fs::read(f.root.join(".opdev/evidence.yaml"))?)?;
    ledger["changes"][0]["fingerprint"] = json!(staged_fingerprint(&f.root)?);
    fs::write(
        f.root.join(".opdev/evidence.yaml"),
        serde_json::to_vec(&ledger)?,
    )?;
    git(&f.root, &["add", "."])?;
    let paths: Value = serde_json::from_slice(&f.cli(&f.root, &["state", "resolve"])?.stdout)?;
    let runs = paths["locations"]["runs"].as_str().ok_or("runs")?;
    let failed: Value = serde_json::from_slice(
        &f.cli(&f.root, &["check", "--retain-state", "--format", "json"])?
            .stdout,
    )?;
    assert_eq!(failed["checks"][0]["outcome"], "failed");
    let first = fs::read_dir(runs)?
        .next()
        .ok_or("first")??
        .file_name()
        .into_string()
        .map_err(|_| "id")?;
    let first_file = f.temp.path().join("failed.json");
    let expected_failed = f.export(&first_file, Some(&first))?;
    let original = fs::read(&first_file)?;
    // Change a real command input without rewriting its prior report or staged source.
    git(&f.root, &["branch", "fixture-ready"])?;
    let passed: Value = serde_json::from_slice(
        &f.cli(&f.root, &["check", "--retain-state", "--format", "json"])?
            .stdout,
    )?;
    assert_eq!(passed["checks"][0]["outcome"], "passed");
    let ids: Vec<_> = fs::read_dir(runs)?
        .map(|entry| entry.map(|e| e.file_name()))
        .collect::<std::io::Result<_>>()?;
    assert_eq!(ids.len(), 2);
    let second = ids
        .iter()
        .filter_map(|id| id.to_str())
        .find(|id| *id != first)
        .ok_or("second")?;
    let second_file = f.temp.path().join("settled.json");
    let expected_passed = f.export(&second_file, Some(second))?;
    assert_ne!(expected_failed["sha256"], expected_passed["sha256"]);
    assert_eq!(fs::read(&first_file)?, original);
    let envelope: Value = serde_json::from_slice(&original)?;
    let preserved: Value = serde_json::from_str(
        envelope["attempt"]["report_json"]
            .as_str()
            .ok_or("report")?,
    )?;
    assert_eq!(preserved["checks"][0]["outcome"], "failed");
    assert!(
        f.inspect(&f.root, &first_file, &expected_failed, "local")?
            .status
            .success()
    );
    assert_eq!(
        f.inspect(&f.root, &second_file, &expected_failed, "local")?
            .status
            .code(),
        Some(2)
    );
    let unchecked: Value = serde_json::from_slice(
        &f.cli(&f.root, &["check", "--no-exec", "--format", "json"])?
            .stdout,
    )?;
    assert_eq!(unchecked["checks"][0]["outcome"], "unverified");
    Ok(())
}

#[test]
fn retrieval_rejects_untrusted_locator_and_product_destinations_without_network_or_writes() -> Result
{
    let f = Fixture::new()?;
    let file = f.temp.path().join("locator.json");
    let destination = f.temp.path().join("output.json");
    let valid = json!({"schema":1,"provider":"github","repository_id":42,
        "commit":"a".repeat(40),"path":"attempt.json","sha256":"b".repeat(64)});
    for control in [
        "unknown-version",
        "caller-approval",
        "outside-host",
        "product-output",
    ] {
        let mut locator = valid.clone();
        match control {
            "unknown-version" => locator["schema"] = json!(99),
            "caller-approval" => locator["approved"] = json!(true),
            "outside-host" => locator["provider"] = json!("https://example.invalid"),
            _ => {}
        }
        fs::write(&file, serde_json::to_vec(&locator)?)?;
        let output = if control == "product-output" {
            f.root.join("output.json")
        } else {
            destination.clone()
        };
        let observed = Command::new(env!("CARGO_BIN_EXE_opdev"))
            .current_dir(&f.root)
            // Any accidental HTTP request cannot reach a provider from this fixture.
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .env("NO_PROXY", "")
            .env("OPDEV_GITHUB_TOKEN", "invalid-fixture-token")
            .args([
                "evidence",
                "bundle",
                "retrieve",
                "--stage",
                "local",
                "--locator",
                file.to_str().ok_or("file")?,
                "--acceptance-sha256",
                &"a".repeat(64),
                "--output",
                output.to_str().ok_or("output")?,
            ])
            .output()?;
        assert_eq!(observed.status.code(), Some(2), "{control}");
        assert!(!output.exists());
        let error = String::from_utf8_lossy(&observed.stderr);
        assert!(!error.contains("Archive request"), "{error}");
        assert!(!error.contains("invalid-fixture-token"));
    }
    assert!(!f.state.exists());
    Ok(())
}
