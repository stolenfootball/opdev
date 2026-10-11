//! Policy-2 duties consume current acceptance and existing execution, never declarations.
use std::path::Path;

use opdev_core::{Outcome, RuleResult};
use opdev_project::{
    AcceptanceEvidence, AcceptanceMethod, Capability, CapabilityState, EvidenceLedger,
    ProjectManifest, TestStage,
    requirements::{CatalogSnapshot, Method},
};

use crate::CheckResult;

const CONTROLS: [&str; 4] = [
    "OPDEV-FLOW-001",
    "OPDEV-DELIVERY-001",
    "OPDEV-PIPELINE-001",
    "OPDEV-RECOVERY-001",
];

#[allow(clippy::too_many_arguments)] // The already-evaluated source/stage, not another execution.
pub(crate) fn qualify(
    root: &Path,
    manifest: &ProjectManifest,
    rules: &mut [RuleResult],
    checks: &[CheckResult],
    ledger: Option<&EvidenceLedger>,
    fingerprint: Option<&str>,
    acceptance_outcome: Outcome,
    stage: TestStage,
) {
    if manifest
        .assurance
        .engineering
        .as_ref()
        .is_none_or(|p| p.version != "2")
    {
        return;
    }
    let change = ledger.and_then(|l| fingerprint.and_then(|f| l.matching_change(f)));
    let acceptance = change.and_then(|c| c.acceptance.as_ref());
    let catalog = manifest
        .assurance
        .requirements
        .as_ref()
        .and_then(|_| opdev_project::requirements::load_index(root).ok());
    for rule in rules
        .iter_mut()
        .filter(|r| CONTROLS.contains(&r.rule_id.as_str()))
    {
        // Preserve actual negative evidence; missing review cannot turn a failure green.
        if matches!(
            rule.outcome,
            Outcome::Failed | Outcome::Error | Outcome::MigrationRequired
        ) {
            continue;
        }
        let assertion =
            change.and_then(|c| c.assertions.iter().find(|a| a.rule_id == rule.rule_id));
        let gap = if acceptance_outcome != Outcome::Passed || acceptance.is_none() {
            Some(
                "Current acceptance and capability review must pass; a project-level assertion or declared strategy is not verification.",
            )
        } else if assertion.is_none() {
            Some(
                "Review this control for the exact current change; historical project assertions do not qualify it.",
            )
        } else if rule.outcome == Outcome::NotApplicable {
            if rule.rule_id.as_str() != "OPDEV-FLOW-001" && delivery_absent(manifest) {
                None
            } else {
                Some(
                    "Known or unresolved capabilities do not support not-applicable. Review the actual delivery/service scope; a green pipeline is not absence of the CI obligation.",
                )
            }
        } else {
            acceptance.and_then(|a| {
                mapping_gap(a, catalog.as_ref(), checks, rule.rule_id.as_str(), stage)
            })
        };
        if let Some(gap) = gap {
            rule.outcome = Outcome::Unverified;
            rule.diagnostic = Some(gap.into());
        }
    }
}

pub(crate) fn delivery_absent(manifest: &ProjectManifest) -> bool {
    manifest.assurance.safeguards.as_ref().is_some_and(|p| {
        [Capability::Distribution, Capability::Operations]
            .iter()
            .all(|c| {
                p.capabilities
                    .get(c)
                    .is_some_and(|f| f.state == CapabilityState::Absent)
            })
    })
}

fn mapping_gap(
    acceptance: &AcceptanceEvidence,
    catalog: Option<&CatalogSnapshot>,
    checks: &[CheckResult],
    control: &str,
    stage: TestStage,
) -> Option<&'static str> {
    let Some(review) = &acceptance.policy_controls else {
        return Some(
            "Link this policy control to existing acceptance conditions or durable criteria. A general passed assertion does not establish current verification.",
        );
    };
    if review.version != "2"
        || review.stage != stage
        || !opdev_core::resolve_engineering_policy("2")
            .is_ok_and(|d| d.definition_sha256 == review.definition_sha256)
    {
        return Some(
            "Policy definition or stage changed; review the current control links before qualification.",
        );
    }
    let Some(ids) = review.bindings.get(control).filter(|ids| !ids.is_empty()) else {
        return Some("This control has no reviewed verification links for the selected boundary.");
    };
    if ids
        .iter()
        .any(|id| !verified_condition(acceptance, catalog, checks, id, stage))
    {
        return Some(
            "A linked condition lacks current-stage execution or a current durable observation. Mapping review alone is not a test result.",
        );
    }
    if control == "OPDEV-RECOVERY-001"
        && !ids
            .iter()
            .any(|id| automated(acceptance, catalog, id, stage))
    {
        return Some(
            "Recovery needs an automated exercised path at this boundary, not only a strategy description or manual mapping review.",
        );
    }
    None
}

