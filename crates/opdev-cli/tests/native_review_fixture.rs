//! Opt-in fixture producer for native provider-token trials. No network calls,
//! provider writes or automatically passing real-project attestations.
use opdev_core::{Evidence, Outcome, VerificationMethod};
use opdev_project::{
    ChangeEvidence, EvidenceAssertion, EvidenceLedger, MANIFEST_PATH, ProjectManifest,
    ReviewRecord, TestStage, TrackedEvidence, discover, staged_fingerprint,
};
use serde_json::json;
use std::{fs, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> anyhow::Result<()> {
    anyhow::ensure!(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()?
            .success(),
        "fixture Git failed"
    );
    Ok(())
}

#[test]
#[ignore = "explicit disposable native-review fixture root, repository identity and mode required"]
#[allow(clippy::too_many_lines)] // One explicit synthetic init/review fixture protocol, never a product attestation.
fn prepare_fixture_or_stage_review() -> anyhow::Result<()> {
    let root = std::path::PathBuf::from(std::env::var("OPDEV_NATIVE_REVIEW_ROOT")?);
    let mode = std::env::var("OPDEV_NATIVE_REVIEW_MODE")?;
    anyhow::ensure!(root.is_absolute(), "fixture root must be absolute");
    if mode == "init" {
        anyhow::ensure!(!root.exists(), "fixture root already exists");
        fs::create_dir_all(&root)?;
        git(&root, &["init", "-b", "main", "--quiet"])?;
        let mut manifest = discover(&root)?.manifest;
        manifest.schema = 3;
        manifest.assurance.profiles.clear();
        manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
            version: "1".into(),
            minimumcd: None,
            review_reference: "Explicit synthetic provider-token trial, not product adoption"
                .into(),
            maintenance_branches: vec![],
        });
        manifest.project.trunk = "main".into();
        manifest.project.ci.provider = opdev_project::CiProvider::Github;
        manifest.project.ci.remote = Some(std::env::var("OPDEV_NATIVE_REVIEW_REMOTE")?);
        manifest.assurance.review_storage = Some(opdev_project::ReviewStorage {
            version: 2,
            provider: opdev_project::CiProvider::Github,
            repository_id: std::env::var("OPDEV_NATIVE_REVIEW_REPOSITORY_ID")?.parse()?,
            review_reference: "Synthetic fixture boundary".into(),
            retention_authority: "requirements.md".into(),
            report_retention_days: Some(1),
        });
        manifest.commands.clear();
        manifest.commands.insert(
            "behavior".into(),
            opdev_project::CommandSpec {
                argv: vec!["python".into(), "-B".into(), "test_product.py".into()],
                working_directory: None,
                timeout_seconds: Some(10),
            },
        );
        manifest.testing.suites = vec![opdev_project::TestSuite {
            id: "behavior".into(),
            command: "behavior".into(),
            stages: vec![TestStage::PreMerge, TestStage::PostMerge],
        }];
        manifest.write_new(&root.join(MANIFEST_PATH))?;
        fs::write(
            root.join("requirements.md"),
            "Synthetic native-token trial only. Preserve caller order. Reports expire after one day. No release or whole-project compliance claim.\n",
        )?;
        fs::write(
            root.join("product.py"),
            "def select(items):\n    return items[:2]\n",
        )?;
        fs::write(
            root.join("test_product.py"),
            "from product import select\nassert select(['c', 'a', 'b']) == ['c', 'a']\nprint('NATIVE_REVIEW_CANONICAL_EXECUTED')\n",
        )?;
        fs::write(
            root.join(".gitattributes"),
            "* text=auto eol=lf\n*.exe binary\n",
        )?;
        fs::write(root.join(".gitignore"), "__pycache__/\n")?;
        opdev_project::reconcile_agent_files(&root)?;
        return Ok(());
    }
    let stage = match mode.as_str() {
        "pre_merge" => TestStage::PreMerge,
        "post_merge" => TestStage::PostMerge,
        _ => anyhow::bail!("unknown mode"),
    };
    let manifest = ProjectManifest::load(&root.join(MANIFEST_PATH))?;
    anyhow::ensure!(
        fs::read_to_string(root.join("requirements.md"))?
            .starts_with("Synthetic native-token trial only."),
        "not a native trial fixture"
    );
    let fingerprint = staged_fingerprint(&root)?;
    let source = TrackedEvidence::bind(
        &root,
        "requirements.md".into(),
        "Preserve caller order.".into(),
    )?;
    let target = TrackedEvidence::bind(
        &root,
        "test_product.py".into(),
        "assert select(['c', 'a', 'b']) == ['c', 'a']".into(),
    )?;
    let mut acceptance: opdev_project::AcceptanceEvidence = serde_json::from_value(json!({
        "scope":"behavioral", "rationale":"Frozen synthetic fixture tests native auth, binding and execution, not real-project compliance",
        "conditions":[{"id":"R1","statement":"Preserve caller order", "authority":"requirements.md", "source":source}],
        "verifications":[{"condition":"R1","method":"automated","target":target,"assertion":"Exact first two items in caller order",
            "discriminating_case":"Sorting or changing count fails", "suite":"behavior","outcome":"passed"}],
        "review":{"outcome":"passed","reviewer":"synthetic fixture reviewer","reference":"requirements.md",
            "rationale":"Reviewed exact assertion for the frozen fixture only", "subject_sha256":""}
    }))?;
    let work = "Explicit synthetic native-review transport trial";
    acceptance.review.subject_sha256 = acceptance.digest(&fingerprint, work)?;
    let ledger = EvidenceLedger { schema: 2, project: vec![], changes: vec![ChangeEvidence { fingerprint, work: work.into(), acceptance: Some(acceptance),
        assertions: manifest.catalog()?.rules.iter().filter(|r| r.verification.contains(&VerificationMethod::Evidence) || r.verification.contains(&VerificationMethod::Agent)).map(|r| EvidenceAssertion {
            rule_id: r.id.clone(), outcome: Outcome::Passed, summary: "Synthetic prerequisite only; not an actual product capability or release approval".into(),
            evidence: vec![Evidence { kind: "synthetic_fixture".into(), summary: "Frozen transport trial; other gates are controlled fixture inputs".into(), location: Some("requirements.md".into()) }]
        }).collect() }] };
    let review = ReviewRecord::prepare(&root, &manifest, stage, &ledger)?;
    let output = std::path::PathBuf::from(std::env::var("OPDEV_NATIVE_REVIEW_OUTPUT")?);
    anyhow::ensure!(
        output.is_absolute() && !output.starts_with(&root) && !output.exists(),
        "review output must be a new external file"
    );
    fs::write(&output, review.discussion_body()?)?;
    println!("review_acceptance_sha256={}", review.acceptance_sha256);
    Ok(())
}
