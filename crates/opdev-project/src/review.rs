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
    /// Hosted provider supplying authenticated exact Git bytes.
    pub provider: CiProvider,
    /// Stable archive repository identity, not a mutable name.
    pub repository_id: u64,
    /// Actual scoped developer decision; a string does not authenticate consent.
    pub review_reference: String,
    /// Existing authority for owner, access, lifetime, reachability and recovery evidence.
    pub retention_authority: String,
}

impl ReviewStorage {
    /// Validate only declaration shape; retention behavior needs its own evidence.
    /// # Errors
    /// Reject unsupported boundaries and missing authority references.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.version != 1
            || self.repository_id == 0
            || !matches!(self.provider, CiProvider::Github | CiProvider::Gitlab)
            || self.review_reference.trim().is_empty()
            || self.retention_authority.trim().is_empty()
        {
            return Err(ManifestError::Semantic("review storage needs version 1, a supported provider, numeric repository identity, actual decision and retention authority; no retention or consent inferred".into()));
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
