//! Non-qualifying, explicitly scoped execution; never construct a gate report.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    process::ExitCode,
};

use anyhow::{Result, ensure};
use clap::ValueEnum;
use opdev_project::{CommandSpec, MANIFEST_PATH, TestStage};
use serde::Serialize;
use serde_json::json;

use crate::{CheckArgs, CheckFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Purpose {
    Design,
    Feedback,
    Integration,
}

pub(super) fn validate(args: &CheckArgs) -> Result<()> {
    let preparation = matches!(args.purpose, Some(Purpose::Design | Purpose::Feedback));
    ensure!(
        preparation || (args.commands.is_empty() && args.reason.is_none()),
        "Integration cannot select a command subset or an early-probe reason. All required checks remain selected. No checks ran."
    );
    if !preparation {
        ensure!(
            args.purpose.is_none() || !args.evaluation,
            "Evaluation is a separate boundary; do not label it integration. No checks ran."
        );
        return Ok(());
    }
    ensure!(
        !args.ci
            && !args.delivery
            && !args.post_merge
            && !args.evaluation
            && !args.remote
            && !args.review_ci
            && args.review_locator.is_none()
            && args.review_acceptance_sha256.is_none()
            && !args.retain_state
            && !args.require_minimumcd
            && args.reuse_ci_policy.is_none()
            && args.reuse_native_candidate.is_none()
            && args.execution_environment.is_none()
            && args.expected_revision.is_none(),
        "Design/feedback checks cannot request CI, qualification, review transport or execution reuse. Select integration only when that boundary is ready. No checks ran."
    );
    ensure!(
        !args.commands.is_empty(),
        "Design/feedback needs at least one explicit --command ID. No default or full suite was selected. No checks ran."
    );
    ensure!(
        args.format != CheckFormat::Summary,
        "Design/feedback uses human or JSON preparation results, not a qualification summary. No checks ran."
    );
    ensure!(
        args.reason.as_ref().is_none_or(|s| !s.trim().is_empty()),
        "The early-probe reason must not be blank. No checks ran."
    );
    Ok(())
}

const LIMITS: &str = "Selected command observations only. Integration remains unverified; no design approval, policy waiver or release authorization. A reason is attributed rationale, not authenticated consent.";

pub(super) fn check(args: &CheckArgs) -> Result<ExitCode> {
    let (root, commands, required, policy) = if let Some((root, contract)) =
        crate::practical::selected(&args.root)?
    {
        let policy = fs::read(root.join(MANIFEST_PATH))?;
        ensure!(
            opdev_project::practical::Contract::from_yaml(std::str::from_utf8(&policy)?)?
                == contract,
            "Project policy changed since selection. No checks ran."
        );
        let required = contract
            .checks
            .iter()
            .filter(|c| c.stages.iter().any(|stage| integration_stage(*stage)))
            .filter(|c| !c.command.is_empty())
            .map(|c| c.command.clone())
            .collect::<BTreeSet<_>>();
        (root, contract.commands, required, policy)
    } else {
        let (root, manifest) = crate::load_project(&args.root)?;
        let policy = fs::read(root.join(MANIFEST_PATH))?;
        ensure!(
            opdev_project::ProjectManifest::from_yaml(std::str::from_utf8(&policy)?)? == manifest,
            "Project policy changed since selection. No checks ran."
        );
        let required = manifest
            .testing
            .suites
            .iter()
            .filter(|s| s.stages.iter().any(|stage| integration_stage(*stage)))
            .map(|s| s.command.clone())
            .collect::<BTreeSet<_>>();
        (root, manifest.commands, required, policy)
    };
    let selected = select(args, &commands, &required)?;
    // Validate every invocation before executing any, including contained cwd.
    for (_, command) in &selected {
        if let Some(directory) = &command.working_directory {
            let path = root.join(directory).canonicalize()?;
            ensure!(
                path.starts_with(&root) && path.is_dir(),
                "Selected command working directory escapes the project or is not a directory. No checks ran."
            );
        }
    }
    if let Some(path) = &args.report {
        ensure!(
            path.symlink_metadata().is_err(),
            "Report already exists; choose a new path. No checks ran."
        );
    }
    let scope: Vec<_> = selected
        .iter()
        .map(|(id, command)| {
            json!({
                "id": id, "command": command, "required_at_integration": required.iter().any(|key| commands.get(key) == Some(*command))
            })
        })
        .collect();
    if args.plan {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema": 1, "kind": "preparation_plan", "purpose": args.purpose,
                "commands": scope, "reason": args.reason, "integration_status": "unverified",
                "limits": LIMITS
            }))?
        );
        return Ok(ExitCode::SUCCESS);
    }
    let mut results = Vec::new();
    let mut stopped = args.no_exec;
    let mut policy_changed = false;
    for (id, command) in selected {
        policy_changed |= fs::read(root.join(MANIFEST_PATH)).ok().as_ref() != Some(&policy);
        if stopped || policy_changed {
            results.push(json!({"command": id, "status": "not_run"}));
            continue;
        }
        let result = match opdev_engine::execute(&root, command, None) {
            Ok(run) => json!({"command": id,
                "status": if run.timed_out { "error" } else if run.exit_code == Some(0) { "passed" } else { "failed" },
                "exit_code": run.exit_code, "duration_ms": run.duration_ms,
                "stdout": run.stdout, "stderr": run.stderr}),
            Err(error) => json!({"command": id, "status": "error", "error": error.to_string()}),
        };
        stopped = args.fail_fast && result["status"] != "passed";
        results.push(result);
    }
    policy_changed |= fs::read(root.join(MANIFEST_PATH)).ok().as_ref() != Some(&policy);
    let code = if policy_changed || results.iter().any(|r| r["status"] == "error") {
        2
    } else {
        u8::from(!results.iter().all(|r| r["status"] == "passed"))
    };
    let report = json!({"schema": 1, "kind": "preparation_check", "purpose": args.purpose,
        "commands": scope, "checks": results, "reason": args.reason,
        "execution_status": match code { 0 => "passed", 1 => "failed_or_not_run", _ => "error" },
        "policy_changed": policy_changed, "integration_status": "unverified", "limits": LIMITS});
    present(args, &report)?;
    Ok(ExitCode::from(code))
}

