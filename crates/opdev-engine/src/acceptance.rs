//! Mechanical acceptance evidence checks plus explicit reviewed semantic claims.

use std::path::Path;

use opdev_core::Outcome;
use opdev_project::{
    AcceptanceMethod, AcceptanceScope, EvidenceError, EvidenceLedger, ProjectManifest, TestStage,
};

use crate::report::{CheckKind, CheckResult};

fn incomplete(message: &str) -> (Outcome, AcceptanceScope, String) {
    (
        Outcome::Unverified,
        AcceptanceScope::Behavioral,
        message.to_owned(),
    )
}

pub(crate) fn evaluate(
    root: &Path,
    manifest: &ProjectManifest,
    checks: &[CheckResult],
    ledger: Option<&EvidenceLedger>,
    fingerprint: Option<&str>,
    fresh: bool,
    stage: TestStage,
) -> (Outcome, AcceptanceScope, String) {
    if !fresh {
        return incomplete(
            "Acceptance evidence needs one unchanged staged source and ledger before and after checks: the files selected for commit or .opdev/evidence.yaml changed while checking. Stage the intended files, review the affected test links, and check again once those contents are stable",
        );
    }
    let Some(change) = ledger
        .filter(|ledger| ledger.schema == 2)
        .and_then(|ledger| fingerprint.and_then(|fingerprint| ledger.matching_change(fingerprint)))
    else {
        return incomplete(
            "Current schema-2 change acceptance evidence is missing: OpDev has no reviewed record linking this version of the change's expected results to its tests. Review or prepare those links in .opdev/evidence.yaml using record format 2. Older generic assertions do not establish test adequacy; this is missing review evidence, not a failing software test",
        );
    };
    let Some(acceptance) = &change.acceptance else {
        return incomplete(
            "The current change needs a reviewed acceptance inventory and assertion mappings: list its expected results and identify the test assertions or observations that demonstrate each one in .opdev/evidence.yaml",
        );
    };
    let review = &acceptance.review;
    if !review_is_current(acceptance, change) {
        return incomplete(
            "Acceptance review is pending, incomplete or bound to different contents: the recorded review does not confirm the current expected results and test links. Review those contents, then record who reviewed them, the decision reference and the current review identifier in .opdev/evidence.yaml",
        );
    }
    if review.outcome == Outcome::Failed {
        return (Outcome::Failed, acceptance.scope, "The current acceptance review records a contradiction; a green suite cannot override it".into());
    }
    if let Some(diagnostic) = safeguard_gap(manifest, acceptance) {
        return incomplete(&diagnostic);
    }
    if acceptance.scope == AcceptanceScope::NoMaterialConditions {
        return if acceptance.conditions.is_empty() && acceptance.verifications.is_empty() {
            (Outcome::NotApplicable, acceptance.scope, "Reviewed exact-change rationale identifies no material acceptance conditions; this is not inferred from filenames".into())
        } else {
            incomplete("No-material-conditions scope contradicts a nonempty acceptance inventory")
        };
    }
    if acceptance.conditions.is_empty() {
        return incomplete("A material change needs a nonempty reviewed acceptance inventory");
    }
    let mut missing = false;
    let mut failed = false;
    let mut error = false;
    let mut diagnostics = Vec::new();
    for condition in &acceptance.conditions {
        if let Some(message) = missing_mapping(acceptance, &condition.id, stage) {
            missing = true;
            diagnostics.push(message);
        }
        let status = reference_outcome(&condition.source.verify(root));
        missing |= status == Outcome::Unverified;
        error |= status == Outcome::Error;
    }
    for mapping in &acceptance.verifications {
        let status = reference_outcome(&mapping.target.verify(root));
        missing |= status == Outcome::Unverified || mapping.outcome == Outcome::Unverified;
        error |= status == Outcome::Error;
        failed |= mapping.outcome == Outcome::Failed;
        if mapping.method == AcceptanceMethod::Automated && mapping.applies_to(stage) {
            let declared = manifest
                .testing
                .suites
                .iter()
                .find(|suite| Some(&suite.id) == mapping.suite.as_ref());
            let run = checks.iter().find(|check| {
                check.kind == CheckKind::Suite && Some(&check.id) == mapping.suite.as_ref()
            });
            match run.filter(|_| declared.is_some()) {
                Some(check) if check.outcome == Outcome::Passed => (),
                Some(check) if check.outcome == Outcome::Failed => failed = true,
                Some(check) if check.outcome == Outcome::Error => error = true,
                _ => {
                    missing = true;
                    let stage_name = serde_json::to_value(stage).unwrap_or_default();
                    diagnostics.push(format!(
                        "Condition `{}` maps to suite `{}`: {}; selected stage {stage_name}",
                        mapping.condition,
                        mapping.suite.as_deref().unwrap_or("<missing>"),
                        match declared {
                            None => "suite is not declared",
                            Some(suite) if !suite.stages.contains(&stage) =>
                                "suite is not assigned to this stage",
                            Some(_) => "suite did not execute in this evaluation",
                        }
                    ));
                }
            }
        }
    }
    let (outcome, reason) = result(failed, error, missing);
    let reason = if diagnostics.is_empty() {
        reason.into()
    } else {
        format!("{reason}; {}", diagnostics.join("; "))
    };
    (outcome, acceptance.scope, reason)
}

