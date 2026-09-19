//! Change-scoped acceptance evidence, not an automated semantic proof.

use std::path::Path;

use opdev_core::Outcome;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::evidence::{EvidenceError, git_output};

/// Exact tracked bytes and excerpt supporting a reviewed claim.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrackedEvidence {
    /// Repository-relative regular file; never a URL or a symlink.
    pub path: String,
    /// SHA-256 of the complete Git-index blob, before checkout transformations.
    pub sha256: String,
    /// Exact, nonempty excerpt identifying the requirement or assertion.
    pub excerpt: String,
}

impl TrackedEvidence {
    /// Checks the referenced staged regular file without following filesystem links.
    ///
    /// # Errors
    /// Returns an evidence error for missing, stale, nonregular or oversized input.
    pub fn verify(&self, root: &Path) -> Result<(), EvidenceError> {
        self.validate()?;
        let entries = git_output(
            root,
            &[
                "--literal-pathspecs",
                "ls-files",
                "--stage",
                "-z",
                "--",
                &self.path,
            ],
        )?;
        let entry = String::from_utf8_lossy(&entries);
        if !(entry.starts_with("100644 ") || entry.starts_with("100755 "))
            || !entry.ends_with(&format!(" 0\t{}\0", self.path))
            || entry.bytes().filter(|byte| *byte == 0).count() != 1
        {
            return Err(EvidenceError::Semantic(
                "acceptance reference must identify one staged regular file".into(),
            ));
        }
        let object = format!(":{}", self.path);
        let size = git_output(root, &["cat-file", "-s", &object])?;
        let size = String::from_utf8_lossy(&size)
            .trim()
            .parse::<usize>()
            .map_err(|_| EvidenceError::Semantic("invalid acceptance source size".into()))?;
        if size > 8 * 1024 * 1024 {
            return Err(EvidenceError::Semantic(
                "acceptance source exceeds 8 MiB".into(),
            ));
        }
        let bytes = git_output(root, &["cat-file", "blob", &object])?;
        if format!("{:x}", Sha256::digest(&bytes)) != self.sha256
            || !std::str::from_utf8(&bytes).is_ok_and(|text| text.contains(&self.excerpt))
        {
            return Err(EvidenceError::Semantic(
                "acceptance source digest or excerpt does not match the staged file".into(),
            ));
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), EvidenceError> {
        if self.path.is_empty()
            || self.path.contains(['\\', ':', '\0', '\n', '\r'])
            || self
                .path
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".." | ".git"))
            || self.path == crate::EVIDENCE_PATH
            || !digest_valid(&self.sha256)
            || self.excerpt.trim().is_empty()
        {
            return Err(EvidenceError::Semantic(
                "invalid tracked acceptance reference".into(),
            ));
        }
        Ok(())
    }
}

/// Reviewed classification, not inferred from file extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceScope {
    /// At least one delivered behavior is changed.
    Behavioral,
    /// Material conditions exist but delivered behavior is unchanged.
    NonBehavioral,
    /// No material acceptance conditions; requires a reviewed justification.
    NoMaterialConditions,
}

/// One material condition in the reviewed inventory.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCondition {
    /// Change-local stable identity.
    pub id: String,
    /// Observable accepted condition, not inferred from implementation.
    pub statement: String,
    /// Original authority (including a tracker URL when applicable).
    pub authority: String,
    /// Versioned source or explicitly identified captured authority excerpt.
    pub source: TrackedEvidence,
}

/// Tool-independent method; no reporter or test-runner dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceMethod {
    /// A reviewed assertion exercised by a declared canonical suite.
    Automated,
    /// Concrete non-executable verification, with automation limitations stated.
    Review,
}

/// Verification mapping for exactly one inventoried acceptance condition.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceVerification {
    /// Inventory condition ID.
    pub condition: String,
    /// Appropriate reviewed verification method.
    pub method: AcceptanceMethod,
    /// Exact assertion or reviewed non-code evidence.
    pub target: TrackedEvidence,
    /// Explanation of the observable property actually checked.
    pub assertion: String,
    /// Counterexample/boundary and expected result derived from the requirement.
    pub discriminating_case: String,
    /// Canonical suite ID, required only for automated verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suite: Option<String>,
    /// Why automation cannot establish this condition; not a core-rule waiver.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation_limitation: Option<String>,
    /// Reviewed mapping verdict: passed, failed or unverified; never execution proof.
    pub outcome: Outcome,
}