fn present(args: &CheckArgs, report: &serde_json::Value) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    if let Some(path) = &args.report {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(json.as_bytes())?;
    }
    if args.format == CheckFormat::Json {
        println!("{json}");
    } else {
        println!(
            "{:?} checks: {}. Integration remains unverified.",
            args.purpose
                .ok_or_else(|| anyhow::anyhow!("Missing preparation purpose"))?,
            report["execution_status"]
        );
        for result in report["checks"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing check results"))?
        {
            println!("{}: {}", result["command"], result["status"]);
            if result["status"] != "passed" {
                for field in ["error", "stderr", "stdout"] {
                    if let Some(text) = result[field].as_str().filter(|s| !s.is_empty()) {
                        println!("{text}");
                    }
                }
            }
        }
        if report["policy_changed"] == true {
            println!("Project policy changed during execution; inspect it before retrying.");
        }
        println!("{LIMITS}");
    }
    Ok(())
}

fn integration_stage(stage: TestStage) -> bool {
    matches!(
        stage,
        TestStage::Local | TestStage::PreMerge | TestStage::PostMerge | TestStage::Delivery
    )
}

fn select<'a>(
    args: &'a CheckArgs,
    commands: &'a BTreeMap<String, CommandSpec>,
    required: &BTreeSet<String>,
) -> Result<Vec<(&'a str, &'a CommandSpec)>> {
    let mut seen = BTreeSet::new();
    let mut selected = Vec::new();
    for id in &args.commands {
        ensure!(
            seen.insert(id),
            "Duplicate command {id}; list each command once. No checks ran."
        );
        let command = commands.get(id).ok_or_else(|| {
            anyhow::anyhow!("Unknown command {id}. Choose a declared command ID. No checks ran.")
        })?;
        ensure!(
            args.plan
                || !required
                    .iter()
                    .any(|key| commands.get(key) == Some(command))
                || args.reason.is_some(),
            "Command {id} is required at an integration boundary. For a necessary early probe, supply --reason explaining why it is needed now; otherwise choose a focused command. No checks ran."
        );
        selected.push((id.as_str(), command));
    }
    Ok(selected)
}
