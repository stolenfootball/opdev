use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use opdev_core::Outcome;
use opdev_remote::{RunExpectation, verify_run_with_jobs};

use crate::OutputFormat;

#[derive(Debug, Args)]
pub(crate) struct VerifyRunArgs {
    /// Directory inside the initialized Git repository.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Full expected commit ID; never inferred from branch-latest status.
    #[arg(long)]
    revision: String,
    /// Exact GitHub workflow run ID or GitLab pipeline ID.
    #[arg(long)]
    run: u64,
    /// Expected branch/ref returned by the provider.
    #[arg(long = "ref")]
    reference: String,
    /// Expected GitHub event or GitLab pipeline source.
    #[arg(long)]
    source: String,
    /// Expected numeric workflow ID: required for GitHub, unsupported for GitLab.
    #[arg(long)]
    workflow: Option<u64>,
    /// Exact required job name; repeat for each job (opts into schema-2 output).
    #[arg(long = "require-job")]
    required_jobs: Vec<String>,
    /// Full human or JSON observations, including remaining uncertainty.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,
}

pub(crate) fn run(args: &VerifyRunArgs) -> Result<ExitCode> {
    let (_, manifest) = crate::load_project(&args.root)?;
    let result = verify_run_with_jobs(
        &manifest,
        &RunExpectation {
            revision: args.revision.clone(),
            run_id: args.run,
            reference: args.reference.clone(),
            source: args.source.clone(),
            workflow_id: args.workflow,
        },
        &args.required_jobs,
    )?;
    match args.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputFormat::Human => {
            println!("CI run observation: {:?}", result.outcome);
            println!(
                "Repository: {:?}; expected identity: {}",
                result.repository,
                serde_json::to_string(&result.expected)?
            );
            if let Some(observed) = &result.observed {
                println!("Observed: {}", serde_json::to_string(observed)?);
            }
            println!("Qualification: {:?}", result.qualification);
            if let Some(jobs) = &result.jobs {
                println!("Required jobs observation: {:?}", jobs.outcome);
                println!("Jobs: {}", serde_json::to_string(jobs)?);
            }
            for diagnostic in &result.diagnostics {
                println!("{diagnostic}");
            }
        }
    }
    Ok(
        if result.outcome == Outcome::Passed
            && result
                .jobs
                .as_ref()
                .is_none_or(|jobs| jobs.outcome == Outcome::Passed)
        {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        },
    )
}
