//! Additional standards project existing observations; they never launch verification.
use opdev_core::{AggregateVerdict, Outcome, ProfileSource};
use opdev_project::{StandardMode, StandardSelection};
use serde::{Deserialize, Serialize};

use crate::{CheckKind, CheckReport, CheckResult, EvaluationError, FrameworkAssessment};

const CHECK_PREFIX: &str = "opdev-standard:";

/// Exact selected mapping and honest assessment scope at the report's stage.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StandardAssessment {
    /// Selection under the existing project decision authority.
    pub selection: StandardSelection,
    /// Exact resolved mapping and selection identity.
    pub definition_sha256: String,
    /// Exact upstream reference, never fetched during evaluation.
    pub source: ProfileSource,
    /// Original mapping's stated scope and limits.
    pub claim: String,
    /// False for incomplete/derived/evidence-format mappings.
    pub complete_mapping: bool,
    /// Absent for guidance or a stage not selected for assessment; never a pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessment: Option<FrameworkAssessment>,
    /// Plain-language explanation of what was and was not evaluated.
    pub diagnostic: String,
}

impl StandardAssessment {
    pub(crate) fn new(selection: StandardSelection) -> Result<Self, EvaluationError> {
        let resolved = selection
            .resolve()
            .map_err(|e| EvaluationError::Report(e.to_string()))?;
        Ok(Self {
            selection,
            definition_sha256: resolved.definition_sha256,
            source: resolved.profile.source,
            claim: resolved.profile.claim,
            complete_mapping: resolved.complete_mapping,
            assessment: None,
            diagnostic: "Definition resolved; no conformance assessment or execution established."
                .into(),
        })
    }
}

pub(crate) fn selected(
    policy: &opdev_core::EngineeringPolicy,
    selections: &[StandardSelection],
) -> Result<Vec<StandardAssessment>, EvaluationError> {
    let mut names = std::collections::HashSet::new();
    if !selections.is_empty() && policy.version != "2" || selections.len() > 16 {
        return Err(EvaluationError::Report(
            "Additional standards need policy 2 and at most 16 selections.".into(),
        ));
    }
    for s in selections {
        if !names.insert(&s.name) || s.name == "minimumcd" && policy.minimumcd.is_some() {
            return Err(EvaluationError::Report("Conflicting standard selections; select each mapping once and do not duplicate the MinimumCD shortcut.".into()));
        }
    }
    selections
        .iter()
        .cloned()
        .map(StandardAssessment::new)
        .collect()
}

pub(crate) fn remove_generated_checks(report: &mut CheckReport) {
    report
        .checks
        .retain(|c| !(c.kind == CheckKind::Policy && c.id.starts_with(CHECK_PREFIX)));
}

