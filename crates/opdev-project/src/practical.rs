//! Small, explicit development contracts. No legacy-policy inference or execution.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::{CommandSpec, TestStage};

/// Assurance is a policy selection, not a model-effort setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// Low-consequence work with appropriate observable checks.
    Lightweight,
    /// Maintained software with agentic review and trusted integration checks.
    Standard,
    /// Additional explicitly source-bound assurance.
    Assured,
}

/// Versioned contract for the practical workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    /// Exactly 5. Older contracts are never interpreted using these defaults.
    pub schema: u32,
    /// Exact preset-definition version.
    pub policy_version: u32,
    /// Explicit developer-selected assurance level.
    pub preset: Preset,
    /// Existing integration branch; it need not be named main.
    pub trunk: String,
    /// Actual decision reference; attribution is not authenticated consent.
    pub decision_reference: String,
    /// Existing canonical tools, with no ecosystem-specific substitutions.
    pub commands: BTreeMap<String, CommandSpec>,
    /// Required executions at their actual boundaries.
    pub checks: Vec<Check>,
    /// Existing owners, e.g. documentation, work, architecture, security.
    #[serde(default)]
    pub authorities: BTreeMap<String, String>,
    /// Explicit catalog files, never a directory-wide context dump.
    #[serde(default)]
    pub requirements: Vec<String>,
    /// Explicit additive obligation definitions; not implied certification.
    #[serde(default)]
    pub policies: Vec<String>,
}

/// Project-reviewed mapping of an additional obligation to current requirements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Exact definition format.
    pub schema: u32,
    /// Stable local identifier.
    pub id: String,
    /// Original authority defining the obligation.
    pub source: String,
    /// Actual review of this mapping's scope and limits.
    pub decision_reference: String,
    /// Preserved original selection on migration; not a conformance verdict.
    pub origin: serde_json::Value,
    /// Required current guarantees.
    pub requirements: Vec<String>,
    /// Required verification boundaries.
    pub stages: Vec<TestStage>,
}

/// One canonical check, independently of its requirement mappings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    /// Project-local stable identifier.
    pub id: String,
    /// Canonical command ID.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub command: String,
    /// Repository-relative manual procedure, never an execution receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manual: Option<String>,
    /// Actual execution boundaries, never inferred from filenames.
    pub stages: Vec<TestStage>,
}

/// Current requirement definitions, not execution history or an approval ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    /// Exactly 2: stage-independent mappings.
    pub schema: u32,
    /// Supported observable promises in this document's scope.
    pub requirements: Vec<Requirement>,
}

/// An enduring promise and its current verification design.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    /// Stable across edits and file moves.
    pub id: String,
    /// Observable behavior, not a description of the implementation.
    pub statement: String,
    /// Verification links; no duplicate stage-specific plan is necessary.
    pub verification: Vec<Verification>,
    /// Optional stronger mapping review. Required by Assured, never execution evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<MappingReview>,
}

/// Attributed review of exact mapping inputs; not authenticated human consent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingReview {
    /// Actual reviewer, which may be the implementing agent.
    pub reviewer: String,
    /// Existing review/decision location.
    pub reference: String,
    /// Concrete adequacy findings and limits.
    pub rationale: String,
    /// Current subject emitted by inspection, not manually reconstructed.
    pub subject_sha256: String,
    /// Review finding, independently of current test execution.
    pub outcome: ReviewOutcome,
}

/// A review cannot manufacture test execution or resolve a known contradiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewOutcome {
    /// Reviewed mapping is adequate within the stated limits.
    Passed,
    /// A known inadequacy remains unresolved.
    Failed,
    /// Review has not established adequacy.
    Unverified,
}

/// A suite link is not proof that an individual test ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    /// Canonical check ID.
    pub check: String,
    /// Repository-relative file containing the assertions.
    pub path: String,
    /// Human-readable assertion/test locator, not a runner receipt.
    pub assertion: String,
    /// Known helpers/fixtures; not a claim of a complete dependency graph.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
}

