//! Prospective authority routing, not semantic classification or write permission.

use std::{fs, io, path::Path, process::ExitCode};

use anyhow::{Result, bail};
use clap::{Args, Subcommand, ValueEnum};
use opdev_project::{AuthorityKind, AuthorityRef, ProjectManifest};
use serde::Serialize;

use crate::{OutputFormat, load_project};

const LIMITS: &str = "Routing only: content meaning, existing-content adequacy, access and write permission remain for review. No files, tracker items or project commands were changed or executed. This is not verification or approval.";

#[derive(Debug, Args)]
pub(super) struct DocumentationArgs {
    #[command(subcommand)]
    command: DocumentationCommand,
}

#[derive(Debug, Subcommand)]
enum DocumentationCommand {
    /// Check a proposed destination against declared ownership; never write it.
    Plan(PlanArgs),
}

#[derive(Debug, Args)]
struct PlanArgs {
    /// Directory inside the initialized project.
    #[arg(long, default_value = ".")]
    root: std::path::PathBuf,
    /// Caller-reviewed content purpose; the CLI does not classify prose.
    #[arg(long, value_enum)]
    purpose: Purpose,
    /// Declared owner of durable content, e.g. architecture or documentation.
    #[arg(long)]
    authority: Option<String>,
    /// Proposed repository-relative path or exact external authority identifier.
    #[arg(long)]
    target: Option<String>,
    /// Human text or schema-versioned JSON. Exit 1 means routing is unresolved.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,
}

#[derive(Debug, Clone, Copy, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum Purpose {
    Durable,
    Work,
    Temporary,
    Mixed,
}

#[derive(Serialize)]
struct Route {
    authority: String,
    declared: Option<AuthorityRef>,
    target: Option<String>,
    resolved: bool,
    next_step: String,
}

#[derive(Serialize)]
struct Report {
    schema: u32,
    purpose: Purpose,
    routes: Vec<Route>,
    resolved: bool,
    next_step: &'static str,
    limits: &'static str,
}

pub(super) fn run(args: &DocumentationArgs) -> Result<ExitCode> {
    let DocumentationCommand::Plan(args) = &args.command;
    let durable = matches!(args.purpose, Purpose::Durable | Purpose::Mixed);
    if durable != args.authority.is_some() {
        bail!(
            "Use --authority for durable or mixed content only; work uses the declared work authority."
        );
    }
    if matches!(args.purpose, Purpose::Mixed | Purpose::Temporary) && args.target.is_some() {
        bail!(
            "Mixed content needs separate durable/work destinations; temporary notes stay in conversation or disposable context. Check retained destinations separately without a combined --target."
        );
    }
    if args.authority.as_deref() == Some("work") {
        bail!(
            "Use --purpose work for implementation tracking; select its actual durable owner for permanent documentation."
        );
    }
    let (root, project) = load_project(&args.root)?;
    let mut routes = Vec::new();
    if let Some(authority) = &args.authority {
        routes.push(route(&root, &project, authority, args.target.as_deref()));
    }
    if matches!(args.purpose, Purpose::Work | Purpose::Mixed) {
        routes.push(route(&root, &project, "work", args.target.as_deref()));
    }
    let report = Report {
        schema: 1,
        purpose: args.purpose,
        resolved: routes.iter().all(|route| route.resolved),
        routes,
        next_step: match args.purpose {
            Purpose::Mixed => {
                "Split durable behavior/rationale from tasks, sequencing and progress. Reuse existing material at each authority and cross-link without copying live status. If work access or permission is unavailable, keep that proposal in the conversation, not a substitute repository backlog."
            }
            Purpose::Temporary => {
                "Keep proportionate notes in the conversation or disposable task context. No project document or registry is required."
            }
            Purpose::Work => {
                "Inspect and reuse the existing work item. If access or write permission is unavailable, keep the proposal in the conversation; do not create a substitute tracker."
            }
            Purpose::Durable => {
                "Inspect and reuse adequate existing documentation. Permanent operator steps and examples can stay here; task-specific acceptance, sequencing and progress belong to work tracking. Apply existing task authorization; ask only about an unresolved material choice."
            }
        },
        limits: LIMITS,
    };
    match args.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputFormat::Human => {
            println!(
                "Document placement: {}",
                if report.resolved {
                    "declared routing resolved"
                } else {
                    "unresolved; review needed before writing"
                }
            );
            for route in &report.routes {
                println!("{}: {}", route.authority.escape_debug(), route.next_step);
            }
            println!("{}\n{}", report.next_step, report.limits);
        }
    }
    Ok(ExitCode::from(u8::from(!report.resolved)))
}

