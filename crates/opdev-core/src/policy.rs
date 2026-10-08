//! Exact policy identities. Selection changes assessment, never observed facts.
use serde::{Deserialize, Serialize};

use crate::{CatalogError, Gate, RuleCatalog, embedded_catalog};

/// Explicit project-owned opt-in; absence retains the legacy catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineeringPolicy {
    /// Exact engineering baseline version (currently `1`).
    pub version: String,
    /// Exact `MinimumCD` mapping version, or no requested assessment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimumcd: Option<String>,
    /// Existing developer decision authorizing this policy migration.
    pub review_reference: String,
    /// Supported maintenance lines, not additional integration trunks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub maintenance_branches: Vec<MaintenanceBranch>,
}

/// A declaration requiring branch-specific review and CI, never a waiver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceBranch {
    /// Exact branch name, not a wildcard.
    pub name: String,
    /// Supported product/version line.
    pub supported_version: String,
    /// Existing authority describing lifetime, permitted fixes and protection/CI.
    pub authority: String,
}

/// Obligation in engineering policy 1; preferences are not required rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleClass {
    /// Always required at its relevant boundary.
    Baseline,
    /// Required when its evidence-backed applicability condition holds.
    Conditional,
    /// Only the separately requested `MinimumCD` assessment requires this rule.
    MinimumcdAssessment,
}

/// Exhaustive classification; unknown IDs must not silently disappear.
#[must_use]
pub fn rule_class(id: &str) -> Option<RuleClass> {
    match id {
        "MCD-TRUNK-002" | "MCD-TRUNK-003" | "MCD-RECOVERY-002" => {
            Some(RuleClass::MinimumcdAssessment)
        }
        "OPDEV-AUTH-001" | "MCD-CI-001" | "MCD-TRUNK-001" | "OPDEV-TEST-001"
        | "OPDEV-BUILD-001" | "OPDEV-STYLE-001" | "OPDEV-SEC-003" => Some(RuleClass::Baseline),
        "OPDEV-AUTH-002" | "OPDEV-WORK-001" | "OPDEV-DESIGN-001" | "OPDEV-BRANCH-001"
        | "MCD-TEST-001" | "MCD-TEST-002" | "MCD-FLOW-001" | "MCD-COMPAT-001"
        | "MCD-DELIVERY-001" | "MCD-PIPELINE-001" | "MCD-ARTIFACT-001" | "MCD-ARTIFACT-002"
        | "MCD-ENV-001" | "MCD-RECOVERY-001" | "MCD-CONFIG-001" | "MCD-CONFIG-002"
        | "OPDEV-TEST-002" | "OPDEV-TEST-003" | "OPDEV-TEST-004" | "OPDEV-TEST-005"
        | "OPDEV-TEST-006" | "OPDEV-TEST-007" | "OPDEV-EVAL-001" | "OPDEV-SEC-001"
        | "OPDEV-SEC-002" | "OPDEV-SUPPLY-001" | "OPDEV-SUPPLY-002" | "OPDEV-A11Y-001"
        | "OPDEV-OPS-001" | "OPDEV-AI-001" | "OPDEV-LEARN-001" | "OPDEV-EXT-001" => {
            Some(RuleClass::Conditional)
        }
        _ => None,
    }
}

/// Resolve a supported catalog without changing historical catalog 2.
///
/// # Errors
/// Rejects unknown versions and malformed embedded rules.
pub fn catalog_for_version(version: u32) -> Result<RuleCatalog, CatalogError> {
    let mut catalog = embedded_catalog()?;
    if version == 2 {
        return Ok(catalog);
    }
    if version != 3 {
        return Err(CatalogError::UnsupportedVersion(version));
    }
    let additions = RuleCatalog::from_yaml(include_str!("../../../rules/engineering.yaml"))?;
    catalog.catalog_version = 3;
    catalog.rules.extend(additions.rules);
    for rule in &mut catalog.rules {
        if rule_class(rule.id.as_str()).is_none() {
            return Err(CatalogError::Unclassified(rule.id.clone()));
        }
        if rule_class(rule.id.as_str()) == Some(RuleClass::MinimumcdAssessment) {
            rule.gates = vec![Gate::Compliance];
        }
    }
    // Reuse catalog validation, including duplicate IDs across both documents.
    catalog.validate()?;
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_classifies_every_rule_and_accounts_for_every_minimumcd_rule()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy = embedded_catalog()?;
        let next = catalog_for_version(3)?;
        assert_eq!(legacy.rules.len(), 37);
        assert_eq!(next.rules.len(), 42);
        assert!(
            next.rules
                .iter()
                .all(|r| rule_class(r.id.as_str()).is_some())
        );
        assert!(catalog_for_version(99).is_err());
        assert!(rule_class("OPDEV-UNKNOWN-001").is_none());
        let profile = crate::resolve_profile("minimumcd", "1", None)?;
        assert_eq!(profile.requirements.len(), 19, "all pinned source clauses");
        for rule in legacy
            .rules
            .iter()
            .filter(|r| r.kind == crate::RuleKind::Minimumcd)
        {
            // The broader recovery requirement remains mandatory engineering;
            // new rollback-specific evidence replaces it in the external claim.
            assert!(
                rule.id.as_str() == "MCD-RECOVERY-001"
                    || profile
                        .requirements
                        .iter()
                        .any(|p| p.mapped_rules.contains(&rule.id))
            );
        }
        for id in [
            "MCD-CI-001",
            "MCD-TRUNK-001",
            "MCD-DELIVERY-001",
            "MCD-ARTIFACT-002",
            "MCD-RECOVERY-001",
        ] {
            assert_ne!(rule_class(id), Some(RuleClass::MinimumcdAssessment));
        }
        assert!(
            legacy
                .rules
                .iter()
                .find(|r| r.id.as_str() == "MCD-TRUNK-002")
                .ok_or("legacy rule")?
                .gates
                .contains(&Gate::Integration)
        );
        assert!(
            !next
                .rules
                .iter()
                .find(|r| r.id.as_str() == "MCD-TRUNK-002")
                .ok_or("new rule")?
                .gates
                .contains(&Gate::Integration)
        );
        Ok(())
    }
}
