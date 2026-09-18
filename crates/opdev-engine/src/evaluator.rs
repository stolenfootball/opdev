use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use opdev_core::{
    AggregateVerdict, EXTENSION_PROTOCOL_VERSION, Evidence, ExtensionRequest, ExtensionResponse,
    Gate, GateVerdict, Outcome, Rule, RuleCatalog, RuleResult, VerificationSource,
    embedded_catalog,
};
use opdev_project::{
    CiProvider, CoverageMode, DeliveryStatus, EVIDENCE_PATH, EvidenceAssertion, EvidenceLedger,
    ExtensionCheck, ExtensionStage, ProjectManifest, TestStage, staged_fingerprint,
};
use thiserror::Error;

use crate::command::{CommandError, Execution, execute};
use crate::report::{CheckKind, CheckReport, CheckResult};

/// Selection of executable checks for one evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckOptions {
    /// Canonical test-suite stage to execute.
    pub test_stage: TestStage,
    /// Project-extension stage to execute.
    pub extension_stage: ExtensionStage,
    /// Whether canonical and extension commands may execute.
    pub execute_checks: bool,
}

impl CheckOptions {
    /// Local-development evaluation with canonical local suites.
    #[must_use]
    pub const fn local() -> Self {
        Self {
            test_stage: TestStage::Local,
            extension_stage: ExtensionStage::Verify,
            execute_checks: true,
        }
    }

    /// Pre-integration CI evaluation.
    #[must_use]
    pub const fn pre_merge() -> Self {
        Self {
            test_stage: TestStage::PreMerge,
            extension_stage: ExtensionStage::PreMerge,
            execute_checks: true,
        }
    }
}

