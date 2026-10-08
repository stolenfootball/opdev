//! Pure assessment projections: no commands, network calls or evidence refresh.
use opdev_core::{AggregateVerdict, Outcome, RequirementCoverage, resolve_profile};
use serde::{Deserialize, Serialize};

use crate::{CheckReport, EvaluationError};

/// Policy identity and independently requested assessment in report schema 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineeringAssessment {
    /// Exact engineering policy version.
    pub version: String,
    /// Absent means not requested, never passed or not applicable.
    pub minimumcd: Option<FrameworkAssessment>,
}

/// Complete result for a pinned external requirement mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameworkAssessment {
    /// Exact `OpDev` mapping version (not an invented upstream version).
    pub version: String,
    /// Exact authoritative upstream source identity.
    pub source_version: String,
    /// All applicable requirements and selected blocking checks must qualify.
    pub verdict: AggregateVerdict,
    /// Every mapped clause remains visible, including gaps.
    pub requirements: Vec<RequirementAssessment>,
    /// Required command failures or missing executions cannot disappear.
    pub blocking_checks: Vec<String>,
}

/// Result of evaluating one upstream clause against existing rule evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementAssessment {
    /// Stable identifier in the mapping.
    pub id: String,
    /// Evidence outcome, not mapping coverage.
    pub outcome: Outcome,
    /// Rule findings which do not establish this clause.
    pub blocking_rules: Vec<String>,
}

impl EngineeringAssessment {
    pub(crate) fn requested(minimumcd: Option<&str>) -> Self {
        Self {
            version: "1".into(),
            minimumcd: minimumcd.map(|version| FrameworkAssessment {
                version: version.into(),
                source_version: String::new(),
                verdict: AggregateVerdict::Blocked,
                requirements: vec![],
                blocking_checks: vec![],
            }),
        }
    }
}

pub(crate) fn refresh(report: &mut CheckReport) -> Result<(), EvaluationError> {
    let Some(assessment) = report
        .engineering
        .as_mut()
        .and_then(|p| p.minimumcd.as_mut())
    else {
        return Ok(());
    };
    let profile = resolve_profile("minimumcd", &assessment.version, None)?;
    assess_requirements(assessment, &profile.requirements, &report.rules);
    assessment.source_version = profile.source.version;
    assessment.blocking_checks = report
        .checks
        .iter()
        .filter(|c| c.blocking && !c.outcome.satisfies_required_rule())
        .map(|c| c.id.clone())
        .collect();
    assessment.verdict = if !assessment.requirements.is_empty()
        && assessment
            .requirements
            .iter()
            .all(|r| r.outcome.satisfies_required_rule())
        && assessment.blocking_checks.is_empty()
    {
        AggregateVerdict::Passed
    } else {
        AggregateVerdict::Blocked
    };
    Ok(())
}

