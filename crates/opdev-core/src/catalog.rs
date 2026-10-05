use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const EMBEDDED_CATALOG: &str = include_str!("../../../rules/core.yaml");

/// Failures encountered while loading the normative rule catalog.
#[derive(Debug, Error)]
pub enum CatalogError {
    /// The YAML document could not be parsed.
    #[error("the OpDev rule catalog is invalid YAML: {0}")]
    Yaml(#[from] serde_saphyr::Error),

    /// A rule ID is malformed.
    #[error("invalid rule ID `{0}`")]
    InvalidRuleId(String),

    /// Two catalog entries use the same stable ID.
    #[error("duplicate rule ID `{0}`")]
    DuplicateRuleId(RuleId),

    /// The catalog does not contain any rules.
    #[error("the OpDev rule catalog contains no rules")]
    Empty,
}

/// A stable `OpDev` rule identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct RuleId(String);

impl RuleId {
    /// Returns the identifier as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for RuleId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for RuleId {
    type Err = CatalogError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = value.split('-').collect();
        let valid = parts.len() >= 3
            && parts.iter().all(|part| {
                !part.is_empty()
                    && part
                        .chars()
                        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
            })
            && parts.last().is_some_and(|suffix| {
                suffix.len() == 3 && suffix.chars().all(|ch| ch.is_ascii_digit())
            });

        if valid {
            Ok(Self(value.to_owned()))
        } else {
            Err(CatalogError::InvalidRuleId(value.to_owned()))
        }
    }
}

impl<'de> Deserialize<'de> for RuleId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Origin of a core requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleKind {
    /// A requirement derived from `MinimumCD`.
    Minimumcd,
    /// A requirement defined by `OpDev`.
    Opdev,
}

/// A decision boundary affected by a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    /// Whether ordinary implementation work may proceed.
    Development,
    /// Whether a change may integrate into trunk.
    Integration,
    /// Whether an artifact may be delivered.
    Delivery,
    /// Whether a conformance claim may be made.
    Compliance,
}

/// A mechanism that can contribute evidence for a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    /// Agent workflow and evidence inspection.
    Agent,
    /// CI configuration or results.
    Ci,
    /// Stored evidence records.
    Evidence,
    /// A project-owned or packaged extension.
    Extension,
    /// Local Git state.
    Git,
    /// Project-manifest declarations.
    Manifest,
    /// Remote repository policy or pipeline state.
    Remote,
}

/// A normative or informative source for a rule.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Human-readable source name.
    pub name: String,
    /// Stable URL or repository-relative path.
    pub url: String,
}

/// A single normative `OpDev` rule.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Permanent identifier.
    pub id: RuleId,
    /// Requirement origin.
    pub kind: RuleKind,
    /// Short diagnostic title.
    pub title: String,
    /// Normative statement.
    pub statement: String,
    /// Human-readable applicability condition.
    pub applicability: String,
    /// Gates affected by the rule.
    pub gates: Vec<Gate>,
    /// Evidence mechanisms that can verify the rule.
    pub verification: Vec<VerificationMethod>,
    /// Normative or informative sources.
    pub sources: Vec<Source>,
}

/// Versioned collection of core rules.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleCatalog {
    /// Catalog compatibility version.
    pub catalog_version: u32,
    /// Core rules in stable presentation order.
    pub rules: Vec<Rule>,
}

