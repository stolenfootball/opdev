//! Versioned, bounded requirements catalog. No database or stored execution verdicts.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{Context, Result, ensure};
use opdev_core::Outcome;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ProjectManifest, TestStage, TrackedEvidence};

static SCHEMA: std::sync::LazyLock<std::result::Result<jsonschema::Validator, String>> =
    std::sync::LazyLock::new(|| {
        let value = serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../schema/requirements.schema.json"
        ))
        .map_err(|e| e.to_string())?;
        jsonschema::validator_for(&value).map_err(|e| e.to_string())
    });

mod source;
pub use source::portable_catalog_path;
pub use source::{CatalogSnapshot, load_index, load_revision};

/// Canonical catalog directory, enabled only by explicit policy selection.
pub const DIRECTORY: &str = ".opdev/requirements";

/// Change-local catalog impact review in the existing MR/PR, not in product source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeReview {
    /// Exact candidate catalog identity including the current mapping reviews.
    pub catalog_sha256: String,
    /// Full accepted baseline commit selected for this comparison.
    pub baseline_commit: String,
    /// Independently inspected baseline catalog identity.
    pub baseline_catalog_sha256: String,
    /// Completeness, changed guarantees, scope reductions, one-off conditions and limits.
    pub rationale: String,
    /// Original authorization when changing promises; not authenticated by a string.
    pub decision_reference: String,
    /// Current attributed results, retained in the MR/PR instead of product source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manual_observations: Vec<ManualObservation>,
}

/// Manual result bound to a current plan and enclosing source-bound change review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualObservation {
    /// Exact plan (including configuration and stage).
    pub plan: String,
    /// Exact plan member.
    pub verification: String,
    /// Procedure and criterion subject at observation time.
    pub plan_subject_sha256: String,
    /// Unix seconds. Future timestamps cannot qualify.
    pub observed_at: u64,
    /// Passed, failed or unverified observation, not mapping adequacy.
    pub outcome: Outcome,
    /// Actual human or agent attribution.
    pub observer: String,
    /// What was seen, not merely expected behavior.
    pub observed_result: String,
    /// Procedure, environment, scope and limitations.
    pub context_and_limits: String,
    /// Existing observation reference; not a new evidence store.
    pub reference: String,
}

impl ChangeReview {
    /// Check exact selected baseline/candidate identities, not authorization claims.
    /// # Errors
    /// Missing or changed snapshots and incomplete review metadata fail closed.
    pub fn verify(&self, root: &Path, candidate: &CatalogSnapshot) -> Result<()> {
        ensure!(
            matches!(self.baseline_commit.len(), 40 | 64)
                && self
                    .baseline_commit
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "Catalog change review needs a full accepted baseline commit"
        );
        ensure!(
            nonempty(&self.rationale) && nonempty(&self.decision_reference),
            "Catalog impact review needs rationale and original decision reference; a digest is not consent"
        );
        ensure!(
            candidate.digest()? == self.catalog_sha256,
            "Catalog changed since the selected change review; inspect current guarantees, mappings and scope"
        );
        let baseline = load_revision(root, &self.baseline_commit)?;
        ensure!(
            baseline.digest()? == self.baseline_catalog_sha256,
            "Accepted baseline catalog does not match the selected review"
        );
        let mut observed_members = BTreeSet::new();
        for observed in &self.manual_observations {
            ensure!(
                observed_members.insert((&observed.plan, &observed.verification)),
                "Multiple manual observations for one plan member are ambiguous; retain failed history in the work record, not as competing current results"
            );
            let plan = candidate
                .plans()
                .find(|p| p.id == observed.plan)
                .context("Manual observation names an unknown plan")?;
            ensure!(
                plan.members
                    .iter()
                    .any(|m| m.verification == observed.verification)
                    && candidate
                        .verifications()
                        .any(|v| v.id == observed.verification
                            && matches!(v.method, Method::Manual { .. })),
                "Manual observation must name an actual manual member of its selected plan"
            );
        }
        Ok(())
    }
}

