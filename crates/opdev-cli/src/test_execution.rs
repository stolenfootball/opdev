use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use opdev_core::Outcome;

pub(crate) fn run(start: &Path, suite_id: &str, junit: &Path) -> Result<ExitCode> {
    let (root, _) = crate::load_project(start)?;
    let receipt = opdev_engine::observe_test_execution(&root, suite_id, junit)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(match receipt.outcome {
        Outcome::Passed => ExitCode::SUCCESS,
        Outcome::Error => ExitCode::from(2),
        _ => ExitCode::from(1),
    })
}
