//! Provider-observed semantic inputs cannot construct canonical executions.
use opdev_project::{EvidenceLedger, ProjectManifest, ReviewRecord, TestStage};
use opdev_remote::ArchiveObservation;
use std::path::Path;

/// Sealed, authenticated storage observation; attributed judgments are not consent.
/// No deserialization or conversion from a saved check report exists.
pub struct ValidatedReview {
    record: ReviewRecord,
    location: String,
    discussion: Option<opdev_remote::DiscussionObservation>,
}

impl ValidatedReview {
    /// Read the selected semantic inputs only while their exact subject is current.
    /// This does not establish consent or replace current command execution.
    ///
    /// # Errors
    /// Reject source, configuration, stage or acceptance changes since retrieval.
    pub fn reviewed_ledger(
        &self,
        root: &Path,
        manifest: &ProjectManifest,
        stage: TestStage,
    ) -> Result<&EvidenceLedger, String> {
        self.current(root, manifest, stage)?;
        Ok(self.ledger())
    }

    /// Validate exact provider bytes against selected policy and independent subject.
    /// # Errors
    /// Refuse absent/wrong storage policy, unsupported bytes or mismatched subjects.
    pub fn from_archive(
        root: &Path,
        manifest: &ProjectManifest,
        stage: TestStage,
        acceptance: &str,
        observed: &ArchiveObservation,
    ) -> Result<Self, String> {
        let policy = manifest.assurance.review_storage.as_ref().ok_or("External semantic review policy was not selected; do not substitute an archive for the project ledger")?;
        let locator = observed.locator();
        if policy.version != 1
            || policy.provider != locator.provider
            || policy.repository_id != locator.repository_id
        {
            return Err(
                "Semantic review came from a repository outside the selected policy".into(),
            );
        }
        let record = validate_bytes(root, manifest, stage, acceptance, observed.bytes())?;
        opdev_remote::recheck_work(&record.authority_observations())?;
        Ok(Self {
            record,
            discussion: None,
            location: format!(
                "authenticated archive {:?}:{}/{}:{}#sha256={}",
                locator.provider,
                locator.repository_id,
                locator.commit,
                locator.path,
                locator.sha256
            ),
        })
    }

    /// Validate a provider-observed current MR/PR record under explicit storage policy 2.
    /// # Errors
    /// Reject other policies, identities and stale semantic inputs; never supply execution.
    pub fn from_discussion(
        root: &Path,
        manifest: &ProjectManifest,
        stage: TestStage,
        acceptance: &str,
        observed: opdev_remote::DiscussionObservation,
    ) -> Result<Self, String> {
        let policy = manifest
            .assurance
            .review_storage
            .as_ref()
            .ok_or("MR/PR review policy was not selected")?;
        let selection = &observed.locator().selector;
        if policy.version != 2
            || policy.provider != selection.provider
            || policy.repository_id != selection.repository_id
        {
            return Err("MR/PR review is outside the selected project policy".into());
        }
        let record = observed.record().clone();
        observed.verify_project(manifest)?;
        observed.verify_source(root, stage)?;
        record.verify_current(root, manifest, stage, acceptance).map_err(|_| "MR/PR review does not match the current source, configuration, stage or acceptance conditions")?;
        opdev_remote::recheck_work(&record.authority_observations())?;
        let location = format!(
            "provider-observed MR/PR {:?}:{}/{} note {:?} body {}",
            selection.provider,
            selection.repository_id,
            selection.number,
            selection.note_id,
            observed.locator().body_sha256
        );
        Ok(Self {
            record,
            location,
            discussion: Some(observed),
        })
    }

    pub(crate) fn current(
        &self,
        root: &Path,
        manifest: &ProjectManifest,
        stage: TestStage,
    ) -> Result<(), String> {
        if let Some(observed) = &self.discussion {
            observed.verify_source(root, stage)?;
        }
        self.record.verify_current(root, manifest, stage, &self.record.acceptance_sha256)
            .map_err(|_| "Selected review no longer matches the current source, configuration, stage or inventory".into())
    }