pub(crate) fn refresh(report: &mut CheckReport) -> Result<(), EvaluationError> {
    let Some(engineering) = &report.engineering else {
        return Ok(());
    };
    if engineering.standards.is_empty() {
        return Ok(());
    }
    let stage = engineering.stage.ok_or_else(|| {
        EvaluationError::Report("Standard assessment needs the actual evaluated stage.".into())
    })?;
    let selections: Vec<_> = engineering
        .standards
        .iter()
        .map(|s| s.selection.clone())
        .collect();
    let policy = opdev_core::EngineeringPolicy {
        version: engineering.version.clone(),
        minimumcd: engineering.minimumcd.as_ref().map(|a| a.version.clone()),
        review_reference: "projection, not authorization".into(),
        maintenance_branches: vec![],
    };
    let mut results = selected(&policy, &selections)?;
    let mut required = Vec::new();
    for (current, result) in engineering.standards.iter().zip(&mut results) {
        if current.definition_sha256 != result.definition_sha256 {
            return Err(EvaluationError::Report(
                "Selected standard definition changed; no old assessment substituted.".into(),
            ));
        }
        if result.selection.mode == StandardMode::Guidance {
            result.diagnostic =
                "Guidance only; no conformance verdict and no additional checks executed.".into();
            continue;
        }
        if !result.selection.stages.contains(&stage) {
            result.diagnostic = format!(
                "Not assessed at {stage:?}; this stage was not selected. No conformance or later-boundary readiness claim."
            );
            continue;
        }
        let resolved = result
            .selection
            .resolve()
            .map_err(|e| EvaluationError::Report(e.to_string()))?;
        let assessment = project_assessment(result, &resolved.profile.requirements, report);
        if result.selection.mode == StandardMode::Require {
            required.push(CheckResult {
                id: format!(
                    "{CHECK_PREFIX}{}@{}",
                    result.selection.name, result.selection.version
                ),
                kind: CheckKind::Policy,
                blocking: true,
                gates: crate::evaluator::gates_for_test_stage(stage),
                outcome: assessment_outcome(&assessment, &report.checks),
                summary: format!(
                    "Required standard {}@{} at {stage:?}: {:?}. {}",
                    result.selection.name,
                    result.selection.version,
                    assessment.verdict,
                    result.diagnostic
                ),
                evidence: vec![],
                stdout: None,
                stderr: None,
                duration_ms: None,
            });
        }
        result.assessment = Some(assessment);
    }
    if let Some(engineering) = &mut report.engineering {
        engineering.standards = results;
    }
    report.checks.extend(required);
    Ok(())
}

fn project_assessment(
    result: &mut StandardAssessment,
    requirements: &[opdev_core::ProfileRequirement],
    report: &CheckReport,
) -> FrameworkAssessment {
    let mut assessment = FrameworkAssessment {
        version: result.selection.version.clone(),
        source_version: result.source.version.clone(),
        verdict: AggregateVerdict::Blocked,
        requirements: vec![],
        blocking_checks: report
            .checks
            .iter()
            .filter(|c| c.blocking && !c.outcome.satisfies_required_rule())
            .map(|c| c.id.clone())
            .collect(),
    };
    crate::assessment::assess_requirements(&mut assessment, requirements, &report.rules);
    if result.complete_mapping
        && !assessment.requirements.is_empty()
        && assessment
            .requirements
            .iter()
            .all(|r| r.outcome.satisfies_required_rule())
        && assessment.blocking_checks.is_empty()
    {
        assessment.verdict = AggregateVerdict::Passed;
    }
    result.diagnostic = if result.complete_mapping {
            "Assessed the complete pinned mapping using this stage's existing findings; no extra commands ran. This is not third-party certification or release authority."
        } else {
            "This mapping is incomplete or limited in scope. Its contributing checks cannot establish full conformance; missing organizational or external evidence remains unresolved."
        }.into();
    assessment
}