fn safeguard_gap(
    manifest: &ProjectManifest,
    acceptance: &opdev_project::AcceptanceEvidence,
) -> Option<String> {
    use opdev_project::{Capability, CapabilityState, Impact};
    let Some(policy) = &manifest.assurance.safeguards else {
        return acceptance.safeguards.as_ref().map(|_| "This change supplies safeguard mappings without a reviewed project safeguard policy. Select policy explicitly; the mappings did not qualify this change".into());
    };
    let Some(review) = &acceptance.safeguards else {
        return Some("Capability safeguards are not reviewed for this change. Review which present capabilities are affected and link their required safeguards to the existing acceptance conditions".into());
    };
    if review.version != policy.version {
        return Some("Unsupported safeguard review version; use a reader and reviewed policy supporting the same version".into());
    }
    let mut required = std::collections::BTreeSet::new();
    for capability in Capability::ALL {
        let name = serde_json::to_value(capability).unwrap_or_default();
        let Some(fact) = policy.capabilities.get(&capability) else {
            return Some(format!(
                "Capability {name} has no reviewed applicability evidence. Assess actual behavior; missing configuration does not mean it is absent"
            ));
        };
        match fact.state {
            CapabilityState::Unknown => {
                return Some(format!(
                    "Capability {name} is unresolved. Review its actual presence or absence before claiming this change is verified"
                ));
            }
            CapabilityState::Absent => {
                if review.impacts.contains_key(&capability) {
                    return Some(format!(
                        "Capability {name} is declared absent but has change-impact evidence. Reconcile the project facts; do not use absence to waive safeguards"
                    ));
                }
            }
            CapabilityState::Present => {
                let Some(impact) = review
                    .impacts
                    .get(&capability)
                    .filter(|i| !i.rationale.trim().is_empty())
                else {
                    return Some(format!(
                        "Present capability {name} needs a reviewed impact explanation for this change. It cannot be omitted or marked not applicable"
                    ));
                };
                if impact.impact == Impact::Affected {
                    required.extend(capability.objectives());
                }
            }
        }
    }
    if review.objectives.keys().any(|o| !required.contains(o)) {
        return Some("Safeguard objectives conflict with the reviewed capability impacts. Reconcile affected scope instead of treating unrelated checks as evidence".into());
    }
    for objective in required {
        let ids = review.objectives.get(objective);
        if ids.is_none_or(|ids| {
            ids.is_empty()
                || ids
                    .iter()
                    .any(|id| !acceptance.conditions.iter().any(|c| &c.id == id))
        }) {
            let name = serde_json::to_value(objective).unwrap_or_default();
            return Some(format!(
                "Affected behavior needs safeguard {name}. Link it to a concrete acceptance condition and its reviewed test or observation; a general passed assertion is not enough"
            ));
        }
    }
    None
}

fn review_is_current(
    acceptance: &opdev_project::AcceptanceEvidence,
    change: &opdev_project::ChangeEvidence,
) -> bool {
    let review = &acceptance.review;
    review.outcome != Outcome::Unverified
        && [
            &review.reviewer,
            &review.reference,
            &review.rationale,
            &acceptance.rationale,
            &change.work,
        ]
        .iter()
        .all(|value| !value.trim().is_empty())
        && acceptance
            .digest(&change.fingerprint, &change.work)
            .is_ok_and(|digest| digest == review.subject_sha256)
}

fn missing_mapping(
    acceptance: &opdev_project::AcceptanceEvidence,
    condition: &str,
    stage: TestStage,
) -> Option<String> {
    let count = acceptance
        .verifications
        .iter()
        .filter(|mapping| mapping.condition == condition && mapping.applies_to(stage))
        .count();
    (count != 1).then(|| {
        let stage_name = serde_json::to_value(stage).unwrap_or_default();
        format!("Condition `{condition}` needs exactly one reviewed test or observation for stage {stage_name}. Evidence for another stage does not verify this one")
    })
}

fn reference_outcome(result: &Result<(), EvidenceError>) -> Outcome {
    match result {
        Ok(()) => Outcome::Passed,
        Err(EvidenceError::Git(_) | EvidenceError::Read { .. }) => Outcome::Error,
        Err(_) => Outcome::Unverified,
    }
}

fn result(failed: bool, error: bool, missing: bool) -> (Outcome, &'static str) {
    if failed {
        (
            Outcome::Failed,
            "A current mapping contradicts acceptance or a required canonical suite failed",
        )
    } else if error {
        (
            Outcome::Error,
            "Git source verification or a mapped canonical suite could not complete; inspect the index and command diagnostics. This is not a product failure",
        )
    } else if missing {
        (
            Outcome::Unverified,
            "Acceptance mappings are incomplete, pending, stale, or lack current-stage canonical suite execution: some expected results are not yet supported by reviewed tests for these file contents and this workflow stage. Inspect the condition-specific findings, update only the affected review links, and run any missing required suite",
        )
    } else {
        (
            Outcome::Passed,
            "Exact-change sources and all inventoried mappings checked; automated suites mapped to this stage passed in this check. Other stages and review-only mappings do not imply execution here. Semantic adequacy and inventory completeness remain identified reviewer claims, not machine proof",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_failures_are_not_missing_evidence_or_product_failures() {
        assert_eq!(
            reference_outcome(&Err(EvidenceError::Git("tool unavailable".into()))),
            Outcome::Error
        );
        assert_eq!(
            reference_outcome(&Err(EvidenceError::Semantic("stale excerpt".into()))),
            Outcome::Unverified
        );
        assert_eq!(result(false, true, true).0, Outcome::Error);
        assert_eq!(result(true, true, true).0, Outcome::Failed);
    }
}
