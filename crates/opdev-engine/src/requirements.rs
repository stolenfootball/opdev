//! Catalog qualification consumes current invocation results only; no report imports.
use crate::{CheckKind, CheckResult};
use opdev_core::{Gate, Outcome};
use opdev_project::{
    EvidenceLedger, ProjectManifest, TestStage,
    requirements::{AssuranceLevel, CatalogSnapshot, Method},
};
use std::path::Path;

fn finding(id: &str, outcome: Outcome, message: String) -> CheckResult {
    CheckResult {
        id: format!("requirements:{id}"),
        kind: CheckKind::Policy,
        blocking: true,
        gates: vec![
            Gate::Development,
            Gate::Integration,
            Gate::Delivery,
            Gate::Compliance,
        ],
        outcome,
        summary: message,
        evidence: vec![],
        stdout: None,
        stderr: None,
        duration_ms: None,
    }
}

#[allow(clippy::too_many_arguments)] // Shared evaluated subject, not a second execution path.
pub(crate) fn evaluate(
    root: &Path,
    manifest: &ProjectManifest,
    snapshot: anyhow::Result<CatalogSnapshot>,
    checks: &[CheckResult],
    stage: TestStage,
    fresh: bool,
    acceptance: Option<&EvidenceLedger>,
    fingerprint: Option<&str>,
    acceptance_outcome: Outcome,
) -> Vec<CheckResult> {
    let mut findings = Vec::new();
    let snapshot = match snapshot {
        Ok(s) => s,
        Err(e) => return vec![finding("catalog", Outcome::Error, e.to_string())],
    };
    let review = acceptance
        .and_then(|l| fingerprint.and_then(|f| l.matching_change(f)))
        .and_then(|c| c.acceptance.as_ref())
        .and_then(|a| a.requirements.as_ref());
    if let Some(review) = review {
        for observed in &review.manual_observations {
            if observed.outcome == Outcome::Failed {
                findings.push(finding(&format!("{}:{}:observation", observed.plan, observed.verification), Outcome::Failed,
                    "Recorded manual observation contradicts the criterion; staleness does not erase the finding".into()));
            }
        }
    }
    match snapshot.inspect(root, manifest) {
        Ok(report) => {
            for f in report.findings {
                findings.push(finding(
                    &format!("{}:{}", f.subject, f.code),
                    f.outcome,
                    f.message,
                ));
            }
        }
        Err(e) => findings.push(finding("catalog", Outcome::Error, e.to_string())),
    }
    if !fresh
        || !opdev_project::requirements::load_index(root).is_ok_and(|now| now.tree == snapshot.tree)
    {
        findings.push(finding("source", Outcome::Unverified, "Source changed during verification; catalog review cannot qualify execution against a different snapshot".into()));
    }
    for plan in snapshot.plans().filter(|p| p.stage == stage) {
        for member in &plan.members {
            let Some(v) = snapshot
                .verifications()
                .find(|v| v.id == member.verification)
            else {
                continue;
            };
            let subject = format!("{}:{}", plan.id, v.id);
            match &v.method {
                Method::Automated { suite, assurance } => {
                    let run = checks
                        .iter()
                        .find(|c| c.kind == CheckKind::Suite && &c.id == suite);
                    match run {
                        Some(c) if matches!(c.outcome, Outcome::Failed | Outcome::Error) => findings.push(finding(&subject, c.outcome, format!("Required suite {suite} did not pass: {}", c.summary))),
                        Some(c) if c.outcome == Outcome::Passed => {},
                        _ => findings.push(finding(&subject, Outcome::Unverified, format!("Required suite {suite} has no passing execution in this invocation at {stage:?}; stored results cannot substitute"))),
                    }
                    if *assurance == AssuranceLevel::Case {
                        findings.push(finding(&format!("{subject}:case"), Outcome::Unverified, "Individual-case assurance was selected, but canonical command exit does not establish case execution. No automatic downgrade to suite assurance".into()));
                    }
                }
                Method::Manual {
                    max_age_seconds, ..
                } => {
                    let observations: Vec<_> = review
                        .into_iter()
                        .flat_map(|r| &r.manual_observations)
                        .filter(|o| o.plan == plan.id && o.verification == v.id)
                        .collect();
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs());
                    let subject_digest = snapshot.plan_digest(plan, manifest).ok();
                    let current = acceptance_outcome == Outcome::Passed
                        && observations.len() == 1
                        && observations.first().is_some_and(|o| {
                            manual_current(o, subject_digest.as_deref(), *max_age_seconds, now)
                        });
                    if !current {
                        findings.push(finding(&subject, Outcome::Unverified, "Manual verification needs exactly one current source-bound MR/PR observation with actual result, observer, context, limits and a matching plan subject within its declared age. Mapping review alone is not an observation".into()));
                    }
                }
            }
        }
    }
    if findings.is_empty() {
        findings.push(finding("catalog", Outcome::Passed, format!("Current catalog inputs and all required plan members for {stage:?} verified. Only this invocation's mapped suite outcomes/current manual observations were credited; no individual-case, inventory-completeness or developer-consent claim")));
    }
    findings
}

