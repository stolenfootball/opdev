use super::*;
use opdev_project::{AdoptionReview, AuthorityKind, AuthorityRef, LayoutPolicy, ReviewStorage};
use serde_json::json;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> Result<()> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    ensure!(out.status.success(), "Fixture Git failed");
    Ok(())
}

struct Fixture {
    temp: tempfile::TempDir,
    root: PathBuf,
    request: PathBuf,
}
impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("source");
        fs::create_dir(&root)?;
        git(&root, &["init", "--quiet"])?;
        let mut manifest = opdev_project::discover(&root)?.manifest;
        manifest.authorities.clear();
        manifest.authorities.insert(
            "contracts".into(),
            AuthorityRef {
                kind: AuthorityKind::Path,
                location: "knowledge".into(),
            },
        );
        manifest.context.always = vec!["contracts".into()];
        manifest.context.routes.clear();
        manifest.write_new(&root.join(MANIFEST_PATH))?;
        fs::create_dir(root.join("knowledge"))?;
        fs::write(
            root.join("knowledge/design.md"),
            "Durable product contract; never owned by the migration.\n",
        )?;
        fs::write(root.join("AGENTS.md"), "Personal instructions remain.\n")?;
        fs::write(root.join("CLAUDE.md"), "@other-instructions.md\n")?;
        opdev_project::reconcile_agent_files(&root)?;
        let mut record = AdoptionRecord::pending()?;
        record.scope = "Existing scope, pending issue fixture:17".into();
        let decision = record.practices.values_mut().next().context("practice")?;
        decision.state = opdev_project::AdoptionState::InProgress;
        decision.reason = "Original narrow approval; broader rollout revoked in fixture:18".into();
        decision.references = vec!["fixture:17".into(), "fixture:18".into()];
        record.review = Some(AdoptionReview {
            plan_id: record.plan_id(&manifest)?,
            reviewer: "historical fixture owner".into(),
            reference: "fixture:original-scope-only".into(),
            delegation: None,
        });
        record.write_new(&root)?;
        git(&root, &["add", "."])?;
        let mut candidate = manifest;
        candidate.schema = 3;
        candidate
            .assurance
            .profiles
            .retain(|p| p.name != "opdev-core");
        candidate.assurance.engineering = Some(opdev_core::EngineeringPolicy {
            version: "1".into(),
            minimumcd: None,
            review_reference: "fixture:selected policy".into(),
            maintenance_branches: vec![],
        });
        candidate.layout = Some(LayoutPolicy {
            version: 1,
            review_reference: "fixture:selected layout".into(),
        });
        candidate.assurance.review_storage = Some(ReviewStorage {
            version: 1,
            provider: opdev_project::CiProvider::Gitlab,
            repository_id: 7,
            review_reference: "fixture:selected storage".into(),
            retention_authority: "fixture:reviewed retention".into(),
        });
        let request = temp.path().join("request.json");
        fs::write(
            &request,
            serde_json::to_vec(
                &json!({"schema":1,"decision_reference":"fixture:actual request", "project":candidate,
            "ci_review_reference":"fixture:CI remains unverified; no pin claim","ci":{},"history":null}),
            )?,
        )?;
        Ok(Self {
            temp,
            root,
            request,
        })
    }
    fn preview(&self) -> Result<MigrationPlan> {
        assess(&self.root, &self.request, None)
    }
}

#[test]
fn preview_is_read_only_and_apply_preserves_owners_pending_choices_and_root_text() -> Result<()> {
    let f = Fixture::new()?;
    let before = inventory::text(&f.root.join(MANIFEST_PATH))?;
    let original = AdoptionRecord::load(&f.root)?.context("adoption")?;
    let plan = f.preview()?;
    assert!(!plan.blocked());
    assert_eq!(before, inventory::text(&f.root.join(MANIFEST_PATH))?);
    assert!(!f.root.join(".opdev/guidance.md").exists());
    let recovery = f.temp.path().join("recovery.json");
    run(
        &f.root,
        Some(&f.request),
        None,
        Some(&plan.plan_id),
        Some(&recovery),
        None,
    )?;
    let saved: MigrationPlan = serde_json::from_slice(&fs::read(&recovery)?)?;
    assert_eq!(saved.identity()?, plan.plan_id);
    let old = saved
        .changes
        .iter()
        .find(|c| c.path == ADOPTION_PATH)
        .context("old")?;
    assert!(
        old.before
            .as_ref()
            .context("original")?
            .contains("fixture:original-scope-only")
    );
    let new = AdoptionRecord::load(&f.root)?.context("adoption")?;
    assert_eq!(new.scope, original.scope);
    assert!(
        new.review.is_none(),
        "old approval cannot authorize different policy"
    );
    for (id, decision) in original.practices {
        assert_eq!(
            serde_json::to_value(decision)?,
            serde_json::to_value(&new.practices[&id])?
        );
    }
    assert!(new.practices.values().all(|p| matches!(
        p.state,
        opdev_project::AdoptionState::Pending | opdev_project::AdoptionState::InProgress
    )));
    assert!(
        fs::read_to_string(f.root.join("AGENTS.md"))?.starts_with("Personal instructions remain.")
    );
    assert!(fs::read_to_string(f.root.join("CLAUDE.md"))?.starts_with("@other-instructions.md"));
    assert_eq!(
        fs::read_to_string(f.root.join("knowledge/design.md"))?,
        "Durable product contract; never owned by the migration.\n"
    );
    let guide = fs::read_to_string(f.root.join(".opdev/guidance.md"))?;
    assert!(guide.contains("Never recreate an active .opdev/evidence.yaml"));
    assert!(!guide.contains("record only reviewable facts in `.opdev/evidence.yaml`"));
    run(
        &f.root,
        None,
        Some(&recovery),
        Some(&plan.plan_id),
        None,
        None,
    )?;
    assert!(
        !f.preview()?.changed(),
        "already applied proposal is a no-op"
    );
    Ok(())
}

