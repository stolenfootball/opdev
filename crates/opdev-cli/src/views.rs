//! Read-only projections. Full artifacts and the evaluator remain authoritative.

use std::collections::{BTreeMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use opdev_core::{AggregateVerdict, Gate, GateVerdict, Outcome, RuleId, embedded_catalog};
use opdev_engine::{CheckKind, CheckReport, reaggregate};
use opdev_project::{
    ChangeEvidence, EVIDENCE_PATH, EvidenceAssertion, EvidenceLedger, staged_fingerprint,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const EXCERPT_CHARS: usize = 512;

#[derive(Debug, Serialize)]
pub(super) struct ReportSource {
    path: PathBuf,
    sha256: String,
}

impl ReportSource {
    fn new(path: &Path, bytes: &[u8]) -> Result<Self> {
        Ok(Self {
            path: path.canonicalize()?,
            sha256: format!("{:x}", Sha256::digest(bytes)),
        })
    }
}

#[derive(Debug, Serialize)]
struct Excerpt {
    text: String,
    truncated: bool,
}

impl Excerpt {
    fn new(text: &str) -> Self {
        let mut chars = text.chars();
        let text = chars.by_ref().take(EXCERPT_CHARS).collect();
        Self {
            text,
            truncated: chars.next().is_some(),
        }
    }
}

#[derive(Debug, Serialize)]
struct RuleFinding<'a> {
    rule_id: &'a RuleId,
    outcome: Outcome,
    diagnostic: Option<Excerpt>,
    source_pointer: String,
}

#[derive(Debug, Serialize)]
struct CheckSummary<'a> {
    id: &'a str,
    kind: CheckKind,
    blocking: bool,
    gates: &'a [Gate],
    outcome: Outcome,
    summary: Excerpt,
    duration_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<Excerpt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<Excerpt>,
    source_pointer: String,
}

#[derive(Debug, Serialize)]
struct ReportSummary<'a> {
    schema: u32,
    kind: &'static str,
    catalog_version: u32,
    subject: &'a str,
    evaluated_at: u64,
    // Schema-1 full reports do not encode the requested stage or CLI flags.
    stage: Option<&'static str>,
    freshness: &'static str,
    full_report: ReportSource,
    gates: &'a [GateVerdict],
    rule_counts: BTreeMap<&'static str, usize>,
    findings: Vec<RuleFinding<'a>>,
    checks: Vec<CheckSummary<'a>>,
}

fn outcome_name(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Passed => "passed",
        Outcome::Failed => "failed",
        Outcome::Unverified => "unverified",
        Outcome::NotApplicable => "not_applicable",
        Outcome::Error => "error",
        Outcome::MigrationRequired => "migration_required",
    }
}

fn validate_report(report: &CheckReport) -> Result<()> {
    let catalog = embedded_catalog()?;
    if report.schema != 1 || report.catalog_version != catalog.catalog_version {
        bail!("unsupported report schema or catalog version; use the originating CLI");
    }
    // Aggregation uses catalog order. Reject incomplete, duplicate or reordered input.
    if report.rules.len() != catalog.rules.len()
        || report
            .rules
            .iter()
            .zip(&catalog.rules)
            .any(|(result, rule)| {
                result.rule_id != rule.id || result.catalog_version != report.catalog_version
            })
    {
        bail!("report must contain the complete matching catalog in catalog order");
    }
    let mut check_ids = HashSet::new();
    if report
        .checks
        .iter()
        .any(|check| !check_ids.insert((matches!(check.kind, CheckKind::Suite), &check.id)))
    {
        bail!("report contains ambiguous duplicate check IDs");
    }
    let mut recomputed = report.clone();
    reaggregate(&mut recomputed)?;
    if recomputed.gates != report.gates {
        bail!("recorded gates are inconsistent with the report's rules and checks");
    }
    Ok(())
}