/// Reviewed catalog selection. Configurations name existing supported modes, not scripts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementsPolicy {
    /// Supported policy version.
    pub version: u32,
    /// Actual decision reference; a string is not authenticated consent.
    pub review_reference: String,
    /// Nonempty named configurations and required verification boundaries.
    pub configurations: BTreeMap<String, Vec<TestStage>>,
}

/// One capability-sized source document; IDs are global across all documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogDocument {
    /// Wire format version, independent of project schema.
    pub schema: u32,
    /// Current supported guarantees, not a milestone backlog.
    pub requirements: Vec<Requirement>,
    /// Reusable verification methods linked by plans.
    pub verifications: Vec<Verification>,
    /// Exactly one plan for each applicable criterion/configuration/stage.
    pub plans: Vec<VerificationPlan>,
}

/// One durable product promise. Moving its file does not change its identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    /// Stable, project-unique identity.
    pub id: String,
    /// Human-readable label; not a second normative statement.
    pub title: String,
    /// Sole normative owner: inline text or exact specification fragment.
    pub statement: Statement,
    /// Originating product/specification authority.
    pub origin: String,
    /// Why this guarantee is needed.
    pub rationale: String,
    /// Explicit supported configurations where this guarantee is required.
    pub configurations: Vec<String>,
    /// Observable criteria. Empty means incomplete, never automatically passed.
    pub criteria: Vec<Criterion>,
}

/// A promise has one normative owner, never both editable copies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Statement {
    /// Catalog-owned guarantee.
    Inline {
        /// Normative behavior.
        text: String,
    },
    /// Existing versioned specification owns the guarantee.
    Source {
        /// Exact tracked specification fragment.
        source: TrackedEvidence,
    },
}

/// An observable part of a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    /// Stable catalog-wide identity.
    pub id: String,
    /// Observable expected result, including important failure behavior.
    pub expected: String,
}

/// Reusable verification definition, not an execution observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    /// Stable catalog-wide identity.
    pub id: String,
    /// Actual assertion or repeatable observation procedure.
    pub target: TrackedEvidence,
    /// Known helpers, fixtures and selector inputs; not proof of a complete graph.
    pub inputs: Vec<TrackedEvidence>,
    /// Canonical suite or explicit manual method.
    pub method: Method,
}

/// Tool-neutral verification methods. Stronger assurance cannot silently downgrade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Method {
    /// Execute the existing canonical suite at this boundary.
    Automated {
        /// Project suite ID.
        suite: String,
        /// Honest observation level; case-level needs a supported producer.
        assurance: AssuranceLevel,
    },
    /// A current, source-bound observation, not a persistent passing claim.
    Manual {
        /// Inspection, analysis or demonstration.
        technique: ManualTechnique,
        /// Why an automated check is inadequate or unavailable.
        automation_limitation: String,
        /// Explicit recheck procedure and freshness requirements.
        recheck: String,
        /// Maximum observation age; exact change/stage binding also remains required.
        max_age_seconds: u32,
    },
}

/// What the selected command evidence actually establishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssuranceLevel {
    /// Command exit only; no individual-case execution claim.
    Suite,
    /// Requires observed individual-case execution, not exit zero alone.
    Case,
}

/// Nonautomated verification type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManualTechnique {
    /// Inspect a concrete result.
    Inspection,
    /// Analyze an explicit property.
    Analysis,
    /// Demonstrate consumer behavior.
    Demonstration,
}

/// All members jointly establish one criterion at one boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationPlan {
    /// Stable plan identity.
    pub id: String,
    /// Catalog-wide criterion ID.
    pub criterion: String,
    /// Exact configuration from reviewed project policy.
    pub configuration: String,
    /// Exact verification boundary.
    pub stage: TestStage,
    /// All listed members are required; any-green-link is not sufficient.
    pub members: Vec<PlanMember>,
    /// Current adequacy judgment, never execution or developer authorization.
    pub review: MappingReview,
}

/// A specific relationship between a criterion and a verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanMember {
    /// Referenced verification ID.
    pub verification: String,
    /// Why the actual assertion/observation proves this part of the criterion.
    pub assertion: String,
    /// Requirement-derived example distinguishing a plausible incorrect result.
    pub discriminating_case: String,
}