#[test]
fn interrupted_apply_resumes_original_states_but_never_overwrites_later_work() -> Result<()> {
    let f = Fixture::new()?;
    let plan = f.preview()?;
    assert!(apply_plan(&plan, 2).is_err());
    assert_eq!(
        ProjectManifest::load(&f.root.join(MANIFEST_PATH))?.schema,
        3
    );
    assert!(!f.root.join(".opdev/guidance.md").exists());
    apply_plan(&plan, usize::MAX)?;
    assert!(f.root.join(".opdev/guidance.md").exists());
    let changed = "Later developer instruction must survive.\n";
    fs::write(f.root.join("AGENTS.md"), changed)?;
    assert!(apply_plan(&plan, usize::MAX).is_err());
    assert_eq!(fs::read_to_string(f.root.join("AGENTS.md"))?, changed);
    Ok(())
}

#[test]
fn changed_inputs_new_junk_missing_recovery_and_forged_snapshot_are_not_applied() -> Result<()> {
    let f = Fixture::new()?;
    let plan = f.preview()?;
    assert!(
        run(
            &f.root,
            Some(&f.request),
            None,
            Some(&plan.plan_id),
            None,
            None
        )
        .is_err()
    );
    fs::write(f.root.join(".opdev/random.md"), "task scratch work")?;
    assert!(apply_plan(&plan, usize::MAX).is_err());
    assert!(f.preview()?.blocked());
    assert_eq!(
        ProjectManifest::load(&f.root.join(MANIFEST_PATH))?.schema,
        1
    );
    let mut value = serde_json::to_value(&plan)?;
    value["changes"][0]["after"] = json!("forged replacement");
    let snapshot = f.temp.path().join("forged.json");
    fs::write(&snapshot, serde_json::to_vec(&value)?)?;
    assert!(
        run(
            &f.root,
            None,
            Some(&snapshot),
            Some(&plan.plan_id),
            None,
            None
        )
        .is_err()
    );
    assert_eq!(
        fs::read_to_string(f.root.join(".opdev/random.md"))?,
        "task scratch work"
    );
    Ok(())
}

#[test]
fn retained_history_missing_archive_and_unsupported_records_never_get_removed() -> Result<()> {
    let f = Fixture::new()?;
    let history = "schema: 2\nproject: []\nchanges: []\n";
    fs::write(f.root.join(EVIDENCE_PATH), history)?;
    assert!(f.preview()?.blocked());
    assert_eq!(fs::read_to_string(f.root.join(EVIDENCE_PATH))?, history);
    fs::write(
        f.root.join(EVIDENCE_PATH),
        "schema: 99\nproject: []\nchanges: []\n",
    )?;
    assert!(f.preview().is_err());
    assert!(fs::read_to_string(f.root.join(EVIDENCE_PATH))?.contains("99"));
    Ok(())
}

#[test]
fn staged_links_and_authority_reassignment_need_explicit_separate_resolution() -> Result<()> {
    let f = Fixture::new()?;
    let mut request: serde_json::Value = serde_json::from_slice(&fs::read(&f.request)?)?;
    request["project"]["authorities"]["contracts"]["location"] = json!("other");
    fs::write(&f.request, serde_json::to_vec(&request)?)?;
    assert!(f.preview()?.blocked());
    let object = Command::new("git")
        .arg("-C")
        .arg(&f.root)
        .args(["hash-object", "AGENTS.md"])
        .output()?;
    assert!(object.status.success());
    let hash = std::str::from_utf8(&object.stdout)?.trim();
    git(
        &f.root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("120000,{hash},AGENTS.md"),
        ],
    )?;
    assert!(f.preview().is_err());
    assert!(
        fs::read_to_string(f.root.join("AGENTS.md"))?.contains("Personal instructions remain.")
    );
    Ok(())
}

