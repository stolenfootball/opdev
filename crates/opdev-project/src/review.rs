//! Portable semantic review, deliberately separate from execution and consent.
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    CiProvider, EvidenceLedger, ManifestError, ProjectManifest, TestStage, staged_fingerprint,
};

/// Explicit original work item; no URL guessing or broader transcript capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkSelector {
    /// Hosted provider.
    pub provider: CiProvider,
    /// Stable provider project ID.
    pub repository_id: u64,
    /// Issue or change request.
    pub kind: WorkKind,
    /// Project-local issue/change-request number.
    pub number: u64,
    /// Optional exact comment/note; omission selects the item description.
    pub note_id: Option<u64>,
}

/// Portable work authority kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkKind {
    /// Issue description or note.
    Issue,
    /// GitLab MR or GitHub PR description or note.
    MergeRequest,
}

/// Bound minimal observation, not proof of approval or permanently current policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkObservation {
    /// Observation format.
    pub schema: u32,
    /// Original provider location.
    pub selector: WorkSelector,
    /// Provider-observed author's stable numeric identity.
    pub author_id: u64,
    /// Original provider timestamp.
    pub created_at: String,
    /// Provider timestamp of the observed body version.
    pub updated_at: String,
    /// SHA-256 of the complete original Markdown body, not just the excerpt.
    pub body_sha256: String,
    /// Explicit exact excerpt needed for this work; no automatic transcript copy.
    pub excerpt: String,
    /// Local observation time in Unix seconds; not a signed timestamp.
    pub observed_at: u64,
}

impl WorkSelector {
    /// # Errors
    /// Reject unsupported providers or absent numeric identities.
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            matches!(self.provider, CiProvider::Github | CiProvider::Gitlab)
                && self.repository_id > 0
                && self.number > 0
                && self.note_id != Some(0),
            "Unsupported work observation selection"
        );
        Ok(())
    }
}

impl WorkObservation {
    /// Validate shape only; saved observations do not authenticate their own origin.
    /// # Errors
    /// Reject unsupported records, unbounded excerpts and incomplete provenance.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.selector.validate()?;
        anyhow::ensure!(
            self.schema == 1
                && self.author_id > 0
                && self.observed_at > 0
                && !self.created_at.trim().is_empty()
                && !self.updated_at.trim().is_empty()
                && self.body_sha256.len() == 64
                && self
                    .body_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && !self.excerpt.trim().is_empty()
                && self.excerpt.len() <= 16 * 1024,
            "Unsupported work observation or incomplete provenance"
        );
        Ok(())
    }
}

/// Explicit storage trust boundary, selected through reviewed project migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewStorage {
    /// Supported boundary version.
    pub version: u32,
    /// Hosted provider supplying authenticated review content.
    pub provider: CiProvider,
    /// Stable code repository identity in version 2; archive repository in version 1.
    pub repository_id: u64,
    /// Actual scoped developer decision; a string does not authenticate consent.
    pub review_reference: String,
    /// Existing authority for owner, access, lifetime, reachability and recovery evidence.
    pub retention_authority: String,
    /// Routine CI report lifetime for discussion storage (version 2), not review expiry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_retention_days: Option<u32>,
}

impl ReviewStorage {
    /// Validate only declaration shape; retention behavior needs its own evidence.
    /// # Errors
    /// Reject unsupported boundaries and missing authority references.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if !matches!(self.version, 1 | 2)
            || self.repository_id == 0
            || !matches!(self.provider, CiProvider::Github | CiProvider::Gitlab)
            || self.review_reference.trim().is_empty()
            || self.retention_authority.trim().is_empty()
            || match self.version {
                1 => self.report_retention_days.is_some(),
                2 => !matches!(self.report_retention_days, Some(1..=3650)),
                _ => true,
            }
        {
            return Err(ManifestError::Semantic("Review storage needs a supported provider, repository identity, decision and retention authority. Version 1 selects a Git archive; version 2 selects MR/PR review and requires report_retention_days (1-3650). Configuration does not prove retention or consent.".into()));
        }
        Ok(())
    }
}