/// Review of the exact inventory and mappings; claims, not authenticated approval.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceReview {
    /// Passed, failed or unverified; templates start unverified.
    pub outcome: Outcome,
    /// Identified human or agent reviewer; never implies developer consent.
    pub reviewer: String,
    /// Concrete review record, not merely an owner label.
    pub reference: String,
    /// Completeness, applicability and assertion-adequacy reasoning and limits.
    pub rationale: String,
    /// Digest of the exact change/work/inventory/mappings, excluding this review.
    pub subject_sha256: String,
}

/// Structured acceptance section inside the existing schema-2 evidence ledger.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceEvidence {
    /// Reviewed applicability.
    pub scope: AcceptanceScope,
    /// Why this scope and its exclusions are appropriate.
    pub rationale: String,
    /// All material conditions and selected risk objectives in this change.
    pub conditions: Vec<AcceptanceCondition>,
    /// One mapping per condition; incomplete mappings remain unverified.
    pub verifications: Vec<AcceptanceVerification>,
    /// Review bound to this exact payload and staged change.
    pub review: AcceptanceReview,
}

impl Default for AcceptanceEvidence {
    fn default() -> Self {
        Self {
            scope: AcceptanceScope::Behavioral,
            rationale: String::new(),
            conditions: Vec::new(),
            verifications: Vec::new(),
            review: AcceptanceReview {
                outcome: Outcome::Unverified,
                reviewer: String::new(),
                reference: String::new(),
                rationale: String::new(),
                subject_sha256: String::new(),
            },
        }
    }
}

impl AcceptanceEvidence {
    /// Computes the review subject without making any approval decision.
    ///
    /// # Errors
    /// Returns a serialization error if the payload cannot be encoded.
    pub fn digest(&self, fingerprint: &str, work: &str) -> Result<String, EvidenceError> {
        let payload = serde_json::json!({
            "protocol": 1, "fingerprint": fingerprint, "work": work,
            "scope": self.scope, "rationale": self.rationale,
            "conditions": self.conditions, "verifications": self.verifications,
        });
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&payload)?)
        ))
    }

    pub(crate) fn validate(&self) -> Result<(), EvidenceError> {
        let mut ids = std::collections::HashSet::new();
        for condition in &self.conditions {
            if condition.id.trim().is_empty()
                || condition.statement.trim().is_empty()
                || condition.authority.trim().is_empty()
                || !ids.insert(&condition.id)
            {
                return Err(EvidenceError::Semantic(
                    "acceptance conditions need unique IDs, statements and authorities".into(),
                ));
            }
            condition.source.validate()?;
        }
        let mut mapped = std::collections::HashSet::new();
        for verification in &self.verifications {
            if !ids.contains(&verification.condition)
                || !mapped.insert(&verification.condition)
                || verification.assertion.trim().is_empty()
                || verification.discriminating_case.trim().is_empty()
                || !review_outcome(verification.outcome)
            {
                return Err(EvidenceError::Semantic(
                    "invalid, duplicate or unknown acceptance mapping".into(),
                ));
            }
            verification.target.validate()?;
            match verification.method {
                AcceptanceMethod::Automated if verification.suite.as_ref().is_some_and(|id| !id.trim().is_empty())
                    && verification.automation_limitation.is_none() => (),
                AcceptanceMethod::Review if verification.suite.is_none()
                    && verification.automation_limitation.as_ref().is_some_and(|reason| !reason.trim().is_empty()) => (),
                _ => return Err(EvidenceError::Semantic("automated mappings need a suite; review mappings need an automation limitation".into())),
            }
        }
        if !review_outcome(self.review.outcome) {
            return Err(EvidenceError::Semantic(
                "acceptance reviews allow only passed, failed or unverified".into(),
            ));
        }
        Ok(())
    }
}

fn review_outcome(outcome: Outcome) -> bool {
    matches!(
        outcome,
        Outcome::Passed | Outcome::Failed | Outcome::Unverified
    )
}

fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