impl Rule {
    /// Plain-language recovery guidance; this does not grant approval or satisfy a rule.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one declarative guidance entry per core rule"
    )]
    pub fn next_step(&self) -> &'static str {
        match self.id.as_str() {
            "OPDEV-AUTH-001" => {
                "Next: list the existing sources of project decisions in .opdev/project.yaml; do not create duplicate documents."
            }
            "OPDEV-AUTH-002" => {
                "Next: identify the issue tracker or other place that holds current work and progress in .opdev/project.yaml."
            }
            "OPDEV-WORK-001" => {
                "Next: record what this change should achieve, its limits, and how you will test it in the existing work item; then link the reviewed facts in .opdev/evidence.yaml."
            }
            "OPDEV-DESIGN-001" => {
                "Next: record the important design decision, alternatives, reasons, and when to revisit it in the project's existing design source; link that review in .opdev/evidence.yaml."
            }
            "MCD-CI-001" => {
                "Next: configure the automated CI pipeline to check proposed changes and the main development branch."
            }
            "MCD-TRUNK-001" => {
                "Next: confirm which single branch receives integrated changes and supplies releases; record its name and the reviewed workflow."
            }
            "MCD-TRUNK-002" => {
                "Next: review where the branch started, its scope, and its merge/deletion plan. Do not infer a violation from a one-day age threshold alone."
            }
            "MCD-TRUNK-003" => {
                "Next: review why work has not merged and choose a smaller tested change or resolve the blocker. Daily merging is a target, not a development or merge blocker; missing or missed cadence still prevents a compliance claim."
            }
            "MCD-TEST-001" => {
                "Next: run the required tests on the proposed change in CI before merging; inspect any failed or missing job."
            }
            "MCD-TEST-002" => {
                "Next: verify the required CI jobs on the actual merged revision, not just the feature branch or an older successful run."
            }
            "MCD-FLOW-001" => {
                "Next: check the main branch's required CI jobs. If they fail, repair them before adding features; diagnosis and repair remain allowed."
            }
            "MCD-COMPAT-001" => {
                "Next: test the behavior existing users depend on, and review any intentional change to supported behavior."
            }
            "MCD-DELIVERY-001" => {
                "Next: declare the automated pipeline and destination used to deliver software in .opdev/project.yaml."
            }
            "MCD-PIPELINE-001" => {
                "Next: verify that delivery requires the pipeline's checks to pass and cannot bypass them."
            }
            "MCD-ARTIFACT-001" => {
                "Next: define what the installable package or other deliverable must contain, and verify the actual pipeline output against that definition."
            }
            "MCD-ARTIFACT-002" => {
                "Next: identify the built package by its checksum and promote those same tested bytes without rebuilding them."
            }
            "MCD-ENV-001" => {
                "Next: test the actual deliverable in an environment representative of where users will run it."
            }
            "MCD-RECOVERY-001" => {
                "Next: automate and test how to recover from a bad delivery, using rollback or a safe forward fix."
            }
            "MCD-CONFIG-001" => {
                "Next: keep behavior-changing configuration in version control and test it with the software."
            }
            "MCD-CONFIG-002" => {
                "Next: supply environment-specific settings through the delivery system without changing or rebuilding the tested package; keep secrets out of reports."
            }
            "OPDEV-TEST-001" => {
                "Next: identify the risks of this project and the tests that address them in .opdev/project.yaml."
            }
            "OPDEV-TEST-002" => {
                "Next: link each expected result for this change to a test or reviewed observation in .opdev/evidence.yaml. Review the actual assertions; a green test suite alone is not enough."
            }
            "OPDEV-TEST-003" => {
                "Next: add or identify tests that check the changed behavior, review their assertions, and run the required suites for this stage."
            }
            "OPDEV-TEST-004" => {
                "Next: add a test that reproduces the reported defect and verifies the fix, or record the specific reason automation cannot cover it."
            }
            "OPDEV-TEST-005" => {
                "Next: investigate failing or inconsistent tests; retain failed attempts and explicitly record skipped or quarantined tests rather than counting them as passes."
            }
            "OPDEV-TEST-006" => {
                "Next: review untested risky behavior; do not treat a coverage percentage as proof that tests check the right results."
            }
            "OPDEV-TEST-007" => {
                "Next: record the external service or hardware, environment, timing limits, and required result for live tests; missing runs are not passes."
            }
            "OPDEV-EVAL-001" => {
                "Next: distinguish tests that show the software works correctly from observations that show it solves the user's problem."
            }
            "OPDEV-SEC-001" => {
                "Next: review security risks, dependencies, tests, and vulnerability handling appropriate to this project; link the reviewed evidence."
            }
            "OPDEV-SEC-002" => {
                "Next: check that CI cannot expose privileged credentials or permissions to untrusted changes."
            }
            "OPDEV-SUPPLY-001" => {
                "Next: provide a machine-readable record connecting the delivered package checksum to its source revision and build."
            }
            "OPDEV-SUPPLY-002" => {
                "Next: include a machine-readable list of shipped third-party components, associated with the delivered package checksum."
            }
            "OPDEV-A11Y-001" => {
                "Next: confirm the accessibility expectations for this interface and collect the required automated checks and actual user review."
            }
            "OPDEV-OPS-001" => {
                "Next: demonstrate how a user or operator can tell whether the software is working and investigate failures."
            }
            "OPDEV-AI-001" => {
                "Next: review agent-produced changes against the expected results and actual tests; the author's claim of correctness is not independent verification."
            }
            "OPDEV-LEARN-001" => {
                "Next: record the observed failure and the concrete follow-up or prevention work in the existing work item."
            }
            "OPDEV-EXT-001" => {
                "Next: fix the additional project check without bypassing a required OpDev check."
            }
            _ => {
                "Next: review this requirement and supply current supporting facts; do not mark it passed just to clear the blocker."
            }
        }
    }
}

