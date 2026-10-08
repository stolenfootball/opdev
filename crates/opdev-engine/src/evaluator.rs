use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use opdev_core::embedded_catalog;
use opdev_core::{
    AggregateVerdict, EXTENSION_PROTOCOL_VERSION, Evidence, ExtensionRequest, ExtensionResponse,
    Gate, GateVerdict, Outcome, Rule, RuleCatalog, RuleResult, VerificationSource,
};
use opdev_project::{
    CiProvider, CoverageMode, DeliveryStatus, EVIDENCE_PATH, EvidenceAssertion, EvidenceLedger,
    ExtensionCheck, ExtensionStage, ProjectManifest, TestStage, staged_fingerprint,
};
use thiserror::Error;

use crate::command::{CommandError, Execution, execute};
use crate::plan::{extension_command, selected_extensions, selected_suites};
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
    /// Invalid or unsupported assessment mapping.
    #[error("could not evaluate the selected assessment: {0}")]
    Profile(#[from] opdev_core::ProfileError),
    /// A report cannot be aggregated under a different or incomplete policy.
    #[error("cannot aggregate report: {0}")]
    Report(String),
    /// Authenticated execution no longer describes this evaluation subject.
    #[error("saved execution cannot qualify this change: {0}")]
    ExecutionBinding(String),
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
    evaluate_inner(root, manifest, options, None)
}

/// Evaluate with provider-authenticated same-run executions. No provider calls
/// occur here. With `execute_checks: false`, no canonical/extension commands run.
/// With execution enabled, only selected checks missing from the validated set run.
/// Historical diagnostic receipts cannot construct the required sealed input.
///
/// # Errors
/// Rejects changed source, command, configuration or stage before execution.
pub fn evaluate_with_executions(
    root: &Path,
    manifest: &ProjectManifest,
    options: CheckOptions,
    executions: &crate::ValidatedExecutions,
) -> Result<CheckReport, EvaluationError> {
    crate::execution_record::current_subject(root, manifest, options.test_stage, executions, true)
        .map_err(EvaluationError::ExecutionBinding)?;
    evaluate_inner(root, manifest, options, Some(executions))
}

fn evaluate_inner(
    root: &Path,
    manifest: &ProjectManifest,
    options: CheckOptions,
    executions: Option<&crate::ValidatedExecutions>,
) -> Result<CheckReport, EvaluationError> {
    let catalog = manifest.catalog()?;
    let evaluated_at = unix_timestamp();
    let subject = root.display().to_string();
    let acceptance_ledger = EvidenceLedger::load_optional(root, &catalog)?;
    let initial_fingerprint = staged_fingerprint(root);
    let acceptance_fingerprint = initial_fingerprint.as_ref().ok();
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
    explain_source_gap(&mut rules, initial_fingerprint.as_ref().err());
    apply_workflow_contradictions(root, manifest, &mut rules)?;
    let checks = collect_checks(root, manifest, options, executions)?;
    if let Some(executions) = executions {
        crate::execution_record::current_subject(
            root,
            manifest,
            options.test_stage,
            executions,
            false,
        )
        .map_err(EvaluationError::ExecutionBinding)?;
    }
    let final_fingerprint = staged_fingerprint(root);
    let fresh = acceptance_fingerprint.is_some()
        && acceptance_fingerprint == final_fingerprint.as_ref().ok()
        && acceptance_ledger == EvidenceLedger::load_optional(root, &catalog)?;
    let source_error = initial_fingerprint
        .as_ref()
        .err()
        .or_else(|| final_fingerprint.as_ref().err());
    let (acceptance_outcome, scope, diagnostic) = if let Some(error) = source_error {
        source_failure(error)
    } else {
        crate::acceptance::evaluate(
            root,
            manifest,
            &checks,
            acceptance_ledger.as_ref(),
            acceptance_fingerprint.map(String::as_str),
            fresh,
            options.test_stage,
        )
    };
    apply_acceptance_results(
        &mut rules,
        acceptance_outcome,
        scope,
        &diagnostic,
        acceptance_fingerprint,
    );
    qualify_test_execution(
        &mut rules,
        &checks,
        manifest,
        options.test_stage,
        acceptance_outcome,
    );
    let mut report = CheckReport {
        engineering: manifest
            .assurance
            .engineering
            .as_ref()
            .map(|p| crate::EngineeringAssessment::requested(p.minimumcd.as_deref())),
        schema: if manifest.assurance.engineering.is_some() {
            2
        } else {
            1
        },
        catalog_version: catalog.catalog_version,
        subject,
        evaluated_at,
        rules,
        checks,
        gates: vec![],
    };
    reaggregate(&mut report)?;
    Ok(report)
}