/// Replaceable review judgment; no append-only history or approval inference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingReview {
    /// Passed, failed or unverified mapping adequacy; never execution.
    pub outcome: Outcome,
    /// Actual human or agent attribution.
    pub reviewer: String,
    /// Existing review location.
    pub reference: String,
    /// Adequacy rationale and limitations.
    pub rationale: String,
    /// Exact canonical mapping subject; computing it does not approve anything.
    pub subject_sha256: String,
}

/// Actionable finding. Multiple contradictory/missing facts remain visible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Requirement, criterion, plan or document identity.
    pub subject: String,
    /// Stable diagnostic code.
    pub code: String,
    /// Semantic outcome, not a gate pass.
    pub outcome: Outcome,
    /// Explanation and next step.
    pub message: String,
}

/// Read-only report: an empty finding list establishes inputs, never execution.
#[derive(Debug, Serialize)]
pub struct Inspection {
    /// Inspection wire version.
    pub schema: u32,
    /// Always unverified: this reader runs no tests.
    pub qualification: Outcome,
    /// Snapshot identity including current review records.
    pub catalog_sha256: String,
    /// Expected subjects to review; never automatically written as approvals.
    pub review_subjects: BTreeMap<String, String>,
    /// All discovered gaps and contradictions.
    pub findings: Vec<Finding>,
    /// Current guarantee count; not a completeness claim.
    pub requirements: usize,
    /// Visible assurance boundary.
    pub limits: &'static str,
}

/// Canonical RFC 8785 digest using a maintained serializer, not map iteration order.
/// # Errors
/// Rejects values the canonical serializer cannot represent.
pub fn digest(value: &impl Serialize) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json_canonicalizer::to_vec(value)?)
    ))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}
fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

impl RequirementsPolicy {
    /// Validate explicit selection without selecting policy or running checks.
    /// # Errors
    /// Rejects unknown versions, empty/duplicate stages and nonportable identifiers.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1 && nonempty(&self.review_reference),
            "Requirements policy needs version 1 and an actual decision reference"
        );
        ensure!(
            !self.configurations.is_empty() && self.configurations.len() <= 64,
            "Requirements policy needs 1..64 supported configurations"
        );
        for (name, stages) in &self.configurations {
            ensure!(
                id(name) && !stages.is_empty(),
                "Configuration needs a portable ID and required stages"
            );
            ensure!(
                stages
                    .iter()
                    .enumerate()
                    .all(|(i, s)| !stages[..i].contains(s)),
                "Duplicate configuration stage"
            );
            ensure!(
                stages.contains(&TestStage::PreMerge) && stages.contains(&TestStage::PostMerge),
                "Supported configurations need pre_merge and post_merge verification; extra stages are additive"
            );
        }
        Ok(())
    }
}

impl CatalogDocument {
    /// Strict parsing: direct typed deserialization rejects duplicate and unknown fields.
    /// # Errors
    /// Invalid JSON, unsupported version and malformed records fail closed.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() <= 8 * 1024 * 1024,
            "Catalog document exceeds 8 MiB"
        );
        let doc: Self =
            serde_json::from_slice(bytes).context("Invalid requirements catalog JSON")?;
        ensure!(
            doc.schema == 1,
            "Unsupported requirements catalog schema {}; supported: 1",
            doc.schema
        );
        let validator = SCHEMA
            .as_ref()
            .map_err(|e| anyhow::anyhow!("Bundled catalog schema: {e}"))?;
        ensure!(
            validator.is_valid(&serde_json::to_value(&doc)?),
            "Requirements catalog violates schema/requirements.schema.json"
        );
        ensure!(
            doc.requirements.len() + doc.verifications.len() + doc.plans.len() <= 20000,
            "Catalog document exceeds 20000 records"
        );
        for r in &doc.requirements {
            ensure!(
                id(&r.id) && nonempty(&r.title) && nonempty(&r.origin) && nonempty(&r.rationale),
                "Requirement needs identity, label, origin and rationale"
            );
            ensure!(
                !r.configurations.is_empty() && unique(&r.configurations),
                "Requirement needs unique, explicit configurations"
            );
            if let Statement::Inline { text } = &r.statement {
                ensure!(nonempty(text), "Requirement statement is empty");
            }
            for c in &r.criteria {
                ensure!(
                    id(&c.id) && nonempty(&c.expected),
                    "Criterion needs identity and expected behavior"
                );
            }
        }
        for v in &doc.verifications {
            ensure!(id(&v.id), "Verification needs a stable identity");
            match &v.method {
                Method::Automated { suite, .. } => {
                    ensure!(id(suite), "Automated verification needs a suite");
                }
                Method::Manual {
                    automation_limitation,
                    recheck,
                    ..
                } => ensure!(
                    nonempty(automation_limitation) && nonempty(recheck),
                    "Manual method needs automation limits and a recheck procedure"
                ),
            }
        }
        for p in &doc.plans {
            ensure!(
                id(&p.id) && id(&p.criterion) && id(&p.configuration),
                "Plan needs valid identities"
            );
            let members: Vec<_> = p.members.iter().map(|m| m.verification.clone()).collect();
            ensure!(unique(&members), "Plan repeats a verification");
            for m in &p.members {
                ensure!(
                    id(&m.verification)
                        && nonempty(&m.assertion)
                        && nonempty(&m.discriminating_case),
                    "Plan member needs an assertion relationship and discriminating case"
                );
            }
            ensure!(
                matches!(
                    p.review.outcome,
                    Outcome::Passed | Outcome::Failed | Outcome::Unverified
                ),
                "Mapping review must be passed, failed or unverified"
            );
        }
        Ok(doc)
    }
}

