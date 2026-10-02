use std::path::Path;

use opdev_core::Outcome;
use opdev_project::{
    CommandSpec, ExtensionCheck, ExtensionStage, ProjectManifest, TestStage, TestSuite,
};
use serde::Serialize;

use crate::{CheckKind, CheckOptions, command::DEFAULT_TIMEOUT_SECONDS};

/// Read-only selection of commands; never a gate verdict or saved execution.
#[derive(Debug, Serialize)]
pub struct CheckPlan {
    /// Independent plan format version.
    pub schema: u32,
    /// Distinguishes this document from an evaluated check report.
    pub kind: &'static str,
    /// Selected project root.
    pub subject: String,
    /// Selected canonical-suite stage.
    pub test_stage: TestStage,
    /// Selected extension stage.
    pub extension_stage: ExtensionStage,
    /// Always unverified: planning cannot qualify a gate.
    pub qualification: Outcome,
    /// Ordered suite commands followed by extension commands.
    pub commands: Vec<PlannedCommand>,
}

/// One declared invocation, without executable resolution or environment probes.
#[derive(Debug, Serialize)]
pub struct PlannedCommand {
    /// Stable suite or extension ID.
    pub id: String,
    /// Distinguishes suites and extensions, even when IDs or argv match.
    pub kind: CheckKind,
    /// Project contract command key.
    pub command: String,
    /// Literal argument vector, which may contain project-private values.
    pub argv: Vec<String>,
    /// Root joined with the declared relative directory; not a launch guarantee.
    pub working_directory: String,
    /// Effective timeout, including extension overrides and the runtime default.
    pub timeout_seconds: u64,
    /// Whether failure blocks the applicable gate.
    pub blocking: bool,
}

pub(crate) fn selected_suites(
    manifest: &ProjectManifest,
    stage: TestStage,
) -> impl Iterator<Item = &TestSuite> {
    manifest
        .testing
        .suites
        .iter()
        .filter(move |suite| suite.stages.contains(&stage))
}

pub(crate) fn selected_extensions(
    manifest: &ProjectManifest,
    stage: ExtensionStage,
) -> impl Iterator<Item = &ExtensionCheck> {
    manifest
        .extensions
        .checks
        .iter()
        .filter(move |check| check.stage == stage)
}

pub(crate) fn extension_command(manifest: &ProjectManifest, check: &ExtensionCheck) -> CommandSpec {
    let mut command = manifest.commands[&check.command].clone();
    if check.timeout_seconds.is_some() {
        command.timeout_seconds = check.timeout_seconds;
    }
    command
}

fn planned(
    root: &Path,
    id: &str,
    key: &str,
    kind: CheckKind,
    blocking: bool,
    command: &CommandSpec,
) -> PlannedCommand {
    PlannedCommand {
        id: id.into(),
        kind,
        command: key.into(),
        argv: command.argv.clone(),
        working_directory: command
            .working_directory
            .as_ref()
            .map_or_else(|| root.to_path_buf(), |directory| root.join(directory))
            .display()
            .to_string(),
        timeout_seconds: command.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS),
        blocking,
    }
}

/// Preview one check's declared invocations using the evaluator's selectors.
/// No command, extension, evidence review, remote audit or filesystem write occurs.
/// The manifest must have been validated by the project loader.
#[must_use]
pub fn plan_checks(root: &Path, manifest: &ProjectManifest, options: CheckOptions) -> CheckPlan {
    let mut commands: Vec<_> = selected_suites(manifest, options.test_stage)
        .map(|suite| {
            planned(
                root,
                &suite.id,
                &suite.command,
                CheckKind::Suite,
                true,
                &manifest.commands[&suite.command],
            )
        })
        .collect();
    commands.extend(
        selected_extensions(manifest, options.extension_stage).map(|check| {
            planned(
                root,
                &check.id,
                &check.command,
                CheckKind::Extension,
                check.blocking,
                &extension_command(manifest, check),
            )
        }),
    );
    CheckPlan {
        schema: 1,
        kind: "check_plan",
        subject: root.display().to_string(),
        test_stage: options.test_stage,
        extension_stage: options.extension_stage,
        qualification: Outcome::Unverified,
        commands,
    }
}