impl Contract {
    /// Inspect the selected namespace without deleting or reclassifying files.
    ///
    /// # Errors
    /// Unknown, linked, nested or unselected files require explicit migration.
    pub fn inspect_namespace(&self, root: &Path, require_entrypoints: bool) -> Result<()> {
        let directory = root.join(".opdev");
        if !directory.try_exists()? {
            ensure!(!require_entrypoints, "Missing .opdev directory");
            return Ok(());
        }
        ensure!(
            !linked(&fs::symlink_metadata(&directory)?),
            "Linked .opdev directory is not supported"
        );
        let mut count = 0;
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            count += 1;
            ensure!(
                count <= 16,
                "Unexpected .opdev contents; inspect and explicitly migrate them"
            );
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("Non-UTF-8 .opdev path"))?;
            if matches!(name.as_str(), "requirements" | "policies") {
                let selected = if name == "requirements" {
                    &self.requirements
                } else {
                    &self.policies
                };
                ensure!(
                    !selected.is_empty(),
                    "Unselected definition directory; review before removal or selection"
                );
                let meta = fs::symlink_metadata(entry.path())?;
                ensure!(
                    !linked(&meta) && meta.is_dir(),
                    "Requirements must be a regular directory"
                );
                for (index, item) in fs::read_dir(entry.path())?.enumerate() {
                    ensure!(index < 1024, "Requirements directory exceeds 1024 entries");
                    let item = item?;
                    let name = item
                        .file_name()
                        .into_string()
                        .map_err(|_| anyhow::anyhow!("Non-UTF-8 requirement path"))?;
                    let path = format!(".opdev/{}/{name}", entry.file_name().to_string_lossy());
                    ensure!(
                        selected.contains(&path),
                        "Unselected file {path}; classify and explicitly migrate it"
                    );
                    read_regular(root, &path)?;
                }
            } else {
                ensure!(
                    matches!(name.as_str(), "project.yaml" | "guidance.md"),
                    "File .opdev/{name} is outside the selected namespace; classify and explicitly migrate it. Nothing removed."
                );
                read_regular(root, &format!(".opdev/{name}"))?;
            }
        }
        if require_entrypoints {
            for path in [".opdev/project.yaml", ".opdev/guidance.md"] {
                ensure!(
                    !read_regular(root, path)?.trim().is_empty(),
                    "Required entry point {path} is empty"
                );
            }
        }
        Ok(())
    }
    /// Parse without silently accepting unknown policy fields.
    ///
    /// # Errors
    /// Invalid YAML, unknown fields, versions or contradictory commands fail.
    pub fn from_yaml(text: &str) -> Result<Self> {
        let result: Self = serde_saphyr::from_str(text)?;
        result.validate()?;
        Ok(result)
    }

    /// Validate policy without executing or contacting a provider.
    ///
    /// # Errors
    /// Reports the actual unsupported or inconsistent field.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == 5 && self.policy_version == 1,
            "Unsupported practical contract; expected schema 5, policy_version 1"
        );
        ensure!(
            self.commands.len() <= 256
                && self.checks.len() <= 1024
                && self.requirements.len() <= 1024
                && self.policies.len() <= 32,
            "Project exceeds supported command/check/catalog bounds"
        );
        ensure!(
            !self.trunk.trim().is_empty(),
            "An integration branch is required"
        );
        ensure!(
            !self.decision_reference.trim().is_empty(),
            "Select the policy using an actual developer decision reference"
        );
        for (id, command) in &self.commands {
            ensure!(
                !id.trim().is_empty() && command.argv.first().is_some_and(|s| !s.trim().is_empty()),
                "Command {id:?} needs a name and executable"
            );
            ensure!(
                !command.argv.iter().any(|s| s.contains('\0')),
                "Command {id:?} contains a NUL argument"
            );
            ensure!(
                command
                    .timeout_seconds
                    .is_none_or(|t| (1..=86400).contains(&t)),
                "Command {id:?} timeout must be between 1 and 86400 seconds"
            );
            if let Some(directory) = &command.working_directory {
                relative_path(directory)?;
            }
        }
        let mut ids = BTreeSet::new();
        for check in &self.checks {
            ensure!(
                !check.id.trim().is_empty() && ids.insert(&check.id),
                "Check ID {:?} is empty or duplicated",
                check.id
            );
            ensure!(
                check.manual.is_some() == check.command.is_empty(),
                "Check {} needs exactly one command or manual procedure",
                check.id
            );
            if let Some(path) = &check.manual {
                relative_path(path)?;
            }
            ensure!(
                check.manual.is_some() || self.commands.contains_key(&check.command),
                "Check {} names unknown command {}",
                check.id,
                check.command
            );
            ensure!(
                !check.stages.is_empty(),
                "Check {} needs an execution boundary",
                check.id
            );
            let unique: std::collections::HashSet<_> = check.stages.iter().collect();
            ensure!(
                unique.len() == check.stages.len(),
                "Check {} repeats a stage",
                check.id
            );
        }
        let mut files = BTreeSet::new();
        for path in self.requirements.iter().chain(&self.policies) {
            relative_path(path)?;
            let prefix = if self.policies.contains(path) {
                ".opdev/policies/"
            } else {
                ".opdev/requirements/"
            };
            let name = path.strip_prefix(prefix).unwrap_or("");
            ensure!(
                Path::new(name).extension().is_some_and(|e| e == "json")
                    && !name.contains('/')
                    && files.insert(path),
                "Requirement file must be a unique .opdev/requirements/<name>.json path"
            );
        }
        ensure!(
            self.authorities
                .iter()
                .all(|(k, v)| !k.trim().is_empty() && !v.trim().is_empty()),
            "Authority names and locations must be nonempty"
        );
        Ok(())
    }

    /// Unique command IDs for this boundary, in declared first-use order.
    #[must_use]
    pub fn commands_at(&self, stage: TestStage) -> Vec<&str> {
        let mut seen = BTreeSet::new();
        self.checks
            .iter()
            .filter(|c| c.manual.is_none() && c.stages.contains(&stage))
            .filter_map(|c| {
                seen.insert(c.command.as_str())
                    .then_some(c.command.as_str())
            })
            .collect()
    }

    /// Load selected current requirements and validate references, not adequacy.
    ///
    /// # Errors
    /// Dangling links, duplicate IDs, unsafe paths and obsolete formats fail.
    pub fn inspect_requirements(&self, root: &Path) -> Result<Vec<Requirement>> {
        for check in &self.checks {
            if let Some(path) = &check.manual {
                ensure!(
                    !read_regular(root, path)?.trim().is_empty(),
                    "Manual procedure {path} is empty"
                );
            }
        }
        let mut result = Vec::new();
        let mut ids = BTreeSet::new();
        let mut bytes = 0;
        let mut sources = BTreeSet::new();
        let mut source_bytes = 0;
        for file in &self.requirements {
            let content = read_regular(root, file)?;
            bytes += content.len();
            ensure!(
                bytes <= 32 * 1024 * 1024,
                "Requirements exceed the 32 MiB aggregate limit"
            );
            let document: Requirements = serde_json::from_str(&content)?;
            ensure!(
                document.schema == 2 && !document.requirements.is_empty(),
                "{file}: expected nonempty requirement document schema 2; migrate old mappings explicitly"
            );
            for requirement in document.requirements {
                ensure!(
                    !requirement.id.trim().is_empty() && ids.insert(requirement.id.clone()),
                    "Empty or duplicate requirement ID {}",
                    requirement.id
                );
                ensure!(
                    ids.len() <= 4096,
                    "Requirement inventory exceeds 4096 records"
                );
                ensure!(
                    requirement.verification.len() <= 256,
                    "Too many links for one requirement"
                );
                ensure!(
                    !requirement.statement.trim().is_empty()
                        && !requirement.verification.is_empty(),
                    "Requirement {} needs observable behavior and verification links",
                    requirement.id
                );
                for link in &requirement.verification {
                    ensure!(
                        self.checks.iter().any(|c| c.id == link.check),
                        "Requirement {} names missing check {}",
                        requirement.id,
                        link.check
                    );
                    ensure!(
                        !link.assertion.trim().is_empty(),
                        "Requirement {} needs an assertion locator",
                        requirement.id
                    );
                    ensure!(link.inputs.len() <= 256, "Too many verification inputs");
                    for input in std::iter::once(&link.path).chain(&link.inputs) {
                        if sources.insert(input.clone()) {
                            ensure!(
                                sources.len() <= 4096,
                                "Verification input inventory exceeds 4096 files"
                            );
                            source_bytes += read_regular(root, input)?.len();
                            ensure!(
                                source_bytes <= 64 * 1024 * 1024,
                                "Verification inputs exceed 64 MiB"
                            );
                        }
                    }
                }
                result.push(requirement);
            }
        }
        Ok(result)
    }

    /// Inspect additive definitions and their current requirement links.
    /// # Errors
    /// Missing definitions, duplicates, incomplete mappings and unknown versions fail.
    pub fn inspect_policies(
        &self,
        root: &Path,
        requirements: &[Requirement],
    ) -> Result<Vec<Policy>> {
        let mut result = Vec::new();
        let mut ids = BTreeSet::new();
        let mut bytes = 0usize;
        for path in &self.policies {
            let text = read_regular(root, path)?;
            bytes += text.len();
            ensure!(bytes <= 32 * 1024 * 1024, "Selected policies exceed 32 MiB");
            let policy: Policy = serde_json::from_str(&text)?;
            ensure!(
                policy.schema == 1
                    && !policy.id.trim().is_empty()
                    && ids.insert(policy.id.clone())
                    && !policy.source.trim().is_empty()
                    && !policy.decision_reference.trim().is_empty()
                    && !policy.origin.is_null()
                    && !policy.requirements.is_empty()
                    && !policy.stages.is_empty(),
                "Policy {path} needs an exact reviewed definition, original authority and verification scope"
            );
            let mut stages = BTreeSet::new();
            ensure!(
                policy
                    .stages
                    .iter()
                    .all(|s| stages.insert(format!("{s:?}"))),
                "Duplicate policy stage"
            );
            let mut links = BTreeSet::new();
            for id in &policy.requirements {
                ensure!(
                    links.insert(id) && requirements.iter().any(|r| &r.id == id),
                    "Policy {} names missing or repeated requirement {id}",
                    policy.id
                );
            }
            result.push(policy);
        }
        Ok(result)
    }

    /// Mechanically bind an Assured review to its actual current inputs.
    ///
    /// # Errors
    /// Invalid references or unreadable files prevent computing a subject.
    pub fn mapping_subject(&self, root: &Path, requirement: &Requirement) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut inputs = BTreeMap::new();
        let mut checks = BTreeMap::new();
        for link in &requirement.verification {
            let check = self
                .checks
                .iter()
                .find(|c| c.id == link.check)
                .context("Mapping names an unknown check")?;
            checks.insert(&check.id, (check, self.commands.get(&check.command)));
            if let Some(path) = &check.manual {
                inputs.insert(path, read_regular(root, path)?);
            }
            for path in std::iter::once(&link.path).chain(&link.inputs) {
                inputs.insert(path, read_regular(root, path)?);
            }
        }
        let policies: BTreeMap<_, _> = self
            .policies
            .iter()
            .map(|path| Ok((path, read_regular(root, path)?)))
            .collect::<Result<_>>()?;
        let subject = serde_json::json!({"policy_version":self.policy_version, "preset":self.preset, "policies": policies,
            "id":requirement.id, "statement":requirement.statement,
            "verification":requirement.verification, "checks":checks, "inputs":inputs});
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json_canonicalizer::to_vec(&subject)?)
        ))
    }

    /// Review-input findings, never a claim of execution or inventory completeness.
    ///
    /// # Errors
    /// Invalid input cannot become a review finding with a guessed subject.
    pub fn review_gaps(
        &self,
        root: &Path,
        requirements: &[Requirement],
        stage: TestStage,
    ) -> Result<Vec<String>> {
        let policies = self.inspect_policies(root, requirements)?;
        let mut gaps = Vec::new();
        if self.preset == Preset::Assured && requirements.is_empty() {
            gaps.push(
                "Assured needs a declared requirement scope and reviewed verification links".into(),
            );
        }
        for requirement in requirements {
            if requirement
                .review
                .as_ref()
                .is_some_and(|r| r.outcome == ReviewOutcome::Failed)
            {
                gaps.push(format!(
                    "Requirement {} has an unresolved review failure",
                    requirement.id
                ));
            }
            if self.preset != Preset::Assured
                && !policies
                    .iter()
                    .any(|p| p.stages.contains(&stage) && p.requirements.contains(&requirement.id))
            {
                continue;
            }
            let subject = self.mapping_subject(root, requirement)?;
            let reviewed = requirement.review.as_ref().is_some_and(|r| {
                r.outcome == ReviewOutcome::Passed
                    && r.subject_sha256 == subject
                    && !r.reviewer.trim().is_empty()
                    && !r.reference.trim().is_empty()
                    && !r.rationale.trim().is_empty()
            });
            if !reviewed {
                gaps.push(format!("Requirement {} needs review of current mapping inputs; expected subject {subject}", requirement.id));
            }
            if !requirement.verification.iter().any(|v| {
                self.checks
                    .iter()
                    .any(|c| c.id == v.check && c.stages.contains(&stage))
            }) {
                gaps.push(format!(
                    "Requirement {} has no verification at {stage:?}",
                    requirement.id
                ));
            }
        }
        Ok(gaps)
    }
}

/// Validate a portable repository-relative path without traversing it.
///
/// # Errors
/// Absolute, parent, Windows drive and alternate-separator paths are refused.
pub fn relative_path(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && !value.contains(['\\', ':', '\0'])
            && !Path::new(value).is_absolute()
            && Path::new(value)
                .components()
                .all(|c| matches!(c, Component::Normal(_) | Component::CurDir)),
        "Expected a repository-relative path without traversal: {value:?}"
    );
    Ok(())
}

/// Read a bounded regular file without following a symlink in any component.
///
/// # Errors
/// Missing, linked, nonregular or oversized files are not partial success.
pub fn read_regular(root: &Path, relative: &str) -> Result<String> {
    use std::io::Read;
    relative_path(relative)?;
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        path.push(component);
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("Cannot inspect {}", path.display()))?;
        ensure!(
            !linked(&metadata),
            "Linked path is not a regular project input: {relative}"
        );
    }
    ensure!(
        fs::metadata(&path)?.is_file(),
        "Expected a regular file: {relative}"
    );
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "Project input exceeds 8 MiB: {relative}"
    );
    Ok(String::from_utf8(bytes)?)
}

fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}