fn assess_requirements(
    assessment: &mut FrameworkAssessment,
    requirements: &[opdev_core::ProfileRequirement],
    rules: &[opdev_core::RuleResult],
) {
    assessment.requirements = requirements
        .iter()
        .map(|requirement| {
            let blocking_rules: Vec<_> = requirement
                .mapped_rules
                .iter()
                .filter(|id| {
                    !rules.iter().any(|result| {
                        result.rule_id == **id && result.outcome.satisfies_required_rule()
                    })
                })
                .map(ToString::to_string)
                .collect();
            let outcomes: Vec<_> = requirement
                .mapped_rules
                .iter()
                .filter_map(|id| {
                    rules
                        .iter()
                        .find(|result| result.rule_id == *id)
                        .map(|r| r.outcome)
                })
                .collect();
            let outcome = if outcomes.contains(&Outcome::Failed) {
                Outcome::Failed
            } else if outcomes.contains(&Outcome::Error) {
                Outcome::Error
            } else if outcomes.contains(&Outcome::MigrationRequired) {
                Outcome::MigrationRequired
            } else if requirement.coverage != RequirementCoverage::Full
                || requirement.mapped_rules.is_empty()
                || !blocking_rules.is_empty()
            {
                Outcome::Unverified
            } else if outcomes.iter().all(|o| *o == Outcome::NotApplicable) {
                Outcome::NotApplicable
            } else {
                Outcome::Passed
            };
            RequirementAssessment {
                id: requirement.id.clone(),
                outcome,
                blocking_rules,
            }
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reaggregate;
    use opdev_core::{Gate, RuleResult, VerificationSource, catalog_for_version};

    #[test]
    fn incomplete_mapping_cannot_turn_green_results_into_compliance()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut report = reviewed_report()?;
        let assessment = report
            .engineering
            .as_mut()
            .and_then(|p| p.minimumcd.as_mut())
            .ok_or("assessment")?;
        let mut profile = resolve_profile("minimumcd", "1", None)?;
        for coverage in [RequirementCoverage::Partial, RequirementCoverage::Gap] {
            profile.requirements[0].coverage = coverage;
            assess_requirements(assessment, &profile.requirements, &report.rules);
            assert_eq!(assessment.requirements[0].outcome, Outcome::Unverified);
        }
        profile.requirements[0].coverage = RequirementCoverage::Full;
        profile.requirements[0].mapped_rules.clear();
        assess_requirements(assessment, &profile.requirements, &report.rules);
        assert_eq!(assessment.requirements[0].outcome, Outcome::Unverified);
        Ok(())
    }

    fn reviewed_report() -> Result<CheckReport, Box<dyn std::error::Error>> {
        let catalog = catalog_for_version(3)?;
        let rules = catalog
            .rules
            .into_iter()
            .map(|rule| RuleResult {
                rule_id: rule.id,
                catalog_version: 3,
                outcome: Outcome::Passed,
                subject: "synthetic-reviewed-subject".into(),
                verifier: VerificationSource::Agent,
                evaluated_at: 1,
                evidence: vec![],
                diagnostic: None,
            })
            .collect();
        Ok(CheckReport {
            schema: 2,
            catalog_version: 3,
            engineering: Some(EngineeringAssessment::requested(Some("1"))),
            subject: "synthetic-reviewed-subject".into(),
            evaluated_at: 1,
            rules,
            checks: vec![],
            gates: vec![],
        })
    }

    #[test]
    fn cadence_and_rollback_are_separate_from_engineering_gates()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut report = reviewed_report()?;
        reaggregate(&mut report)?;
        assert_eq!(
            report
                .engineering
                .as_ref()
                .and_then(|p| p.minimumcd.as_ref())
                .ok_or("assessment")?
                .verdict,
            AggregateVerdict::Passed
        );
        for id in ["MCD-TRUNK-003", "MCD-RECOVERY-002", "MCD-TRUNK-002"] {
            report
                .rules
                .iter_mut()
                .find(|r| r.rule_id.as_str() == id)
                .ok_or("rule")?
                .outcome = Outcome::Unverified;
        }
        reaggregate(&mut report)?;
        assert!(report.gate_passed(Gate::Integration));
        assert!(report.gate_passed(Gate::Delivery));
        assert!(report.gate_passed(Gate::Compliance));
        let assessment = report
            .engineering
            .as_ref()
            .and_then(|p| p.minimumcd.as_ref())
            .ok_or("assessment")?;
        assert_eq!(assessment.verdict, AggregateVerdict::Blocked);
        assert!(
            assessment
                .requirements
                .iter()
                .any(|r| r.id == "cd-rollback" && r.outcome == Outcome::Unverified)
        );
        assert!(
            assessment
                .requirements
                .iter()
                .any(|r| r.id == "ci-daily" && r.outcome == Outcome::Unverified)
        );
        // A broad recovery pass cannot satisfy the separately mapped rollback.
        assert_eq!(
            report
                .rules
                .iter()
                .find(|r| r.rule_id.as_str() == "MCD-RECOVERY-001")
                .ok_or("recovery")?
                .outcome,
            Outcome::Passed
        );
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/report.schema.json"))?;
        assert!(jsonschema::is_valid(
            &schema,
            &serde_json::to_value(&report)?
        ));
        Ok(())
    }

    #[test]
    fn missing_baseline_and_failure_never_disappear_with_profile_selection()
    -> Result<(), Box<dyn std::error::Error>> {
        for outcome in [
            Outcome::Failed,
            Outcome::Error,
            Outcome::Unverified,
            Outcome::MigrationRequired,
            Outcome::NotApplicable,
        ] {
            let mut report = reviewed_report()?;
            report.engineering = Some(EngineeringAssessment::requested(None));
            report
                .rules
                .iter_mut()
                .find(|r| r.rule_id.as_str() == "MCD-CI-001")
                .ok_or("rule")?
                .outcome = outcome;
            reaggregate(&mut report)?;
            assert!(!report.gate_passed(Gate::Integration));
            assert!(
                report
                    .engineering
                    .as_ref()
                    .ok_or("policy")?
                    .minimumcd
                    .is_none()
            );
        }
        let mut report = reviewed_report()?;
        report.rules.retain(|r| r.rule_id.as_str() != "MCD-CI-001");
        reaggregate(&mut report)?;
        assert!(!report.gate_passed(Gate::Integration));
        assert_eq!(
            report
                .engineering
                .as_ref()
                .and_then(|p| p.minimumcd.as_ref())
                .ok_or("assessment")?
                .verdict,
            AggregateVerdict::Blocked
        );
        report
            .engineering
            .as_mut()
            .ok_or("policy")?
            .minimumcd
            .as_mut()
            .ok_or("assessment")?
            .version = "99".into();
        assert!(reaggregate(&mut report).is_err());
        Ok(())
    }
}