fn project_report(report: &CheckReport, source: ReportSource) -> Result<ReportSummary<'_>> {
    validate_report(report)?;
    let mut rule_counts = [
        "passed",
        "failed",
        "unverified",
        "not_applicable",
        "error",
        "migration_required",
    ]
    .map(|name| (name, 0))
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    for rule in &report.rules {
        *rule_counts.entry(outcome_name(rule.outcome)).or_default() += 1;
    }
    Ok(ReportSummary {
        schema: 1,
        kind: "check_summary",
        catalog_version: report.catalog_version,
        subject: &report.subject,
        evaluated_at: report.evaluated_at,
        stage: None,
        freshness: "saved_evaluation_only; not revalidated against current project or remote state",
        full_report: source,
        gates: &report.gates,
        rule_counts,
        findings: report
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| !rule.outcome.satisfies_required_rule())
            .map(|(index, rule)| RuleFinding {
                rule_id: &rule.rule_id,
                outcome: rule.outcome,
                diagnostic: rule.diagnostic.as_deref().map(Excerpt::new),
                source_pointer: format!("/rules/{index}"),
            })
            .collect(),
        checks: report
            .checks
            .iter()
            .enumerate()
            .map(|(index, check)| {
                let include_output = !check.outcome.satisfies_required_rule();
                CheckSummary {
                    id: &check.id,
                    kind: check.kind,
                    blocking: check.blocking,
                    gates: &check.gates,
                    outcome: check.outcome,
                    summary: Excerpt::new(&check.summary),
                    duration_ms: check.duration_ms,
                    stdout: check
                        .stdout
                        .as_deref()
                        .filter(|_| include_output)
                        .map(Excerpt::new),
                    stderr: check
                        .stderr
                        .as_deref()
                        .filter(|_| include_output)
                        .map(Excerpt::new),
                    source_pointer: format!("/checks/{index}"),
                }
            })
            .collect(),
    })
}

pub(super) fn save_report(report: &CheckReport, path: &Path) -> Result<ReportSource> {
    let mut bytes = serde_json::to_vec_pretty(report)?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("could not create new report {}", path.display()))?;
    file.write_all(&bytes)?;
    ReportSource::new(path, &bytes)
}

pub(super) fn print_summary(report: &CheckReport, source: ReportSource) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string(&project_report(report, source)?)?
    );
    Ok(())
}

pub(super) fn summarize_file(path: &Path) -> Result<ExitCode> {
    let bytes =
        std::fs::read(path).with_context(|| format!("could not read report {}", path.display()))?;
    let report: CheckReport = serde_json::from_slice(&bytes)
        .context("invalid full check report; a summary cannot be used as input")?;
    print_summary(&report, ReportSource::new(path, &bytes)?)?;
    Ok(
        if report
            .gates
            .iter()
            .all(|gate| gate.verdict == AggregateVerdict::Passed)
        {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        },
    )
}

#[derive(Debug, Serialize)]
struct CurrentEvidence {
    schema: u32,
    kind: &'static str,
    subject: PathBuf,
    fingerprint: String,
    ledger: PathBuf,
    ledger_present: bool,
    requested_rule: Option<RuleId>,
    missing_requested_rule: Option<RuleId>,
    project: Vec<EvidenceAssertion>,
    change: Option<ChangeEvidence>,
    notice: &'static str,
}

fn current_evidence(root: &Path, rule: Option<&RuleId>) -> Result<CurrentEvidence> {
    let catalog = embedded_catalog()?;
    if let Some(rule) = rule
        && catalog.find(rule).is_none()
    {
        bail!("unknown rule {rule}");
    }
    let fingerprint = staged_fingerprint(root)?;
    let ledger = EvidenceLedger::load_optional(root, &catalog)?;
    let mut project = ledger
        .as_ref()
        .map_or_else(Vec::new, |ledger| ledger.project.clone());
    let mut change = ledger
        .as_ref()
        .and_then(|ledger| ledger.matching_change(&fingerprint))
        .cloned();
    let selected = |assertion: &EvidenceAssertion| rule.is_none_or(|id| *id == assertion.rule_id);
    project.retain(selected);
    if let Some(change) = &mut change {
        change.assertions.retain(selected);
        if rule.is_some_and(|rule| !matches!(rule.as_str(), "OPDEV-TEST-002" | "OPDEV-TEST-003")) {
            change.acceptance = None;
        }
    }
    let missing_requested_rule = rule
        .filter(|_| {
            project.is_empty()
                && change.as_ref().is_none_or(|change| {
                    change.assertions.is_empty() && change.acceptance.is_none()
                })
        })
        .cloned();
    Ok(CurrentEvidence {
        schema: ledger.as_ref().map_or(1, |ledger| ledger.schema),
        kind: "current_evidence",
        subject: root.to_path_buf(),
        fingerprint,
        ledger: root.join(EVIDENCE_PATH),
        ledger_present: ledger.is_some(),
        requested_rule: rule.cloned(),
        missing_requested_rule,
        project,
        change,
        notice: "Assertions are inputs, not evaluated passes. Missing evidence is unverified. Recheck after staged changes; concrete failures cannot be overridden.",
    })
}

