//! Versioned adoption decisions. Labels never substitute for execution/evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{ProjectManifest, TestStage};

/// Optional for legacy projects; created during new initialization.
pub const ADOPTION_PATH: &str = ".opdev/adoption.yaml";
const CATALOG: &str = include_str!("../../../rules/adoption.json");
const SCHEMA: &str = include_str!("../../../schema/adoption.schema.json");

/// Fixed practice inventory; tool choices are project-specific.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdoptionCatalog {
    /// Version selected by a decision record.
    pub version: u32,
    /// Every supplied practice must receive a decision.
    pub practices: Vec<Practice>,
}

/// Assessment requirement, independent of rule-result outcomes.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    /// Cannot be ignored or marked inapplicable.
    Required,
    /// May be explicitly declined, without waiving any core rule.
    Optional,
    /// May be inapplicable with reviewed evidence, but not ignored.
    Conditional,
}

/// One supplied practice.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Practice {
    /// Stable identifier.
    pub id: String,
    /// User-facing assessment title.
    pub title: String,
    /// Allowed dispositions.
    pub requirement: Requirement,
    /// Implementations require named executable pre/post-merge suites.
    pub automated: bool,
}

/// A reviewed decision is not a gate verdict.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdoptionState {
    /// Implementation is unfinished; choices may already have plan-bound approval.
    Pending,
    /// An approved implementation is underway, not verified.
    InProgress,
    /// Implementation is claimed, subject to verification.
    Implemented,
    /// An optional practice was deliberately declined.
    Ignored,
    /// Reviewed applicability is false.
    NotApplicable,
}

/// One project-owned assessment decision.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdoptionDecision {
    /// Claimed disposition.
    pub state: AdoptionState,
    /// Accountable reviewer/owner.
    pub owner: String,
    /// Project-specific rationale; not a copy of a tool recommendation.
    pub reason: String,
    /// Canonical implementation, review or applicability references.
    pub references: Vec<String>,
    /// Existing project suite identifiers, not duplicate command definitions.
    pub suites: Vec<String>,
    /// Authoritative sources consulted for unresolved tool choices, if needed.
    pub research: Vec<String>,
}

/// Resumable, project-owned adoption record.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdoptionRecord {
    /// Record schema version.
    pub schema: u32,
    /// Pinned practice-catalog version.
    pub catalog_version: u32,
    /// Reviewed project/components covered by the assessment.
    pub scope: String,
    /// Complete set of catalog decisions.
    pub practices: BTreeMap<String, AdoptionDecision>,
    /// Review of choices, independent of implementation and acceptance evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<AdoptionReview>,
    /// Reviewed branch roles; branch names alone do not establish trunk workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<AdoptionWorkflow>,
}

/// A reviewable claim of approval, not authentication of a human identity.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdoptionReview {
    /// Hash of the exact choices and project contract presented for approval.
    pub plan_id: String,
    /// Actual decision maker, not an invented owner label.
    pub reviewer: String,
    /// User response or independent review authorizing these choices.
    pub reference: String,
    /// Explicit bounded delegation, when choices were delegated rather than selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegation: Option<String>,
}

/// Developer's branch-name choice, separate from workflow compliance.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MainOption {
    /// The trunk was already named main.
    AlreadyMain,
    /// The developer chose to retain a non-main name.
    KeepName,
    /// The developer chose a completed rename to main.
    RenameToMain,
}

/// Audited branch roles, backed by project instructions and CI references.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdoptionWorkflow {
    /// All ongoing integration branches, not all branches in the repository.
    pub integration_branches: Vec<String>,
    /// Source branch for the consumer release path (tags point to this trunk).
    pub release_source: String,
    /// Retaining a non-main name is a first-class choice.
    pub main_option: MainOption,
    /// Reviewed branch policy, CI and protection locations.
    pub references: Vec<String>,
}