fn apply_workflow_contradictions(
    root: &Path,
    manifest: &ProjectManifest,
    rules: &mut [RuleResult],
) -> Result<(), EvaluationError> {
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
    Ok(())
}

fn apply_acceptance_results(
    rules: &mut [RuleResult],
    acceptance_outcome: Outcome,
    scope: opdev_project::AcceptanceScope,
    diagnostic: &str,
    acceptance_fingerprint: Option<&String>,
) {
    for result in rules
        .iter_mut()
        .filter(|result| matches!(result.rule_id.as_str(), "OPDEV-TEST-002" | "OPDEV-TEST-003"))
    {
        result.outcome = if result.rule_id.as_str() == "OPDEV-TEST-003"
            && acceptance_outcome == Outcome::Passed
            && scope != opdev_project::AcceptanceScope::Behavioral
        {
            Outcome::NotApplicable
        } else {
            acceptance_outcome
        };
        result.verifier = VerificationSource::Evidence;
        result.diagnostic = Some(diagnostic.to_owned());
        result.evidence.clear();
        if let Some(fingerprint) = &acceptance_fingerprint {
            result.evidence.push(Evidence { kind: "acceptance_subject".into(),
                summary: format!("Evaluated staged fingerprint {fingerprint}; inspect the diagnostic for review status and checks for suite execution"),
                location: Some(EVIDENCE_PATH.into()) });
        }
    }
}

// A policy declaration or caller-written review cannot replace execution at the
// selected boundary. Other stages retain their own evidence, never this run's.
fn qualify_test_execution(
    rules: &mut [RuleResult],
    checks: &[CheckResult],
    manifest: &ProjectManifest,
    stage: TestStage,
    acceptance: Outcome,
) {
    let id = match stage {
        TestStage::PreMerge => "MCD-TEST-001",
        TestStage::PostMerge => "MCD-TEST-002",
        _ => return,
    };
    let Some(rule) = rules.iter_mut().find(|r| r.rule_id.as_str() == id) else {
        return;
    };
    if matches!(
        rule.outcome,
        Outcome::Failed | Outcome::Error | Outcome::MigrationRequired
    ) {
        return;
    }
    let selected: Vec<_> = selected_suites(manifest, stage).collect();
    let outcomes: Vec<_> = selected
        .iter()
        .map(|suite| {
            checks
                .iter()
                .find(|c| c.kind == CheckKind::Suite && c.id == suite.id)
                .map_or(Outcome::Unverified, |c| c.outcome)
        })
        .collect();
    rule.outcome = if outcomes.contains(&Outcome::Failed) {
        Outcome::Failed
    } else if outcomes.contains(&Outcome::Error) {
        Outcome::Error
    } else if selected.is_empty() || outcomes.iter().any(|o| *o != Outcome::Passed) {
        Outcome::Unverified
    } else if acceptance.satisfies_required_rule() {
        Outcome::Passed
    } else {
        acceptance
    };
    rule.verifier = VerificationSource::Command;
    rule.evidence.push(Evidence {
        kind: "observed".into(),
        summary: format!("{stage:?}: selected {} canonical suites; execution results are in checks. Acceptance review outcome: {acceptance:?}", selected.len()),
        location: None,
    });
    rule.diagnostic = Some(if selected.is_empty() {
        format!(
            "No canonical suite is selected for {stage:?}. Declare the checks that verify this boundary; an empty selection is not a pass."
        )
    } else if outcomes.contains(&Outcome::Failed) {
        format!(
            "A required {stage:?} suite failed. Inspect the named check and failing assertion; configuration or review cannot erase this result."
        )
    } else if outcomes.contains(&Outcome::Error) {
        format!(
            "A required {stage:?} check could not complete. Inspect its tool diagnostic, repair the execution problem, and rerun the affected check."
        )
    } else if outcomes.iter().any(|o| *o != Outcome::Passed) {
        format!(
            "Required {stage:?} execution is missing. Inspect the unverified checks and run the required selection, or supply supported same-run evidence. Earlier green runs are not substitutes."
        )
    } else if rule.outcome == Outcome::Passed {
        format!(
            "Selected {stage:?} suites passed with current acceptance evidence. This does not qualify another stage or a release."
        )
    } else {
        format!(
            "Selected {stage:?} suites passed, but current acceptance evidence is not satisfied. Inspect OPDEV-TEST-002 for the missing, stale or contradicted review. Repeating tests alone cannot repair a review gap."
        )
    });
}

