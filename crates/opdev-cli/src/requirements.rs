//! Read-only catalog inspection and semantic comparison. No approvals or execution.
use anyhow::{Context, Result, ensure};
use clap::{Args, Subcommand};
use opdev_project::{
    MANIFEST_PATH, ProjectManifest, TrackedEvidence, discover,
    requirements::{self, CatalogSnapshot},
};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf, process::ExitCode};

#[derive(Debug, Args)]
pub struct RequirementsArgs {
    #[command(subcommand)]
    command: RequirementsCommand,
}

#[derive(Debug, Subcommand)]
enum RequirementsCommand {
    /// Print supported catalog JSON Schema; no network, commands or writes.
    Schema {
        /// Catalog format: 1 for historical plans, 2 for practical stage-independent links.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=2))]
        version: u32,
    },
    /// Inspect staged catalog, references and review subjects; no tests or writes.
    Inspect {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Reviewed candidate contract for migration preview; does not select it.
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Read one requirement and its complete linked plans/checks, with no execution.
    Show {
        id: String,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Compare staged guarantees against an explicit accepted Git baseline.
    Diff {
        #[arg(long)]
        base: String,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Bind an exact excerpt to staged Git bytes; this does not review its meaning.
    Bind {
        path: String,
        #[arg(long)]
        excerpt: String,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

pub fn run(args: &RequirementsArgs) -> Result<ExitCode> {
    match &args.command {
        RequirementsCommand::Schema { version } => {
            println!(
                "{}",
                if *version == 2 {
                    include_str!("../../../schema/practical-requirements.schema.json")
                } else {
                    include_str!("../../../schema/requirements.schema.json")
                }
            );
            Ok(ExitCode::SUCCESS)
        }
        RequirementsCommand::Inspect { root, project } => {
            if project.is_none()
                && let Some((root, contract)) = crate::practical::selected(root)?
            {
                let requirements = contract.inspect_requirements(&root)?;
                let subjects: std::collections::BTreeMap<_, _> = requirements
                    .iter()
                    .map(|r| Ok((&r.id, contract.mapping_subject(&root, r)?)))
                    .collect::<anyhow::Result<_>>()?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema":2,"requirements":requirements,"mapping_subjects":subjects,
                        "limits":"Current stored links and mechanical subjects only. No tests ran; no review or completeness claimed."
                    }))?
                );
                return Ok(ExitCode::SUCCESS);
            }
            let root = discover(root)?.root;
            let manifest = ProjectManifest::load(
                &project.clone().unwrap_or_else(|| root.join(MANIFEST_PATH)),
            )?;
            let snapshot = requirements::load_index(&root)?;
            let report = snapshot.inspect(&root, &manifest)?;
            ensure!(
                snapshot.tree == requirements::load_index(&root)?.tree,
                "Source changed during inspection; inspect current inputs again"
            );
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(if report.findings.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        RequirementsCommand::Bind {
            root,
            path,
            excerpt,
        } => {
            let root = discover(root)?.root;
            let reference = TrackedEvidence::bind(&root, path.clone(), excerpt.clone())?;
            println!("{}", serde_json::to_string_pretty(&reference)?);
            Ok(ExitCode::SUCCESS)
        }
        RequirementsCommand::Show { root, id } => {
            if let Some((root, contract)) = crate::practical::selected(root)? {
                show_practical(&root, &contract, id)?;
                return Ok(ExitCode::SUCCESS);
            }
            let snapshot = requirements::load_index(&discover(root)?.root)?;
            let records: Vec<_> = snapshot.requirements().filter(|r| &r.id == id).collect();
            ensure!(records.len() == 1, "Requirement is missing or ambiguous");
            let r = records[0];
            let plans: Vec<_> = snapshot
                .plans()
                .filter(|p| r.criteria.iter().any(|c| c.id == p.criterion))
                .collect();
            let verifications: Vec<_> = snapshot
                .verifications()
                .filter(|v| {
                    plans
                        .iter()
                        .any(|p| p.members.iter().any(|m| m.verification == v.id))
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"schema":1,"qualification":"unverified","complete":true,"requirement":r,"plans":plans,"verifications":verifications,"limits":"Complete stored neighborhood, not proof of valid links, adequate tests or execution; run requirements inspect for gaps."})
                )?
            );
            Ok(ExitCode::SUCCESS)
        }
        RequirementsCommand::Diff { root, base } => {
            let root = discover(root)?.root;
            let before = requirements::load_revision(&root, base)?;
            let after = requirements::load_index(&root)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&difference(&before, &after)?)?
            );
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn show_practical(
    root: &std::path::Path,
    contract: &opdev_project::practical::Contract,
    id: &str,
) -> Result<()> {
    let requirements = contract.inspect_requirements(root)?;
    let requirement = requirements
        .iter()
        .find(|r| r.id == id)
        .context("Requirement is missing")?;
    let checks: Vec<_> = contract
        .checks
        .iter()
        .filter(|c| requirement.verification.iter().any(|v| v.check == c.id))
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema":2, "requirement":requirement, "checks":checks,
            "mapping_subject":contract.mapping_subject(root, requirement)?,
            "limits":"Current stored links only; no tests ran, adequacy or execution claimed."
        }))?
    );
    Ok(())
}

#[derive(Serialize)]
struct Difference {
    schema: u32,
    qualification: &'static str,
    base_tree: String,
    candidate_tree: String,
    base_catalog_sha256: String,
    candidate_catalog_sha256: String,
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<String>,
    limits: &'static str,
}

fn records(snapshot: &CatalogSnapshot) -> Result<BTreeMap<String, serde_json::Value>> {
    let mut result = BTreeMap::new();
    for value in snapshot
        .requirements()
        .map(serde_json::to_value)
        .chain(snapshot.verifications().map(serde_json::to_value))
        .chain(snapshot.plans().map(serde_json::to_value))
    {
        let value = value?;
        let id = value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .context("Missing record identity")?
            .to_owned();
        ensure!(
            result.insert(id, value).is_none(),
            "Duplicate catalog identity; semantic comparison is ambiguous"
        );
    }
    Ok(result)
}

fn difference(before: &CatalogSnapshot, after: &CatalogSnapshot) -> Result<Difference> {
    let old = records(before)?;
    let new = records(after)?;
    Ok(Difference {
        schema: 1,
        qualification: "unverified",
        base_tree: before.tree.clone(),
        candidate_tree: after.tree.clone(),
        base_catalog_sha256: before.digest()?,
        candidate_catalog_sha256: after.digest()?,
        added: new
            .keys()
            .filter(|k| !old.contains_key(*k))
            .cloned()
            .collect(),
        removed: old
            .keys()
            .filter(|k| !new.contains_key(*k))
            .cloned()
            .collect(),
        changed: new
            .iter()
            .filter(|(k, v)| old.get(*k).is_some_and(|old| old != *v))
            .map(|(k, _)| k.clone())
            .collect(),
        limits: "Review removals, applicability and assertion changes against the accepted baseline. A textual change may strengthen tests; this diff does not classify semantic weakening or authenticate authorization. No tests ran.",
    })
}
