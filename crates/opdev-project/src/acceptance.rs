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
    /// Binds an explicitly chosen excerpt to staged bytes without approving it.
    ///
    /// # Errors
    /// Rejects unsafe paths, missing/nonregular/oversized files and absent excerpts.
    pub fn bind(root: &Path, path: String, excerpt: String) -> Result<Self, EvidenceError> {
        let mut reference = Self {
            path,
            excerpt,
            sha256: "0".repeat(64),
        };
        let bytes = reference.staged_bytes(root)?;
        reference.sha256 = format!("{:x}", Sha256::digest(&bytes));
        reference.verify(root)?;
        Ok(reference)
    }

    /// Checks the referenced staged regular file without following filesystem links.
    ///
    /// # Errors
    /// Returns an evidence error for missing, stale, nonregular or oversized input.
    pub fn verify(&self, root: &Path) -> Result<(), EvidenceError> {
        let bytes = self.staged_bytes(root)?;
        if format!("{:x}", Sha256::digest(&bytes)) != self.sha256
            || !std::str::from_utf8(&bytes).is_ok_and(|text| text.contains(&self.excerpt))
        {
            return Err(EvidenceError::Semantic(
                "acceptance source digest or excerpt does not match the staged file".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn staged_bytes(&self, root: &Path) -> Result<Vec<u8>, EvidenceError> {
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
        git_output(root, &["cat-file", "blob", &object])
    }

    pub(crate) fn validate(&self) -> Result<(), EvidenceError> {
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
    pub source: RequirementSource,
}

/// A permanent source file or a minimal provider-observed work excerpt.
/// Work observations qualify only through authenticated discussion policy 2.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RequirementSource {
    /// Legacy representation preserved byte-for-byte in acceptance digests.
    Tracked(TrackedEvidence),
    /// No duplicate requirement capture is committed to the project.
    Work(crate::WorkObservation),
}

impl RequirementSource {
    /// Shape/source check only. Work currency is checked by the sealed provider reader.
    /// # Errors
    /// Reject malformed work observations or stale tracked bytes.
    pub fn verify(&self, root: &Path) -> Result<(), EvidenceError> {
        match self {
            Self::Tracked(source) => source.verify(root),
            Self::Work(_) => self.validate(),
        }
    }
    fn validate(&self) -> Result<(), EvidenceError> {
        match self {
            Self::Tracked(source) => source.validate(),
            Self::Work(observation) => observation.validate().map_err(|_| {
                EvidenceError::Semantic("Invalid work requirement observation".into())
            }),
        }
    }
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
    /// Explicit boundaries for this mapping; omission preserves all-stage behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stages: Option<Vec<crate::TestStage>>,
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

impl AcceptanceVerification {
    /// Whether this reviewed mapping supports the selected verification boundary.
    #[must_use]
    pub fn applies_to(&self, stage: crate::TestStage) -> bool {
        self.stages
            .as_ref()
            .is_none_or(|stages| stages.contains(&stage))
    }
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
    /// Additional organization duties, bound to this same source and review.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_controls: Option<OrganizationControlReview>,
    /// Current policy duties linked to existing conditions, not another evidence store.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_controls: Option<PolicyControlReview>,
    /// Current catalog and accepted baseline selection; no duplicate enduring criteria.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirements: Option<crate::requirements::ChangeReview>,
    /// Optional versioned safeguard mappings covered by this same review.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safeguards: Option<crate::SafeguardReview>,
    /// Reviewed applicability.
    pub scope: AcceptanceScope,
    /// Why this scope and its exclusions are appropriate.
    pub rationale: String,
    /// All material conditions and selected risk objectives in this change.
    pub conditions: Vec<AcceptanceCondition>,
    /// One applicable mapping per condition/stage; incomplete mappings remain unverified.
    pub verifications: Vec<AcceptanceVerification>,
    /// Review bound to this exact payload and staged change.
    pub review: AcceptanceReview,
}

/// Source-, definition- and stage-bound links within the ordinary acceptance review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyControlReview {
    /// Exact engineering policy version.
    pub version: String,
    /// Resolved definition identity; not an authorization or execution record.
    pub definition_sha256: String,
    /// Boundary this review actually covers.
    pub stage: crate::TestStage,
    /// Control IDs to existing condition or durable criterion IDs.
    pub bindings: std::collections::BTreeMap<String, Vec<String>>,
}

/// Links additive controls to existing verification, never an independent ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationControlReview {
    /// Exact resolved definitions and selected parameter values.
    pub resolution_sha256: String,
    /// Actual boundary covered by the review.
    pub stage: crate::TestStage,
    /// Unique control links; every selected applicable control needs an entry.
    pub bindings: Vec<OrganizationControlBinding>,
}

/// Existing accepted conditions/criteria, optionally reinforced by an extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationControlBinding {
    /// Exact organization control identifier.
    pub control: String,
    /// Reviewed existing condition or durable criterion identifiers.
    pub conditions: Vec<String>,
    /// Existing project extension checks; not commands or new execution routes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
}

impl OrganizationControlReview {
    fn validate(&self) -> Result<(), EvidenceError> {
        let controls = self;
        let mut ids = std::collections::HashSet::new();
        let unique = |values: &[String]| {
            values.len() <= 256
                && values
                    .iter()
                    .all(|v| !v.trim().is_empty() && v.len() <= 256)
                && values
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    == values.len()
        };
        if !digest_valid(&controls.resolution_sha256)
            || controls.bindings.len() > 256
            || controls.bindings.iter().any(|b| {
                !b.control.starts_with("ORG-")
                    || b.control.len() > 80
                    || !ids.insert(&b.control)
                    || !unique(&b.conditions)
                    || !unique(&b.extensions)
            })
        {
            return Err(EvidenceError::Semantic("Organization control review needs the exact resolution identity and bounded unique links; definitions determine applicability, never an empty link list.".into()));
        }
        Ok(())
    }
}

impl Default for AcceptanceEvidence {
    fn default() -> Self {
        Self {
            organization_controls: None,
            policy_controls: None,
            safeguards: None,
            requirements: None,
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
        let mut payload = serde_json::json!({
            "protocol": 1, "fingerprint": fingerprint, "work": work,
            "scope": self.scope, "rationale": self.rationale,
            "conditions": self.conditions, "verifications": self.verifications,
        });
        // Omission keeps every historical digest stable. Present mappings are material.
        if let Some(controls) = &self.organization_controls {
            payload["organization_controls"] = serde_json::to_value(controls)?;
        }
        if let Some(controls) = &self.policy_controls {
            payload["policy_controls"] = serde_json::to_value(controls)?;
        }
        if let Some(safeguards) = &self.safeguards {
            payload["safeguards"] = serde_json::to_value(safeguards)?;
        }
        if let Some(requirements) = &self.requirements {
            payload["requirements"] = serde_json::to_value(requirements)?;
        }
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&payload)?)
        ))
    }

    pub(crate) fn validate(&self) -> Result<(), EvidenceError> {
        if let Some(controls) = &self.organization_controls {
            controls.validate()?;
        }
        if let Some(controls) = &self.policy_controls {
            let definition = opdev_core::resolve_engineering_policy(&controls.version)
                .map_err(|e| EvidenceError::Semantic(e.to_string()))?;
            if controls.version != "2"
                || controls.definition_sha256 != definition.definition_sha256
                || controls.bindings.iter().any(|(id, conditions)| {
                    !matches!(
                        id.as_str(),
                        "OPDEV-FLOW-001"
                            | "OPDEV-DELIVERY-001"
                            | "OPDEV-PIPELINE-001"
                            | "OPDEV-RECOVERY-001"
                    ) || conditions.is_empty()
                        || conditions.len() > 256
                        || conditions.iter().any(|c| c.trim().is_empty())
                        || conditions
                            .iter()
                            .collect::<std::collections::HashSet<_>>()
                            .len()
                            != conditions.len()
                })
            {
                return Err(EvidenceError::Semantic("Policy control links need the exact supported definition, known controls and nonempty unique condition IDs.".into()));
            }
        }
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
            let stages = verification
                .stages
                .as_deref()
                .unwrap_or(&crate::TestStage::ALL);
            if !ids.contains(&verification.condition)
                || stages.is_empty()
                || stages
                    .iter()
                    .any(|stage| !mapped.insert((&verification.condition, *stage)))
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

#[cfg(test)]
mod organization_tests {
    use super::*;

    #[test]
    fn organization_links_are_material_and_legacy_digest_is_unchanged() -> Result<(), EvidenceError>
    {
        let mut acceptance = AcceptanceEvidence::default();
        let legacy = acceptance.digest("source", "work")?;
        assert!(
            serde_json::to_value(&acceptance)?
                .get("organization_controls")
                .is_none()
        );
        let review = OrganizationControlReview {
            resolution_sha256: "a".repeat(64),
            stage: crate::TestStage::Local,
            bindings: vec![OrganizationControlBinding {
                control: "ORG-EXAMPLE-001".into(),
                conditions: vec!["C1".into()],
                extensions: vec![],
            }],
        };
        review.validate()?;
        acceptance.organization_controls = Some(review.clone());
        let bound = acceptance.digest("source", "work")?;
        assert_ne!(legacy, bound);
        for change in 0..4 {
            let mut changed = review.clone();
            match change {
                0 => changed.resolution_sha256 = "b".repeat(64),
                1 => changed.stage = crate::TestStage::PostMerge,
                2 => changed.bindings[0].conditions = vec!["C2".into()],
                _ => changed.bindings[0].extensions = vec!["security".into()],
            }
            acceptance.organization_controls = Some(changed);
            assert_ne!(bound, acceptance.digest("source", "work")?);
        }
        acceptance.organization_controls = None;
        assert_eq!(legacy, acceptance.digest("source", "work")?);
        let mut duplicate = review.clone();
        duplicate.bindings.push(duplicate.bindings[0].clone());
        assert!(duplicate.validate().is_err());
        duplicate = review;
        duplicate.bindings[0].conditions.push("C1".into());
        assert!(duplicate.validate().is_err());
        Ok(())
    }
}