pub(super) fn show_evidence(root: &Path, rule: Option<&RuleId>) -> Result<()> {
    println!("{}", serde_json::to_string(&current_evidence(root, rule)?)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opdev_core::{Evidence, RuleResult, VerificationSource};
    use opdev_engine::CheckResult;
    use std::process::Command;

    fn report() -> Result<CheckReport> {
        let catalog = embedded_catalog()?;
        let rules = catalog
            .rules
            .iter()
            .map(|rule| RuleResult {
                rule_id: rule.id.clone(),
                catalog_version: catalog.catalog_version,
                outcome: Outcome::Passed,
                subject: "fixture".into(),
                verifier: VerificationSource::Manifest,
                evaluated_at: 42,
                evidence: vec![],
                diagnostic: None,
            })
            .collect();
        let mut report = CheckReport {
            schema: 1,
            catalog_version: catalog.catalog_version,
            subject: "fixture".into(),
            evaluated_at: 42,
            rules,
            checks: vec![],
            gates: vec![],
        };
        reaggregate(&mut report)?;
        Ok(report)
    }

    fn source() -> ReportSource {
        ReportSource {
            path: "full.json".into(),
            sha256: "fixture-digest".into(),
        }
    }

    #[test]
    fn projections_preserve_every_outcome_and_all_gate_blockers() -> Result<()> {
        for outcome in [
            Outcome::Passed,
            Outcome::Failed,
            Outcome::Unverified,
            Outcome::NotApplicable,
            Outcome::Error,
            Outcome::MigrationRequired,
        ] {
            let mut report = report()?;
            report.rules[0].outcome = outcome;
            report.rules[1].outcome = Outcome::Unverified;
            report.checks = [true, false]
                .into_iter()
                .map(|blocking| CheckResult {
                    id: "shared".into(),
                    kind: if blocking {
                        CheckKind::Suite
                    } else {
                        CheckKind::Extension
                    },
                    blocking,
                    gates: vec![Gate::Development],
                    outcome,
                    summary: "result".into(),
                    evidence: vec![],
                    stdout: Some("details".into()),
                    stderr: None,
                    duration_ms: Some(7),
                })
                .collect();
            reaggregate(&mut report)?;
            let summary = project_report(&report, source())?;
            assert_eq!(summary.gates, report.gates);
            assert_eq!(
                summary.rule_counts.values().sum::<usize>(),
                report.rules.len()
            );
            for (check, original) in summary.checks.iter().zip(&report.checks) {
                assert_eq!(check.outcome, original.outcome);
                assert_eq!(check.blocking, original.blocking);
                assert_eq!(check.gates, original.gates);
            }
            assert_eq!(
                summary.findings.len(),
                if outcome.satisfies_required_rule() {
                    1
                } else {
                    2
                }
            );
            assert!(
                summary
                    .findings
                    .iter()
                    .any(|f| f.outcome == Outcome::Unverified)
            );
        }
        Ok(())
    }

    #[test]
    fn truncation_is_explicit_unicode_safe_and_full_diagnostics_are_recoverable() -> Result<()> {
        let mut report = report()?;
        let diagnostic = "界".repeat(600);
        report.rules[0].outcome = Outcome::Error;
        report.rules[0].diagnostic = Some(diagnostic.clone());
        reaggregate(&mut report)?;
        let full = serde_json::to_value(&report)?;
        let summary = project_report(&report, source())?;
        let finding = &summary.findings[0];
        let excerpt = finding.diagnostic.as_ref().context("missing diagnostic")?;
        assert!(excerpt.truncated);
        assert_eq!(excerpt.text.chars().count(), EXCERPT_CHARS);
        assert_eq!(
            full.pointer(&format!("{}/diagnostic", finding.source_pointer)),
            Some(&serde_json::json!(diagnostic))
        );
        assert!(!Excerpt::new(&"a".repeat(EXCERPT_CHARS)).truncated);
        assert!(summary.stage.is_none());
        Ok(())
    }

    #[test]
    fn malformed_or_inconsistent_saved_reports_are_rejected() -> Result<()> {
        let good = report()?;
        let mut variants = vec![];
        let mut bad = good.clone();
        bad.schema = 99;
        variants.push(bad);
        let mut bad = good.clone();
        bad.catalog_version += 1;
        variants.push(bad);
        let mut bad = good.clone();
        bad.rules.pop();
        variants.push(bad);
        let mut bad = good.clone();
        bad.rules.swap(0, 1);
        variants.push(bad);
        let mut bad = good.clone();
        bad.rules[0].outcome = Outcome::Failed;
        variants.push(bad);
        let mut bad = good.clone();
        bad.gates.clear();
        variants.push(bad);
        for bad in variants {
            assert!(project_report(&bad, source()).is_err());
        }
        Ok(())
    }

    #[test]
    fn saved_source_digest_and_no_overwrite_are_enforced() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("report.json");
        let report = report()?;
        let source = save_report(&report, &path)?;
        let bytes = std::fs::read(&path)?;
        assert_eq!(source.sha256, format!("{:x}", Sha256::digest(&bytes)));
        assert!(save_report(&report, &path).is_err());
        assert_eq!(std::fs::read(&path)?, bytes);
        assert_eq!(summarize_file(&path)?, ExitCode::SUCCESS);
        std::fs::write(&path, b"{\"schema\":1,\"kind\":\"check_summary\"}")?;
        assert!(summarize_file(&path).is_err());
        Ok(())
    }

    fn git(root: &Path, args: &[&str]) -> Result<()> {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }

    fn repository() -> Result<tempfile::TempDir> {
        let temp = tempfile::tempdir()?;
        git(temp.path(), &["init", "--quiet"])?;
        std::fs::create_dir(temp.path().join(".opdev"))?;
        std::fs::write(temp.path().join("file"), "one")?;
        git(temp.path(), &["add", "file"])?;
        Ok(temp)
    }

    fn assertion(id: &str) -> Result<EvidenceAssertion> {
        Ok(EvidenceAssertion {
            rule_id: id.parse()?,
            outcome: Outcome::Passed,
            summary: "reviewed fixture fact".into(),
            evidence: vec![Evidence {
                kind: "test".into(),
                summary: "fixture".into(),
                location: Some("file".into()),
            }],
        })
    }

    #[test]
    fn evidence_selects_exact_state_not_last_entry_and_retains_scopes() -> Result<()> {
        let temp = repository()?;
        let root = temp.path();
        let fingerprint = staged_fingerprint(root)?;
        let ledger = EvidenceLedger {
            schema: 1,
            project: vec![assertion("OPDEV-SEC-001")?],
            changes: vec![
                ChangeEvidence {
                    fingerprint: fingerprint.clone(),
                    acceptance: None,
                    work: "https://example.test/1".into(),
                    assertions: vec![assertion("OPDEV-WORK-001")?],
                },
                ChangeEvidence {
                    fingerprint: "0".repeat(64),
                    acceptance: None,
                    work: "https://example.test/old".into(),
                    assertions: vec![assertion("OPDEV-WORK-001")?],
                },
            ],
        };
        let path = ledger.write_new(root, &embedded_catalog()?)?;
        let before = std::fs::read(&path)?;
        let view = current_evidence(root, None)?;
        assert_eq!(view.fingerprint, fingerprint);
        assert_eq!(view.project, ledger.project);
        assert_eq!(view.change, Some(ledger.changes[0].clone()));
        let filtered = current_evidence(root, Some(&"OPDEV-WORK-001".parse()?))?;
        assert!(filtered.project.is_empty());
        assert_eq!(
            filtered
                .change
                .context("missing matching change")?
                .assertions
                .len(),
            1
        );
        assert!(filtered.missing_requested_rule.is_none());
        assert!(
            current_evidence(root, Some(&"OPDEV-TEST-001".parse()?))?
                .missing_requested_rule
                .is_some()
        );
        assert!(current_evidence(root, Some(&"OPDEV-UNKNOWN-999".parse()?)).is_err());
        assert_eq!(std::fs::read(path)?, before);
        std::fs::write(root.join("file"), "two")?;
        assert!(current_evidence(root, None).is_err());
        git(root, &["add", "file"])?;
        let stale = current_evidence(root, None)?;
        assert!(stale.change.is_none());
        assert_eq!(stale.project, ledger.project);
        Ok(())
    }

    #[test]
    fn missing_and_invalid_ledgers_do_not_produce_assertions() -> Result<()> {
        let temp = repository()?;
        let root = temp.path();
        let absent = current_evidence(root, Some(&"OPDEV-WORK-001".parse()?))?;
        assert!(!absent.ledger_present);
        assert!(absent.change.is_none());
        assert!(absent.missing_requested_rule.is_some());
        std::fs::write(root.join(EVIDENCE_PATH), "schema: 99\n")?;
        assert!(current_evidence(root, None).is_err());
        Ok(())
    }
}