pub(crate) fn verified_condition(
    acceptance: &AcceptanceEvidence,
    catalog: Option<&CatalogSnapshot>,
    checks: &[CheckResult],
    id: &str,
    stage: TestStage,
) -> bool {
    if acceptance.conditions.iter().any(|c| c.id == id) {
        // Older review-only mappings are adequacy judgments, not fresh observations.
        return acceptance.verifications.iter().any(|v| {
            v.condition == id && v.applies_to(stage) && v.method == AcceptanceMethod::Automated
        });
    }
    acceptance.requirements.is_some()
        && catalog.is_some_and(|c| {
            c.plans()
                .any(|p| p.criterion == id && p.stage == stage && !p.members.is_empty())
                && checks.iter().any(|r| r.id.starts_with("requirements:"))
                && checks
                    .iter()
                    .filter(|r| r.id.starts_with("requirements:"))
                    .all(|r| r.outcome == Outcome::Passed)
        })
}

pub(crate) fn automated(
    acceptance: &AcceptanceEvidence,
    catalog: Option<&CatalogSnapshot>,
    id: &str,
    stage: TestStage,
) -> bool {
    acceptance.verifications.iter().any(|v| {
        v.condition == id && v.applies_to(stage) && v.method == AcceptanceMethod::Automated
    }) || catalog.is_some_and(|c| {
        c.plans()
            .filter(|p| p.criterion == id && p.stage == stage)
            .flat_map(|p| &p.members)
            .any(|m| {
                c.verifications()
                    .any(|v| v.id == m.verification && matches!(v.method, Method::Automated { .. }))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use opdev_project::{
        AcceptanceCondition, AcceptanceVerification, PolicyControlReview, RequirementSource,
        TrackedEvidence,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    struct Fixture {
        temp: tempfile::TempDir,
        project: ProjectManifest,
        ledger: EvidenceLedger,
        rule: RuleResult,
    }

    fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
        use opdev_core::{EngineeringPolicy, VerificationSource};
        use opdev_project::{CapabilityFact, ChangeEvidence, EvidenceAssertion, SafeguardPolicy};
        let temp = tempfile::tempdir()?;
        assert!(
            std::process::Command::new("git")
                .args(["init", "-q"])
                .arg(temp.path())
                .status()?
                .success()
        );
        let mut project = opdev_project::discover(temp.path())?.manifest;
        project.schema = 4;
        project.assurance.profiles.clear();
        project.assurance.engineering = Some(EngineeringPolicy {
            version: "2".into(),
            minimumcd: None,
            review_reference: "fixture".into(),
            maintenance_branches: vec![],
        });
        project.assurance.safeguards = Some(SafeguardPolicy {
            version: 1,
            review_reference: "fixture".into(),
            capabilities: Capability::ALL
                .into_iter()
                .map(|c| {
                    (
                        c,
                        CapabilityFact {
                            state: CapabilityState::Present,
                            rationale: "fixture".into(),
                            authority: "contracts".into(),
                        },
                    )
                })
                .collect(),
        });
        let id = "OPDEV-RECOVERY-001".parse()?;
        let assertion = EvidenceAssertion {
            rule_id: id,
            outcome: Outcome::Passed,
            summary: "fixture semantic review".into(),
            evidence: vec![],
        };
        let ledger = EvidenceLedger {
            schema: 2,
            project: vec![assertion.clone()],
            changes: vec![ChangeEvidence {
                fingerprint: "a".repeat(64),
                work: "fixture".into(),
                assertions: vec![assertion.clone()],
                acceptance: Some(acceptance()?),
            }],
        };
        let rule = RuleResult {
            rule_id: assertion.rule_id,
            catalog_version: 4,
            outcome: Outcome::Passed,
            subject: "fixture".into(),
            verifier: VerificationSource::Evidence,
            evaluated_at: 1,
            evidence: vec![],
            diagnostic: None,
        };
        Ok(Fixture {
            temp,
            project,
            ledger,
            rule,
        })
    }

    #[test]
    fn missing_execution_stale_review_and_false_absence_do_not_qualify() -> TestResult {
        let Fixture {
            temp,
            project,
            mut ledger,
            rule,
        } = fixture()?;
        for outcome in [
            Outcome::Unverified,
            Outcome::Failed,
            Outcome::Error,
            Outcome::NotApplicable,
        ] {
            let mut rules = vec![rule.clone()];
            qualify(
                temp.path(),
                &project,
                &mut rules,
                &[],
                Some(&ledger),
                Some(&"a".repeat(64)),
                outcome,
                TestStage::Recovery,
            );
            assert_eq!(
                rules[0].outcome,
                Outcome::Unverified,
                "a claimed control pass cannot replace execution/review"
            );
        }
        let mut rules = vec![rule.clone()];
        qualify(
            temp.path(),
            &project,
            &mut rules,
            &[],
            Some(&ledger),
            Some(&"b".repeat(64)),
            Outcome::Passed,
            TestStage::Recovery,
        );
        assert_eq!(
            rules[0].outcome,
            Outcome::Unverified,
            "project assertions cannot mask a changed source"
        );
        rules[0] = rule.clone();
        rules[0].outcome = Outcome::NotApplicable;
        qualify(
            temp.path(),
            &project,
            &mut rules,
            &[],
            Some(&ledger),
            Some(&"a".repeat(64)),
            Outcome::Passed,
            TestStage::Recovery,
        );
        assert_eq!(
            rules[0].outcome,
            Outcome::Unverified,
            "present delivery cannot be marked absent"
        );
        ledger.changes[0].assertions.clear();
        rules[0] = rule.clone();
        qualify(
            temp.path(),
            &project,
            &mut rules,
            &[],
            Some(&ledger),
            Some(&"a".repeat(64)),
            Outcome::Passed,
            TestStage::Recovery,
        );
        assert_eq!(
            rules[0].outcome,
            Outcome::Unverified,
            "historical project assertion is not current review"
        );
        for outcome in [Outcome::Failed, Outcome::Error, Outcome::MigrationRequired] {
            rules[0] = rule.clone();
            rules[0].outcome = outcome;
            qualify(
                temp.path(),
                &project,
                &mut rules,
                &[],
                None,
                None,
                Outcome::Passed,
                TestStage::Recovery,
            );
            assert_eq!(
                rules[0].outcome, outcome,
                "known blockers survive missing review"
            );
        }
        Ok(())
    }

    fn acceptance() -> Result<AcceptanceEvidence, Box<dyn std::error::Error>> {
        let target = TrackedEvidence {
            path: "recovery.rs".into(),
            sha256: "a".repeat(64),
            excerpt: "assert recovered".into(),
        };
        Ok(AcceptanceEvidence {
            policy_controls: Some(PolicyControlReview {
                version: "2".into(),
                definition_sha256: opdev_core::resolve_engineering_policy("2")?.definition_sha256,
                stage: TestStage::Recovery,
                bindings: [("OPDEV-RECOVERY-001".into(), vec!["restore".into()])].into(),
            }),
            conditions: vec![AcceptanceCondition {
                id: "restore".into(),
                statement: "Restore safe writes after interrupted transition".into(),
                authority: "contracts".into(),
                source: RequirementSource::Tracked(target.clone()),
            }],
            verifications: vec![AcceptanceVerification {
                condition: "restore".into(),
                stages: Some(vec![TestStage::Recovery]),
                method: AcceptanceMethod::Automated,
                target,
                assertion: "Preserves committed writes".into(),
                discriminating_case: "Interrupted transition must not lose an acknowledged write"
                    .into(),
                suite: Some("recover".into()),
                automation_limitation: None,
                outcome: Outcome::Passed,
            }],
            ..Default::default()
        })
    }

    #[test]
    fn control_links_need_exact_policy_stage_and_current_verification() -> TestResult {
        let mut a = acceptance()?;
        assert!(
            mapping_gap(&a, None, &[], "OPDEV-RECOVERY-001", TestStage::Recovery).is_none(),
            "this helper consumes an already successful acceptance/execution evaluation"
        );
        assert!(mapping_gap(&a, None, &[], "OPDEV-RECOVERY-001", TestStage::PreMerge).is_some());
        a.policy_controls
            .as_mut()
            .ok_or("review")?
            .definition_sha256 = "b".repeat(64);
        assert!(mapping_gap(&a, None, &[], "OPDEV-RECOVERY-001", TestStage::Recovery).is_some());
        a = acceptance()?;
        a.policy_controls
            .as_mut()
            .ok_or("review")?
            .bindings
            .get_mut("OPDEV-RECOVERY-001")
            .ok_or("binding")?[0] = "missing".into();
        assert!(mapping_gap(&a, None, &[], "OPDEV-RECOVERY-001", TestStage::Recovery).is_some());
        a = acceptance()?;
        a.verifications[0].method = AcceptanceMethod::Review;
        assert!(
            mapping_gap(&a, None, &[], "OPDEV-RECOVERY-001", TestStage::Recovery).is_some(),
            "a mapping review is not an executed recovery"
        );
        a.policy_controls = None;
        assert!(mapping_gap(&a, None, &[], "OPDEV-RECOVERY-001", TestStage::Recovery).is_some());
        Ok(())
    }

    #[test]
    fn new_links_are_digest_material_but_omission_preserves_legacy() -> TestResult {
        let mut a = acceptance()?;
        let bound = a.digest("fingerprint", "work")?;
        a.policy_controls.as_mut().ok_or("review")?.stage = TestStage::Delivery;
        assert_ne!(bound, a.digest("fingerprint", "work")?);
        a.policy_controls = None;
        let legacy = a.digest("fingerprint", "work")?;
        let encoded = serde_json::to_value(&a)?;
        assert!(encoded.get("policy_controls").is_none());
        let roundtrip: AcceptanceEvidence = serde_json::from_value(encoded)?;
        assert_eq!(legacy, roundtrip.digest("fingerprint", "work")?);
        assert_ne!(bound, legacy);
        Ok(())
    }

    fn organization_fixture()
    -> Result<(Fixture, opdev_project::organization::PolicySnapshot), Box<dyn std::error::Error>>
    {
        use opdev_project::organization::{
            Applicability, OrganizationControl, PolicyPack, PolicySelection,
            VerificationRequirement,
        };
        let mut f = fixture()?;
        let pack = PolicyPack {
            schema: 1,
            id: "example".into(),
            version: "1".into(),
            title: "Example".into(),
            source: "fixture authority".into(),
            parameters: std::collections::BTreeMap::new(),
            controls: vec![OrganizationControl {
                id: "ORG-EXAMPLE-001".into(),
                statement: "Recover retained writes".into(),
                source: "fixture".into(),
                stages: vec![TestStage::Recovery],
                applicability: Applicability::Always,
                verification: VerificationRequirement::Automated,
            }],
        };
        f.project.assurance.organization_policies = vec![PolicySelection {
            id: pack.id.clone(),
            version: pack.version.clone(),
            definition_sha256: pack.definition_sha256()?,
            parameters: std::collections::BTreeMap::new(),
        }];
        std::fs::create_dir_all(f.temp.path().join(".opdev/policies"))?;
        std::fs::write(
            f.temp.path().join(".opdev/policies/example.json"),
            serde_json::to_vec(&pack)?,
        )?;
        assert!(
            std::process::Command::new("git")
                .current_dir(f.temp.path())
                .args(["add", "."])
                .status()?
                .success()
        );
        let snapshot = opdev_project::organization::load_selected(
            f.temp.path(),
            &f.project.assurance.organization_policies,
        )?;
        f.ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or("acceptance")?
            .organization_controls = Some(opdev_project::OrganizationControlReview {
            resolution_sha256: snapshot.resolution_sha256.clone(),
            stage: TestStage::Recovery,
            bindings: vec![opdev_project::OrganizationControlBinding {
                control: "ORG-EXAMPLE-001".into(),
                conditions: vec!["restore".into()],
                extensions: vec![],
            }],
        });
        Ok((f, snapshot))
    }

    fn organization_results(
        f: &Fixture,
        snapshot: &opdev_project::organization::PolicySnapshot,
        outcome: Outcome,
        stage: TestStage,
    ) -> Vec<CheckResult> {
        crate::organization_controls::qualify(
            f.temp.path(),
            &f.project,
            snapshot,
            &[],
            Some(&f.ledger),
            Some(&"a".repeat(64)),
            outcome,
            stage,
        )
    }

    #[test]
    fn organization_controls_require_current_accepted_verification_and_stage() -> TestResult {
        let (mut f, snapshot) = organization_fixture()?;
        // This pure projection consumes the acceptance evaluator's actual outcome;
        // it does not claim to execute the synthetic mapping below.
        assert_eq!(
            organization_results(&f, &snapshot, Outcome::Passed, TestStage::Recovery)[0].outcome,
            Outcome::Passed
        );
        assert!(
            organization_results(&f, &snapshot, Outcome::Passed, TestStage::PreMerge).is_empty()
        );
        for outcome in [Outcome::Failed, Outcome::Error, Outcome::Unverified] {
            assert_eq!(
                organization_results(&f, &snapshot, outcome, TestStage::Recovery)[0].outcome,
                outcome
            );
        }
        let original = f.ledger.clone();
        f.ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or("acceptance")?
            .organization_controls
            .as_mut()
            .ok_or("review")?
            .stage = TestStage::PreMerge;
        assert_eq!(
            organization_results(&f, &snapshot, Outcome::Passed, TestStage::Recovery)[0].outcome,
            Outcome::Unverified
        );
        f.ledger = original.clone();
        f.ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or("acceptance")?
            .organization_controls
            .as_mut()
            .ok_or("review")?
            .resolution_sha256 = "b".repeat(64);
        assert_eq!(
            organization_results(&f, &snapshot, Outcome::Passed, TestStage::Recovery)[0].outcome,
            Outcome::Unverified
        );
        f.ledger = original.clone();
        f.ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or("acceptance")?
            .verifications[0]
            .method = AcceptanceMethod::Review;
        assert_eq!(
            organization_results(&f, &snapshot, Outcome::Passed, TestStage::Recovery)[0].outcome,
            Outcome::Unverified
        );
        f.ledger = original;
        f.ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or("acceptance")?
            .organization_controls
            .as_mut()
            .ok_or("review")?
            .bindings[0]
            .extensions
            .push("missing-extension".into());
        assert_eq!(
            organization_results(&f, &snapshot, Outcome::Passed, TestStage::Recovery)[0].outcome,
            Outcome::Unverified
        );
        Ok(())
    }

    #[test]
    fn organization_source_edits_during_checks_invalidate_positive_projection() -> TestResult {
        let (f, snapshot) = organization_fixture()?;
        assert_eq!(
            organization_results(&f, &snapshot, Outcome::Passed, TestStage::Recovery)[0].outcome,
            Outcome::Passed
        );
        std::fs::write(f.temp.path().join(".opdev/policies/example.json"), "{}")?;
        assert_eq!(
            organization_results(&f, &snapshot, Outcome::Passed, TestStage::Recovery)[0].outcome,
            Outcome::Unverified
        );
        Ok(())
    }

    #[test]
    fn organization_extension_links_require_the_declared_current_stage_and_result() -> TestResult {
        let (mut f, snapshot) = organization_fixture()?;
        f.project
            .extensions
            .checks
            .push(opdev_project::ExtensionCheck {
                id: "extra".into(),
                stage: opdev_project::ExtensionStage::Recover,
                command: "already-executed".into(),
                blocking: false,
                authority: None,
                timeout_seconds: None,
            });
        f.ledger.changes[0]
            .acceptance
            .as_mut()
            .ok_or("acceptance")?
            .organization_controls
            .as_mut()
            .ok_or("review")?
            .bindings[0]
            .extensions
            .push("extra".into());
        for (stage, result, expected) in [
            (
                opdev_project::ExtensionStage::Recover,
                Outcome::Passed,
                Outcome::Passed,
            ),
            (
                opdev_project::ExtensionStage::Deliver,
                Outcome::Passed,
                Outcome::Unverified,
            ),
            (
                opdev_project::ExtensionStage::Recover,
                Outcome::Failed,
                Outcome::Failed,
            ),
            (
                opdev_project::ExtensionStage::Recover,
                Outcome::Error,
                Outcome::Error,
            ),
            (
                opdev_project::ExtensionStage::Recover,
                Outcome::NotApplicable,
                Outcome::Unverified,
            ),
        ] {
            f.project.extensions.checks[0].stage = stage;
            let checks = [CheckResult {
                id: "extra".into(),
                kind: crate::CheckKind::Extension,
                blocking: false,
                gates: vec![],
                outcome: result,
                summary: "existing invocation result".into(),
                evidence: vec![],
                stdout: None,
                stderr: None,
                duration_ms: None,
            }];
            let additional = crate::organization_controls::qualify(
                f.temp.path(),
                &f.project,
                &snapshot,
                &checks,
                Some(&f.ledger),
                Some(&"a".repeat(64)),
                Outcome::Passed,
                TestStage::Recovery,
            );
            assert_eq!(additional[0].outcome, expected);
            assert!(
                additional[0].blocking,
                "selected organization obligation remains mandatory even for an otherwise advisory extension"
            );
            assert_eq!(
                checks[0].outcome, result,
                "projection must not rewrite the original result"
            );
        }
        Ok(())
    }
}