fn assessment_outcome(assessment: &FrameworkAssessment, checks: &[CheckResult]) -> Outcome {
    if assessment.verdict == AggregateVerdict::Passed {
        return Outcome::Passed;
    }
    for known in [Outcome::Failed, Outcome::Error, Outcome::MigrationRequired] {
        if assessment.requirements.iter().any(|r| r.outcome == known)
            || checks
                .iter()
                .any(|c| assessment.blocking_checks.contains(&c.id) && c.outcome == known)
        {
            return known;
        }
    }
    // Incomplete mapping and missing checks are unverified, never failed tests.
    Outcome::Unverified
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EngineeringAssessment, reaggregate};
    use opdev_core::{EngineeringPolicy, Gate, RuleResult, VerificationSource};
    use opdev_project::TestStage;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn selection(mode: StandardMode) -> StandardSelection {
        StandardSelection {
            name: "minimumcd".into(),
            version: "1".into(),
            mode,
            stages: if mode == StandardMode::Guidance {
                vec![]
            } else {
                vec![TestStage::Delivery]
            },
            level: None,
        }
    }

    fn report(mode: StandardMode) -> Result<CheckReport, Box<dyn std::error::Error>> {
        let catalog = opdev_core::catalog_for_version(4)?;
        Ok(CheckReport {
            schema: 3,
            catalog_version: 4,
            engineering: Some(EngineeringAssessment::for_policy(
                &EngineeringPolicy {
                    version: "2".into(),
                    minimumcd: None,
                    review_reference: "synthetic projection fixture, not consent".into(),
                    maintenance_branches: vec![],
                },
                TestStage::Delivery,
                &[selection(mode)],
            )?),
            subject: "synthetic-all-green".into(),
            evaluated_at: 1,
            rules: catalog
                .rules
                .into_iter()
                .map(|r| RuleResult {
                    rule_id: r.id,
                    catalog_version: 4,
                    outcome: Outcome::Passed,
                    subject: "synthetic-all-green".into(),
                    verifier: VerificationSource::Agent,
                    evaluated_at: 1,
                    evidence: vec![],
                    diagnostic: None,
                })
                .collect(),
            checks: vec![],
            gates: vec![],
        })
    }

    fn result(report: &CheckReport) -> Result<&StandardAssessment, &'static str> {
        report
            .engineering
            .as_ref()
            .and_then(|p| p.standards.first())
            .ok_or("standard")
    }

    #[test]
    fn guidance_assessment_and_requirement_have_different_effects() -> TestResult {
        for mode in [
            StandardMode::Guidance,
            StandardMode::Assess,
            StandardMode::Require,
        ] {
            let mut report = report(mode)?;
            report
                .rules
                .iter_mut()
                .find(|r| r.rule_id.as_str() == "MCD-RECOVERY-002")
                .ok_or("rollback")?
                .outcome = Outcome::Failed;
            reaggregate(&mut report)?;
            assert!(report.gate_passed(Gate::Integration));
            assert_eq!(
                report.gate_passed(Gate::Delivery),
                mode != StandardMode::Require
            );
            let standard = result(&report)?;
            if mode == StandardMode::Guidance {
                assert!(standard.assessment.is_none());
                assert!(report.checks.is_empty());
            } else {
                let assessment = standard.assessment.as_ref().ok_or("assessment")?;
                assert_eq!(assessment.verdict, AggregateVerdict::Blocked);
                assert!(
                    assessment
                        .requirements
                        .iter()
                        .any(|r| r.id == "cd-rollback" && r.outcome == Outcome::Failed)
                );
                assert_eq!(
                    report.checks.len(),
                    usize::from(mode == StandardMode::Require)
                );
            }
            let before = serde_json::to_value(&report)?;
            reaggregate(&mut report)?;
            assert_eq!(
                serde_json::to_value(&report)?,
                before,
                "reprojection must not duplicate or self-block"
            );
            let schema: serde_json::Value =
                serde_json::from_str(include_str!("../../../schema/report.schema.json"))?;
            assert!(jsonschema::is_valid(&schema, &before));
        }
        Ok(())
    }

    #[test]
    fn required_mapping_is_scoped_and_does_not_waive_baseline() -> TestResult {
        let mut report = report(StandardMode::Require)?;
        reaggregate(&mut report)?;
        assert!(report.gate_passed(Gate::Delivery));
        assert_eq!(report.checks[0].outcome, Outcome::Passed);
        report
            .rules
            .iter_mut()
            .find(|r| r.rule_id.as_str() == "MCD-RECOVERY-002")
            .ok_or("rollback")?
            .outcome = Outcome::Unverified;
        reaggregate(&mut report)?;
        assert!(!report.gate_passed(Gate::Delivery));
        report.engineering.as_mut().ok_or("policy")?.stage = Some(TestStage::PreMerge);
        reaggregate(&mut report)?;
        assert!(result(&report)?.assessment.is_none());
        assert!(report.checks.is_empty());
        assert!(report.gate_passed(Gate::Integration));
        report
            .rules
            .iter_mut()
            .find(|r| r.rule_id.as_str() == "OPDEV-FLOW-001")
            .ok_or("flow")?
            .outcome = Outcome::Failed;
        reaggregate(&mut report)?;
        assert!(!report.gate_passed(Gate::Integration));
        Ok(())
    }

    #[test]
    fn partial_mappings_and_changed_definitions_cannot_establish_conformance() -> TestResult {
        for (name, version) in [
            ("nist-ssdf-derived", "1.1"),
            ("slsa-build-provenance", "1.2"),
        ] {
            let mut report = report(StandardMode::Require)?;
            let mut selected = selection(StandardMode::Require);
            selected.name = name.into();
            selected.version = version.into();
            report.engineering.as_mut().ok_or("policy")?.standards =
                vec![StandardAssessment::new(selected)?];
            reaggregate(&mut report)?;
            assert!(!result(&report)?.complete_mapping);
            assert_eq!(
                result(&report)?
                    .assessment
                    .as_ref()
                    .ok_or("assessment")?
                    .verdict,
                AggregateVerdict::Blocked
            );
            assert_eq!(report.checks[0].outcome, Outcome::Unverified);
            assert!(!report.gate_passed(Gate::Delivery));
            report.engineering.as_mut().ok_or("policy")?.standards[0].definition_sha256 =
                "0".repeat(64);
            assert!(reaggregate(&mut report).is_err());
        }
        Ok(())
    }

    #[test]
    fn command_failure_survives_projection_and_cannot_be_shadowed_by_a_name() -> TestResult {
        for outcome in [
            Outcome::Failed,
            Outcome::Error,
            Outcome::Unverified,
            Outcome::MigrationRequired,
        ] {
            let mut report = report(StandardMode::Require)?;
            report.checks.push(CheckResult {
                id: "opdev-standard:minimumcd@1".into(),
                kind: CheckKind::Suite,
                blocking: true,
                gates: vec![Gate::Delivery],
                outcome,
                summary: "synthetic original command result".into(),
                evidence: vec![],
                stdout: None,
                stderr: None,
                duration_ms: None,
            });
            reaggregate(&mut report)?;
            assert!(!report.gate_passed(Gate::Delivery));
            assert_eq!(report.checks.len(), 2);
            assert_eq!(report.checks[0].kind, CheckKind::Suite);
            assert_eq!(report.checks[0].outcome, outcome);
            assert_eq!(report.checks[1].outcome, outcome);
            let schema: serde_json::Value =
                serde_json::from_str(include_str!("../../../schema/report.schema.json"))?;
            assert!(jsonschema::is_valid(
                &schema,
                &serde_json::to_value(&report)?
            ));
            reaggregate(&mut report)?;
            assert_eq!(report.checks.len(), 2);
            assert_eq!(report.checks[0].outcome, outcome);
        }
        Ok(())
    }

    #[test]
    fn conflicts_and_forged_metadata_never_survive_reprojection() -> TestResult {
        let mut report = report(StandardMode::Assess)?;
        reaggregate(&mut report)?;
        let original = report.engineering.clone();
        let standard = &mut report.engineering.as_mut().ok_or("policy")?.standards[0];
        standard.source.version = "invented upstream".into();
        standard.claim = "certified".into();
        standard.complete_mapping = false;
        reaggregate(&mut report)?;
        assert_eq!(report.engineering, original);
        let duplicate = result(&report)?.clone();
        report
            .engineering
            .as_mut()
            .ok_or("policy")?
            .standards
            .push(duplicate);
        assert!(reaggregate(&mut report).is_err());
        report.engineering = original;
        report.engineering.as_mut().ok_or("policy")?.minimumcd = Some(FrameworkAssessment {
            version: "1".into(),
            source_version: String::new(),
            verdict: AggregateVerdict::Blocked,
            requirements: vec![],
            blocking_checks: vec![],
        });
        assert!(reaggregate(&mut report).is_err());
        Ok(())
    }
}
