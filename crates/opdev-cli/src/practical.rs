//! The practical check path executes canonical commands without review transport.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use anyhow::{Context, Result, ensure};
use opdev_project::{
    MANIFEST_PATH, TestStage,
    practical::{Contract, Preset, read_regular},
};
use serde::Serialize;

use crate::{CheckArgs, CheckFormat};

pub(super) fn generate_ci(
    args: &crate::CiGenerateArgs,
    root: &Path,
    contract: &Contract,
) -> Result<()> {
    let provider: opdev_project::CiProvider = args.provider.into();
    let adapter = opdev_ci::adapter_for(provider)?;
    let job_image = match (provider, &args.image) {
        (opdev_project::CiProvider::Gitlab, Some(image)) => Some(image.clone()),
        (opdev_project::CiProvider::Gitlab, None) => Some(opdev_ci::infer_gitlab_image(root)?),
        (_, Some(_)) => anyhow::bail!("--image applies only to GitLab"),
        _ => None,
    };
    let context = opdev_ci::TemplateContext {
        opdev_version: args.opdev_version.clone(),
        trunk: contract.trunk.clone(),
        job_image,
    };
    eprintln!(
        "Native integration template; no review handoff or release authorization. Verify the selected runtime supports practical.check.v1 before use. Published 0.5.0 does not contain this source preview; use a reviewed source build during development. Generation is not remote qualification."
    );
    let rendered = practical_ci(adapter, &context)?;
    if args.write {
        println!(
            "created {}",
            opdev_ci::write_rendered(&root.join(adapter.configuration_path()), &rendered)?
                .display()
        );
    } else {
        print!("{rendered}");
    }
    Ok(())
}

pub(super) fn inspect_ci(root: &Path, contract: &Contract) -> Result<()> {
    let mut found = false;
    for provider in [
        opdev_project::CiProvider::Github,
        opdev_project::CiProvider::Gitlab,
    ] {
        let adapter = opdev_ci::adapter_for(provider)?;
        if !(root.join(adapter.configuration_path()).exists()
            || provider == opdev_project::CiProvider::Github
                && root.join(".github/workflows").exists())
        {
            continue;
        }
        found = true;
        let report = adapter.inspect_for_trunk(root, &contract.trunk)?;
        println!(
            "{provider:?}: local inspection only; runtime capability and remote protection unverified"
        );
        for (name, capability) in [
            ("configuration", report.configuration),
            ("pre_merge", report.pre_merge),
            ("post_merge", report.post_merge),
            ("integrity", report.integrity),
        ] {
            crate::print_capability(name, &capability);
        }
    }
    ensure!(
        found,
        "No first-class CI configuration found; native integration remains unverified"
    );
    Ok(())
}

