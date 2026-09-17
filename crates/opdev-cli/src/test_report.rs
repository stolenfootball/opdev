use std::fs::File;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use opdev_core::Outcome;

use crate::OutputFormat;

#[derive(Debug, Args)]
pub(crate) struct TestReportArgs {
    #[command(subcommand)]
    command: TestReportCommand,
}

#[derive(Debug, Subcommand)]
enum TestReportCommand {
    /// Run one canonical suite and emit JSON execution evidence, not gate qualification.
    Run {
        /// Directory in an initialized repository with clean, committed source.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Existing suite identifier; its argv, working directory and timeout are preserved.
        #[arg(long)]
        suite: String,
        /// Previously absent `JUnit` output, relative to the Git root or absolute.
        #[arg(long)]
        junit: PathBuf,
    },
    /// Inspect one `JUnit` XML file; does not verify execution, freshness or any gate.
    Inspect {
        /// UTF-8 `JUnit` XML file (maximum 8 MiB).
        path: PathBuf,
        /// Report presentation; errors use stderr and exit 2 in either mode.
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
}

pub(crate) fn run(args: &TestReportArgs) -> Result<ExitCode> {
    match &args.command {
        TestReportCommand::Run { root, suite, junit } => {
            crate::test_execution::run(root, suite, junit)
        }
        TestReportCommand::Inspect { path, format } => inspect(path, *format),
    }
}

fn inspect(path: &std::path::Path, format: OutputFormat) -> Result<ExitCode> {
    anyhow::ensure!(
        std::fs::metadata(path)
            .context("could not inspect JUnit input")?
            .is_file(),
        "JUnit input must be a regular file"
    );
    let file = File::open(path).context("could not open JUnit report")?;
    anyhow::ensure!(
        file.metadata()?.is_file(),
        "JUnit input must be a regular file"
    );
    let report = opdev_engine::inspect_junit(file)?;
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputFormat::Human => {
            println!(
                "Report inspection: {}",
                serde_json::to_value(report.outcome)?
            );
            println!(
                "Cases: {}; failures: {}; errors: {}; skipped: {}; retry signals: {}",
                report.cases, report.failures, report.errors, report.skipped, report.retry_signals
            );
            println!("Input SHA-256: {}", report.input_sha256);
            for diagnostic in &report.diagnostics {
                println!("{diagnostic}");
            }
        }
    }
    Ok(if report.outcome == Outcome::Passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
