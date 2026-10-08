use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail, ensure};
use clap::Args;
use opdev_engine::{CheckOptions, CheckReport, ExecutionPolicy, prepare_execution_bindings};
use opdev_project::{CiProvider, ProjectManifest, TestStage};
use opdev_remote::RunExpectation;

#[derive(Debug, Args)]
pub struct ExecuteArgs {
    /// Directory in the initialized project.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Reviewed tracked execution-policy JSON, relative to the project root.
    #[arg(long)]
    policy: PathBuf,
    /// Canonical suite selected for this producer job.
    #[arg(long)]
    suite: String,
    /// Actual nonsecret environment identity from reviewed CI configuration.
    #[arg(long)]
    environment: String,
    /// Run integrated-trunk checks instead of pre-merge checks.
    #[arg(long)]
    post_merge: bool,
}

struct ContextIdentity {
    run: RunExpectation,
    repository: String,
    attempt: Option<u64>,
}

fn required(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| {
        format!("current CI identity is missing {name}; saved CI execution cannot be used locally")
    })?;
    ensure!(
        !value.trim().is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control),
        "current CI identity {name} is invalid"
    );
    Ok(value)
}

fn number(name: &str) -> Result<u64> {
    let value = required(name)?
        .parse::<u64>()
        .with_context(|| format!("current CI identity {name} must be a positive integer"))?;
    ensure!(value > 0, "current CI identity {name} must be positive");
    Ok(value)
}

fn current_run(manifest: &ProjectManifest, policy: &ExecutionPolicy) -> Result<ContextIdentity> {
    match manifest.project.ci.provider {
        CiProvider::Gitlab => {
            ensure!(
                required("GITLAB_CI")? == "true",
                "execution reuse is restricted to the current GitLab pipeline"
            );
            ensure!(
                policy.github_workflow_id.is_none(),
                "GitLab policy must not name a GitHub workflow"
            );
            Ok(ContextIdentity {
                repository: required("CI_PROJECT_PATH")?,
                attempt: None,
                run: RunExpectation {
                    revision: required("CI_COMMIT_SHA")?,
                    run_id: number("CI_PIPELINE_ID")?,
                    reference: required("CI_COMMIT_REF_NAME")?,
                    source: required("CI_PIPELINE_SOURCE")?,
                    workflow_id: None,
                },
            })
        }
        CiProvider::Github => {
            ensure!(
                required("GITHUB_ACTIONS")? == "true",
                "execution reuse is restricted to the current GitHub Actions run"
            );
            ensure!(
                policy.github_workflow_id.is_some_and(|id| id > 0),
                "reviewed GitHub workflow ID is missing"
            );
            let event = required("GITHUB_EVENT_NAME")?;
            // Pull-request checkout SHA can be a synthetic merge while the run
            // API identifies the head commit. Until explicitly modeled, reject
            // this mismatch rather than use a same-SHA or branch-name shortcut.
            ensure!(
                event == "push" || event == "workflow_dispatch",
                "this execution-reuse version supports GitHub push and workflow_dispatch runs only; use fresh execution for other event subjects"
            );
            Ok(ContextIdentity {
                repository: required("GITHUB_REPOSITORY")?,
                attempt: Some(number("GITHUB_RUN_ATTEMPT")?),
                run: RunExpectation {
                    revision: required("GITHUB_SHA")?,
                    run_id: number("GITHUB_RUN_ID")?,
                    reference: required("GITHUB_REF_NAME")?,
                    source: event,
                    workflow_id: policy.github_workflow_id,
                },
            })
        }
        _ => bail!("same-run execution reuse requires a supported GitLab or GitHub provider"),
    }
}

pub fn execute(args: &ExecuteArgs) -> Result<ExitCode> {
    let (root, manifest) = super::load_project(&args.root)?;
    let policy = ExecutionPolicy::load(&root, &args.policy)?;
    let context = current_run(&manifest, &policy)?;
    let stage = if args.post_merge {
        TestStage::PostMerge
    } else {
        TestStage::PreMerge
    };
    let bindings = prepare_execution_bindings(
        &root,
        &manifest,
        &policy,
        &context.run,
        &context.repository,
        context.attempt,
        stage,
        &args.environment,
    )?;
    let binding = bindings
        .iter()
        .find(|b| b.suite == args.suite)
        .context("suite has no reviewed producer in this stage")?;
    if manifest.project.ci.provider == CiProvider::Gitlab {
        ensure!(
            required("CI_JOB_NAME")? == binding.producer,
            "current job differs from the reviewed producer"
        );
    }
    let record = opdev_engine::run_canonical_producer(&root, &manifest, binding)?;
    // A single JSON line escapes captured child output; child-generated marker
    // text cannot become a separate provider-log record through ordinary stdout.
    println!("OPDEV_EXECUTION_V1 {}", serde_json::to_string(&record)?);
    Ok(match record.result.outcome {
        opdev_core::Outcome::Error => ExitCode::from(2),
        opdev_core::Outcome::Passed if record.inputs_unchanged => ExitCode::SUCCESS,
        _ => ExitCode::from(1),
    })
}