fn practical_ci(
    adapter: &dyn opdev_ci::CiAdapter,
    context: &opdev_ci::TemplateContext,
) -> Result<String> {
    let mut document: serde_json::Value = serde_saphyr::from_str(&adapter.render(context)?)?;
    if adapter.provider() == opdev_project::CiProvider::Gitlab {
        document["variables"]["OPDEV_TRUNK"] = serde_json::json!(context.trunk);
        let rules = serde_json::json!([
            {"if":"$CI_PIPELINE_SOURCE == \"merge_request_event\""},
            {"if":"$CI_PIPELINE_SOURCE == \"push\" && $CI_COMMIT_BRANCH == $OPDEV_TRUNK"}
        ]);
        document["workflow"] = serde_json::json!({"rules":rules});
        document["opdev"]["rules"] = rules;
        document["opdev"]["script"] = serde_json::json!([r#"set -e
set --
if [ "$CI_PIPELINE_SOURCE" = "push" ] && [ "$CI_COMMIT_BRANCH" = "$OPDEV_TRUNK" ]; then
    set -- --post-merge
fi
report_file="$(mktemp)"
set +e
opdev check --ci "$@" --expected-revision "$CI_COMMIT_SHA" --format json > "$report_file"
opdev_status=$?
cp "$report_file" opdev-report.json
copy_status=$?
if [ "$copy_status" -ne 0 ]; then exit "$copy_status"; fi
exit "$opdev_status"
"#]);
    } else if adapter.provider() == opdev_project::CiProvider::Github
        && let Some(steps) = document["jobs"]["opdev"]["steps"].as_array_mut()
    {
        for step in steps {
            if let Some(script) = step["run"].as_str() {
                step["run"] = serde_json::json!(script.replace(
                    "opdev check --ci",
                    "opdev check --ci --expected-revision \"$OPDEV_CHECKOUT_SHA\""
                ));
                step["env"]["OPDEV_CHECKOUT_SHA"] =
                    serde_json::json!("${{ github.event.pull_request.head.sha || github.sha }}");
            }
        }
    }
    Ok(serde_saphyr::to_string(&document)?)
}

pub(super) fn adoption_status(
    root: &Path,
    contract: &Contract,
    format: crate::OutputFormat,
    remote: bool,
) -> Result<()> {
    ensure!(
        !remote,
        "Practical remote adoption inspection is not supported; no checks ran or obligations waived"
    );
    contract.inspect_namespace(root, true)?;
    let requirements = contract.inspect_requirements(root)?;
    let mut gaps = Vec::new();
    for stage in [TestStage::PreMerge, TestStage::PostMerge] {
        if contract.commands_at(stage).is_empty() {
            gaps.push(format!(
                "No checks selected for {stage:?}; no verification claimed"
            ));
        }
        gaps.extend(contract.review_gaps(root, &requirements, stage)?);
    }
    if matches!(format, crate::OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema":2, "status":"workflow_configured", "preset":contract.preset,
                "product_verification":"not_run", "remote_qualification":"not_run",
                "decision_reference":contract.decision_reference, "findings":gaps,
                "limits":"Read-only configuration inspection, not completed product verification, authenticated consent or release permission."
            }))?
        );
    } else {
        println!(
            "Workflow configured ({:?}). Product verification and remote qualification: not run.",
            contract.preset
        );
        for gap in gaps {
            println!("  {gap}");
        }
        println!("Decision reference recorded, not authenticated consent. No release authorized.");
    }
    Ok(())
}

pub(super) fn adoption_plan(root: &Path, preset: &str, remote: bool) -> Result<()> {
    ensure!(
        !remote,
        "Practical planning is local inspection only; remote qualification was not requested through a supported adapter"
    );
    let (commands, checks, trunk, authorities, requirements, policies, prior_obligations) =
        if let Some((_, current)) = selected(root)? {
            let prior = serde_json::to_value(&current)?;
            (
                serde_json::to_value(current.commands)?,
                serde_json::to_value(current.checks)?,
                current.trunk,
                serde_json::to_value(current.authorities)?,
                serde_json::to_value(current.requirements)?,
                serde_json::to_value(current.policies)?,
                prior,
            )
        } else {
            let discovery = opdev_project::discover(root)?;
            let current = discovery.manifest;
            let authorities: std::collections::BTreeMap<_, _> = current
                .authorities
                .iter()
                .map(|(id, owner)| (id, &owner.location))
                .collect();
            (
                serde_json::to_value(&current.commands)?,
                serde_json::to_value(&current.testing.suites)?,
                current.project.trunk,
                serde_json::to_value(authorities)?,
                serde_json::json!([]),
                serde_json::json!([]),
                if discovery.root.join(MANIFEST_PATH).exists() {
                    serde_json::to_value(&current.assurance)?
                } else {
                    serde_json::Value::Null
                },
            )
        };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema":1,"kind":"practical_adoption_proposal",
            "project":{"schema":5,"policy_version":1,"preset":preset,"trunk":trunk,
                "decision_reference":null,"commands":commands,"checks":checks,"authorities":authorities,"requirements":requirements,"policies":policies},
            "prior_obligations":prior_obligations,
            "choices":["Confirm assurance level and actual decision reference",
                "Review existing check boundaries; inherited repetition is not automatically necessary",
                "Inventory supported requirements and preserve actual external obligations",
                "Keep the integration branch name or explicitly choose a rename"],
            "limits":"Proposal only. Null decision reference intentionally prevents initialization. Existing requirements and external obligations are not removed or migrated by this preview. No checks or remote qualification ran."
        }))?
    );
    Ok(())
}

