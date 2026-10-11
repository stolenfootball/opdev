//! Read-only policy definition inspection, deliberately separate from qualification.

use std::{path::PathBuf, process::ExitCode};

use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use opdev_core::{PolicyResolution, resolve_engineering_policy};
use serde::Serialize;

use crate::{OutputFormat, load_project};

#[derive(Debug, Args)]
pub(super) struct PolicyArgs {
    #[command(subcommand)]
    command: PolicyCommand,
}

#[derive(Debug, Subcommand)]
enum PolicyCommand {
    /// Explain an exact embedded policy; does not load or migrate a project.
    Explain {
        /// Exact engineering definition version, not a latest alias.
        #[arg(long)]
        engineering: String,
        /// Optional exact rule ID, including a historical replaced rule.
        #[arg(long)]
        rule: Option<String>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
    /// Compare the current project selection with a proposed definition, without writes.
    Preview {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        engineering: String,
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
}

#[derive(Serialize)]
struct Preview {
    schema: u32,
    current_project_schema: u32,
    current_catalog: u32,
    current_engineering: Option<String>,
    current_minimumcd: Option<String>,
    added_engineering: Vec<String>,
    removed_engineering: Vec<String>,
    changed_engineering: Vec<String>,
    proposed: PolicyResolution,
    limits: &'static str,
}

pub(super) fn run(args: &PolicyArgs) -> Result<ExitCode> {
    match &args.command {
        PolicyCommand::Explain {
            engineering,
            rule,
            format,
        } => {
            let resolved = resolve_engineering_policy(engineering)?;
            if let Some(id) = rule {
                let selected = resolved
                    .controls
                    .iter()
                    .map(|c| &c.rule)
                    .chain(&resolved.external_rules)
                    .find(|r| r.id.as_str() == id);
                let Some(selected) = selected else {
                    bail!("Unknown rule {id} for engineering policy {engineering}. No checks ran.");
                };
                if matches!(format, OutputFormat::Json) {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "schema": 1, "engineering_version": engineering,
                            "definition_sha256": resolved.definition_sha256,
                            "rule": selected,
                            "change": resolved.changes.iter().find(|c| c.rule.as_str() == id || c.replacement.as_ref().is_some_and(|r| r.as_str() == id)),
                            "limits": resolved.limits,
                        }))?
                    );
                } else {
                    let role = if resolved.controls.iter().any(|c| c.rule.id == selected.id) {
                        "engineering obligation"
                    } else {
                        "historical external-standard obligation, not an engineering gate"
                    };
                    println!(
                        "{}: {}\nRole: {role}\n{}\nApplies to: {}\nDefinition boundaries: {:?}\n{}",
                        selected.id,
                        selected.title,
                        selected.statement,
                        selected.applicability,
                        selected.gates,
                        resolved.limits
                    );
                }
            } else if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&resolved)?);
            } else {
                print_resolution(&resolved);
            }
        }
        PolicyCommand::Preview {
            root,
            engineering,
            format,
        } => {
            let (_, project) = load_project(root)?;
            let current = project.assurance.engineering.as_ref();
            let proposed = resolve_engineering_policy(engineering)?;
            let catalog = project.catalog()?;
            let (added_engineering, removed_engineering, changed_engineering) =
                difference(&catalog, current.is_some(), &proposed);
            let preview = Preview {
                schema: 1,
                current_project_schema: project.schema,
                current_catalog: project.catalog()?.catalog_version,
                current_engineering: current.map(|p| p.version.clone()),
                current_minimumcd: current.and_then(|p| p.minimumcd.clone()),
                added_engineering,
                removed_engineering,
                changed_engineering,
                proposed,
                limits: "Proposed definition only, not a project migration plan or approval. Existing standard selections, commands, capability facts and decisions are unchanged. Compare actual applicability and change-control authority before migration. No checks ran.",
            };
            if matches!(format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&preview)?);
            } else {
                println!(
                    "Current project schema {}, catalog {}, engineering {:?}, MinimumCD {:?}",
                    preview.current_project_schema,
                    preview.current_catalog,
                    preview.current_engineering,
                    preview.current_minimumcd
                );
                print_resolution(&preview.proposed);
                println!(
                    "Compared with this project's engineering obligations:\nAdded: {:?}\nRemoved: {:?}\nChanged: {:?}",
                    preview.added_engineering,
                    preview.removed_engineering,
                    preview.changed_engineering
                );
                println!("{}", preview.limits);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_resolution(policy: &PolicyResolution) {
    println!(
        "Engineering policy {} definition: {}",
        policy.engineering_version, policy.definition_sha256
    );
    for change in &policy.changes {
        println!(
            "{}: {:?}{} — {}",
            change.rule,
            change.disposition,
            change
                .replacement
                .as_ref()
                .map_or_else(String::new, |id| format!(" -> {id}")),
            change.rationale
        );
    }
    println!("{}", policy.limits);
}

fn difference(
    catalog: &opdev_core::RuleCatalog,
    engineering_selected: bool,
    proposed: &PolicyResolution,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let before: Vec<_> = catalog
        .rules
        .iter()
        .filter(|r| {
            !engineering_selected
                || opdev_core::rule_class(r.id.as_str())
                    != Some(opdev_core::RuleClass::MinimumcdAssessment)
        })
        .collect();
    let after: Vec<_> = proposed.controls.iter().map(|c| &c.rule).collect();
    let added = after
        .iter()
        .filter(|r| !before.iter().any(|old| old.id == r.id))
        .map(|r| r.id.to_string())
        .collect();
    let removed = before
        .iter()
        .filter(|r| !after.iter().any(|new| new.id == r.id))
        .map(|r| r.id.to_string())
        .collect();
    let changed = before
        .iter()
        .filter(|r| after.iter().any(|new| new.id == r.id && *new != **r))
        .map(|r| r.id.to_string())
        .collect();
    (added, removed, changed)
}