pub fn evaluate(
    root: &Path,
    manifest: &ProjectManifest,
    options: CheckOptions,
    policy_path: &Path,
    environment: &str,
    review: Option<&opdev_engine::ValidatedReview>,
) -> Result<CheckReport> {
    let policy = ExecutionPolicy::load(root, policy_path)?;
    let context = current_run(manifest, &policy)?;
    let bindings = prepare_execution_bindings(
        root,
        manifest,
        &policy,
        &context.run,
        &context.repository,
        context.attempt,
        options.test_stage,
        environment,
    )?;
    let required: Vec<_> = bindings
        .iter()
        .map(|b| b.producer.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let snapshot = match opdev_remote::observe_producers(manifest, &context.run, &required)? {
        Ok(snapshot) => snapshot,
        Err(reason) => return unavailable(root, manifest, options, &reason, review),
    };
    let executions = match opdev_engine::validate_producer_records(&snapshot, &bindings) {
        Ok(executions) => executions,
        Err(reason) => return unavailable(root, manifest, options, &reason, review),
    };
    let ledger = std::fs::read(root.join(opdev_project::EVIDENCE_PATH)).ok();
    let mut report = if let Some(review) = review {
        opdev_engine::evaluate_with_review(root, manifest, options, review, Some(&executions))
    } else {
        opdev_engine::evaluate_with_executions(root, manifest, options, &executions)
    }
    .context("same-run evaluation failed")?;
    // Missing checks may be legitimately long. Re-observe the selected producer
    // attempts afterward instead of expiring their results and rerunning tests.
    let refreshed = opdev_remote::observe_producers(manifest, &context.run, &required)?
        .and_then(|snapshot| opdev_engine::validate_producer_records(&snapshot, &bindings));
    let fresh = refreshed.and_then(|current| {
        current.verify_subject(root, manifest, options.test_stage)?;
        for observed in current.checks().iter().filter(|check| {
            matches!(
                check.outcome,
                opdev_core::Outcome::Failed | opdev_core::Outcome::Error
            )
        }) {
            if let Some(previous) = report
                .checks
                .iter_mut()
                .find(|check| check.kind == observed.kind && check.id == observed.id)
            {
                *previous = observed.clone();
            }
        }
        if current.checks() != executions.checks()
            || ledger != std::fs::read(root.join(opdev_project::EVIDENCE_PATH)).ok()
        {
            return Err("Producer attempt or acceptance record changed during evaluation".into());
        }
        Ok(())
    });
    if let Err(reason) = fresh {
        invalidate_reuse(&mut report, &bindings, &reason)?;
    }
    Ok(report)
}

fn invalidate_reuse(
    report: &mut CheckReport,
    bindings: &[opdev_engine::ExecutionBinding],
    reason: &str,
) -> Result<()> {
    for check in &mut report.checks {
        if check.kind == opdev_engine::CheckKind::Suite
            && bindings.iter().any(|b| b.suite == check.id)
            && check.outcome == opdev_core::Outcome::Passed
        {
            check.outcome = opdev_core::Outcome::Unverified;
            check.summary = format!(
                "Current execution evidence changed or became unavailable: {reason}. Already executed checks were not rerun"
            );
        }
    }
    for rule in &mut report.rules {
        if matches!(rule.rule_id.as_str(), "OPDEV-TEST-002" | "OPDEV-TEST-003")
            && rule.outcome == opdev_core::Outcome::Passed
        {
            rule.outcome = opdev_core::Outcome::Unverified;
            rule.diagnostic = Some("Execution or acceptance evidence changed during the final provider observation; this result cannot qualify the change".into());
        }
    }
    opdev_engine::reaggregate(report)?;
    Ok(())
}

fn unavailable(
    root: &Path,
    manifest: &ProjectManifest,
    mut options: CheckOptions,
    reason: &str,
    review: Option<&opdev_engine::ValidatedReview>,
) -> Result<CheckReport> {
    options.execute_checks = false;
    let mut report = if let Some(review) = review {
        opdev_engine::evaluate_with_review(root, manifest, options, review, None)?
    } else {
        opdev_engine::evaluate(root, manifest, options)?
    };
    let gates = if options.test_stage == TestStage::PostMerge {
        vec![opdev_core::Gate::Integration, opdev_core::Gate::Delivery]
    } else {
        vec![opdev_core::Gate::Integration]
    };
    for command in opdev_engine::plan_checks(root, manifest, options).commands {
        report
            .checks
            .retain(|check| check.kind != command.kind || check.id != command.id);
        report.checks.push(opdev_engine::CheckResult { id: command.id, kind: command.kind,
            blocking: command.blocking, gates: gates.clone(), outcome: opdev_core::Outcome::Unverified,
            summary: format!("Saved execution could not be verified: {reason}. No project command ran. Omit --reuse-ci-policy to explicitly run fresh checks"),
            evidence: vec![], stdout: None, stderr: None, duration_ms: None });
    }
    opdev_engine::reaggregate(&mut report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opdev_core::{Gate, Outcome};
    use opdev_engine::{CheckKind, CheckResult, ExecutionBinding};

    #[test]
    fn unavailable_post_merge_execution_blocks_integration_and_delivery() -> Result<()> {
        let root = tempfile::tempdir()?;
        std::fs::create_dir(root.path().join(".git"))?;
        let mut manifest = opdev_project::discover(root.path())?.manifest;
        manifest.commands.insert(
            "test".into(),
            opdev_project::CommandSpec {
                argv: vec!["must-not-run".into()],
                working_directory: None,
                timeout_seconds: None,
            },
        );
        manifest.testing.suites = vec![opdev_project::TestSuite {
            id: "post".into(),
            command: "test".into(),
            stages: vec![TestStage::PostMerge],
        }];
        let report = unavailable(
            root.path(),
            &manifest,
            CheckOptions {
                test_stage: TestStage::PostMerge,
                extension_stage: opdev_project::ExtensionStage::PostMerge,
                execute_checks: true,
            },
            "controlled unavailable provider",
            None,
        )?;
        let check = report
            .checks
            .iter()
            .find(|check| check.id == "post")
            .context("selected check")?;
        assert_eq!(check.outcome, Outcome::Unverified);
        assert_eq!(check.gates, vec![Gate::Integration, Gate::Delivery]);
        assert!(!report.gate_passed(Gate::Integration));
        assert!(!report.gate_passed(Gate::Delivery));
        Ok(())
    }

    #[test]
    fn expired_observations_invalidate_only_reused_passes_without_hiding_failures() -> Result<()> {
        let root = tempfile::tempdir()?;
        ensure!(
            std::process::Command::new("git")
                .args(["init", "-q"])
                .arg(root.path())
                .status()?
                .success(),
            "fixture Git initialization failed"
        );
        let manifest = opdev_project::discover(root.path())?.manifest;
        let mut report = opdev_engine::evaluate(
            root.path(),
            &manifest,
            CheckOptions {
                execute_checks: false,
                ..CheckOptions::pre_merge()
            },
        )?;
        let binding: ExecutionBinding = serde_json::from_value(serde_json::json!({
            "repository":"neutral/project", "provider":"gitlab", "run_id":1,
            "run_attempt":null, "revision":"a".repeat(40), "stage":"pre_merge",
            "suite":"unit", "producer":"unit-job", "command_sha256":"b".repeat(64),
            "inputs_sha256":"c".repeat(64), "configuration_sha256":"d".repeat(64),
            "environment":"neutral", "executable_sha256":"e".repeat(64),
            "executor_sha256":"f".repeat(64), "policy_sha256":"a".repeat(64),
            "ledger_is_input":true
        }))?;
        for outcome in [
            Outcome::Passed,
            Outcome::Failed,
            Outcome::Error,
            Outcome::Unverified,
        ] {
            report.checks = vec![CheckResult {
                id: "unit".into(),
                kind: CheckKind::Suite,
                blocking: true,
                gates: vec![Gate::Integration],
                outcome,
                summary: "observed result".into(),
                evidence: vec![],
                stdout: None,
                stderr: None,
                duration_ms: Some(1),
            }];
            let mut extension = report.checks[0].clone();
            extension.kind = CheckKind::Extension;
            extension.outcome = Outcome::Passed;
            report.checks.push(extension.clone());
            invalidate_reuse(
                &mut report,
                std::slice::from_ref(&binding),
                "job log expired",
            )?;
            assert_eq!(
                report.checks[0].outcome,
                if outcome == Outcome::Passed {
                    Outcome::Unverified
                } else {
                    outcome
                }
            );
            assert_eq!(report.checks[1], extension);
            assert!(!report.gate_passed(Gate::Integration));
        }
        Ok(())
    }
}