pub(super) fn initialize(args: &crate::InitArgs) -> Result<bool> {
    let Some(path) = &args.project else {
        return Ok(false);
    };
    let text = fs::read_to_string(path)?;
    let header: serde_json::Value = serde_saphyr::from_str(&text)?;
    if header.get("schema").and_then(serde_json::Value::as_u64) != Some(5) {
        return Ok(false);
    }
    let contract = Contract::from_yaml(&text)?;
    let root =
        PathBuf::from(git(&args.root, &["rev-parse", "--show-toplevel"])?.trim()).canonicalize()?;
    let target = root.join(MANIFEST_PATH);
    ensure!(
        !target.try_exists()?,
        "Existing policy preserved. Use a reviewed migration, not init, to replace it. Nothing changed."
    );
    contract.inspect_namespace(&root, false)?;
    contract.inspect_policies(&root, &contract.inspect_requirements(&root)?)?;
    let mut preview = opdev_project::preview_practical_agent_files(&root)?;
    preview.push(opdev_project::AgentFilePreview {
        file: opdev_project::ManagedFile {
            path: target,
            change: opdev_project::FileChange::Created,
        },
        before: None,
        after: serde_saphyr::to_string(&contract)?,
    });
    if args.dry_run {
        let changes: Vec<_> = preview
            .iter()
            .map(|item| {
                serde_json::json!({
                    "path": item.file.path, "before": item.before, "after": item.after
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "project": contract, "changes": changes,
                "limits": "Preview only; no files changed, checks or product verification."
            }))?
        );
    } else {
        opdev_project::apply_agent_preview(&preview)?;
        println!(
            "OpDev workflow configured. Product checks and remote integration have not been verified. No release is authorized."
        );
    }
    Ok(true)
}

/// Return only explicitly selected schema-5 projects. Never infer a migration.
pub(super) fn selected(start: &Path) -> Result<Option<(PathBuf, Contract)>> {
    let root = git(start, &["rev-parse", "--show-toplevel"])?;
    let root = PathBuf::from(root.trim()).canonicalize()?;
    if !root.join(MANIFEST_PATH).try_exists()? {
        return Ok(None);
    }
    let content = read_regular(&root, MANIFEST_PATH)?;
    let header: serde_json::Value = serde_saphyr::from_str(&content)?;
    if header.get("schema").and_then(serde_json::Value::as_u64) != Some(5) {
        return Ok(None);
    }
    Ok(Some((root, Contract::from_yaml(&content)?)))
}

