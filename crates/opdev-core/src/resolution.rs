//! Pure inspection of engineering obligations; no execution or consent inference.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{CatalogError, Rule, RuleCatalog, RuleClass, RuleId, catalog_for_version, rule_class};

/// How a historical rule relates to the selected engineering baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDisposition {
    /// Same normative meaning and engineering membership.
    Retained,
    /// New engineering meaning has a distinct stable rule ID.
    Replaced,
    /// Historical rule is evaluated only for a separately selected standard.
    ExternalOnly,
}

/// Every old rule remains accounted for, even if its engineering meaning changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicyChange {
    /// Historical identifier; its original statement is never rewritten.
    pub rule: RuleId,
    /// Relationship to the proposed baseline.
    pub disposition: PolicyDisposition,
    /// Distinct replacement where semantics change.
    pub replacement: Option<RuleId>,
    /// Reason for membership or semantic change.
    pub rationale: String,
}

/// An obligation definition, not a finding that it applies or passes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedControl {
    /// Complete versioned statement, applicability, gates and verification routes.
    pub rule: Rule,
    /// Baseline or evidence-dependent obligation; never a per-project waiver.
    pub class: RuleClass,
}

/// Deterministic in-memory preview. No project has migrated merely by resolving it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicyResolution {
    /// Preview wire format, independent of check-report schema.
    pub schema: u32,
    /// Exact engineering policy version.
    pub engineering_version: String,
    /// Intended catalog identity; does not enable its use in project contracts.
    pub catalog_version: u32,
    /// SHA-256 of the serialized semantic fields, excluding this identity itself.
    pub definition_sha256: String,
    /// Required engineering controls in stable catalog order.
    pub controls: Vec<ResolvedControl>,
    /// Complete catalog-3 accounting, including unchanged and external-only rules.
    pub changes: Vec<PolicyChange>,
    /// Historical definitions outside engineering gates. A selected external
    /// mapping determines which are actually required; this is not that mapping.
    pub external_rules: Vec<Rule>,
    /// Interpretation limits; never an approval or verification result.
    pub limits: String,
}

/// Preview failures cannot silently select an older or weaker policy.
#[derive(Debug, thiserror::Error)]
pub enum PolicyResolutionError {
    /// Exact supported versions only.
    #[error(
        "Unsupported engineering policy {0}; supported preview versions are 1 and 2. Nothing changed."
    )]
    UnsupportedVersion(String),
    /// Incomplete or invalid policy definitions.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    /// A replacement must have precisely one owner and a valid definition.
    #[error("Policy definition conflict: {0}. No policy resolved.")]
    Conflict(String),
    /// Deterministic encoding must succeed before returning an identity.
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
}

const REPLACEMENTS: &[(&str, &str, &str)] = &[
    (
        "MCD-FLOW-001",
        "OPDEV-FLOW-001",
        "Prioritize restoration and block affected work; demonstrated independent work need not stop. Unknown impact is not independence.",
    ),
    (
        "MCD-DELIVERY-001",
        "OPDEV-DELIVERY-001",
        "Separate local feedback, authorized previews and supported delivery; readiness never grants release authority.",
    ),
    (
        "MCD-PIPELINE-001",
        "OPDEV-PIPELINE-001",
        "Require a complete verdict at the supported delivery boundary, not full delivery qualification for every feedback iteration.",
    ),
    (
        "MCD-RECOVERY-001",
        "OPDEV-RECOVERY-001",
        "Require demonstrated safe recovery covering state and external effects; retain literal rollback as a separate MinimumCD obligation.",
    ),
];