/// One exact review inventory and attributed judgments; never an execution report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewRecord {
    /// Minimal original work observations; integrity-bound, never inferred approvals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub work_observations: Vec<WorkObservation>,
    /// Record format.
    pub schema: u32,
    /// Must be `semantic_review`.
    pub kind: String,
    /// All material staged source (not a commit containing this envelope).
    pub source_sha256: String,
    /// Exact effective project configuration.
    pub configuration_sha256: String,
    /// Boundary for which this mapping was selected.
    pub stage: TestStage,
    /// Independent identity of conditions, assertions and judgments.
    pub acceptance_sha256: String,
    /// Reuses schema-2 evidence without growing history or command output.
    pub ledger: EvidenceLedger,
}

impl ReviewRecord {
    /// All external authorities needed by this exact review, including requirements.
    #[must_use]
    pub fn authority_observations(&self) -> Vec<WorkObservation> {
        let mut observations = self.work_observations.clone();
        for change in &self.ledger.changes {
            if let Some(inventory) = &change.acceptance {
                for condition in &inventory.conditions {
                    if let crate::acceptance::RequirementSource::Work(observation) =
                        &condition.source
                        && !observations.contains(observation)
                    {
                        observations.push(observation.clone());
                    }
                }
            }
        }
        observations
    }
    /// Render one bounded review for an existing MR/PR, without logs or older changes.
    /// # Errors
    /// Refuse records exceeding the portable 60 KiB discussion limit.
    pub fn discussion_body(&self) -> anyhow::Result<String> {
        let json = crate::review_wire::encode(self)?
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('`', "\\u0060");
        let body = format!(
            "### OpDev acceptance review\n\nSource: `{}`\n\nStage: `{:?}`. Review identity: `{}`.\n\nThis records assertion review, not test execution or developer consent. Required checks remain separate.\n\n<details><summary>Conditions, test mappings and review limits</summary>\n\n<!-- opdev-review:v2 -->\n```json\n{json}\n```\n<!-- opdev-review:end -->\n</details>\n",
            self.source_sha256, self.stage, self.acceptance_sha256
        );
        anyhow::ensure!(
            body.len() <= 60 * 1024,
            "Review exceeds 60 KiB. Keep only this change's conditions, mappings and necessary context; do not attach logs or history."
        );
        Ok(body)
    }

    /// Parse only an unambiguous current review section; never infer the latest record.
    /// # Errors
    /// Refuse missing, duplicate, oversized or malformed records without echoing content.
    pub fn from_discussion_body(body: &str) -> anyhow::Result<Self> {
        const V1: &str = "<!-- opdev-review:v1 -->\n```json\n";
        const V2: &str = "<!-- opdev-review:v2 -->\n```json\n";
        const END: &str = "\n```\n<!-- opdev-review:end -->";
        anyhow::ensure!(
            body.len() <= 60 * 1024
                && body.matches("<!-- opdev-review:").count() == 2
                && body.matches("<!-- opdev-review:end -->").count() == 1,
            "Select exactly one bounded MR/PR review section; missing or ambiguous content cannot qualify a check."
        );
        let version_two = body.contains(V2);
        let json = body
            .split_once(if version_two { V2 } else { V1 })
            .and_then(|(_, rest)| rest.split_once(END))
            .map(|(json, _)| json)
            .ok_or_else(|| anyhow::anyhow!("MR/PR review section is incomplete or unsupported"))?;
        (if version_two {
            crate::review_wire::decode(json)
        } else {
            serde_json::from_str(json).map_err(Into::into)
        })
        .map_err(|_| {
            anyhow::anyhow!("MR/PR review is malformed or unsupported; no private content echoed")
        })
    }

    /// Bind explicitly selected retained work observations into the review identity.
    /// # Errors
    /// Reject unsupported/malformed observations; never infer decision meaning.
    pub fn bind_observations(&mut self, observations: Vec<WorkObservation>) -> anyhow::Result<()> {
        anyhow::ensure!(
            observations.len() <= 64,
            "Too many work observations for one review"
        );
        for observation in &observations {
            observation.validate()?;
        }
        self.work_observations = observations;
        self.acceptance_sha256 = self.review_digest()?;
        Ok(())
    }

