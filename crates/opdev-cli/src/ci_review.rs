use std::{path::PathBuf, process::ExitCode};

use anyhow::{Context, Result, bail};
use clap::Args;
use opdev_project::TestStage;

use crate::OutputFormat;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum VerificationStage {
    PreMerge,
    PostMerge,
    Delivery,
}

#[derive(Debug, Args)]
pub(crate) struct VerifyArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Actual verification boundary; no stage inferred from a tag or saved report.
    #[arg(long, value_enum)]
    stage: VerificationStage,
    /// Inspect current inputs before installing test-only tools; executes no suites.
    #[arg(long, conflicts_with = "report")]
    preflight: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,
    /// Save the full report to a new file.
    #[arg(long)]
    report: Option<PathBuf>,
}

pub(crate) fn verify(args: &VerifyArgs) -> Result<ExitCode> {
    let post_merge = matches!(args.stage, VerificationStage::PostMerge);
    let delivery = matches!(args.stage, VerificationStage::Delivery);
    if args.preflight {
        return check(&CheckArgs {
            root: args.root.clone(),
            post_merge,
            delivery,
            format: args.format,
        });
    }
    // Compose the existing gate. This adapter cannot select an older review,
    // downgrade a gate, execute a second copy of suites, or publish anything.
    crate::check_project(&crate::CheckArgs {
        purpose: Some(crate::preparation::Purpose::Integration),
        commands: Vec::new(),
        reason: None,
        root: args.root.clone(),
        ci: true,
        review_ci: true,
        fail_fast: true,
        post_merge,
        delivery,
        evaluation: false,
        expected_revision: None,
        format: match args.format {
            OutputFormat::Human => crate::CheckFormat::Human,
            OutputFormat::Json => crate::CheckFormat::Json,
        },
        report: args.report.clone(),
        review_locator: None,
        review_acceptance_sha256: None,
        retain_state: false,
        require_minimumcd: false,
        plan: false,
        remote: false,
        no_exec: false,
        reuse_ci_policy: None,
        reuse_native_candidate: None,
        execution_environment: None,
    })
}

#[derive(Debug, Args)]
pub(crate) struct PrepareArgs {
    /// Save machine-readable handoff to a new file outside source, avoiding merged console streams.
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Exact existing MR/PR number; no latest-change lookup.
    #[arg(long)]
    change: u64,
    /// Exact posted review comment; descriptions cannot select themselves.
    #[arg(long)]
    note: u64,
    /// Independently reviewed acceptance identity, not inferred from comment content.
    #[arg(long)]
    acceptance_sha256: String,
    #[arg(long)]
    post_merge: bool,
    /// Prepare an artifact-bound delivery review, without publishing anything.
    #[arg(long, conflicts_with = "post_merge")]
    delivery: bool,
}

pub(crate) fn prepare(args: &PrepareArgs) -> Result<ExitCode> {
    let (root, manifest) = crate::load_project(&args.root)?;
    let revision = crate::clean_remote_revision(&root)
        .context("Review handoff needs a clean committed checkout; nothing posted")?;
    let stage = stage(args.post_merge, args.delivery);
    let handoff = opdev_remote::prepare_review_handoff(
        &manifest,
        stage,
        &revision,
        args.change,
        args.note,
        &args.acceptance_sha256,
    )
    .map_err(anyhow::Error::msg)?;
    let observed =
        opdev_remote::retrieve_discussion(&handoff.locator).map_err(anyhow::Error::msg)?;
    let review = opdev_engine::ValidatedReview::from_discussion(
        &root,
        &manifest,
        stage,
        &args.acceptance_sha256,
        observed,
    )
    .map_err(anyhow::Error::msg)?;
    let gaps = review
        .preflight(&root, &manifest, stage)
        .map_err(anyhow::Error::msg)?;
    if !gaps.is_empty() {
        bail!(
            "Review handoff needs current reviewed inputs: {}. Nothing posted",
            gaps.join("; ")
        );
    }
    if crate::clean_remote_revision(&root).as_deref() != Some(&revision) {
        bail!("Source changed during handoff; nothing posted");
    }
    let json = serde_json::to_vec_pretty(&handoff)?;
    if let Some(path) = &args.output {
        let path = crate::evidence_bundle::export_destination(&root, path)?;
        crate::local_state::write_new(&path, &json)?;
    } else {
        println!("{}", String::from_utf8(json)?);
    }
    eprintln!(
        "Review handoff prepared. Nothing was posted or approved; inspect the proposed description before applying it. Recheck its original digest before writing to avoid overwriting someone else's edit."
    );
    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Args)]
pub(crate) struct CheckArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long)]
    post_merge: bool,
    /// Inspect delivery-stage inputs; readiness is not artifact qualification.
    #[arg(long, conflicts_with = "post_merge")]
    delivery: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,
}

pub(crate) fn check(args: &CheckArgs) -> Result<ExitCode> {
    let (root, manifest) = crate::load_project(&args.root)?;
    let revision = crate::clean_remote_revision(&root)
        .context("CI review requires a clean committed checkout; no checks ran")?;
    let stage = stage(args.post_merge, args.delivery);
    let selection =
        opdev_remote::select_ci_review(&manifest, stage, &revision).map_err(anyhow::Error::msg)?;
    let observed =
        opdev_remote::retrieve_discussion(selection.locator()).map_err(anyhow::Error::msg)?;
    let review = opdev_engine::ValidatedReview::from_discussion(
        &root,
        &manifest,
        stage,
        selection.acceptance(),
        observed,
    )
    .map_err(anyhow::Error::msg)?;
    let mut gaps = review
        .preflight(&root, &manifest, stage)
        .map_err(anyhow::Error::msg)?;
    selection.recheck().map_err(anyhow::Error::msg)?;
    if crate::clean_remote_revision(&root).as_deref() != Some(&revision) {
        gaps.push("Source changed during CI review inspection".into());
    }
    let ready = gaps.is_empty();
    match args.format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"schema":1,
            "kind":"ci_review_inputs", "inputs_ready":ready, "qualification":"not_run", "checks_ran":false,
            "stage":stage, "revision":revision, "blockers":gaps}))?
        ),
        OutputFormat::Human => {
            println!(
                "CI review inputs: {}. No project checks ran; integration is not verified.",
                if ready { "ready" } else { "not ready" }
            );
            for gap in gaps {
                println!("  {gap}");
            }
        }
    }
    Ok(if ready {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn stage(post: bool, delivery: bool) -> TestStage {
    if delivery {
        TestStage::Delivery
    } else if post {
        TestStage::PostMerge
    } else {
        TestStage::PreMerge
    }
}