pub(crate) fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_ALLOW_PROTOCOL", "")
        .output()?;
    ensure!(
        output.status.success(),
        "Could not inspect repository: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

#[derive(Serialize)]
struct CheckResult {
    command: String,
    status: &'static str,
    exit_code: Option<i32>,
    duration_ms: u128,
    stdout: String,
    stderr: String,
}

#[derive(Serialize)]
struct Report {
    schema: u32,
    kind: &'static str,
    preset: Preset,
    stage: TestStage,
    execution_status: &'static str,
    source_commit: Option<String>,
    source_clean: bool,
    checks: Vec<CheckResult>,
    manual_checks: Vec<ManualCheck>,
    mapping_findings: Vec<String>,
    limits: &'static str,
}

#[derive(Serialize)]
struct ManualCheck {
    id: String,
    procedure: String,
    status: &'static str,
}

fn manual_checks(contract: &Contract, stage: TestStage) -> Vec<ManualCheck> {
    contract
        .checks
        .iter()
        .filter(|check| check.stages.contains(&stage))
        .filter_map(|check| {
            check.manual.as_ref().map(|procedure| ManualCheck {
                id: check.id.clone(),
                procedure: procedure.clone(),
                status: "not_run",
            })
        })
        .collect()
}

const LIMITS: &str = "Canonical command results only; suite success is not proof that a named test ran or a requirement was adequately reviewed. Native CI owns remote integration status. No external standard was assessed and no release is authorized.";

fn validate_options(args: &CheckArgs) -> Result<()> {
    ensure!(
        !args.review_ci
            && args.review_locator.is_none()
            && args.review_acceptance_sha256.is_none()
            && args.reuse_ci_policy.is_none()
            && args.reuse_native_candidate.is_none()
            && args.execution_environment.is_none()
            && !args.remote
            && !args.retain_state
            && !args.require_minimumcd,
        "This contract uses native check results, not legacy review or execution-reuse arguments. No checks ran; requested additional obligations were not waived."
    );
    ensure!(
        args.format != CheckFormat::Summary,
        "Practical checks already use concise human output; use --format human or json."
    );
    if let Some(path) = &args.report {
        ensure!(
            path.symlink_metadata().is_err(),
            "Report output already exists; choose a new path. No checks ran."
        );
    }
    Ok(())
}

fn verify_revision(expected: Option<&String>, actual: Option<&String>) -> Result<()> {
    if let Some(expected) = expected {
        ensure!(
            matches!(expected.len(), 40 | 64)
                && expected
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && actual == Some(expected),
            "Checkout does not match the expected full Git revision. No checks ran."
        );
    }
    Ok(())
}

pub(super) fn check(args: &CheckArgs, root: &Path, contract: &Contract) -> Result<ExitCode> {
    validate_options(args)?;
    contract.inspect_namespace(root, true)?;
    let requirements = contract.inspect_requirements(root)?;
    let stage = if args.evaluation {
        TestStage::Evaluation
    } else if args.delivery {
        TestStage::Delivery
    } else if args.post_merge {
        TestStage::PostMerge
    } else if args.ci {
        TestStage::PreMerge
    } else {
        TestStage::Local
    };
    let selected = contract.commands_at(stage);
    if args.plan {
        print_plan(contract, stage)?;
        return Ok(ExitCode::SUCCESS);
    }
    let source_commit = git(root, &["rev-parse", "--verify", "HEAD"])
        .ok()
        .map(|s| s.trim().to_owned());
    let source_clean = git(root, &["status", "--porcelain", "--untracked-files=normal"])?
        .trim()
        .is_empty();
    verify_revision(args.expected_revision.as_ref(), source_commit.as_ref())?;
    if args.ci && !args.no_exec {
        ensure!(
            source_commit.is_some() && source_clean,
            "Integration checks need a clean committed checkout. Use local focused checks while editing. No checks ran."
        );
    }
    let initial_contract = read_regular(root, MANIFEST_PATH)?;
    ensure!(
        Contract::from_yaml(&initial_contract)? == *contract,
        "Policy changed since inspection; no checks ran"
    );
    let mut mapping_findings = contract.review_gaps(root, &requirements, stage)?;
    let results = execute_checks(
        args,
        root,
        contract,
        selected,
        !args.no_exec && mapping_findings.is_empty(),
    )?;
    if !read_regular(root, MANIFEST_PATH).is_ok_and(|current| current == initial_contract) {
        mapping_findings.push(
            "Project policy changed during verification; results cannot qualify the changed policy"
                .into(),
        );
    }
    if args.ci
        && !args.no_exec
        && (!git(root, &["status", "--porcelain", "--untracked-files=normal"])?
            .trim()
            .is_empty()
            || git(root, &["rev-parse", "--verify", "HEAD"])?.trim()
                != source_commit.as_deref().unwrap_or(""))
    {
        mapping_findings.push(
            "Checkout changed during verification; inspect the change before retrying".into(),
        );
    }
    inspect_after_execution(root, contract, &requirements, stage, &mut mapping_findings);
    let manual_checks = manual_checks(contract, stage);
    let status = if results.iter().any(|r| r.status == "error") {
        "error"
    } else if results.iter().any(|r| r.status == "failed") {
        "failed"
    } else if !manual_checks.is_empty()
        || results.is_empty()
        || results.iter().any(|r| r.status == "not_run")
    {
        "not_run"
    } else {
        "passed"
    };
    let report = Report {
        schema: 1,
        kind: "practical_check",
        preset: contract.preset,
        stage,
        execution_status: status,
        source_commit,
        source_clean,
        checks: results,
        manual_checks,
        mapping_findings,
        limits: LIMITS,
    };
    present(args, &report)?;
    Ok(ExitCode::from(match status {
        "passed" if report.mapping_findings.is_empty() => 0,
        "error" => 2,
        _ => 1,
    }))
}

fn inspect_after_execution(
    root: &Path,
    contract: &Contract,
    requirements: &[opdev_project::practical::Requirement],
    stage: TestStage,
    findings: &mut Vec<String>,
) {
    if let Err(error) = contract.inspect_namespace(root, true) {
        findings.push(format!(
            "Project namespace became invalid during verification: {error}"
        ));
    }
    match contract.inspect_requirements(root) {
        Ok(current) => {
            if current != requirements {
                findings.push("Requirement definitions changed during verification".into());
            }
            match contract.review_gaps(root, &current, stage) {
                Ok(gaps) => {
                    for gap in gaps {
                        if !findings.contains(&gap) {
                            findings.push(gap);
                        }
                    }
                }
                Err(error) => findings.push(format!(
                    "Review inputs became invalid during verification: {error}"
                )),
            }
        }
        Err(error) => findings.push(format!(
            "Requirement inputs became invalid during verification: {error}"
        )),
    }
}

fn print_plan(contract: &Contract, stage: TestStage) -> Result<()> {
    let commands: Vec<_> = contract
        .commands_at(stage)
        .iter()
        .map(|id| serde_json::json!({"id": id, "command": contract.commands[*id]}))
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": 1, "kind": "practical_plan", "stage": stage,
            "preset": contract.preset, "commands": commands,
            "manual_checks": manual_checks(contract, stage),
            "limits": "Read-only execution plan. No checks ran; no approval or qualification."
        }))?
    );
    Ok(())
}