fn explain_source_gap(rules: &mut [RuleResult], error: Option<&opdev_project::EvidenceError>) {
    if let Some(error) = error {
        for rule in rules
            .iter_mut()
            .filter(|rule| rule.outcome == Outcome::Unverified)
        {
            rule.diagnostic = Some(format!(
                "{}; change evidence unavailable: {error}",
                rule.diagnostic.as_deref().unwrap_or("Evidence unavailable")
            ));
        }
    }
}

fn collect_checks(
    root: &Path,
    manifest: &ProjectManifest,
    options: CheckOptions,
    executions: Option<&crate::ValidatedExecutions>,
) -> Result<Vec<CheckResult>, EvaluationError> {
    let mut checks = executions.map_or_else(Vec::new, |record| record.checks.clone());
    if options.execute_checks {
        checks.extend(run_missing_suites(
            root,
            manifest,
            options.test_stage,
            &checks,
        ));
        checks.extend(run_extensions(root, manifest, options.extension_stage)?);
    } else {
        for planned in crate::plan_checks(root, manifest, options).commands {
            if !checks
                .iter()
                .any(|check| check.kind == planned.kind && check.id == planned.id)
            {
                checks.push(CheckResult {
                    id: planned.id,
                    kind: planned.kind,
                    blocking: planned.blocking,
                    gates: if planned.kind == CheckKind::Suite {
                        gates_for_test_stage(options.test_stage)
                    } else {
                        gates_for_extension_stage(options.extension_stage)
                    },
                    outcome: Outcome::Unverified,
                    summary:
                        "Required check has no verified execution; evaluation-only did not run it"
                            .into(),
                    evidence: vec![],
                    stdout: None,
                    stderr: None,
                    duration_ms: None,
                });
            }
        }
    }
    Ok(checks)
}