fn route(root: &Path, project: &ProjectManifest, name: &str, target: Option<&str>) -> Route {
    let declared = project.authorities.get(name).cloned();
    let result = declared.as_ref().map_or_else(
        || Err("No declared owner. Resolve ownership with the developer; keep the proposal in the conversation meanwhile.".to_owned()),
        |authority| match authority.kind {
            AuthorityKind::Path => path_route(root, &authority.location, target),
            AuthorityKind::Url | AuthorityKind::Tracker => {
                if target.is_some_and(|target| target != authority.location) {
                    Err("External target membership is not established. Inspect the declared authority using authorized read-only access; this command does not contact providers or infer ownership from URL prefixes.".to_owned())
                } else {
                    Ok("Use the declared external authority; inspect existing content and access separately. If unavailable or unauthorized, keep the proposal in the conversation.".to_owned())
                }
            }
        },
    );
    Route {
        authority: name.to_owned(),
        declared,
        target: target.map(str::to_owned),
        resolved: result.is_ok(),
        next_step: result.unwrap_or_else(|reason| reason),
    }
}

// Treat both separator styles portably; reject Windows drives, ADS and traversal
// even on Unix. This check establishes placement, not an authorization boundary.
fn components(value: &str) -> Result<Vec<&str>, String> {
    if value.is_empty()
        || value.starts_with(['/', '\\'])
        || value.contains(':')
        || value.chars().any(char::is_control)
    {
        return Err(
            "Use a repository-relative path without a drive, traversal or control characters."
                .to_owned(),
        );
    }
    let parts: Vec<_> = value
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    if parts
        .iter()
        .any(|part| *part == ".." || part.ends_with([' ', '.']))
    {
        return Err("Use a repository-relative path without parent traversal or ambiguous trailing characters.".to_owned());
    }
    Ok(parts)
}

fn path_route(root: &Path, owner: &str, target: Option<&str>) -> Result<String, String> {
    let owner_parts = components(owner)?;
    let target_parts = components(target.unwrap_or(owner))?;
    if !target_parts.starts_with(&owner_parts) {
        return Err("The proposed path is outside the declared authority. Use that authority or resolve a deliberate ownership change first.".to_owned());
    }
    let mut path = root.to_path_buf();
    for (index, component) in target_parts.iter().enumerate() {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err("The destination traverses a link or leaves the repository. Inspect ownership and the real destination before writing; no link was followed for content.".to_owned());
                }
                let resolved = path.canonicalize().map_err(|_| "The destination could not be resolved. Inspect filesystem access before writing; no ownership was inferred.".to_owned())?;
                if !resolved.starts_with(root) || !(metadata.is_file() || metadata.is_dir()) {
                    return Err("The destination is outside the repository or is not a regular file/directory. Resolve its ownership and type before writing.".to_owned());
                }
                if index + 1 < target_parts.len() && !metadata.is_dir() {
                    return Err("A parent path is a file, not a directory. Reuse the declared file or resolve the conflicting destination.".to_owned());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if index < owner_parts.len() {
                    return Err("The declared authority does not exist. Inspect intended ownership and choose its file/directory role before creating content; no placeholder was created.".to_owned());
                }
                break;
            }
            Err(_) => return Err("The destination could not be inspected. Resolve filesystem access before writing; no ownership was inferred.".to_owned()),
        }
    }
    Ok(format!(
        "Declared repository owner: {owner}. Reuse adequate existing content and check task write permission before editing."
    ))
}