fn manual_current(
    observed: &opdev_project::requirements::ManualObservation,
    subject: Option<&str>,
    max_age: u32,
    now: u64,
) -> bool {
    observed.outcome == Outcome::Passed
        && subject == Some(observed.plan_subject_sha256.as_str())
        && observed.observed_at > 0
        && observed.observed_at <= now
        && now - observed.observed_at <= u64::from(max_age)
        && max_age > 0
        && [
            &observed.observer,
            &observed.observed_result,
            &observed.context_and_limits,
            &observed.reference,
        ]
        .iter()
        .all(|s| !s.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use serde_json::json;

    fn fixture() -> Result<(tempfile::TempDir, ProjectManifest, CatalogSnapshot)> {
        use opdev_project::{CommandSpec, TestSuite, TrackedEvidence};
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let git = |args: &[&str]| -> Result<()> {
            let o = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()?;
            anyhow::ensure!(o.status.success(), "fixture Git failed");
            Ok(())
        };
        git(&["init", "-q"])?;
        std::fs::create_dir_all(root.join(".opdev/requirements"))?;
        std::fs::write(root.join("assert.txt"), "assert failure_preserves_bytes\n")?;
        git(&["add", "."])?;
        let target = TrackedEvidence::bind(
            root,
            "assert.txt".into(),
            "assert failure_preserves_bytes".into(),
        )?;
        let mut manifest = opdev_project::discover(root)?.manifest;
        manifest.assurance.requirements = Some(serde_json::from_value(
            json!({"version":1,"review_reference":"fixture choice","configurations":{"default":["pre_merge","post_merge"]}}),
        )?);
        for id in ["unit", "consumer"] {
            manifest.commands.insert(
                id.into(),
                CommandSpec {
                    argv: vec!["fixture".into()],
                    working_directory: None,
                    timeout_seconds: Some(30),
                },
            );
            manifest.testing.suites.push(TestSuite {
                id: id.into(),
                command: id.into(),
                stages: vec![TestStage::PreMerge, TestStage::PostMerge],
            });
        }
        let doc = json!({"schema":1,"requirements":[{"id":"R","title":"Integrity","statement":{"kind":"inline","text":"Rejected input preserves data"},"origin":"fixture promise","rationale":"Integrity","configurations":["default"],"criteria":[{"id":"C","expected":"Reject malformed input without changing stored bytes"}]}],
            "verifications":[{"id":"V1","target":target,"inputs":[],"method":{"kind":"automated","suite":"unit","assurance":"suite"}}, {"id":"V2","target":target,"inputs":[],"method":{"kind":"automated","suite":"consumer","assurance":"suite"}}],
            "plans":(["pre_merge","post_merge"].map(|s| json!({"id":format!("P-{s}"),"criterion":"C","configuration":"default","stage":s,"members":[{"verification":"V1","assertion":"Internal invariant","discriminating_case":"Existing bytes survive null"},{"verification":"V2","assertion":"Consumer assembled path","discriminating_case":"Caller receives error and data survives"}],"review":{"outcome":"passed","reviewer":"fixture agent","reference":"fixture review","rationale":"Both layers necessary","subject_sha256":""}}))) });
        let path = root.join(".opdev/requirements/storage.json");
        std::fs::write(&path, serde_json::to_vec(&doc)?)?;
        git(&["add", "."])?;
        let mut snapshot = opdev_project::requirements::load_index(root)?;
        let digests: Vec<_> = snapshot
            .plans()
            .map(|p| snapshot.plan_digest(p, &manifest))
            .collect::<Result<_>>()?;
        let doc = snapshot
            .documents
            .values_mut()
            .next()
            .ok_or_else(|| anyhow::anyhow!("fixture"))?;
        for (p, d) in doc.plans.iter_mut().zip(digests) {
            p.review.subject_sha256 = d;
        }
        std::fs::write(path, serde_json::to_vec(doc)?)?;
        git(&["add", "."])?;
        let snapshot = opdev_project::requirements::load_index(root)?;
        Ok((temp, manifest, snapshot))
    }

    fn run_result(id: &str, outcome: Outcome) -> CheckResult {
        let mut check = finding(id, outcome, "fixture command observation".into());
        check.kind = CheckKind::Suite;
        check.id = id.into();
        check
    }

    #[test]
    fn all_members_current_stage_missing_execution_and_case_assurance() -> Result<()> {
        let (temp, manifest, snapshot) = fixture()?;
        let checks = [
            run_result("unit", Outcome::Passed),
            run_result("consumer", Outcome::Passed),
        ];
        let assess = |snapshot, checks: &[CheckResult], stage| {
            evaluate(
                temp.path(),
                &manifest,
                Ok(snapshot),
                checks,
                stage,
                true,
                None,
                None,
                Outcome::Passed,
            )
        };
        assert!(
            assess(snapshot.clone(), &checks, TestStage::PreMerge)
                .iter()
                .all(|f| f.outcome == Outcome::Passed)
        );
        let missing = assess(snapshot.clone(), &checks[..1], TestStage::PreMerge);
        assert!(
            missing
                .iter()
                .any(|f| f.outcome == Outcome::Unverified && f.summary.contains("consumer"))
        );
        assert!(
            assess(snapshot.clone(), &[], TestStage::PostMerge)
                .iter()
                .any(|f| f.outcome == Outcome::Unverified)
        );
        let mut stronger = snapshot;
        let doc = stronger
            .documents
            .values_mut()
            .next()
            .ok_or_else(|| anyhow::anyhow!("fixture"))?;
        doc.verifications[0].method = Method::Automated {
            suite: "unit".into(),
            assurance: AssuranceLevel::Case,
        };
        assert!(
            assess(stronger, &checks, TestStage::PreMerge)
                .iter()
                .any(|f| f.id.ends_with(":case") && f.outcome == Outcome::Unverified)
        );
        Ok(())
    }

    #[test]
    fn stale_review_does_not_hide_failure_and_saved_or_missing_execution_cannot_pass() -> Result<()>
    {
        let (temp, manifest, mut snapshot) = fixture()?;
        let doc = snapshot
            .documents
            .values_mut()
            .next()
            .ok_or_else(|| anyhow::anyhow!("fixture"))?;
        doc.plans[0].review.subject_sha256.clear();
        let checks = [
            run_result("unit", Outcome::Passed),
            run_result("consumer", Outcome::Failed),
        ];
        let findings = evaluate(
            temp.path(),
            &manifest,
            Ok(snapshot),
            &checks,
            TestStage::PreMerge,
            false,
            None,
            None,
            Outcome::Unverified,
        );
        assert!(
            findings
                .iter()
                .any(|f| f.id.ends_with("review_not_current") && f.outcome == Outcome::Unverified)
        );
        assert!(
            findings
                .iter()
                .any(|f| f.outcome == Outcome::Failed && f.summary.contains("consumer"))
        );
        assert!(findings.iter().any(|f| f.id == "requirements:source"));
        Ok(())
    }

    #[test]
    fn manual_expiry_future_missing_attribution_and_stale_subject_are_unverified() {
        let mut o = opdev_project::requirements::ManualObservation {
            plan: "P".into(),
            verification: "V".into(),
            plan_subject_sha256: "abc".into(),
            observed_at: 100,
            outcome: Outcome::Passed,
            observer: "fixture agent".into(),
            observed_result: "Keyboard navigation reached all controls".into(),
            context_and_limits: "Desktop only; human inspection".into(),
            reference: "fixture observation".into(),
        };
        assert!(manual_current(&o, Some("abc"), 10, 110));
        assert!(!manual_current(&o, Some("abc"), 10, 111));
        assert!(!manual_current(&o, Some("abc"), 10, 99));
        assert!(!manual_current(&o, Some("changed"), 10, 110));
        o.observer.clear();
        assert!(!manual_current(&o, Some("abc"), 10, 110));
    }
}