fn source_failure(
    error: &opdev_project::EvidenceError,
) -> (Outcome, opdev_project::AcceptanceScope, String) {
    let outcome = if matches!(
        error,
        opdev_project::EvidenceError::Git(_) | opdev_project::EvidenceError::Read { .. }
    ) {
        Outcome::Error
    } else {
        Outcome::Unverified
    };
    (
        outcome,
        opdev_project::AcceptanceScope::Behavioral,
        format!(
            "Acceptance source identity unavailable: {error}; stage material source or keep generated artifacts outside the checkout, then review evidence"
        ),
    )
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
    let mut catalog = opdev_core::catalog_for_version(report.catalog_version)?;
    if (report.schema, report.catalog_version)
        != if report.engineering.is_some() {
            (2, 3)
        } else {
            (1, 2)
        }
    {
        return Err(EvaluationError::Report(
            "schema, catalog and policy identity do not agree".into(),
        ));
    }
    if let Some(policy) = &report.engineering {
        if policy.version != "1" {
            return Err(EvaluationError::Report(
                "unsupported engineering policy version".into(),
            ));
        }
        for result in &mut report.rules {
            if result.outcome == Outcome::NotApplicable
                && opdev_core::rule_class(result.rule_id.as_str())
                    == Some(opdev_core::RuleClass::Baseline)
            {
                result.outcome = Outcome::Unverified;
                result.diagnostic = Some("This engineering baseline requirement cannot be marked not applicable. Supply evidence of an appropriate implementation; tool choice remains flexible.".into());
            }
        }
        for rule in &mut catalog.rules {
            if opdev_core::rule_class(rule.id.as_str())
                == Some(opdev_core::RuleClass::MinimumcdAssessment)
            {
                rule.gates.clear();
            }
        }
    }
    report.gates = aggregate_gates(&catalog, &report.rules, &report.checks);
    crate::assessment::refresh(report)?;
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
        "MCD-CI-001" if manifest.project.ci.provider != CiProvider::Unconfigured => {
            configured_review(
                format!("CI provider is {:?}", manifest.project.ci.provider),
                Some(".opdev/project.yaml"),
            )
        }
        "MCD-CI-001" => migration("A CI provider must be configured"),
        "MCD-DELIVERY-001" if manifest.delivery.status == DeliveryStatus::Configured => {
            configured_review(
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
            configured_review(
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
            configured_review(
                "At least one canonical suite is required on integrated trunk".into(),
                Some(".opdev/project.yaml"),
            )
        }
        "OPDEV-TEST-001" if !manifest.quality.risks.is_empty() => configured_review(
            format!(
                "{} quality risks are declared",
                manifest.quality.risks.len()
            ),
            Some(".opdev/project.yaml"),
        ),
        "OPDEV-TEST-001" => migration("Declare the quality risks that drive verification"),
        "OPDEV-TEST-004" => configured_review(
            "The project contract requires regression protection or a specific justification"
                .into(),
            Some(".opdev/project.yaml"),
        ),
        "OPDEV-TEST-005" => configured_review(
            "Retry visibility and owned, expiring quarantine are mandatory".into(),
            Some(".opdev/project.yaml"),
        ),
        "OPDEV-TEST-006" if manifest.testing.coverage.mode == CoverageMode::Unconfigured => {
            not_applicable("The project contract does not declare coverage collection")
        }
        "OPDEV-TEST-006" => configured_review(
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
            configured_review(
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
                "OpDev could not confirm this requirement: {}. Missing evidence does not mean the software failed. {}",
                rule.title,
                rule.next_step()
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

fn configured_review(summary: String, location: Option<&str>) -> Evaluation {
    (
        Outcome::Unverified,
        VerificationSource::Manifest,
        vec![Evidence {
            kind: "configured".into(),
            summary,
            location: location.map(ToOwned::to_owned),
        }],
        Some("Configuration is present, but the required behavior has not been verified. Supply current observations and review against this requirement; declaring a policy is not evidence that it worked.".into()),
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

fn run_missing_suites(
    root: &Path,
    manifest: &ProjectManifest,
    stage: TestStage,
    reused: &[CheckResult],
) -> Vec<CheckResult> {
    selected_suites(manifest, stage)
        .filter(|suite| {
            !reused
                .iter()
                .any(|check| check.kind == CheckKind::Suite && check.id == suite.id)
        })
        .map(|suite| {
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

fn run_extensions(
    root: &Path,
    manifest: &ProjectManifest,
    stage: ExtensionStage,
) -> Result<Vec<CheckResult>, EvaluationError> {
    selected_extensions(manifest, stage)
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
    let command = extension_command(manifest, check);
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

pub(crate) fn execution_result(
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
            } else if kind == CheckKind::Extension {
                Outcome::Error
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

pub(crate) fn gates_for_test_stage(stage: TestStage) -> Vec<Gate> {
    match stage {
        TestStage::Local => vec![Gate::Development],
        TestStage::PreMerge => vec![Gate::Integration],
        TestStage::PostMerge => vec![Gate::Integration, Gate::Delivery],
        TestStage::Package | TestStage::Delivery | TestStage::Recovery => {
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
        ExtensionStage::PostMerge => vec![Gate::Integration, Gate::Delivery],
        ExtensionStage::Package
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
            .filter(|rule| {
                rule.gates.contains(&gate)
                    && !results.iter().any(|result| {
                        result.rule_id == rule.id && result.outcome.satisfies_required_rule()
                    })
            })
            .map(|rule| rule.id.clone())
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
    fn post_merge_checks_block_integration_without_importing_delivery_rules()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let mut report = evaluate(
            root.path(),
            &manifest(),
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::pre_merge()
            },
        )?;
        let catalog = embedded_catalog()?;
        for (rule, result) in catalog.rules.iter().zip(&mut report.rules) {
            result.outcome = if rule.gates.contains(&Gate::Delivery)
                && !rule.gates.contains(&Gate::Integration)
            {
                Outcome::Unverified
            } else {
                Outcome::Passed
            };
        }
        for (kind, gates) in [
            (CheckKind::Suite, gates_for_test_stage(TestStage::PostMerge)),
            (
                CheckKind::Extension,
                gates_for_extension_stage(ExtensionStage::PostMerge),
            ),
        ] {
            for outcome in [
                Outcome::Passed,
                Outcome::NotApplicable,
                Outcome::Failed,
                Outcome::Error,
                Outcome::Unverified,
                Outcome::MigrationRequired,
            ] {
                report.checks = vec![CheckResult {
                    id: "post".into(),
                    kind,
                    blocking: true,
                    gates: gates.clone(),
                    outcome,
                    summary: "controlled gate aggregation fixture".into(),
                    evidence: vec![],
                    stdout: None,
                    stderr: None,
                    duration_ms: None,
                }];
                reaggregate(&mut report)?;
                assert_eq!(
                    report.gate_passed(Gate::Integration),
                    outcome.satisfies_required_rule()
                );
                assert!(!report.gate_passed(Gate::Delivery));
                report.checks[0].blocking = false;
                reaggregate(&mut report)?;
                assert!(report.gate_passed(Gate::Integration));
            }
        }
        for stage in [TestStage::Package, TestStage::Delivery, TestStage::Recovery] {
            assert_eq!(gates_for_test_stage(stage), vec![Gate::Delivery]);
        }
        for stage in [
            ExtensionStage::Package,
            ExtensionStage::Deliver,
            ExtensionStage::Smoke,
            ExtensionStage::Recover,
        ] {
            assert_eq!(gates_for_extension_stage(stage), vec![Gate::Delivery]);
        }
        Ok(())
    }

    #[test]
    fn cadence_never_blocks_work_or_merge_but_tests_still_do()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let mut report = evaluate(
            root.path(),
            &manifest(),
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::local()
            },
        )?;
        let catalog = embedded_catalog()?;
        for result in &mut report.rules {
            result.outcome = Outcome::Passed;
        }
        for outcome in [
            Outcome::Failed,
            Outcome::Unverified,
            Outcome::Error,
            Outcome::MigrationRequired,
        ] {
            report
                .rules
                .iter_mut()
                .find(|r| r.rule_id.as_str() == "MCD-TRUNK-003")
                .ok_or("cadence")?
                .outcome = outcome;
            let gates = aggregate_gates(&catalog, &report.rules, &[]);
            for gate in gates {
                assert_eq!(
                    gate.verdict,
                    if gate.gate == Gate::Compliance {
                        AggregateVerdict::Blocked
                    } else {
                        AggregateVerdict::Passed
                    }
                );
            }
        }
        report
            .rules
            .iter_mut()
            .find(|r| r.rule_id.as_str() == "MCD-TEST-001")
            .ok_or("tests")?
            .outcome = Outcome::Failed;
        let gates = aggregate_gates(&catalog, &report.rules, &[]);
        let integration = gates
            .iter()
            .find(|g| g.gate == Gate::Integration)
            .ok_or("integration")?;
        assert_eq!(integration.verdict, AggregateVerdict::Blocked);
        assert!(
            integration
                .blocking_rules
                .iter()
                .any(|r| r.as_str() == "MCD-TEST-001")
        );
        assert!(
            !integration
                .blocking_rules
                .iter()
                .any(|r| r.as_str() == "MCD-TRUNK-003")
        );
        Ok(())
    }

    #[test]
    fn extension_process_failure_is_not_a_test_failure() {
        for exit in [Some(7), None] {
            for (kind, expected) in [
                (CheckKind::Extension, Outcome::Error),
                (CheckKind::Suite, Outcome::Failed),
            ] {
                let result = execution_result(
                    "selected".into(),
                    kind,
                    true,
                    vec![Gate::Integration],
                    Ok(Execution {
                        exit_code: exit,
                        timed_out: false,
                        stdout: "partial output".into(),
                        stderr: "producer failed".into(),
                        duration_ms: 12,
                    }),
                );
                assert_eq!(result.outcome, expected);
                assert_eq!(result.stdout.as_deref(), Some("partial output"));
                assert_eq!(result.stderr.as_deref(), Some("producer failed"));
                assert_eq!(result.duration_ms, Some(12));
                assert!(!result.evidence.is_empty());
            }
        }
    }

    #[test]
    fn passing_extension_cannot_clear_a_core_failure() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let mut report = evaluate(
            root.path(),
            &manifest(),
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::local()
            },
        )?;
        let catalog = embedded_catalog()?;
        let rule = catalog
            .rules
            .iter()
            .find(|r| r.gates.contains(&Gate::Integration))
            .ok_or("rule")?;
        report
            .rules
            .iter_mut()
            .find(|r| r.rule_id == rule.id)
            .ok_or("result")?
            .outcome = Outcome::Failed;
        let extension = execution_result(
            "additional".into(),
            CheckKind::Extension,
            true,
            vec![Gate::Integration],
            Ok(Execution {
                exit_code: Some(0),
                timed_out: false,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 1,
            }),
        );
        let gates = aggregate_gates(&catalog, &report.rules, &[extension]);
        let gate = gates
            .iter()
            .find(|g| g.gate == Gate::Integration)
            .ok_or("gate")?;
        assert_eq!(gate.verdict, AggregateVerdict::Blocked);
        assert!(gate.blocking_rules.contains(&rule.id));
        assert!(gate.blocking_checks.is_empty());
        Ok(())
    }

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
                    qualification: None,
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
                safeguards: None,
                engineering: None,
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
    #[ignore = "spawned only inside an isolated counting fixture"]
    fn counting_child() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::current_dir()?;
        assert!(root.join(".opdev-counting-fixture").is_file());
        let path = root.join(".count");
        let count = std::fs::read_to_string(&path)
            .ok()
            .map(|s| s.parse::<usize>())
            .transpose()?
            .unwrap_or(0);
        std::fs::write(path, (count + 1).to_string())?;
        Ok(())
    }

    fn counting_fixture()
    -> Result<(tempfile::TempDir, ProjectManifest, String), Box<dyn std::error::Error>> {
        use sha2::{Digest, Sha256};
        let root = tempfile::tempdir()?;
        let executable = std::env::current_exe()?;
        let digest = format!("{:x}", Sha256::digest(std::fs::read(&executable)?));
        let mut project = manifest();
        project.commands.get_mut("check").ok_or("command")?.argv = vec![
            executable.to_string_lossy().into(),
            "--exact".into(),
            "evaluator::tests::counting_child".into(),
            "--ignored".into(),
        ];
        project
            .commands
            .get_mut("check")
            .ok_or("command")?
            .timeout_seconds = Some(20);
        project.testing.suites = ["first", "second"]
            .into_iter()
            .map(|id| opdev_project::TestSuite {
                id: id.into(),
                command: "check".into(),
                stages: vec![TestStage::PreMerge, TestStage::PostMerge],
            })
            .collect();
        std::fs::create_dir(root.path().join(".opdev"))?;
        std::fs::write(
            root.path().join(".opdev/project.yaml"),
            serde_json::to_vec(&project)?,
        )?;
        std::fs::write(
            root.path().join(".opdev-counting-fixture"),
            "neutral test fixture",
        )?;
        std::fs::write(root.path().join(".gitignore"), ".count\n")?;
        for args in [
            vec!["init", "-q", "-b", "main"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=Neutral Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-q",
                "-m",
                "fixture",
            ],
        ] {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(root.path())
                .args(args)
                .output()?;
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok((root, project, digest))
    }

    #[test]
    fn evaluation_only_runs_nothing_and_composition_runs_each_missing_suite_once()
    -> Result<(), Box<dyn std::error::Error>> {
        for provider in [
            opdev_project::CiProvider::Gitlab,
            opdev_project::CiProvider::Github,
        ] {
            counting_scenario(provider)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)] // One sequential counter scenario proves no hidden reruns across the full path.
    fn counting_scenario(
        provider: opdev_project::CiProvider,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (root, mut project, digest) = counting_fixture()?;
        project.project.ci.provider = provider;
        let github = provider == opdev_project::CiProvider::Github;
        let revision = crate::test_execution::clean_revision(root.path())?;
        let policy = crate::ExecutionPolicy {
            schema: 1,
            review_reference: "neutral controlled test fixture".into(),
            inputs_complete: true,
            ledger_is_input: false,
            environment: "controlled-native-test-process".into(),
            executor_sha256: digest.clone(),
            github_workflow_id: github.then_some(9),
            producers: vec![crate::ProducerPolicy {
                suite: "first".into(),
                job: "producer".into(),
                executable_sha256: digest,
            }],
        };
        let run = opdev_remote::RunExpectation {
            revision,
            run_id: 1,
            reference: "main".into(),
            source: "push".into(),
            workflow_id: github.then_some(9),
        };
        let bindings = crate::prepare_execution_bindings(
            root.path(),
            &project,
            &policy,
            &run,
            "neutral/project",
            github.then_some(2),
            TestStage::PreMerge,
            &policy.environment,
        )?;
        let produced = crate::run_canonical_producer(root.path(), &project, &bindings[0])?;
        assert_eq!(produced.result.outcome, Outcome::Passed);
        assert!(produced.inputs_unchanged);
        assert_eq!(std::fs::read_to_string(root.path().join(".count"))?, "1");
        // Private test construction models the already authenticated channel;
        // provider parsing/race tests separately exercise that trust boundary.
        let executions = crate::ValidatedExecutions {
            checks: vec![produced.result],
            bindings,
            validated_at: std::time::Instant::now(),
        };
        let report = evaluate_with_executions(
            root.path(),
            &project,
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::pre_merge()
            },
            &executions,
        )?;
        assert_eq!(std::fs::read_to_string(root.path().join(".count"))?, "1");
        assert_eq!(
            report
                .checks
                .iter()
                .find(|c| c.id == "first")
                .ok_or("first")?
                .outcome,
            Outcome::Passed
        );
        assert_eq!(
            report
                .checks
                .iter()
                .find(|c| c.id == "second")
                .ok_or("second")?
                .outcome,
            Outcome::Unverified
        );
        assert!(!report.gate_passed(Gate::Integration));
        let composed = evaluate_with_executions(
            root.path(),
            &project,
            CheckOptions::pre_merge(),
            &executions,
        )?;
        assert_eq!(std::fs::read_to_string(root.path().join(".count"))?, "2");
        assert!(composed.checks.iter().all(|c| c.outcome == Outcome::Passed));
        let fresh = evaluate(root.path(), &project, CheckOptions::pre_merge())?;
        assert_eq!(std::fs::read_to_string(root.path().join(".count"))?, "4");
        assert_eq!(composed.gates, fresh.gates);
        review_correction_does_not_rerun(root.path(), &project, &executions)?;
        let post_merge = CheckOptions {
            test_stage: TestStage::PostMerge,
            extension_stage: ExtensionStage::PostMerge,
            execute_checks: true,
        };
        assert!(evaluate_with_executions(root.path(), &project, post_merge, &executions).is_err());
        assert_eq!(std::fs::read_to_string(root.path().join(".count"))?, "4");
        std::fs::write(root.path().join("changed-input"), "new input")?;
        assert!(
            evaluate_with_executions(
                root.path(),
                &project,
                CheckOptions::pre_merge(),
                &executions
            )
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(root.path().join(".count"))?, "4");
        Ok(())
    }

    fn review_correction_does_not_rerun(
        root: &Path,
        project: &ProjectManifest,
        executions: &crate::ValidatedExecutions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = root.join(opdev_project::EVIDENCE_PATH);
        for work in [
            "neutral initial reference",
            "neutral corrected review reference",
        ] {
            let ledger = serde_json::json!({"schema":2,"project":[],"changes":[{
                "fingerprint":opdev_project::staged_fingerprint(root)?,"work":work,"assertions":[]}]});
            std::fs::write(&path, serde_json::to_vec(&ledger)?)?;
            let report = evaluate_with_executions(
                root,
                project,
                CheckOptions {
                    execute_checks: false,
                    ..CheckOptions::pre_merge()
                },
                executions,
            )?;
            assert_eq!(std::fs::read_to_string(root.join(".count"))?, "4");
            assert_eq!(
                report
                    .checks
                    .iter()
                    .find(|c| c.id == "first")
                    .ok_or("first")?
                    .outcome,
                Outcome::Passed
            );
            assert_eq!(
                report
                    .rules
                    .iter()
                    .find(|r| r.rule_id.as_str() == "OPDEV-TEST-002")
                    .ok_or("acceptance")?
                    .outcome,
                Outcome::Unverified
            );
            assert!(crate::test_execution::clean_execution_revision(root, true).is_err());
        }
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
    fn declared_behavioral_policies_remain_unverified_without_review()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let project = manifest();
        let report = evaluate(
            root.path(),
            &project,
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::local()
            },
        )?;
        for id in [
            "MCD-CI-001",
            "OPDEV-TEST-001",
            "OPDEV-TEST-004",
            "OPDEV-TEST-005",
        ] {
            let rule = report
                .rules
                .iter()
                .find(|r| r.rule_id.as_str() == id)
                .ok_or("rule")?;
            assert_eq!(rule.outcome, Outcome::Unverified, "{id}");
            assert!(rule.evidence.iter().any(|e| e.kind == "configured"), "{id}");
        }
        assert_eq!(
            report
                .rules
                .iter()
                .find(|r| r.rule_id.as_str() == "OPDEV-AUTH-001")
                .ok_or("rule")?
                .outcome,
            Outcome::Passed
        );
        Ok(())
    }

    #[test]
    fn configured_delivery_is_not_observed_delivery() -> Result<(), Box<dyn std::error::Error>> {
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
        assert_eq!(result.outcome, Outcome::Unverified);
        assert_eq!(result.verifier, VerificationSource::Manifest);
        assert_eq!(result.evidence[0].kind, "configured");
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
    #[test]
    fn engineering_assessment_reuses_one_execution_and_missing_evidence_stays_visible()
    -> Result<(), Box<dyn std::error::Error>> {
        let (root, mut project, _) = counting_fixture()?;
        project.schema = 3;
        project.assurance.profiles.clear();
        project.assurance.engineering = Some(opdev_core::EngineeringPolicy {
            version: "1".into(),
            minimumcd: Some("1".into()),
            review_reference: "synthetic decision".into(),
            maintenance_branches: vec![],
        });
        project.testing.suites.truncate(1);
        std::fs::write(root.path().join(".opdev/project.yaml"), project.to_yaml()?)?;
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(root.path())
                .args(["add", "."])
                .status()?
                .success()
        );
        let report = evaluate(root.path(), &project, CheckOptions::pre_merge())?;
        assert_eq!(std::fs::read_to_string(root.path().join(".count"))?, "1");
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].outcome, Outcome::Passed);
        assert!(
            !report.gate_passed(Gate::Integration),
            "a command pass does not prove review/CI"
        );
        assert_eq!(
            report
                .engineering
                .as_ref()
                .and_then(|p| p.minimumcd.as_ref())
                .ok_or("assessment")?
                .verdict,
            AggregateVerdict::Blocked
        );
        let mut projected = report.clone();
        let projection_started = std::time::Instant::now();
        for _ in 0..100 {
            reaggregate(&mut projected)?;
        }
        eprintln!(
            "100 engineering/MinimumCD projections: {:?}; additional command executions: 0",
            projection_started.elapsed()
        );
        assert_eq!(projected, report);
        assert_eq!(std::fs::read_to_string(root.path().join(".count"))?, "1");
        let no_exec = evaluate(
            root.path(),
            &project,
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::pre_merge()
            },
        )?;
        assert_eq!(no_exec.checks[0].outcome, Outcome::Unverified);
        assert_eq!(std::fs::read_to_string(root.path().join(".count"))?, "1");
        Ok(())
    }
}