fn unique(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

impl CatalogSnapshot {
    /// All records in stable file order; identity does not depend on that order.
    pub fn requirements(&self) -> impl Iterator<Item = &Requirement> {
        self.documents.values().flat_map(|d| &d.requirements)
    }
    /// Reusable verification definitions.
    pub fn verifications(&self) -> impl Iterator<Item = &Verification> {
        self.documents.values().flat_map(|d| &d.verifications)
    }
    /// Complete all-members plans.
    pub fn plans(&self) -> impl Iterator<Item = &VerificationPlan> {
        self.documents.values().flat_map(|d| &d.plans)
    }

    /// Digest of logical contents sorted by ID (moving a file does not change it).
    /// # Errors
    /// Canonical encoding errors are never converted into an empty digest.
    pub fn digest(&self) -> Result<String> {
        let mut reqs: Vec<_> = self.requirements().collect();
        reqs.sort_by_key(|r| &r.id);
        let mut checks: Vec<_> = self.verifications().collect();
        checks.sort_by_key(|v| &v.id);
        let mut plans: Vec<_> = self.plans().collect();
        plans.sort_by_key(|p| &p.id);
        digest(&(1, reqs, checks, plans))
    }

    /// Mapping adequacy subject excludes review and unrelated implementation bytes.
    /// Full referenced test/specification files and declared inputs remain bound.
    /// # Errors
    /// Unresolved/ambiguous records or missing command policy fail closed.
    pub fn plan_digest(
        &self,
        plan: &VerificationPlan,
        manifest: &ProjectManifest,
    ) -> Result<String> {
        let matches: Vec<_> = self
            .requirements()
            .flat_map(|r| r.criteria.iter().map(move |c| (r, c)))
            .filter(|(_, c)| c.id == plan.criterion)
            .collect();
        ensure!(matches.len() == 1, "Plan criterion is missing or ambiguous");
        let (requirement, criterion) = matches[0];
        let mut methods = BTreeMap::new();
        for member in &plan.members {
            let matches: Vec<_> = self
                .verifications()
                .filter(|v| v.id == member.verification)
                .collect();
            ensure!(
                matches.len() == 1,
                "Plan verification is missing or ambiguous"
            );
            let v = matches[0];
            let command = if let Method::Automated { suite, .. } = &v.method {
                let s = manifest
                    .testing
                    .suites
                    .iter()
                    .find(|s| &s.id == suite)
                    .context("Verification suite is not declared")?;
                ensure!(
                    s.stages.contains(&plan.stage),
                    "Verification suite is not assigned to plan stage"
                );
                Some((
                    s,
                    manifest
                        .commands
                        .get(&s.command)
                        .context("Verification command is not declared")?,
                ))
            } else {
                None
            };
            methods.insert(&member.verification, (member, v, command));
        }
        // Policy and verification configuration are conservative whole objects.
        // Sibling criteria do not invalidate this criterion's review.
        digest(&(
            1,
            &requirement.id,
            &requirement.statement,
            &requirement.origin,
            &requirement.rationale,
            &requirement.configurations,
            criterion,
            &plan.id,
            &plan.configuration,
            plan.stage,
            methods,
            &manifest.assurance,
            &manifest.testing,
            &manifest.quality,
        ))
    }

    /// Inspect every obligation and retain all findings. No commands or network.
    /// # Errors
    /// Unsupported/missing policy or canonical serialization fails closed.
    #[allow(clippy::too_many_lines)] // Collect independent findings without first-error masking.
    pub fn inspect(&self, root: &Path, manifest: &ProjectManifest) -> Result<Inspection> {
        let policy =
            manifest.assurance.requirements.as_ref().context(
                "Requirements policy is not selected; preview a reviewed migration first",
            )?;
        policy.validate()?;
        let mut sources = SourceCache::default();
        let mut result = Inspection {
            schema: 1,
            qualification: Outcome::Unverified,
            catalog_sha256: self.digest()?,
            review_subjects: BTreeMap::new(),
            findings: vec![],
            requirements: self.requirements().count(),
            limits: "Input inspection only; no tests ran. Hashes bind contents, not adequacy, completeness or developer consent. Suite exit does not prove individual-case execution. Known dependencies are not a complete dependency graph.",
        };
        let mut add = |subject: &str, code: &str, outcome: Outcome, message: String| {
            result.findings.push(Finding {
                subject: subject.into(),
                code: code.into(),
                outcome,
                message,
            });
        };
        let mut ids = BTreeSet::new();
        for identity in self
            .requirements()
            .map(|r| &r.id)
            .chain(
                self.requirements()
                    .flat_map(|r| r.criteria.iter().map(|c| &c.id)),
            )
            .chain(self.verifications().map(|v| &v.id))
            .chain(self.plans().map(|p| &p.id))
        {
            if !ids.insert(identity) {
                add(
                    identity,
                    "duplicate_identity",
                    Outcome::Error,
                    "Identity is ambiguous across the catalog; use unique stable IDs".into(),
                );
            }
        }
        if self.requirements().next().is_none() {
            add(
                "catalog",
                "empty_inventory",
                Outcome::Unverified,
                "No supported guarantees inventoried; empty inventory is not verified adoption"
                    .into(),
            );
        }
        for r in self.requirements() {
            if let Statement::Source { source } = &r.statement {
                reference(root, &r.id, source, &mut sources, &mut add);
            }
            if r.criteria.is_empty() {
                add(
                    &r.id,
                    "missing_criteria",
                    Outcome::Unverified,
                    "Requirement has no observable criteria; define what would demonstrate it"
                        .into(),
                );
            }
            for config in &r.configurations {
                if let Some(stages) = policy.configurations.get(config) {
                    for c in &r.criteria {
                        for stage in stages {
                            let count = self
                                .plans()
                                .filter(|p| {
                                    p.criterion == c.id
                                        && &p.configuration == config
                                        && p.stage == *stage
                                })
                                .count();
                            if count != 1 {
                                add(
                                    &c.id,
                                    "plan_cardinality",
                                    Outcome::Unverified,
                                    format!(
                                        "Needs exactly one all-members plan for {config}/{stage:?}; found {count}"
                                    ),
                                );
                            }
                        }
                    }
                } else {
                    add(
                        &r.id,
                        "unknown_configuration",
                        Outcome::Error,
                        format!("Configuration {config} is not selected in project policy"),
                    );
                }
            }
        }
        for v in self.verifications() {
            reference(root, &v.id, &v.target, &mut sources, &mut add);
            for input in &v.inputs {
                reference(root, &v.id, input, &mut sources, &mut add);
            }
        }
        for p in self.plans() {
            let owner = self
                .requirements()
                .find(|r| r.criteria.iter().any(|c| c.id == p.criterion));
            if !owner.is_some_and(|r| r.configurations.contains(&p.configuration))
                || !policy
                    .configurations
                    .get(&p.configuration)
                    .is_some_and(|s| s.contains(&p.stage))
            {
                add(
                    &p.id,
                    "unselected_boundary",
                    Outcome::Error,
                    "Plan does not match a required criterion/configuration/stage".into(),
                );
            }
            if p.members.is_empty() {
                add(
                    &p.id,
                    "missing_members",
                    Outcome::Unverified,
                    "Plan needs at least one meaningful verification; an empty plan proves nothing"
                        .into(),
                );
            }
            match self.plan_digest(p, manifest) {
                Ok(subject) => {
                    if subject != p.review.subject_sha256
                        || !nonempty(&p.review.reviewer)
                        || !nonempty(&p.review.reference)
                        || !nonempty(&p.review.rationale)
                        || p.review.outcome == Outcome::Unverified
                    {
                        add(&p.id, "review_not_current", Outcome::Unverified, "Inspect the current criterion and actual assertions, then record the adequacy review; refreshing a hash alone is not a review".into());
                    }
                    result.review_subjects.insert(p.id.clone(), subject);
                }
                Err(e) => add(&p.id, "unresolved_plan", Outcome::Unverified, e.to_string()),
            }
            if p.review.outcome == Outcome::Failed {
                add(&p.id, "review_contradiction", Outcome::Failed, "Mapping review records a contradiction; a stale digest or green suite cannot hide it".into());
            }
        }
        Ok(result)
    }
}

fn reference(
    root: &Path,
    id: &str,
    source: &TrackedEvidence,
    cache: &mut SourceCache,
    add: &mut impl FnMut(&str, &str, Outcome, String),
) {
    if let Err(error) = cache.verify(root, source) {
        add(
            id,
            "source_not_current",
            error
                .downcast_ref::<SourceFailure>()
                .map_or(Outcome::Unverified, |e| e.outcome),
            format!("Referenced source is missing, changed or invalid: {error}"),
        );
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct SourceFailure {
    outcome: Outcome,
    message: String,
}

#[derive(Default)]
struct SourceCache {
    files: BTreeMap<String, std::result::Result<(String, String), SourceFailure>>,
    bytes: usize,
}

impl SourceCache {
    fn verify(&mut self, root: &Path, source: &TrackedEvidence) -> Result<()> {
        source.validate()?;
        ensure!(
            self.bytes <= 64 * 1024 * 1024,
            "Catalog source inspection exceeds 64 MiB; no partial qualification"
        );
        if !self.files.contains_key(&source.path) {
            ensure!(
                self.files.len() < 4096,
                "Catalog source inspection exceeds 4096 files"
            );
            let bytes = source.staged_bytes(root);
            let value = match bytes {
                Ok(bytes) => {
                    self.bytes = self
                        .bytes
                        .checked_add(bytes.len())
                        .context("Source size overflow")?;
                    ensure!(
                        self.bytes <= 64 * 1024 * 1024,
                        "Catalog source inspection exceeds 64 MiB; no partial qualification"
                    );
                    let sha = format!("{:x}", Sha256::digest(&bytes));
                    String::from_utf8(bytes)
                        .map(|text| (sha, text))
                        .map_err(|_| SourceFailure {
                            outcome: Outcome::Unverified,
                            message: "Referenced source must be UTF-8".into(),
                        })
                }
                Err(e) => Err(SourceFailure {
                    outcome: if matches!(e, crate::EvidenceError::Git(_)) {
                        Outcome::Error
                    } else {
                        Outcome::Unverified
                    },
                    message: e.to_string(),
                }),
            };
            self.files.insert(source.path.clone(), value);
        }
        let (sha, text) = self
            .files
            .get(&source.path)
            .context("Referenced source missing")?
            .as_ref()
            .map_err(|e| SourceFailure {
                outcome: e.outcome,
                message: e.message.clone(),
            })?;
        // Comparing the first and last positions catches overlapping occurrences too.
        let first = text.find(&source.excerpt);
        ensure!(
            sha == &source.sha256 && first.is_some() && first == text.rfind(&source.excerpt),
            "Source digest differs, or exact excerpt is absent/ambiguous in staged bytes; include enough context to identify one assertion or specification fragment"
        );
        Ok(())
    }
}
