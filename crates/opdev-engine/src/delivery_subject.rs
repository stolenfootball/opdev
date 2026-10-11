//! Project controls and a particular artifact's readiness are different subjects.
use opdev_core::{Evidence, Outcome, RuleResult};
use opdev_project::{ChangeEvidence, DeliverySubjectKind, TestStage};

/// Rules whose qualification needs identified deliverable bytes, not a repository path.
#[must_use]
pub fn artifact_scoped_rule(id: &str) -> bool {
    matches!(
        id,
        "MCD-ARTIFACT-001"
            | "MCD-ARTIFACT-002"
            | "MCD-ENV-001"
            | "OPDEV-SUPPLY-001"
            | "OPDEV-SUPPLY-002"
    )
}

pub(crate) fn qualify(
    rules: &mut [RuleResult],
    change: Option<&ChangeEvidence>,
    acceptance: Outcome,
    stage: TestStage,
) {
    let selected = change
        .and_then(|c| c.acceptance.as_ref())
        .and_then(|a| a.delivery_subject.as_ref());
    for rule in rules
        .iter_mut()
        .filter(|r| artifact_scoped_rule(r.rule_id.as_str()))
    {
        // Never conceal a known failure or override demonstrated inapplicability.
        if rule.outcome != Outcome::Passed {
            continue;
        }
        let binding = selected.filter(|s| {
            stage == TestStage::Delivery
                && acceptance == Outcome::Passed
                && change.is_some_and(|c| {
                    c.assertions.iter().any(|a| {
                        a.rule_id == rule.rule_id
                            && a.outcome == Outcome::Passed
                            && a.evidence.iter().any(|e| {
                                e.kind == "delivery_subject"
                                    && e.location.as_deref() == Some(s.sha256.as_str())
                            })
                    })
                })
        });
        if let Some(subject) = binding {
            let kind = match subject.kind {
                DeliverySubjectKind::Artifact => "artifact",
                DeliverySubjectKind::Inventory => "inventory",
            };
            rule.subject = format!("{kind}:sha256:{}", subject.sha256);
            rule.evidence.push(Evidence {
                kind: "delivery_subject".into(),
                summary: "Exact reviewed delivery subject; identity is not independent proof of qualification or release authorization".into(),
                location: Some(subject.locator.clone()),
            });
        } else {
            rule.outcome = Outcome::Unverified;
            rule.diagnostic = Some("Artifact delivery is not verified. Use the delivery stage with an exact reviewed artifact or inventory digest and matching current rule evidence. Project adoption and historical release observations do not qualify this checkout.".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opdev_core::{RuleId, VerificationSource};
    use opdev_project::{AcceptanceEvidence, DeliverySubject, EvidenceAssertion};

    #[test]
    fn historical_project_claims_cannot_qualify_new_bytes_or_another_stage() -> anyhow::Result<()> {
        let id: RuleId = "MCD-ARTIFACT-001".parse()?;
        let make = || RuleResult {
            rule_id: id.clone(),
            catalog_version: 4,
            outcome: Outcome::Passed,
            subject: "/project".into(),
            verifier: VerificationSource::Evidence,
            evaluated_at: 1,
            evidence: vec![],
            diagnostic: None,
        };
        let digest = "a".repeat(64);
        let mut change = ChangeEvidence {
            fingerprint: "b".repeat(64),
            work: "review".into(),
            assertions: vec![EvidenceAssertion {
                rule_id: id.clone(),
                outcome: Outcome::Passed,
                summary: "Observed exact bytes".into(),
                evidence: vec![Evidence {
                    kind: "delivery_subject".into(),
                    summary: "Exact inventory".into(),
                    location: Some(digest.clone()),
                }],
            }],
            acceptance: Some(AcceptanceEvidence {
                delivery_subject: Some(DeliverySubject {
                    sha256: digest.clone(),
                    locator: "https://example.test/immutable-manifest".into(),
                    kind: DeliverySubjectKind::Inventory,
                }),
                ..Default::default()
            }),
        };
        for stage in [TestStage::Local, TestStage::PreMerge, TestStage::PostMerge] {
            let mut rules = vec![make()];
            qualify(&mut rules, Some(&change), Outcome::Passed, stage);
            assert_eq!(rules[0].outcome, Outcome::Unverified);
        }
        let mut rules = vec![make()];
        qualify(&mut rules, None, Outcome::Passed, TestStage::Delivery);
        assert_eq!(rules[0].outcome, Outcome::Unverified);
        let mut rules = vec![make()];
        qualify(
            &mut rules,
            Some(&change),
            Outcome::Passed,
            TestStage::Delivery,
        );
        assert_eq!(rules[0].subject, format!("inventory:sha256:{digest}"));
        assert_eq!(rules[0].outcome, Outcome::Passed);
        change.assertions[0].evidence[0].location = Some("c".repeat(64));
        let mut rules = vec![make()];
        qualify(
            &mut rules,
            Some(&change),
            Outcome::Passed,
            TestStage::Delivery,
        );
        assert_eq!(rules[0].outcome, Outcome::Unverified);
        for outcome in [
            Outcome::Failed,
            Outcome::Error,
            Outcome::MigrationRequired,
            Outcome::NotApplicable,
        ] {
            let mut rules = vec![make()];
            rules[0].outcome = outcome;
            qualify(
                &mut rules,
                Some(&change),
                Outcome::Passed,
                TestStage::Delivery,
            );
            assert_eq!(rules[0].outcome, outcome);
        }
        Ok(())
    }
}
