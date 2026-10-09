use std::{path::PathBuf, process::ExitCode};

use anyhow::{Context, Result, bail};
use clap::Args;
use opdev_project::TestStage;

use crate::OutputFormat;

#[derive(Debug, Args)]
pub(crate) struct PrepareArgs {
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
}

pub(crate) fn prepare(args: &PrepareArgs) -> Result<ExitCode> {
    let (root, manifest) = crate::load_project(&args.root)?;
    let revision = crate::clean_remote_revision(&root)
        .context("Review handoff needs a clean committed checkout; nothing posted")?;
    let stage = stage(args.post_merge);
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
    println!("{}", serde_json::to_string_pretty(&handoff)?);
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
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,
}

pub(crate) fn check(args: &CheckArgs) -> Result<ExitCode> {
    let (root, manifest) = crate::load_project(&args.root)?;
    let revision = crate::clean_remote_revision(&root)
        .context("CI review requires a clean committed checkout; no checks ran")?;
    let stage = stage(args.post_merge);
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

fn stage(post: bool) -> TestStage {
    if post {
        TestStage::PostMerge
    } else {
        TestStage::PreMerge
    }
}