    fn review_digest(&self) -> anyhow::Result<String> {
        let change = self
            .ledger
            .changes
            .first()
            .ok_or_else(|| anyhow::anyhow!("Missing change"))?;
        let inventory = change
            .acceptance
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing inventory"))?;
        let digest = inventory.digest(&change.fingerprint, &change.work)?;
        if self.work_observations.is_empty() {
            return Ok(digest);
        }
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&serde_json::json!({
                "protocol":1, "inventory_sha256":digest, "work_observations":self.work_observations
            }))?)
        ))
    }
    /// Mechanically bind an explicitly provided ledger's exact-current change.
    /// Does not approve its judgments or write files.
    /// # Errors
    /// Reject stale source, missing/ambiguous current inventory or unsupported records.
    pub fn prepare(
        root: &Path,
        manifest: &ProjectManifest,
        stage: TestStage,
        ledger: &EvidenceLedger,
    ) -> anyhow::Result<Self> {
        ledger.validate(&manifest.catalog()?)?;
        let source_sha256 = staged_fingerprint(root)?;
        let change = ledger
            .matching_change(&source_sha256)
            .ok_or_else(|| {
                anyhow::anyhow!("No exact-current semantic review; no older change substituted")
            })?
            .clone();
        let acceptance_sha256 = change
            .acceptance
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Acceptance inventory is missing"))?
            .digest(&source_sha256, &change.work)?;
        let record = Self {
            work_observations: vec![],
            schema: 1,
            kind: "semantic_review".into(),
            source_sha256,
            configuration_sha256: configuration_digest(manifest)?,
            stage,
            acceptance_sha256,
            ledger: EvidenceLedger {
                schema: 2,
                project: ledger.project.clone(),
                changes: vec![change],
            },
        };
        record.verify_current(root, manifest, stage, &record.acceptance_sha256)?;
        Ok(record)
    }

    /// Verify the complete review subject independently of storage origin.
    /// Does not turn a passed review into test execution or authenticated approval.
    /// # Errors
    /// Reject a mismatched source/configuration/stage/review or unsupported structure.
    pub fn verify_current(
        &self,
        root: &Path,
        manifest: &ProjectManifest,
        stage: TestStage,
        acceptance: &str,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.schema == 1
                && self.kind == "semantic_review"
                && self.ledger.schema == 2
                && self.ledger.changes.len() == 1,
            "Unsupported semantic review record; no saved report substituted"
        );
        self.ledger.validate(&manifest.catalog()?)?;
        anyhow::ensure!(
            self.work_observations.len() <= 64,
            "Too many work observations"
        );
        for observation in &self.work_observations {
            observation.validate()?;
        }
        let change = &self.ledger.changes[0];
        let inventory = change
            .acceptance
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Review inventory is missing"))?;
        anyhow::ensure!(
            self.source_sha256 == staged_fingerprint(root)?
                && change.fingerprint == self.source_sha256
                && self.configuration_sha256 == configuration_digest(manifest)?
                && self.stage == stage
                && self.acceptance_sha256 == acceptance
                && self.acceptance_sha256 == self.review_digest()?,
            "Semantic review does not match the selected source, configuration, stage or acceptance inventory"
        );
        for condition in &inventory.conditions {
            anyhow::ensure!(
                !matches!(
                    condition.source,
                    crate::acceptance::RequirementSource::Work(_)
                ) || manifest
                    .assurance
                    .review_storage
                    .as_ref()
                    .is_some_and(|p| p.version == 2),
                "Work requirement observations need selected MR/PR review policy 2; no local capture can qualify legacy acceptance"
            );
            condition.source.verify(root)?;
        }
        for mapping in &inventory.verifications {
            mapping.target.verify(root)?;
        }
        Ok(())
    }
}

fn configuration_digest(manifest: &ProjectManifest) -> anyhow::Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(manifest)?)
    ))
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    #[test]
    fn bounded_report_policy_is_explicit_and_archive_version_is_unchanged() -> anyhow::Result<()> {
        let mut policy = ReviewStorage {
            version: 2,
            provider: CiProvider::Gitlab,
            repository_id: 7,
            review_reference: "actual project decision".into(),
            retention_authority: "existing work policy".into(),
            report_retention_days: Some(30),
        };
        policy.validate()?;
        for days in [None, Some(0), Some(3651)] {
            policy.report_retention_days = days;
            assert!(policy.validate().is_err());
        }
        policy.version = 1;
        policy.report_retention_days = None;
        policy.validate()?;
        assert!(
            serde_json::to_value(&policy)?
                .get("report_retention_days")
                .is_none(),
            "legacy serialization and digests unchanged"
        );
        policy.report_retention_days = Some(30);
        assert!(
            policy.validate().is_err(),
            "no automatic reinterpretation of archive lifetime"
        );
        Ok(())
    }
}