/// Errors loading or safely creating adoption state.
#[derive(Debug, Error)]
pub enum AdoptionError {
    /// File access failed.
    #[error("adoption file error: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid YAML.
    #[error("invalid adoption YAML: {0}")]
    Yaml(#[from] serde_saphyr::Error),
    /// Serialization failed.
    #[error("could not serialize adoption record: {0}")]
    Serialize(#[from] serde_saphyr::SerializeError),
    /// Invalid bundled JSON.
    #[error("adoption schema/catalog error: {0}")]
    Json(#[from] serde_json::Error),
    /// Unsupported or malformed record.
    #[error("invalid adoption record: {0}")]
    Invalid(String),
}

/// Loads the immutable practice catalog shipped with this CLI.
///
/// # Errors
/// Returns an error if the embedded catalog is malformed.
pub fn adoption_catalog() -> Result<AdoptionCatalog, AdoptionError> {
    Ok(serde_json::from_str(CATALOG)?)
}

impl AdoptionRecord {
    /// Creates unresolved decisions. Discovery is never implementation evidence.
    ///
    /// # Errors
    /// Returns an error for an invalid bundled catalog.
    pub fn pending() -> Result<Self, AdoptionError> {
        let catalog = adoption_catalog()?;
        Ok(Self {
            schema: 2,
            catalog_version: catalog.version,
            scope: String::new(),
            review: None,
            workflow: None,
            practices: catalog
                .practices
                .into_iter()
                .map(|practice| {
                    (
                        practice.id,
                        AdoptionDecision {
                            state: AdoptionState::Pending,
                            owner: String::new(),
                            reason: String::new(),
                            references: Vec::new(),
                            suites: Vec::new(),
                            research: Vec::new(),
                        },
                    )
                })
                .collect(),
        })
    }

    /// Loads existing state without treating malformed state as absence.
    ///
    /// # Errors
    /// Returns filesystem, schema or catalog-version errors.
    pub fn load(root: &Path) -> Result<Option<Self>, AdoptionError> {
        let path = root.join(ADOPTION_PATH);
        match fs::read_to_string(&path) {
            Ok(yaml) => Ok(Some(Self::from_yaml(&yaml)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // A dangling symlink is an occupied path, not an absent record.
                if path.symlink_metadata().is_ok() {
                    return Err(error.into());
                }
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Validates shape and exact catalog membership; incomplete decisions are valid drafts.
    ///
    /// # Errors
    /// Rejects unsupported versions, unknown/missing practices and malformed fields.
    pub fn from_yaml(yaml: &str) -> Result<Self, AdoptionError> {
        let value: serde_json::Value = serde_saphyr::from_str(yaml)?;
        let schema = serde_json::from_str(SCHEMA)?;
        let validator = jsonschema::validator_for(&schema)
            .map_err(|error| AdoptionError::Invalid(error.to_string()))?;
        let errors: Vec<_> = validator
            .iter_errors(&value)
            .map(|error| format!("{}: {error}", error.instance_path()))
            .collect();
        if !errors.is_empty() {
            return Err(AdoptionError::Invalid(errors.join("; ")));
        }
        let record: Self = serde_json::from_value(value)?;
        let catalog = adoption_catalog()?;
        if record.catalog_version != catalog.version {
            return Err(AdoptionError::Invalid(format!(
                "unsupported catalog {}; supports {} (explicit migration required)",
                record.catalog_version, catalog.version
            )));
        }
        let expected: BTreeSet<_> = catalog.practices.iter().map(|p| p.id.as_str()).collect();
        let actual: BTreeSet<_> = record.practices.keys().map(String::as_str).collect();
        if expected != actual {
            return Err(AdoptionError::Invalid(format!(
                "practice inventory mismatch; missing: {:?}; unknown: {:?}",
                expected.difference(&actual).collect::<Vec<_>>(),
                actual.difference(&expected).collect::<Vec<_>>()
            )));
        }
        Ok(record)
    }

    /// Serializes a validated draft without inferring decisions.
    ///
    /// # Errors
    /// Returns shape or serialization errors.
    pub fn to_yaml(&self) -> Result<String, AdoptionError> {
        let yaml = serde_saphyr::to_string(self)?;
        Self::from_yaml(&yaml)?;
        Ok(yaml)
    }

    /// Creates a record only if absent. Uses an atomic persist so interrupted
    /// creation cannot leave a truncated record masquerading as resumable state.
    ///
    /// # Errors
    /// Returns an error rather than overwriting project-owned decisions.
    pub fn write_new(&self, root: &Path) -> Result<(), AdoptionError> {
        let yaml = self.to_yaml()?;
        let directory = root.join(".opdev");
        if directory.is_symlink() {
            return Err(AdoptionError::Invalid(
                "refusing to create adoption state through a symlinked .opdev directory".into(),
            ));
        }
        fs::create_dir_all(&directory)?;
        let mut file = tempfile::NamedTempFile::new_in(&directory)?;
        file.write_all(yaml.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist_noclobber(root.join(ADOPTION_PATH))
            .map_err(|error| error.error)?;
        Ok(())
    }

    /// Finds unresolved or contradictory decisions. Empty means decisions are
    /// ready for verification, NOT that adoption is complete.
    ///
    /// # Errors
    /// Returns catalog or record-shape errors.
    pub fn blockers(&self, manifest: &ProjectManifest) -> Result<Vec<String>, AdoptionError> {
        self.to_yaml()?;
        let mut blockers = Vec::new();
        blockers.extend(self.approval_blockers(manifest)?);
        blockers.extend(self.workflow_blockers(manifest));
        if self.scope.trim().is_empty() {
            blockers.push("scope: assess all relevant components".into());
        }
        for practice in adoption_catalog()?.practices {
            let decision = &self.practices[&practice.id];
            let mut reasons = Vec::new();
            match decision.state {
                AdoptionState::Pending => {
                    reasons.push("implementation pending; approval is separate".into());
                }
                AdoptionState::InProgress => reasons.push("implementation in progress".into()),
                AdoptionState::Ignored if practice.requirement != Requirement::Optional => {
                    reasons.push("mandatory/conditional practice cannot be ignored".into());
                }
                AdoptionState::NotApplicable if practice.requirement == Requirement::Required => {
                    reasons.push("required practice cannot be marked not applicable".into());
                }
                _ => {}
            }
            if decision.state != AdoptionState::Pending {
                if decision.owner.trim().is_empty()
                    || decision.reason.trim().is_empty()
                    || decision.references.is_empty()
                {
                    reasons.push(
                        "resolved decisions require owner, reason and canonical references".into(),
                    );
                }
                if decision.state == AdoptionState::Implemented
                    && practice.automated
                    && decision.suites.is_empty()
                {
                    reasons.push("implementation requires executable suite references".into());
                }
            }
            for id in &decision.suites {
                match manifest.testing.suites.iter().find(|suite| &suite.id == id) {
                    None => reasons.push(format!("unknown suite `{id}`")),
                    Some(suite)
                        if !suite.stages.contains(&TestStage::PreMerge)
                            || !suite.stages.contains(&TestStage::PostMerge) =>
                    {
                        reasons.push(format!(
                            "suite `{id}` requires pre_merge and post_merge stages"
                        ));
                    }
                    Some(_) => {}
                }
            }
            blockers.extend(
                reasons
                    .into_iter()
                    .map(|reason| format!("{}: {reason}", practice.id)),
            );
        }
        Ok(blockers)
    }

    /// Hashes choices and the full contract, excluding approval and implementation progress.
    /// Pending/in-progress/implemented all propose implementation; exclusions are distinct choices.
    ///
    /// # Errors
    /// Returns serialization errors.
    pub fn plan_id(&self, manifest: &ProjectManifest) -> Result<String, AdoptionError> {
        let mut proposal = self.clone();
        proposal.review = None;
        for decision in proposal.practices.values_mut() {
            if matches!(
                decision.state,
                AdoptionState::Pending | AdoptionState::InProgress
            ) {
                decision.state = AdoptionState::Implemented;
            }
        }
        let bytes = serde_json::to_vec(&(proposal, manifest))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    /// Distinguishes missing approval from stale approval without treating implementation as consent.
    ///
    /// # Errors
    /// Returns serialization errors.
    pub fn approval_blockers(
        &self,
        manifest: &ProjectManifest,
    ) -> Result<Vec<String>, AdoptionError> {
        if self.schema == 1 {
            return Ok(vec!["migration_required: adoption schema 1 has no approval provenance; preview adoption migrate".into()]);
        }
        let Some(review) = &self.review else {
            return Ok(vec![
                "approval: proposed choices need explicit developer review or bounded delegation"
                    .into(),
            ]);
        };
        if review.plan_id != self.plan_id(manifest)? {
            return Ok(vec!["approval: stale plan; choices or project contract changed, present the revised plan for review".into()]);
        }
        Ok(Vec::new())
    }

    /// Checks roles, never requiring a particular trunk spelling.
    #[must_use]
    pub fn workflow_blockers(&self, manifest: &ProjectManifest) -> Vec<String> {
        let Some(workflow) = &self.workflow else {
            return vec!["workflow: review integration and release branch roles, CI and protections; offer main or keeping the current name".into()];
        };
        let mut blockers = Vec::new();
        if workflow.integration_branches != [manifest.project.trunk.clone()]
            || workflow.release_source != manifest.project.trunk
        {
            blockers.push("workflow: migration_required; use one integration trunk and release from it, not a develop-to-main promotion branch".into());
        }
        if workflow.references.is_empty() {
            blockers
                .push("workflow: branch roles require reviewed policy and CI references".into());
        }
        if (manifest.project.trunk == "main") == (workflow.main_option == MainOption::KeepName) {
            blockers.push("workflow: main_option contradicts the declared trunk name".into());
        }
        blockers
    }
}