    pub(crate) const fn ledger(&self) -> &EvidenceLedger {
        &self.record.ledger
    }
    pub(crate) fn location(&self) -> &str {
        &self.location
    }

    pub(crate) fn recheck_authorities(&self) -> Result<(), String> {
        if let Some(observed) = &self.discussion {
            observed.recheck()?;
        }
        opdev_remote::recheck_work(&self.record.authority_observations())
    }
}

fn validate_bytes(
    root: &Path,
    manifest: &ProjectManifest,
    stage: TestStage,
    acceptance: &str,
    bytes: &[u8],
) -> Result<ReviewRecord, String> {
    let record: ReviewRecord = serde_json::from_slice(bytes).map_err(
        |_| "Malformed or unsupported semantic review record; no private content echoed",
    )?;
    record
        .verify_current(root, manifest, stage, acceptance)
        .map_err(|_| "Semantic review subject does not match; no earlier review substituted")?;
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opdev_core::Outcome;
    use opdev_project::{CommandSpec, MANIFEST_PATH, TrackedEvidence, discover};
    use serde_json::json;
    use std::{fs, process::Command};

    fn git(root: &Path, args: &[&str]) -> anyhow::Result<()> {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()?;
        anyhow::ensure!(output.status.success(), "Git fixture failed");
        Ok(())
    }

    fn fixture() -> anyhow::Result<(tempfile::TempDir, ProjectManifest, ValidatedReview)> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        git(root, &["init", "--quiet"])?;
        let mut manifest = discover(root)?.manifest;
        manifest.schema = 3;
        manifest
            .assurance
            .profiles
            .retain(|p| p.name != "opdev-core");
        manifest.assurance.engineering = Some(opdev_core::EngineeringPolicy {
            version: "1".into(),
            minimumcd: None,
            review_reference: "synthetic decision".into(),
            maintenance_branches: vec![],
        });
        manifest.assurance.review_storage = Some(opdev_project::ReviewStorage {
            report_retention_days: None,
            version: 1,
            provider: opdev_project::CiProvider::Gitlab,
            repository_id: 7,
            review_reference: "synthetic storage decision".into(),
            retention_authority: "fixture recovery review".into(),
        });
        manifest.commands.insert(
            "behavior".into(),
            CommandSpec {
                argv: vec![
                    if cfg!(windows) { "python" } else { "python3" }.into(),
                    "-B".into(),
                    "tests.py".into(),
                ],
                working_directory: None,
                timeout_seconds: Some(10),
            },
        );
        manifest.testing.suites = vec![opdev_project::TestSuite {
            id: "behavior".into(),
            command: "behavior".into(),
            stages: vec![TestStage::PreMerge],
        }];
        manifest.write_new(&root.join(MANIFEST_PATH))?;
        fs::write(root.join("requirements.md"), "Preserve caller order.\n")?;
        fs::write(
            root.join("product.py"),
            "def select(items):\n    return items[:2]\n",
        )?;
        fs::write(
            root.join("tests.py"),
            "from product import select\nitems = ['c', 'a', 'b']\nassert select(items) == ['c', 'a']\nprint('verified exact order')\n",
        )?;
        git(root, &["add", "."])?;
        let source = TrackedEvidence::bind(
            root,
            "requirements.md".into(),
            "Preserve caller order.".into(),
        )?;
        let target = TrackedEvidence::bind(
            root,
            "tests.py".into(),
            "assert select(items) == ['c', 'a']".into(),
        )?;
        let mut ledger: EvidenceLedger = serde_json::from_value(
            json!({"schema":2, "project":[], "changes":[{
                "fingerprint":opdev_project::staged_fingerprint(root)?, "work":"fixture work", "assertions":[],
                "acceptance":{"scope":"behavioral", "rationale":"Exact fixture scope only",
                    "conditions":[{"id":"R1", "statement":"Preserve caller order", "authority":"requirements.md", "source":source}],
                    "verifications":[{"condition":"R1", "method":"automated", "target":target, "assertion":"Exact first two items",
                        "discriminating_case":"Sorting or a wrong count changes the expected list", "suite":"behavior", "outcome":"passed"}],
                    "review":{"outcome":"passed", "reviewer":"synthetic reviewer", "reference":"fixture review", "rationale":"Actual assertion checked against R1", "subject_sha256":""}
                }
            }]}),
        )?;
        let change = &mut ledger.changes[0];
        let acceptance = change
            .acceptance
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("inventory"))?;
        acceptance.review.subject_sha256 = acceptance.digest(&change.fingerprint, &change.work)?;
        let record = ReviewRecord::prepare(root, &manifest, TestStage::PreMerge, &ledger)?;
        // Unit-only injection exercises evaluation, not provider authentication.
        let review = ValidatedReview {
            record,
            discussion: None,
            location: "unit fixture (not live provider evidence)".into(),
        };
        Ok((temp, manifest, review))
    }

    #[test]
    fn adoption_can_read_current_selected_review_without_a_repository_ledger() -> anyhow::Result<()>
    {
        let (temp, manifest, review) = fixture()?;
        assert!(!temp.path().join(opdev_project::EVIDENCE_PATH).exists());
        let ledger = review
            .reviewed_ledger(temp.path(), &manifest, TestStage::PreMerge)
            .map_err(anyhow::Error::msg)?;
        assert!(
            ledger
                .matching_change(&opdev_project::staged_fingerprint(temp.path())?)
                .is_some()
        );
        assert!(
            review
                .reviewed_ledger(temp.path(), &manifest, TestStage::PostMerge)
                .is_err()
        );
        fs::write(
            temp.path().join("product.py"),
            "def select(items):\n    return []\n",
        )?;
        git(temp.path(), &["add", "product.py"])?;
        assert!(
            review
                .reviewed_ledger(temp.path(), &manifest, TestStage::PreMerge)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn changing_retained_work_observation_invalidates_its_bound_review_identity()
    -> anyhow::Result<()> {
        let (temp, manifest, mut review) = fixture()?;
        let old = review.record.acceptance_sha256.clone();
        review
            .record
            .bind_observations(vec![opdev_project::WorkObservation {
                schema: 1,
                selector: opdev_project::WorkSelector {
                    provider: opdev_project::CiProvider::Gitlab,
                    repository_id: 7,
                    kind: opdev_project::WorkKind::Issue,
                    number: 3,
                    note_id: Some(11),
                },
                author_id: 9,
                created_at: "2026-10-01T00:00:00Z".into(),
                updated_at: "2026-10-02T00:00:00Z".into(),
                body_sha256: "a".repeat(64),
                excerpt: "Synthetic retained scope, not real approval".into(),
                observed_at: 1_790_000_000,
            }])?;
        assert_ne!(old, review.record.acceptance_sha256);
        assert!(
            review
                .record
                .verify_current(temp.path(), &manifest, TestStage::PreMerge, &old)
                .is_err()
        );
        review
            .current(temp.path(), &manifest, TestStage::PreMerge)
            .map_err(anyhow::Error::msg)?;
        review.record.work_observations[0].excerpt = "Different decision".into();
        assert!(
            review
                .current(temp.path(), &manifest, TestStage::PreMerge)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn discussion_requirements_bind_original_work_without_qualifying_legacy_captures()
    -> anyhow::Result<()> {
        let (temp, mut manifest, review) = fixture()?;
        let root = temp.path();
        let mut ledger = review.record.ledger.clone();
        let observation = opdev_project::WorkObservation {
            schema: 1,
            selector: opdev_project::WorkSelector {
                provider: opdev_project::CiProvider::Gitlab,
                repository_id: 7,
                kind: opdev_project::WorkKind::Issue,
                number: 3,
                note_id: None,
            },
            author_id: 9,
            created_at: "2026-10-08T01:00:00Z".into(),
            updated_at: "2026-10-08T01:00:00Z".into(),
            body_sha256: "a".repeat(64),
            excerpt: "Preserve caller order".into(),
            observed_at: 1_790_000_000,
        };
        ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("inventory"))?
            .conditions[0]
            .source = opdev_project::RequirementSource::Work(observation);
        assert!(
            ReviewRecord::prepare(root, &manifest, TestStage::PreMerge, &ledger).is_err(),
            "archive policy cannot trust a local work capture"
        );
        let policy = manifest
            .assurance
            .review_storage
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("storage"))?;
        policy.version = 2;
        policy.report_retention_days = Some(30);
        manifest.project.ci.provider = opdev_project::CiProvider::Gitlab;
        manifest.project.ci.remote = Some("https://gitlab.com/fixture/product".into());
        fs::write(root.join(MANIFEST_PATH), manifest.to_yaml()?)?;
        git(root, &["add", "."])?;
        let change = &mut ledger.changes[0];
        change.fingerprint = opdev_project::staged_fingerprint(root)?;
        let acceptance = change
            .acceptance
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("inventory"))?;
        acceptance.review.subject_sha256 = acceptance.digest(&change.fingerprint, &change.work)?;
        let mut record = ReviewRecord::prepare(root, &manifest, TestStage::PreMerge, &ledger)?;
        assert_eq!(record.authority_observations().len(), 1);
        let original = record.acceptance_sha256.clone();
        if let opdev_project::RequirementSource::Work(source) = &mut record.ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("inventory"))?
            .conditions[0]
            .source
        {
            source.body_sha256 = "b".repeat(64);
        }
        assert!(
            record
                .verify_current(root, &manifest, TestStage::PreMerge, &original)
                .is_err(),
            "new requirement body invalidates review even with unchanged code"
        );
        assert!(
            crate::evaluate(root, &manifest, crate::CheckOptions::pre_merge()).is_err(),
            "saved work capture is not a provider-observed review"
        );
        Ok(())
    }

    #[test]
    fn semantic_review_never_substitutes_for_actual_execution_or_authenticates_consent()
    -> anyhow::Result<()> {
        let (temp, manifest, review) = fixture()?;
        let root = temp.path();
        let mut options = crate::CheckOptions::pre_merge();
        options.execute_checks = false;
        let report = crate::evaluate_with_review(root, &manifest, options, &review, None)?;
        let acceptance = |report: &crate::CheckReport| {
            report
                .rules
                .iter()
                .find(|r| r.rule_id.as_str() == "OPDEV-TEST-002")
                .map(|r| r.outcome)
        };
        assert_eq!(acceptance(&report), Some(Outcome::Unverified));
        assert!(report.checks.iter().all(|c| c.outcome != Outcome::Passed));
        options.execute_checks = true;
        let report = crate::evaluate_with_review(root, &manifest, options, &review, None)?;
        assert_eq!(acceptance(&report), Some(Outcome::Passed));
        let check = report
            .checks
            .iter()
            .find(|c| c.id == "behavior")
            .ok_or_else(|| anyhow::anyhow!("check"))?;
        assert_eq!(check.outcome, Outcome::Passed);
        assert!(
            check
                .stdout
                .as_deref()
                .unwrap_or_default()
                .contains("verified exact order")
        );
        assert!(
            crate::evaluate(root, &manifest, options).is_err(),
            "selected review cannot fall back to no input"
        );
        fs::write(
            root.join(opdev_project::EVIDENCE_PATH),
            review.record.ledger.to_yaml()?,
        )?;
        assert!(
            crate::evaluate_with_review(root, &manifest, options, &review, None).is_err(),
            "ambiguous legacy input is not silently selected"
        );
        Ok(())
    }

    #[test]
    fn wrong_subjects_unreviewed_judgments_saved_reports_and_private_fields_are_rejected()
    -> anyhow::Result<()> {
        let (temp, manifest, review) = fixture()?;
        let root = temp.path();
        let bytes = serde_json::to_vec(&review.record)?;
        let expected = &review.record.acceptance_sha256;
        assert!(validate_bytes(root, &manifest, TestStage::PreMerge, expected, &bytes).is_ok());
        assert!(validate_bytes(root, &manifest, TestStage::PostMerge, expected, &bytes).is_err());
        assert!(
            validate_bytes(
                root,
                &manifest,
                TestStage::PreMerge,
                &"0".repeat(64),
                &bytes
            )
            .is_err()
        );
        let mut changed = manifest.clone();
        changed
            .assurance
            .review_storage
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("policy"))?
            .repository_id += 1;
        assert!(validate_bytes(root, &changed, TestStage::PreMerge, expected, &bytes).is_err());
        for field in [
            "schema",
            "source_sha256",
            "configuration_sha256",
            "acceptance_sha256",
            "kind",
        ] {
            let mut value = serde_json::to_value(&review.record)?;
            value[field] = if field == "schema" {
                json!(99)
            } else {
                json!("invalid")
            };
            assert!(
                validate_bytes(
                    root,
                    &manifest,
                    TestStage::PreMerge,
                    expected,
                    &serde_json::to_vec(&value)?
                )
                .is_err(),
                "{field}"
            );
        }
        let mut private = serde_json::to_value(&review.record)?;
        private["private-canary-never-echo"] = json!("private content");
        let error = validate_bytes(
            root,
            &manifest,
            TestStage::PreMerge,
            expected,
            &serde_json::to_vec(&private)?,
        )
        .err()
        .ok_or_else(|| anyhow::anyhow!("expected error"))?;
        assert!(!error.contains("canary"));
        Ok(())
    }

    #[test]
    fn pending_review_and_changed_product_cannot_borrow_an_old_pass() -> anyhow::Result<()> {
        let (temp, manifest, review) = fixture()?;
        let root = temp.path();
        let expected = &review.record.acceptance_sha256;
        let mut pending = review.record.clone();
        pending.ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("inventory"))?
            .review
            .outcome = Outcome::Unverified;
        let pending = ValidatedReview {
            discussion: None,
            record: pending,
            location: "unit only".into(),
        };
        let report = crate::evaluate_with_review(
            root,
            &manifest,
            crate::CheckOptions::pre_merge(),
            &pending,
            None,
        )?;
        assert_eq!(
            report
                .rules
                .iter()
                .find(|r| r.rule_id.as_str() == "OPDEV-TEST-002")
                .map(|r| r.outcome),
            Some(Outcome::Unverified)
        );
        assert!(
            validate_bytes(
                root,
                &manifest,
                TestStage::PreMerge,
                expected,
                &serde_json::to_vec(&report)?
            )
            .is_err()
        );
        fs::write(
            root.join("product.py"),
            "def select(items):\n    return sorted(items)\n",
        )?;
        git(root, &["add", "product.py"])?;
        assert!(
            crate::evaluate_with_review(
                root,
                &manifest,
                crate::CheckOptions::pre_merge(),
                &review,
                None
            )
            .is_err()
        );
        let mut ledger = review.record.ledger.clone();
        let change = &mut ledger.changes[0];
        change.fingerprint = opdev_project::staged_fingerprint(root)?;
        let inventory = change
            .acceptance
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("inventory"))?;
        inventory.review.subject_sha256 = inventory.digest(&change.fingerprint, &change.work)?;
        let current = ValidatedReview {
            discussion: None,
            record: ReviewRecord::prepare(root, &manifest, TestStage::PreMerge, &ledger)?,
            location: "unit synthetic refreshed mapping; assertion is unchanged".into(),
        };
        let failed = crate::evaluate_with_review(
            root,
            &manifest,
            crate::CheckOptions::pre_merge(),
            &current,
            None,
        )?;
        assert_eq!(
            failed
                .checks
                .iter()
                .find(|c| c.id == "behavior")
                .map(|c| c.outcome),
            Some(Outcome::Failed)
        );
        assert_eq!(
            failed
                .rules
                .iter()
                .find(|r| r.rule_id.as_str() == "OPDEV-TEST-002")
                .map(|r| r.outcome),
            Some(Outcome::Failed)
        );
        Ok(())
    }
}