impl RuleCatalog {
    /// Parses and performs structural validation on a catalog.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError`] when YAML parsing, rule ID validation, or
    /// structural validation fails.
    pub fn from_yaml(yaml: &str) -> Result<Self, CatalogError> {
        let catalog: Self = serde_saphyr::from_str(yaml)?;
        catalog.validate()?;
        Ok(catalog)
    }

    /// Returns the rule with the requested ID.
    #[must_use]
    pub fn find(&self, id: &RuleId) -> Option<&Rule> {
        self.rules.iter().find(|rule| &rule.id == id)
    }

    fn validate(&self) -> Result<(), CatalogError> {
        if self.rules.is_empty() {
            return Err(CatalogError::Empty);
        }

        let mut ids = HashSet::with_capacity(self.rules.len());
        for rule in &self.rules {
            if !ids.insert(rule.id.clone()) {
                return Err(CatalogError::DuplicateRuleId(rule.id.clone()));
            }
        }

        Ok(())
    }
}

/// Loads the rule catalog embedded in the `OpDev` binary.
///
/// # Errors
///
/// Returns [`CatalogError`] when the embedded catalog cannot be parsed or fails
/// structural validation.
pub fn embedded_catalog() -> Result<RuleCatalog, CatalogError> {
    RuleCatalog::from_yaml(EMBEDDED_CATALOG)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_is_structurally_valid() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = embedded_catalog()?;
        assert_eq!(catalog.catalog_version, 2);
        assert_eq!(catalog.rules.len(), 37);
        Ok(())
    }

    #[test]
    fn rule_ids_reject_invalid_shapes() {
        assert!("OPDEV-TEST-001".parse::<RuleId>().is_ok());
        assert!("opdev-test-001".parse::<RuleId>().is_err());
        assert!("OPDEV-001".parse::<RuleId>().is_err());
        assert!("OPDEV-TEST-1".parse::<RuleId>().is_err());
    }

    #[test]
    fn catalog_matches_its_json_schema() -> Result<(), Box<dyn std::error::Error>> {
        let yaml_value: serde_json::Value = serde_saphyr::from_str(EMBEDDED_CATALOG)?;
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../rules/core.schema.json"))?;
        let validator = jsonschema::validator_for(&schema)?;
        let errors: Vec<_> = validator.iter_errors(&yaml_value).collect();
        assert!(errors.is_empty(), "schema errors: {errors:#?}");
        Ok(())
    }
}