#[test]
fn ci_edits_are_explicit_parsed_and_stale_ci_never_gets_overwritten() -> Result<()> {
    let f = Fixture::new()?;
    let path = f.root.join(".gitlab-ci.yml");
    let before = "job:\n  script: echo original\n";
    let after = "job:\n  script: echo reviewed\n";
    fs::write(&path, before)?;
    let mut request: serde_json::Value = serde_json::from_slice(&fs::read(&f.request)?)?;
    request["ci"] = json!({".gitlab-ci.yml":after});
    fs::write(&f.request, serde_json::to_vec(&request)?)?;
    let plan = f.preview()?;
    assert!(!plan.blocked());
    assert_eq!(fs::read_to_string(&path)?, before);
    fs::write(&path, "job:\n  script: echo later-edit\n")?;
    assert!(apply_plan(&plan, usize::MAX).is_err());
    assert!(fs::read_to_string(&path)?.contains("later-edit"));
    fs::write(&path, before)?;
    apply_plan(&plan, usize::MAX)?;
    assert_eq!(fs::read_to_string(&path)?, after);
    request["ci"][".gitlab-ci.yml"] = json!("job: [broken");
    fs::write(&f.request, serde_json::to_vec(&request)?)?;
    assert!(f.preview().is_err());
    assert_eq!(fs::read_to_string(&path)?, after);
    Ok(())
}

#[test]
fn request_schema_and_runtime_refuse_unknown_policy_without_normalization() -> Result<()> {
    let f = Fixture::new()?;
    let registry = jsonschema::Registry::new()
        .extend([
            (
                "https://opdev.dev/schema/project.json",
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../../schema/project.schema.json"
                ))?,
            ),
            (
                "https://opdev.dev/schema/evidence-archive-v1.json",
                serde_json::from_str::<serde_json::Value>(include_str!(
                    "../../../../schema/evidence-archive.schema.json"
                ))?,
            ),
        ])?
        .prepare()?;
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../schema/migration-request.schema.json"
    ))?;
    let validator = jsonschema::options()
        .with_registry(&registry)
        .build(&schema)?;
    let mut request: serde_json::Value = serde_json::from_slice(&fs::read(&f.request)?)?;
    assert!(validator.is_valid(&request));
    request["automatic_approval"] = json!(true);
    assert!(!validator.is_valid(&request));
    fs::write(&f.request, serde_json::to_vec(&request)?)?;
    assert!(f.preview().is_err());
    request
        .as_object_mut()
        .context("request")?
        .remove("automatic_approval");
    request["project"]["schema"] = json!(99);
    fs::write(&f.request, serde_json::to_vec(&request)?)?;
    assert!(f.preview().is_err());
    assert_eq!(
        ProjectManifest::load(&f.root.join(MANIFEST_PATH))?.schema,
        1
    );
    Ok(())
}

#[test]
fn large_original_history_is_preserved_and_oversized_input_is_not_trimmed() -> Result<()> {
    let f = Fixture::new()?;
    let path = f.root.join(EVIDENCE_PATH);
    let large = format!(
        "# {}\nschema: 2\nproject: []\nchanges: []\n",
        "historical-original ".repeat(60_000)
    );
    fs::write(&path, &large)?;
    let original = sha(large.as_bytes());
    assert!(
        f.preview()?.blocked(),
        "large original still needs retained archive"
    );
    assert_eq!(sha(&fs::read(&path)?), original);
    let oversized = "#".repeat(8 * 1024 * 1024 + 1);
    fs::write(&path, &oversized)?;
    assert!(f.preview().is_err());
    assert_eq!(fs::read(&path)?.len(), oversized.len());
    Ok(())
}

#[test]
fn invalid_history_diagnostic_does_not_echo_retained_private_content() -> Result<()> {
    let f = Fixture::new()?;
    let private = "private-historical-context-never-print";
    let bytes = format!("schema: 2\nproject: []\nchanges: []\n{private}: invalid\n");
    let path = f.root.join(EVIDENCE_PATH);
    fs::write(&path, &bytes)?;
    let error = format!("{:#}", f.preview().err().expect("unknown ledger field"));
    assert!(error.contains("Original ledger is malformed or unsupported"));
    assert!(!error.contains(private));
    assert!(error.len() < 256);
    assert_eq!(fs::read_to_string(&path)?, bytes);
    Ok(())
}