/// Resolve embedded policy definitions without inspecting a project or executing checks.
///
/// # Errors
/// Unknown versions, unclassified rules, missing/duplicate replacements or encoding failures.
pub fn resolve_engineering_policy(
    version: &str,
) -> Result<PolicyResolution, PolicyResolutionError> {
    if !matches!(version, "1" | "2") {
        return Err(PolicyResolutionError::UnsupportedVersion(version.into()));
    }
    let historical = catalog_for_version(3)?;
    let additions = if version == "2" {
        RuleCatalog::from_yaml(include_str!("../../../rules/engineering-2.yaml"))?
    } else {
        RuleCatalog {
            catalog_version: 4,
            rules: Vec::new(),
        }
    };
    resolve_definitions(version, historical, &additions)
}

fn resolve_definitions(
    version: &str,
    historical: RuleCatalog,
    additions: &RuleCatalog,
) -> Result<PolicyResolution, PolicyResolutionError> {
    historical.validate()?;
    if version == "2" {
        additions.validate()?;
    }
    let mut controls = Vec::new();
    let mut changes = Vec::new();
    let mut external_rules = Vec::new();
    let mut consumed = std::collections::HashSet::new();
    for rule in historical.rules {
        let class = rule_class(rule.id.as_str())
            .ok_or_else(|| CatalogError::Unclassified(rule.id.clone()))?;
        let replacement = (version == "2")
            .then(|| {
                REPLACEMENTS
                    .iter()
                    .find(|entry| entry.0 == rule.id.as_str())
            })
            .flatten();
        let (disposition, replacement_id, rationale) = if let Some((_, id, rationale)) = replacement
        {
            let next = additions
                .rules
                .iter()
                .find(|r| r.id.as_str() == *id)
                .ok_or_else(|| {
                    PolicyResolutionError::Conflict(format!("missing replacement {id}"))
                })?;
            if !consumed.insert(next.id.clone()) {
                return Err(PolicyResolutionError::Conflict(format!(
                    "duplicate replacement {id}"
                )));
            }
            controls.push(ResolvedControl {
                rule: next.clone(),
                class,
            });
            external_rules.push(rule.clone());
            (
                PolicyDisposition::Replaced,
                Some(next.id.clone()),
                (*rationale).to_owned(),
            )
        } else if class == RuleClass::MinimumcdAssessment {
            external_rules.push(rule.clone());
            (PolicyDisposition::ExternalOnly, None, "Preserve the exact historical standard requirement separately; it cannot be inferred from engineering readiness. Cadence may guide replanning but is not an engineering timer gate.".into())
        } else {
            controls.push(ResolvedControl {
                rule: rule.clone(),
                class,
            });
            (PolicyDisposition::Retained, None, "Retain the existing requirement, applicability, verification and boundary; adequate implementation choices remain project-owned.".into())
        };
        changes.push(PolicyChange {
            rule: rule.id,
            disposition,
            replacement: replacement_id,
            rationale,
        });
    }
    if version == "2" && consumed.len() != additions.rules.len() {
        return Err(PolicyResolutionError::Conflict(
            "unmapped policy additions".into(),
        ));
    }
    let mut ids = std::collections::HashSet::new();
    for control in &controls {
        if !ids.insert(control.rule.id.clone()) {
            return Err(PolicyResolutionError::Conflict(format!(
                "duplicate control {}",
                control.rule.id
            )));
        }
    }
    let catalog_version = if version == "1" { 3 } else { 4 };
    let semantic = serde_json::to_vec(&(
        1_u32,
        version,
        catalog_version,
        &controls,
        &changes,
        &external_rules,
    ))?;
    Ok(PolicyResolution {
        schema: 1,
        engineering_version: version.into(),
        catalog_version,
        definition_sha256: format!("{:x}", Sha256::digest(semantic)),
        controls,
        changes,
        external_rules,
        limits: "Definition preview only. Applicability still needs current evidence. No checks ran, no project changed, no standard conformance or release is authorized. Policy 2 preview does not enable policy 2 enforcement.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_accounts_for_every_old_rule_without_mutating_it()
    -> Result<(), Box<dyn std::error::Error>> {
        let old = catalog_for_version(3)?;
        let before = serde_json::to_vec(&old)?;
        let one = resolve_engineering_policy("1")?;
        let two = resolve_engineering_policy("2")?;
        assert_eq!(two.changes.len(), 42);
        assert_eq!(two.controls.len(), 39);
        assert_eq!(two.external_rules.len(), 7);
        assert_eq!(one.external_rules.len(), 3);
        assert_eq!(
            two.changes
                .iter()
                .filter(|c| c.disposition == PolicyDisposition::Replaced)
                .count(),
            4
        );
        for rule in old.rules {
            let change = two
                .changes
                .iter()
                .find(|c| c.rule == rule.id)
                .ok_or("missing historical rule")?;
            if change.disposition == PolicyDisposition::Retained {
                assert_eq!(
                    two.controls
                        .iter()
                        .find(|c| c.rule.id == rule.id)
                        .ok_or("missing retained")?
                        .rule,
                    rule
                );
            } else {
                assert_eq!(
                    two.external_rules
                        .iter()
                        .find(|r| r.id == rule.id)
                        .ok_or("missing historical definition")?,
                    &rule
                );
            }
        }
        assert_eq!(before, serde_json::to_vec(&catalog_for_version(3)?)?);
        assert_eq!(catalog_for_version(4)?.rules.len(), 46);
        assert_ne!(one.definition_sha256, two.definition_sha256);
        assert_eq!(two, resolve_engineering_policy("2")?);
        Ok(())
    }

    #[test]
    fn unknown_and_conflicting_definitions_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
        for version in ["", "latest", "02", "3", "1 "] {
            assert!(resolve_engineering_policy(version).is_err());
        }
        let historical = catalog_for_version(3)?;
        let additions = RuleCatalog::from_yaml(include_str!("../../../rules/engineering-2.yaml"))?;
        let mut unknown = historical.clone();
        unknown.rules[0].id = "UNKNOWN-RULE-001".parse()?;
        assert!(resolve_definitions("2", unknown, &additions).is_err());
        let mut missing = additions.clone();
        missing.rules.remove(0);
        assert!(resolve_definitions("2", historical.clone(), &missing).is_err());
        let mut duplicate = additions.clone();
        duplicate.rules.push(duplicate.rules[0].clone());
        assert!(resolve_definitions("2", historical.clone(), &duplicate).is_err());
        let mut unmapped = additions;
        let mut extra = unmapped.rules[0].clone();
        extra.id = "UNKNOWN-RULE-002".parse()?;
        unmapped.rules.push(extra);
        assert!(resolve_definitions("2", historical, &unmapped).is_err());
        Ok(())
    }

    #[test]
    fn mandatory_controls_and_semantic_identity_are_not_optional()
    -> Result<(), Box<dyn std::error::Error>> {
        let policy = resolve_engineering_policy("2")?;
        for id in [
            "OPDEV-BUILD-001",
            "OPDEV-STYLE-001",
            "OPDEV-SEC-003",
            "MCD-TEST-001",
            "MCD-TEST-002",
            "OPDEV-TEST-002",
            "OPDEV-TEST-003",
        ] {
            assert!(policy.controls.iter().any(|c| c.rule.id.as_str() == id));
        }
        let historical = catalog_for_version(3)?;
        let mut additions =
            RuleCatalog::from_yaml(include_str!("../../../rules/engineering-2.yaml"))?;
        additions.rules[0].statement.push_str(" Changed.");
        assert_ne!(
            policy.definition_sha256,
            resolve_definitions("2", historical, &additions)?.definition_sha256
        );
        assert!(
            policy
                .controls
                .iter()
                .all(|c| c.rule.id.as_str() != "MCD-TRUNK-003")
        );
        assert!(
            policy
                .external_rules
                .iter()
                .any(|r| r.id.as_str() == "MCD-RECOVERY-002")
        );
        Ok(())
    }
}