fn execute_checks(
    args: &CheckArgs,
    root: &Path,
    contract: &Contract,
    selected: Vec<&str>,
    execute: bool,
) -> Result<Vec<CheckResult>> {
    let mut results = Vec::new();
    let mut stopped = false;
    for id in selected {
        if !execute || stopped {
            results.push(CheckResult {
                command: id.into(),
                status: "not_run",
                exit_code: None,
                duration_ms: 0,
                stdout: String::new(),
                stderr: String::new(),
            });
            continue;
        }
        let command = &contract.commands[id];
        if let Some(directory) = &command.working_directory {
            let directory = root.join(directory).canonicalize()?;
            ensure!(
                directory.starts_with(root) && directory.is_dir(),
                "Command {id} working directory escapes the project or is not a directory"
            );
        }
        let result = match opdev_engine::execute(root, command, None) {
            Ok(execution) => CheckResult {
                command: id.into(),
                status: if execution.timed_out {
                    "error"
                } else if execution.exit_code == Some(0) {
                    "passed"
                } else {
                    "failed"
                },
                exit_code: execution.exit_code,
                duration_ms: execution.duration_ms,
                stdout: execution.stdout,
                stderr: execution.stderr,
            },
            Err(error) => CheckResult {
                command: id.into(),
                status: "error",
                exit_code: None,
                duration_ms: 0,
                stdout: String::new(),
                stderr: error.to_string(),
            },
        };
        stopped = args.fail_fast && result.status != "passed";
        results.push(result);
    }
    Ok(results)
}

fn present(args: &CheckArgs, report: &Report) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    if let Some(path) = &args.report {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .context("Report output must be a new file")?
            .write_all(json.as_bytes())?;
    }
    if args.format == CheckFormat::Json {
        println!("{json}");
    } else {
        println!(
            "{:?} / {:?}: checks {}",
            report.preset, report.stage, report.execution_status
        );
        for result in &report.checks {
            println!(
                "  {}: {} ({} ms)",
                result.command, result.status, result.duration_ms
            );
            if matches!(result.status, "failed" | "error") {
                println!("{}{}", result.stdout, result.stderr);
            }
        }
        for check in &report.manual_checks {
            println!(
                "  {}: not run; manual procedure {} requires an actual observation. This command cannot verify it.",
                check.id, check.procedure
            );
        }
        if report.checks.is_empty() && report.manual_checks.is_empty() {
            println!("No checks are configured for this boundary. No verification claimed.");
        }
        for finding in &report.mapping_findings {
            println!("  Review input: {finding}");
        }
        println!("{LIMITS}");
    }
    Ok(())
}
