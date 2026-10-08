//! CLI presentation of shared, bounded namespace inspection.
use anyhow::{Result, ensure};
use clap::{Args, Subcommand, ValueEnum};
use opdev_project::layout;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::ExitCode};

#[derive(Debug, Args)]
pub(super) struct LayoutArgs {
    #[command(subcommand)]
    command: LayoutCommand,
}
#[derive(Debug, Subcommand)]
enum LayoutCommand {
    /// Preview deterministic YAML formatting; apply only the exact reviewed plan.
    Format {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Staleness guard from the preview, not authenticated developer consent.
        #[arg(long)]
        apply: Option<String>,
    },
    /// Inspect the proposed strict layout without migrating this project.
    Inspect {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        layout_version: u32,
        #[arg(long, value_enum, default_value_t = Scope::WorkingTree)]
        scope: Scope,
        #[arg(long, value_enum, default_value_t = crate::OutputFormat::Human)]
        format: crate::OutputFormat,
    },
}
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Scope {
    WorkingTree,
    Index,
}

pub(super) fn run(args: &LayoutArgs) -> Result<ExitCode> {
    if let LayoutCommand::Format { root, apply } = &args.command {
        return format_config(root, apply.as_deref());
    }
    let LayoutCommand::Inspect {
        root,
        layout_version,
        scope,
        format,
    } = &args.command
    else {
        unreachable!()
    };
    let scope = match scope {
        Scope::WorkingTree => layout::Scope::WorkingTree,
        Scope::Index => layout::Scope::Index,
    };
    let report = layout::inspect(root, *layout_version, scope)?;
    match format {
        crate::OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        crate::OutputFormat::Human => {
            println!(
                "Proposed layout inspection: {} finding(s); qualification not run",
                report.findings.len()
            );
            for finding in &report.findings {
                println!(
                    "{}: {}. {}",
                    finding.path.escape_debug(),
                    finding.problem,
                    finding.next_step
                );
            }
            println!(
                "{}",
                serde_json::to_value(&report)?["limits"]
                    .as_str()
                    .unwrap_or_default()
            );
        }
    }
    Ok(ExitCode::from(u8::from(!report.findings.is_empty())))
}

fn format_config(root: &std::path::Path, apply: Option<&str>) -> Result<ExitCode> {
    let (root, _) = crate::load_project(root)?;
    let preview = opdev_project::preview_configuration_format(&root)?;
    let changes: Vec<_> = preview.iter().map(|p| serde_json::json!({
        "path": p.file.path.strip_prefix(&root).unwrap_or(&p.file.path),
        "before": p.before, "after": p.after, "changed": p.before.as_deref() != Some(p.after.as_str())
    })).collect();
    let subject = serde_json::json!({ "root": root, "changes": changes, "format_version": 1 });
    let plan_id = format!("{:x}", Sha256::digest(serde_json::to_vec(&subject)?));
    if let Some(expected) = apply {
        ensure!(
            expected == plan_id,
            "Configuration changed since the formatting preview; preview again. Nothing changed"
        );
        opdev_project::apply_agent_preview(&preview)?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": 1, "kind": "configuration_format", "plan_id": plan_id,
            "applied": apply.is_some(), "changes": changes, "qualification": "unverified",
            "limits": "Formatting preserves parsed policy and adoption decisions; comments and style can change. Review the exact diff before apply. This does not approve adoption, migrate policy, refresh evidence or run checks. Writes are atomic per file; on interruption inspect and preview again."
        }))?
    );
    Ok(ExitCode::SUCCESS)
}
