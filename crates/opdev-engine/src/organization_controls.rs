//! Additive controls consume this invocation's accepted mappings and execution.
use std::path::Path;

use opdev_core::Outcome;
use opdev_project::organization::{ApplicabilityResult, PolicySnapshot, VerificationRequirement};
use opdev_project::{EvidenceLedger, ProjectManifest, TestStage};

use crate::{CheckKind, CheckResult};

fn extension_stage(stage: TestStage) -> opdev_project::ExtensionStage {
    use opdev_project::ExtensionStage as E;
    match stage {
        TestStage::Local | TestStage::Scheduled => E::Verify,
        TestStage::PreMerge => E::PreMerge,
        TestStage::PostMerge => E::PostMerge,
        TestStage::Package => E::Package,
        TestStage::Delivery => E::Deliver,
        TestStage::Recovery => E::Recover,
        TestStage::Evaluation => E::Evaluate,
    }
}

#[allow(clippy::too_many_arguments)] // Existing source, stage and execution boundary.
pub(crate) fn qualify(
    root: &Path,
    manifest: &ProjectManifest,
    snapshot: &PolicySnapshot,
    checks: &[CheckResult],
    ledger: Option<&EvidenceLedger>,
    fingerprint: Option<&str>,
    acceptance_outcome: Outcome,
    stage: TestStage,
) -> Vec<CheckResult> {
    let current =
        opdev_project::organization::load_selected(root, &manifest.assurance.organization_policies)
            .is_ok_and(|after| after == *snapshot);
    let acceptance = ledger
        .and_then(|l| fingerprint.and_then(|f| l.matching_change(f)))
        .and_then(|c| c.acceptance.as_ref());
    let review = acceptance.and_then(|a| a.organization_controls.as_ref());
    let catalog = manifest
        .assurance
        .requirements
        .as_ref()
        .and_then(|_| opdev_project::requirements::load_index(root).ok());
    let all_ids: std::collections::HashSet<_> = snapshot
        .policies
        .iter()
        .flat_map(|p| &p.definition.controls)
        .map(|c| c.id.as_str())
        .collect();
    let valid_review = review.is_some_and(|r| {
        r.stage == stage
            && r.resolution_sha256 == snapshot.resolution_sha256
            && r.bindings
                .iter()
                .all(|b| all_ids.contains(b.control.as_str()))
    });
    snapshot.policies.iter().flat_map(|p| p.definition.controls.iter().filter(|c| c.stages.contains(&stage)).map(|control| {
        let binding = review.and_then(|r| r.bindings.iter().find(|b| b.control == control.id));
        let (outcome, reason) = if !current {
            (Outcome::Unverified, "Policy source changed during verification; inspect the current definitions and selections.")
        } else if matches!(acceptance_outcome, Outcome::Failed | Outcome::Error) {
            (acceptance_outcome, "Current acceptance failed or could not be verified; organization policy does not override it.")
        } else if acceptance_outcome != Outcome::Passed || !valid_review || binding.is_none() {
            (Outcome::Unverified, "Review these exact policy definitions, values and control links at this stage; a content pin is not verification.")
        } else {
            match control.applicability.evaluate(manifest.assurance.safeguards.as_ref(), &p.selection.parameters) {
                ApplicabilityResult::Unknown => (Outcome::Unverified, "Applicability is unresolved; missing capability facts do not waive this control."),
                ApplicabilityResult::NotApplicable => (Outcome::NotApplicable, "Current reviewed capability facts and selected values exclude this additional control; the core baseline is unchanged."),
                ApplicabilityResult::Applicable => {
                    let linked = binding.filter(|b| !b.conditions.is_empty());
                    if let (Some(binding), Some(acceptance)) = (linked, acceptance) {
                        let extension_failure = binding.extensions.iter().filter_map(|id| checks.iter().find(|c| c.kind == CheckKind::Extension && c.id == *id))
                            .map(|c| c.outcome).find(|o| matches!(o, Outcome::Error | Outcome::Failed | Outcome::MigrationRequired));
                        let extensions_pass = binding.extensions.iter().all(|id| manifest.extensions.checks.iter().any(|e| e.id == *id && e.stage == extension_stage(stage)) && checks.iter().any(|c| c.kind == CheckKind::Extension && c.id == *id && c.outcome == Outcome::Passed));
                        let conditions_pass = binding.conditions.iter().all(|id| crate::policy_controls::verified_condition(acceptance, catalog.as_ref(), checks, id, stage));
                        let automated = !binding.extensions.is_empty() || binding.conditions.iter().any(|id| crate::policy_controls::automated(acceptance, catalog.as_ref(), id, stage));
                        if let Some(failure) = extension_failure {
                            (failure, "A linked extension failed or could not complete; a mapping review cannot replace its result.")
                        } else if !extensions_pass || !conditions_pass || (control.verification == VerificationRequirement::Automated && !automated) {
                            (Outcome::Unverified, "Linked conditions need current-stage verification, and every linked extension must pass in this invocation. Review-only judgments do not count as observations.")
                        } else {
                            (Outcome::Passed, "Current accepted conditions and this invocation's checks verify this additional control; no duplicate command execution or certification claim.")
                        }
                    } else {
                        (Outcome::Unverified, "Link the control to existing accepted conditions or durable criteria; an empty list cannot verify an applicable obligation.")
                    }
                }
            }
        };
        CheckResult {
            id: format!("opdev-organization:{}", control.id),
            kind: CheckKind::Policy,
            blocking: true,
            gates: crate::evaluator::gates_for_test_stage(stage),
            outcome,
            summary: format!("{}@{} [{}]: {reason}", p.selection.id, p.selection.version, p.resolution_sha256),
            evidence: vec![], stdout: None, stderr: None, duration_ms: None,
        }
    })).collect()
}
