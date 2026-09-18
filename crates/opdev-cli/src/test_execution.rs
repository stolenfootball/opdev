use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use opdev_core::Outcome;

#[derive(Debug, Args)]
pub(crate) struct TestExecutionArgs {
    /// Directory in an initialized repository with clean, committed source.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Existing suite identifier; its argv, working directory and timeout are preserved.
    #[arg(long)]
    suite: String,
}

pub(crate) fn run(args: &TestExecutionArgs) -> Result<ExitCode> {
    let (root, _) = crate::load_project(&args.root)?;
    let receipt = opdev_engine::observe_test_execution(&root, &args.suite)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(match receipt.outcome {
        Outcome::Passed => ExitCode::SUCCESS,
        Outcome::Error => ExitCode::from(2),
        _ => ExitCode::from(1),
    })
}
