//! Read-only structural inspection. Attribution is never authenticated consent.
use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result, ensure};
use opdev_project::{AuthorityKind, ProjectManifest, practical::read_regular};
use serde::{Deserialize, Serialize};

pub(super) const LIMITS: &str = "Document structure and attributed decisions only. Read the original decisions and current content; adequacy, consent, roadmap approval and implementation authority are not established. No checks ran or files changed.";
const ROLES: [&str; 3] = ["design", "interface", "tech_stack"];

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Metadata {
    schema: u32,
    role: String,
    decision: DecisionKind,
    reference: Option<String>,
    scope: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum DecisionKind {
    Proposed,
    Inferred,
    Accepted,
    Delegated,
}

#[derive(Deserialize)]
struct Frontmatter {
    opdev_document: Metadata,
}

#[derive(Serialize)]
pub(super) struct Document {
    role: &'static str,
    path: Option<String>,
    decision: Option<Metadata>,
    finding: Option<String>,
}

#[derive(Serialize)]
pub(super) struct Report {
    schema: u32,
    documents: Vec<Document>,
    pub(super) recorded_decisions_complete: bool,
    pub(super) next_step: String,
    limits: &'static str,
}

impl Report {
    pub(super) fn gaps(&self) -> Vec<String> {
        self.documents
            .iter()
            .filter_map(|doc| {
                doc.finding
                    .as_ref()
                    .map(|finding| format!("{}: {finding}", doc.role))
            })
            .collect()
    }

    pub(super) fn print(&self) {
        println!(
            "Project design: {}",
            if self.recorded_decisions_complete {
                "decision references recorded; actual acceptance needs review"
            } else {
                "unresolved documents or decisions"
            }
        );
        for gap in self.gaps() {
            println!("  {gap}");
        }
        println!("Next: {}\n{}", self.next_step, self.limits);
    }
}

pub(super) fn legacy(root: &Path, manifest: &ProjectManifest) -> Report {
    let owners = manifest
        .authorities
        .iter()
        .map(|(name, owner)| {
            (
                name.clone(),
                if owner.kind == AuthorityKind::Path {
                    owner.location.clone()
                } else {
                    format!("external:{}", owner.location)
                },
            )
        })
        .collect();
    inspect(root, &owners)
}

pub(super) fn inspect(root: &Path, authorities: &BTreeMap<String, String>) -> Report {
    let documents: Vec<_> = ROLES.into_iter().map(|role| {
        let mut document = Document { role, path: None, decision: None, finding: None };
        let result = (|| -> Result<Metadata> {
            let path = if let Some(path) = authorities.get(role) {
                path.clone()
            } else {
                let directory = authorities.get("documentation").map_or("docs", String::as_str);
                ensure!(!Path::new(directory).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md")) && !root.join(directory).is_file(), "Documentation owner is a file; declare a separate {role} authority path. Nothing moved");
                format!("{}/{role}.md", directory.trim_end_matches('/'))
            };
            document.path = Some(path.clone());
            opdev_project::practical::relative_path(&path)?;
            ensure!(!path.split('/').any(|part| part.eq_ignore_ascii_case(".opdev") || part.eq_ignore_ascii_case(".git")), "Design documents belong outside OpDev and Git state directories");
            ensure!(Path::new(&path).file_name().and_then(|s| s.to_str()).is_some_and(|name| name.eq_ignore_ascii_case(&format!("{role}.md"))), "Required filename is {role}.md (case-equivalent existing names are supported)");
            let content = read_regular(root, &path).with_context(|| format!("Required document {path} is missing or unreadable; inspect existing sources and populate it without inventing approval"))?;
            parse(&content, role)
        })();
        match result {
            Ok(decision) => {
                if matches!(decision.decision, DecisionKind::Proposed | DecisionKind::Inferred) {
                    document.finding = Some("Not accepted: discuss material unknowns and review the proposal; inferred facts are not developer decisions".into());
                }
                document.decision = Some(decision);
            }
            Err(error) => document.finding = Some(format!("{error:#}")),
        }
        document
    }).collect();
    let unresolved = documents.iter().find(|doc| doc.finding.is_some());
    Report {
        schema: 1,
        recorded_decisions_complete: unresolved.is_none(),
        next_step: match unresolved.map(|doc| doc.role) {
            Some("design") => "Understand the intended outcome and review the abstract product design.",
            Some("interface") => "Discuss consumer journeys and review the observable interface contract.",
            Some(_) => "Resolve stack preferences and propose the technical approach and development standards.",
            None => "Read the original decisions for current scope, then review the phased MVP roadmap and implementation authorization at the work authority. Recorded references alone do not authorize coding.",
        }.into(),
        documents,
        limits: LIMITS,
    }
}

fn parse(content: &str, role: &str) -> Result<Metadata> {
    let content = content.replace("\r\n", "\n");
    let rest = content.strip_prefix("---\n").context("Missing opdev_document YAML frontmatter; retain existing content and add actual decision metadata")?;
    let (header, body) = rest
        .split_once("\n---\n")
        .context("Unclosed document frontmatter")?;
    let decision = serde_saphyr::from_str::<Frontmatter>(header)?.opdev_document;
    ensure!(
        decision.schema == 1 && decision.role == role,
        "Document schema or role does not match this document"
    );
    ensure!(
        !decision.scope.trim().is_empty(),
        "Decision scope is missing"
    );
    ensure!(
        body.lines()
            .any(|line| !line.trim().is_empty() && !line.trim().starts_with('#')),
        "Document has no substantive body; headings alone are not a design"
    );
    if matches!(
        decision.decision,
        DecisionKind::Accepted | DecisionKind::Delegated
    ) {
        ensure!(
            decision
                .reference
                .as_ref()
                .is_some_and(|r| !r.trim().is_empty()),
            "Accepted or delegated decisions need an actual reference; no consent inferred"
        );
    }
    Ok(decision)
}