/// Failures that prevent creation of a complete check report.
#[derive(Debug, Error)]
pub enum EvaluationError {
    /// An explicit test-report binding is invalid.
    #[error("invalid JUnit binding: {0}")]
    JunitBinding(String),
    /// The embedded rule catalog is invalid.
    #[error("could not load the embedded rule catalog: {0}")]
    Catalog(#[from] opdev_core::CatalogError),
    /// A project evidence ledger is malformed or inconsistent.
    #[error("could not load project evidence: {0}")]
    Evidence(#[from] opdev_project::EvidenceError),
    /// An extension request could not be encoded.
    #[error("could not encode extension request: {0}")]
    ExtensionRequest(#[from] serde_json::Error),
}

/// Evaluates core rules, canonical suites, and eligible project extensions.
///
/// Project commands are executed directly from validated argument vectors and
/// never via a shell. Command failures become evidence-bearing results rather
/// than preventing the remainder of the report.
///
/// # Errors
///
/// Returns [`EvaluationError`] only when bundled policy cannot be loaded or an
/// extension request cannot be encoded.
pub fn evaluate(
    root: &Path,
    manifest: &ProjectManifest,
    options: CheckOptions,
) -> Result<CheckReport, EvaluationError> {
    evaluate_with_junit(root, manifest, options, &[])
}

/// An explicitly required `JUnit` report for an existing canonical suite.
#[derive(Debug, Clone)]
pub struct JunitBinding {
    /// Existing suite identifier; it must run at the selected stage.
    pub suite: String,
    /// Output path relative to the Git root (absolute paths are also accepted).
    pub path: PathBuf,
}

/// Evaluates checks with fresh, revision-bound reports for explicitly selected suites.
///
/// Existing projects and unbound suites keep their command-only behavior. `JUnit`
/// alone cannot qualify unknown internal retry or quarantine history.
///
/// # Errors
/// Returns an error for invalid bindings before running any command, or for the
/// same policy/ledger errors as [`evaluate`].
pub fn evaluate_with_junit(
    root: &Path,
    manifest: &ProjectManifest,
    options: CheckOptions,
    junit: &[JunitBinding],
) -> Result<CheckReport, EvaluationError> {
    let mut bound = std::collections::BTreeSet::new();
    for binding in junit {
        if binding.path.as_os_str().is_empty()
            || !bound.insert(&binding.suite)
            || !manifest.testing.suites.iter().any(|suite| {
                suite.id == binding.suite && suite.stages.contains(&options.test_stage)
            })
        {
            return Err(EvaluationError::JunitBinding(
                "each binding needs a unique suite at the selected stage and a nonempty path"
                    .into(),
            ));
        }
    }
    let catalog = embedded_catalog()?;
    let evaluated_at = unix_timestamp();
    let subject = root.display().to_string();
    let mut rules: Vec<_> = catalog
        .rules
        .iter()
        .map(|rule| {
            evaluate_rule(
                rule,
                manifest,
                &subject,
                catalog.catalog_version,
                evaluated_at,
            )
        })
        .collect();
    apply_evidence_ledger(root, &catalog, &mut rules)?;
    // A known workflow contradiction must not be hidden behind a generic ledger pass.
    if let Some(record) = opdev_project::AdoptionRecord::load(root)
        .map_err(|error| opdev_project::EvidenceError::Semantic(error.to_string()))?
        && record.workflow.is_some()
        && let Some(result) = rules
            .iter_mut()
            .find(|r| r.rule_id.as_str() == "MCD-TRUNK-001")
    {
        let blockers = record.workflow_blockers(manifest);
        if !blockers.is_empty() {
            result.outcome = Outcome::MigrationRequired;
            result.verifier = VerificationSource::Manifest;
            result.diagnostic = Some(blockers.join("; "));
            result.evidence.clear();
        }
    }
    let checks = if options.execute_checks {
        let mut checks = run_suites(root, manifest, options.test_stage, junit);
        checks.extend(run_extensions(root, manifest, options.extension_stage)?);
        checks
    } else {
        junit
            .iter()
            .map(|binding| junit_not_run(binding, options.test_stage))
            .collect()
    };
    if !junit.is_empty()
        && let Some(rule) = rules
            .iter_mut()
            .find(|rule| rule.rule_id.as_str() == "OPDEV-TEST-005")
    {
        // A policy or saved assertion cannot establish the internal history of
        // newly observed executions, even when their report observations passed.
        rule.outcome = Outcome::Unverified;
        rule.verifier = VerificationSource::Command;
        rule.evidence.clear();
        rule.diagnostic = Some("Required JUnit evidence cannot establish complete producer-internal retry/quarantine history; inspect the bound suite results".into());
    }
    let gates = aggregate_gates(&catalog, &rules, &checks);
    Ok(CheckReport {
        schema: 1,
        catalog_version: catalog.catalog_version,
        subject,
        evaluated_at,
        rules,
        checks,
        gates,
    })
}

fn apply_evidence_ledger(
    root: &Path,
    catalog: &RuleCatalog,
    results: &mut [RuleResult],
) -> Result<(), EvaluationError> {
    let Some(ledger) = EvidenceLedger::load_optional(root, catalog)? else {
        return Ok(());
    };
    let change = staged_fingerprint(root)
        .ok()
        .and_then(|fingerprint| ledger.matching_change(&fingerprint));
    for result in results
        .iter_mut()
        .filter(|result| result.outcome == Outcome::Unverified)
    {
        let assertion = change
            .and_then(|change| {
                change
                    .assertions
                    .iter()
                    .find(|assertion| assertion.rule_id == result.rule_id)
            })
            .or_else(|| {
                ledger
                    .project
                    .iter()
                    .find(|assertion| assertion.rule_id == result.rule_id)
            });
        if let Some(assertion) = assertion {
            apply_assertion(result, assertion, change.map(|change| change.work.as_str()));
        }
    }
    Ok(())
}

fn apply_assertion(result: &mut RuleResult, assertion: &EvidenceAssertion, work: Option<&str>) {
    result.outcome = assertion.outcome;
    result.verifier = VerificationSource::Evidence;
    result.diagnostic = None;
    result.evidence = vec![Evidence {
        kind: "evidence_ledger".into(),
        summary: work.map_or_else(
            || assertion.summary.clone(),
            |work| format!("{}; work: {work}", assertion.summary),
        ),
        location: Some(EVIDENCE_PATH.into()),
    }];
    result.evidence.extend(assertion.evidence.clone());
}

/// Recomputes strict gate verdicts after another verifier contributes rule
/// evidence, such as a local CI or remote-provider adapter.
///
/// # Errors
///
/// Returns [`EvaluationError`] when the embedded catalog cannot be loaded.
pub fn reaggregate(report: &mut CheckReport) -> Result<(), EvaluationError> {
    let catalog = embedded_catalog()?;
    report.gates = aggregate_gates(&catalog, &report.rules, &report.checks);
    Ok(())
}

fn evaluate_rule(
    rule: &Rule,
    manifest: &ProjectManifest,
    subject: &str,
    catalog_version: u32,
    evaluated_at: u64,
) -> RuleResult {
    let (outcome, verifier, evidence, diagnostic) = evaluate_project_policy(rule, manifest)
        .or_else(|| evaluate_testing_policy(rule, manifest))
        .or_else(|| evaluate_applicability(rule, manifest))
        .unwrap_or_else(|| default_evaluation(rule, manifest));
    RuleResult {
        rule_id: rule.id.clone(),
        catalog_version,
        outcome,
        subject: subject.to_owned(),
        verifier,
        evaluated_at,
        evidence,
        diagnostic,
    }
}

type Evaluation = (Outcome, VerificationSource, Vec<Evidence>, Option<String>);

fn evaluate_project_policy(rule: &Rule, manifest: &ProjectManifest) -> Option<Evaluation> {
    let evaluation = match rule.id.as_str() {
        "OPDEV-AUTH-001" if !manifest.authorities.is_empty() => manifest_pass(
            format!(
                "{} authoritative sources are declared",
                manifest.authorities.len()
            ),
            Some(".opdev/project.yaml"),
        ),
        "OPDEV-AUTH-002" if manifest.authorities.contains_key("work") => manifest_pass(
            "A work authority is declared for active status".into(),
            Some(".opdev/project.yaml"),
        ),
        "MCD-CI-001" if manifest.project.ci.provider != CiProvider::Unconfigured => manifest_pass(
            format!("CI provider is {:?}", manifest.project.ci.provider),
            Some(".opdev/project.yaml"),
        ),
        "MCD-CI-001" => migration("A CI provider must be configured"),
        "MCD-DELIVERY-001" if manifest.delivery.status == DeliveryStatus::Configured => {
            manifest_pass(
                format!(
                    "The configured {:?} delivery contract uses {:?} CI for the consumer path `{}`",
                    manifest.delivery.mode,
                    manifest.project.ci.provider,
                    manifest.delivery.artifact.locator
                ),
                Some(".opdev/project.yaml"),
            )
        }
        _ => return None,
    };
    Some(evaluation)
}

fn evaluate_testing_policy(rule: &Rule, manifest: &ProjectManifest) -> Option<Evaluation> {
    let evaluation = match rule.id.as_str() {
        "MCD-TEST-001"
            if manifest
                .testing
                .suites
                .iter()
                .any(|suite| suite.stages.contains(&TestStage::PreMerge)) =>
        {
            manifest_pass(
                "At least one canonical suite is required before integration".into(),
                Some(".opdev/project.yaml"),
            )
        }
        "MCD-TEST-002"
            if manifest
                .testing
                .suites
                .iter()
                .any(|suite| suite.stages.contains(&TestStage::PostMerge)) =>
        {
            manifest_pass(
                "At least one canonical suite is required on integrated trunk".into(),
                Some(".opdev/project.yaml"),
            )
        }
        "OPDEV-TEST-001" if !manifest.quality.risks.is_empty() => manifest_pass(
            format!(
                "{} quality risks are declared",
                manifest.quality.risks.len()
            ),
            Some(".opdev/project.yaml"),
        ),
        "OPDEV-TEST-001" => migration("Declare the quality risks that drive verification"),
        "OPDEV-TEST-003" => manifest_pass(
            "The project contract requires tests for behavioral changes".into(),
            Some(".opdev/project.yaml"),
        ),
        "OPDEV-TEST-004" => manifest_pass(
            "The project contract requires regression protection or a specific justification"
                .into(),
            Some(".opdev/project.yaml"),
        ),
        "OPDEV-TEST-005" => manifest_pass(
            "Retry visibility and owned, expiring quarantine are mandatory".into(),
            Some(".opdev/project.yaml"),
        ),
        "OPDEV-TEST-006" if manifest.testing.coverage.mode == CoverageMode::Unconfigured => {
            not_applicable("The project contract does not declare coverage collection")
        }
        "OPDEV-TEST-006" => manifest_pass(
            format!(
                "Coverage is declared as {:?} risk evidence",
                manifest.testing.coverage.mode
            ),
            Some(".opdev/project.yaml"),
        ),
        _ => return None,
    };
    Some(evaluation)
}

fn evaluate_applicability(rule: &Rule, manifest: &ProjectManifest) -> Option<Evaluation> {
    let evaluation = match rule.id.as_str() {
        "OPDEV-OPS-001"
            if manifest.operations.health_evidence.is_some()
                && manifest.operations.observability_authority.is_some() =>
        {
            manifest_pass(
                "Health evidence and an observability authority are declared".into(),
                Some(".opdev/project.yaml"),
            )
        }
        "OPDEV-EXT-001" if manifest.extensions.checks.is_empty() => {
            not_applicable("No project extensions are declared")
        }
        "OPDEV-EXT-001" => manifest_pass(
            "Extensions are additive project checks and cannot replace core rule results".into(),
            Some(".opdev/project.yaml"),
        ),
        _ => return None,
    };
    Some(evaluation)
}

fn default_evaluation(rule: &Rule, manifest: &ProjectManifest) -> Evaluation {
    if is_delivery_rule(rule.id.as_str())
        && manifest.delivery.status == DeliveryStatus::MigrationRequired
    {
        migration("Delivery is explicitly marked migration_required")
    } else {
        (
            Outcome::Unverified,
            VerificationSource::Catalog,
            Vec::new(),
            Some(format!(
                "This evaluator has no sufficient evidence for: {}",
                rule.applicability
            )),
        )
    }
}

fn manifest_pass(summary: String, location: Option<&str>) -> Evaluation {
    (
        Outcome::Passed,
        VerificationSource::Manifest,
        vec![Evidence {
            kind: "manifest".into(),
            summary,
            location: location.map(ToOwned::to_owned),
        }],
        None,
    )
}

fn migration(diagnostic: &str) -> Evaluation {
    (
        Outcome::MigrationRequired,
        VerificationSource::Manifest,
        Vec::new(),
        Some(diagnostic.into()),
    )
}

fn not_applicable(summary: &str) -> Evaluation {
    (
        Outcome::NotApplicable,
        VerificationSource::Manifest,
        vec![Evidence {
            kind: "applicability".into(),
            summary: summary.into(),
            location: Some(".opdev/project.yaml".into()),
        }],
        None,
    )
}

fn is_delivery_rule(id: &str) -> bool {
    matches!(
        id,
        "MCD-DELIVERY-001"
            | "MCD-PIPELINE-001"
            | "MCD-ARTIFACT-001"
            | "MCD-ARTIFACT-002"
            | "MCD-ENV-001"
            | "MCD-RECOVERY-001"
            | "MCD-CONFIG-002"
    )
}

fn run_suites(
    root: &Path,
    manifest: &ProjectManifest,
    stage: TestStage,
    junit: &[JunitBinding],
) -> Vec<CheckResult> {
    manifest
        .testing
        .suites
        .iter()
        .filter(|suite| suite.stages.contains(&stage))
        .map(|suite| {
            if let Some(binding) = junit.iter().find(|binding| binding.suite == suite.id) {
                return run_junit_suite(root, manifest, binding, stage);
            }
            let command = &manifest.commands[&suite.command];
            execution_result(
                suite.id.clone(),
                CheckKind::Suite,
                true,
                gates_for_test_stage(stage),
                execute(root, command, None),
            )
        })
        .collect()
}

fn junit_not_run(binding: &JunitBinding, stage: TestStage) -> CheckResult {
    CheckResult {
        id: binding.suite.clone(),
        kind: CheckKind::Suite,
        blocking: true,
        gates: gates_for_test_stage(stage),
        outcome: Outcome::Unverified,
        summary: "Required JUnit execution evidence was not collected; qualification is unverified"
            .into(),
        evidence: vec![],
        stdout: None,
        stderr: None,
        duration_ms: None,
    }
}

fn run_junit_suite(
    root: &Path,
    manifest: &ProjectManifest,
    binding: &JunitBinding,
    stage: TestStage,
) -> CheckResult {
    let mut result = junit_not_run(binding, stage);
    // Unique process-local observation identity; not a CI identity or an attestation.
    let identity = tempfile::Builder::new()
        .prefix("opdev-attempt-")
        .rand_bytes(24)
        .tempfile();
    let Ok(identity) = identity else {
        result.outcome = Outcome::Error;
        result.summary = "Could not allocate an execution identity; no command was run".into();
        return result;
    };
    result.evidence.push(Evidence {
        kind: "test_execution_attempt".into(),
        summary: identity
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        location: None,
    });
    match crate::test_execution::observe_with_manifest(
        root,
        &binding.suite,
        &binding.path,
        Some(manifest),
    ) {
        Ok(receipt) => {
            result.outcome = match receipt.outcome {
                Outcome::Passed => Outcome::Unverified,
                other => other,
            };
            result.duration_ms = receipt.duration_ms;
            result.summary = "Command/report observations retained; complete retry/quarantine history and CI identity remain unverified".into();
            if let Ok(summary) = serde_json::to_string(&receipt) {
                result.evidence.push(Evidence {
                    kind: "test_execution_receipt_v1".into(),
                    summary,
                    location: None,
                });
            } else {
                if result.outcome != Outcome::Failed {
                    result.outcome = Outcome::Error;
                }
                result.stderr = Some("Could not encode the execution receipt".into());
            }
        }
        Err(error) => {
            result.outcome = Outcome::Error;
            result.summary = "JUnit execution precondition failed; no command was run".into();
            result.stderr = Some(format!("{error:#}"));
        }
    }
    result
}

fn run_extensions(
    root: &Path,
    manifest: &ProjectManifest,
    stage: ExtensionStage,
) -> Result<Vec<CheckResult>, EvaluationError> {
    manifest
        .extensions
        .checks
        .iter()
        .filter(|check| check.stage == stage)
        .map(|check| run_extension(root, manifest, check))
        .collect()
}

fn run_extension(
    root: &Path,
    manifest: &ProjectManifest,
    check: &ExtensionCheck,
) -> Result<CheckResult, EvaluationError> {
    let request = ExtensionRequest {
        protocol_version: EXTENSION_PROTOCOL_VERSION.into(),
        check_id: check.id.clone(),
        project_root: root.display().to_string(),
        stage: extension_stage_name(check.stage).into(),
    };
    let input = serde_json::to_vec(&request)?;
    let mut command = manifest.commands[&check.command].clone();
    if check.timeout_seconds.is_some() {
        command.timeout_seconds = check.timeout_seconds;
    }
    let gates = gates_for_extension_stage(check.stage);
    let execution = match execute(root, &command, Some(&input)) {
        Ok(execution) => execution,
        Err(error) => {
            return Ok(command_error_result(
                check.id.clone(),
                CheckKind::Extension,
                check.blocking,
                gates,
                &error,
            ));
        }
    };
    if execution.timed_out || execution.exit_code != Some(0) {
        return Ok(execution_result(
            check.id.clone(),
            CheckKind::Extension,
            check.blocking,
            gates,
            Ok(execution),
        ));
    }
    let response = serde_json::from_str::<ExtensionResponse>(&execution.stdout);
    let Ok(response) = response else {
        return Ok(CheckResult {
            id: check.id.clone(),
            kind: CheckKind::Extension,
            blocking: check.blocking,
            gates,
            outcome: Outcome::Error,
            summary: "extension returned invalid JSON protocol output".into(),
            evidence: Vec::new(),
            stdout: Some(execution.stdout),
            stderr: optional_text(execution.stderr),
            duration_ms: Some(execution.duration_ms),
        });
    };
    if response.protocol_version != EXTENSION_PROTOCOL_VERSION || response.summary.trim().is_empty()
    {
        return Ok(CheckResult {
            id: check.id.clone(),
            kind: CheckKind::Extension,
            blocking: check.blocking,
            gates,
            outcome: Outcome::Error,
            summary: "extension response has an incompatible protocol version or empty summary"
                .into(),
            evidence: response.evidence,
            stdout: None,
            stderr: optional_text(execution.stderr),
            duration_ms: Some(execution.duration_ms),
        });
    }
    Ok(CheckResult {
        id: check.id.clone(),
        kind: CheckKind::Extension,
        blocking: check.blocking,
        gates,
        outcome: response.outcome,
        summary: response.summary,
        evidence: response.evidence,
        stdout: None,
        stderr: response
            .diagnostic
            .or_else(|| optional_text(execution.stderr)),
        duration_ms: Some(execution.duration_ms),
    })
}

fn execution_result(
    id: String,
    kind: CheckKind,
    blocking: bool,
    gates: Vec<Gate>,
    result: Result<Execution, CommandError>,
) -> CheckResult {
    match result {
        Ok(execution) => {
            let outcome = if execution.timed_out {
                Outcome::Error
            } else if execution.exit_code == Some(0) {
                Outcome::Passed
            } else {
                Outcome::Failed
            };
            let summary = if execution.timed_out {
                "command exceeded its declared timeout".into()
            } else {
                format!("command exited with {:?}", execution.exit_code)
            };
            CheckResult {
                id,
                kind,
                blocking,
                gates,
                outcome,
                summary,
                evidence: vec![Evidence {
                    kind: "command".into(),
                    summary: format!(
                        "exit={:?}; duration_ms={}",
                        execution.exit_code, execution.duration_ms
                    ),
                    location: None,
                }],
                stdout: optional_text(execution.stdout),
                stderr: optional_text(execution.stderr),
                duration_ms: Some(execution.duration_ms),
            }
        }
        Err(error) => command_error_result(id, kind, blocking, gates, &error),
    }
}

fn command_error_result(
    id: String,
    kind: CheckKind,
    blocking: bool,
    gates: Vec<Gate>,
    error: &CommandError,
) -> CheckResult {
    CheckResult {
        id,
        kind,
        blocking,
        gates,
        outcome: Outcome::Error,
        summary: "command could not produce a verdict".into(),
        evidence: Vec::new(),
        stdout: None,
        stderr: Some(error.to_string()),
        duration_ms: None,
    }
}

fn optional_text(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn gates_for_test_stage(stage: TestStage) -> Vec<Gate> {
    match stage {
        TestStage::Local => vec![Gate::Development],
        TestStage::PreMerge => vec![Gate::Integration],
        TestStage::PostMerge | TestStage::Package | TestStage::Delivery | TestStage::Recovery => {
            vec![Gate::Delivery]
        }
        TestStage::Scheduled | TestStage::Evaluation => vec![Gate::Compliance],
    }
}

fn gates_for_extension_stage(stage: ExtensionStage) -> Vec<Gate> {
    match stage {
        ExtensionStage::Specify | ExtensionStage::Design | ExtensionStage::Verify => {
            vec![Gate::Development]
        }
        ExtensionStage::PreMerge => vec![Gate::Integration],
        ExtensionStage::PostMerge
        | ExtensionStage::Package
        | ExtensionStage::Deliver
        | ExtensionStage::Smoke
        | ExtensionStage::Recover => vec![Gate::Delivery],
        ExtensionStage::Evaluate => vec![Gate::Compliance],
    }
}

fn extension_stage_name(stage: ExtensionStage) -> &'static str {
    match stage {
        ExtensionStage::Specify => "specify",
        ExtensionStage::Design => "design",
        ExtensionStage::Verify => "verify",
        ExtensionStage::PreMerge => "pre_merge",
        ExtensionStage::PostMerge => "post_merge",
        ExtensionStage::Package => "package",
        ExtensionStage::Deliver => "deliver",
        ExtensionStage::Smoke => "smoke",
        ExtensionStage::Recover => "recover",
        ExtensionStage::Evaluate => "evaluate",
    }
}

fn aggregate_gates(
    catalog: &RuleCatalog,
    results: &[RuleResult],
    checks: &[CheckResult],
) -> Vec<GateVerdict> {
    [
        Gate::Development,
        Gate::Integration,
        Gate::Delivery,
        Gate::Compliance,
    ]
    .into_iter()
    .map(|gate| {
        let blocking_rules: Vec<_> = catalog
            .rules
            .iter()
            .zip(results)
            .filter(|(rule, result)| {
                rule.gates.contains(&gate) && !result.outcome.satisfies_required_rule()
            })
            .map(|(_, result)| result.rule_id.clone())
            .collect();
        let blocking_checks: Vec<_> = checks
            .iter()
            .filter(|check| {
                check.blocking
                    && check.gates.contains(&gate)
                    && !check.outcome.satisfies_required_rule()
            })
            .map(|check| check.id.clone())
            .collect();
        let verdict = if blocking_rules.is_empty() && blocking_checks.is_empty() {
            AggregateVerdict::Passed
        } else {
            AggregateVerdict::Blocked
        };
        GateVerdict {
            gate,
            verdict,
            blocking_rules,
            blocking_checks,
        }
    })
    .collect()
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use opdev_project::ProjectKind;
    use std::collections::BTreeMap;

    use super::*;
    use opdev_project::{
        Artifact, Assurance, ChangeTests, CiConfig, CommandSpec, Context, Coverage, Delivery,
        DeliveryMode, Environment, EscapedDefectRegressions, Extensions, FlakePolicy, Operations,
        Profile, Project, Quality, QualityRisk, Recovery, RecoveryStrategy, Testing,
    };

    #[test]
    fn software_kind_and_missing_configuration_do_not_prove_inapplicability()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let mut project = manifest();
        for kind in [ProjectKind::Plugin, ProjectKind::Library, ProjectKind::Web] {
            project.project.kind = kind;
            let mut options = CheckOptions::local();
            options.execute_checks = false;
            let report = evaluate(root.path(), &project, options)?;
            for id in [
                "MCD-TRUNK-001",
                "OPDEV-A11Y-001",
                "OPDEV-OPS-001",
                "OPDEV-EVAL-001",
            ] {
                assert_eq!(
                    report
                        .rules
                        .iter()
                        .find(|r| r.rule_id.as_str() == id)
                        .ok_or("rule")?
                        .outcome,
                    Outcome::Unverified,
                    "{kind:?} {id}"
                );
            }
        }
        Ok(())
    }

    fn manifest() -> ProjectManifest {
        ProjectManifest {
            schema: 1,
            project: Project {
                kind: ProjectKind::Library,
                trunk: "main".into(),
                ci: CiConfig {
                    provider: CiProvider::Gitlab,
                    remote: None,
                },
            },
            authorities: BTreeMap::from([(
                "implementation".into(),
                opdev_project::AuthorityRef {
                    kind: opdev_project::AuthorityKind::Path,
                    location: ".".into(),
                },
            )]),
            commands: BTreeMap::from([(
                "check".into(),
                CommandSpec {
                    argv: vec!["opdev-test-command-does-not-exist".into()],
                    working_directory: None,
                    timeout_seconds: Some(1),
                },
            )]),
            quality: Quality {
                risks: vec![QualityRisk::Functional],
            },
            testing: Testing {
                strategy_authority: None,
                change_tests: ChangeTests::Required,
                escaped_defect_regressions: EscapedDefectRegressions::RequiredOrJustified,
                flake_policy: FlakePolicy {
                    retries_visible: true,
                    quarantine_requires_owner_issue_expiry: true,
                },
                coverage: Coverage {
                    mode: CoverageMode::Unconfigured,
                    threshold: None,
                },
                suites: vec![opdev_project::TestSuite {
                    id: "check".into(),
                    command: "check".into(),
                    stages: vec![TestStage::Local],
                }],
            },
            delivery: Delivery {
                status: DeliveryStatus::MigrationRequired,
                mode: DeliveryMode::Publish,
                artifact: Artifact {
                    kind: "package".into(),
                    locator: "registry:unconfigured".into(),
                },
                environments: Vec::<Environment>::new(),
                recovery: Recovery {
                    strategy: RecoveryStrategy::Unconfigured,
                    command: None,
                },
            },
            operations: Operations::default(),
            assurance: Assurance {
                profiles: vec![Profile {
                    name: "opdev-core".into(),
                    version: "1".into(),
                    level: None,
                }],
            },
            extensions: Extensions::default(),
            context: Context {
                always: Vec::new(),
                routes: BTreeMap::new(),
            },
        }
    }

    #[test]
    fn missing_suite_program_is_an_error_and_blocks_development()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let report = evaluate(directory.path(), &manifest(), CheckOptions::local())?;
        assert_eq!(report.checks[0].outcome, Outcome::Error);
        assert!(!report.gate_passed(Gate::Development));
        Ok(())
    }

    #[test]
    fn every_catalog_rule_has_exactly_one_result() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let report = evaluate(
            directory.path(),
            &manifest(),
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::local()
            },
        )?;
        assert_eq!(report.rules.len(), embedded_catalog()?.rules.len());
        assert!(
            report
                .rules
                .iter()
                .any(|result| result.outcome == Outcome::Unverified)
        );
        Ok(())
    }

    #[test]
    fn configured_delivery_declares_the_single_ci_governed_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let mut project = manifest();
        project.delivery.status = DeliveryStatus::Configured;
        project.delivery.environments = vec![Environment {
            name: "consumer-matrix".into(),
            production_like: true,
        }];
        project.delivery.recovery.strategy = RecoveryStrategy::ForwardFix;
        let report = evaluate(
            directory.path(),
            &project,
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::local()
            },
        )?;
        let Some(result) = report
            .rules
            .iter()
            .find(|result| result.rule_id.as_str() == "MCD-DELIVERY-001")
        else {
            return Err(std::io::Error::other("missing delivery rule result").into());
        };
        assert_eq!(result.outcome, Outcome::Passed);
        assert_eq!(result.verifier, VerificationSource::Manifest);
        Ok(())
    }

    #[test]
    fn report_matches_its_json_schema() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let report = evaluate(
            directory.path(),
            &manifest(),
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::local()
            },
        )?;
        let value = serde_json::to_value(report)?;
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/report.schema.json"))?;
        let validator = jsonschema::validator_for(&schema)?;
        let errors: Vec<_> = validator.iter_errors(&value).collect();
        assert!(errors.is_empty(), "schema errors: {errors:#?}");
        Ok(())
    }
}
