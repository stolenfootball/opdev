//! Derived adoption dependencies, not a second approval or execution authority.
use anyhow::Result;
use opdev_project::{ADOPTION_PATH, AdoptionRecord, AdoptionState, MANIFEST_PATH, ProjectManifest};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct Fact {
    id: String,
    kind: &'static str,
    ready: bool,
    details: Vec<String>,
}

#[derive(Serialize)]
struct Action {
    id: String,
    kind: &'static str,
    depends_on: Vec<String>,
    ready: bool,
    argv: Option<Vec<String>>,
    instruction: &'static str,
}

fn action(
    facts: &[Fact],
    id: &str,
    kind: &'static str,
    depends: &[&str],
    argv: Option<Vec<String>>,
    instruction: &'static str,
) -> Action {
    Action {
        id: id.into(),
        kind,
        depends_on: depends.iter().map(|s| (*s).into()).collect(),
        ready: depends
            .iter()
            .all(|id| facts.iter().any(|f| &f.id == id && f.ready)),
        argv,
        instruction,
    }
}

/// Every call reads original authorities again. No cache, writes or project execution.
pub(super) fn inspect(
    root: &Path,
    manifest: &ProjectManifest,
    record: Option<&AdoptionRecord>,
) -> Result<serde_json::Value> {
    let mut facts = observe(root, manifest, record)?;
    let design = crate::design_documents::legacy(root, manifest);
    facts.push(Fact {
        id: "design_documents".into(),
        kind: "attributed_decisions",
        ready: design.recorded_decisions_complete,
        details: design.gaps(),
    });
    let configured = facts.iter().any(|f| f.id == "configured" && f.ready);
    let mut actions = next_actions(root, record, configured, &mut facts);
    if !design.recorded_decisions_complete {
        actions.insert(0, action(&facts, "review_project_design", "decision_review", &[], None,
            "Populate design.md, interface.md and tech_stack.md from inspected facts and actual decisions. Resolve material gaps with the developer; document labels cannot authenticate acceptance. This is not permission to alter policy or start product implementation."));
    }
    Ok(serde_json::json!({
        "schema":1,"kind":"adoption_coordination","target":opdev_project::clean_adoption::target_name(manifest),
        "facts":facts,"actions":actions,"source_fingerprint":opdev_project::staged_fingerprint(root).ok(),
        "design_documents":design,
        "policy_selection":super::policy_selection(configured),
        "qualification":"unverified","complete":false,"checks_ran":false,"writes_performed":false,
        "authorities":[MANIFEST_PATH,ADOPTION_PATH],
        "limits":"Derived next actions, not approval or completion. Ready means prerequisites for the described activity only. Original decision scope controls every mutation. Reinspect changed state; full checks always revalidate current inputs. No release is authorized."
    }))
}

fn observe(
    root: &Path,
    manifest: &ProjectManifest,
    record: Option<&AdoptionRecord>,
) -> Result<Vec<Fact>> {
    let configured = root.join(MANIFEST_PATH).try_exists()?;
    let policy = opdev_project::clean_adoption::policy_gaps(manifest);
    let decisions = opdev_project::clean_adoption::decision_gaps(manifest, record);
    let approval = record
        .map(|r| r.approval_blockers(manifest))
        .transpose()?
        .unwrap_or_else(|| vec!["No adoption assessment is recorded".into()]);
    let implementation = record
        .map(|r| r.blockers(manifest))
        .transpose()?
        .unwrap_or_else(|| vec!["No adoption assessment is recorded".into()]);
    let retirements = record
        .and_then(|r| r.clean_target.as_ref())
        .map(|t| opdev_project::clean_adoption::retirement_gaps(root, t))
        .transpose()
        .map_err(anyhow::Error::msg)?
        .unwrap_or_default();
    let staged = opdev_project::staged_fingerprint(root);
    Ok(vec![
        Fact {
            id: "configured".into(),
            kind: "observation",
            ready: configured,
            details: if configured {
                vec![]
            } else {
                vec!["Discovery is a proposal. Choose and review a project contract before initialization.".into()]
            },
        },
        Fact {
            id: "policy".into(),
            kind: "observation",
            ready: configured && policy.is_empty(),
            details: policy,
        },
        Fact {
            id: "choices".into(),
            kind: "developer_decision",
            ready: record.is_some() && decisions.is_empty(),
            details: decisions,
        },
        Fact {
            id: "approval".into(),
            kind: "attributed_permission",
            ready: record.is_some() && approval.is_empty(),
            details: approval,
        },
        Fact {
            id: "implementation".into(),
            kind: "reviewed_claim",
            ready: record.is_some() && implementation.is_empty(),
            details: implementation,
        },
        Fact {
            id: "cleanup".into(),
            kind: "observation",
            ready: record.and_then(|r| r.clean_target.as_ref()).is_some() && retirements.is_empty(),
            details: retirements,
        },
        Fact {
            id: "source".into(),
            kind: "observation",
            ready: staged.is_ok(),
            details: staged
                .as_ref()
                .err()
                .map(|e| vec![e.to_string()])
                .unwrap_or_default(),
        },
    ])
}

fn next_actions(
    root: &Path,
    record: Option<&AdoptionRecord>,
    configured: bool,
    facts: &mut Vec<Fact>,
) -> Vec<Action> {
    let command = |parts: &[&str]| {
        Some(
            parts
                .iter()
                .map(|s| (*s).to_owned())
                .chain(["--root".into(), root.to_string_lossy().into_owned()])
                .collect(),
        )
    };
    let mut actions = vec![
        action(
            facts,
            "inspect_runtime",
            "observation",
            &[],
            command(&["doctor"]),
            "Inspect the local runtime and compare the actual CI reader capabilities. A version label or plan is not runtime parity.",
        ),
        action(
            facts,
            "review_choices",
            "developer_decision",
            &[],
            command(&["adoption", "plan"]),
            "Read original decisions, assess actual capabilities and resolve material choices. Preserve adequate tooling; unknown is not absent. A recorded reference does not authenticate consent.",
        ),
        action(
            facts,
            "prepare_migration",
            "planning",
            &["configured"],
            None,
            "For changed policy, prepare an explicit upgrade --migration request from reviewed choices; preview its exact diff. Do not initialize over existing policy or invent approval.",
        ),
        action(
            facts,
            "rehearse_handoff",
            "observation",
            &["configured"],
            None,
            "Inspect the actual CI includes and consumer commands. Exercise the smallest safe review-to-verifier handoff; no publication or release is authorized.",
        ),
    ];
    if !configured {
        actions.push(action(facts,"initialize","transformation", &["choices","approval"], None, "Resolve the material project choices, then preview init --project with the reviewed contract. No code means the first real behavior and checks are still needed; do not invent passing suites."));
    }
    if let Some(record) = record {
        for (id, practice) in &record.practices {
            if matches!(
                practice.state,
                AdoptionState::Pending | AdoptionState::InProgress
            ) {
                actions.push(action(facts, &format!("implement:{id}"), "engineering_work", &["configured","policy","choices","approval"], None, "Implement this approved practice with meaningful checks. Other independent approved practices may proceed. Follow real decision scope, not merely this readiness observation."));
            }
        }
    } else if configured {
        actions.push(action(
            facts,
            "start_assessment",
            "transformation",
            &["configured"],
            command(&["adoption", "start", "--dry-run"]),
            "Preview pending assessment; starting it is explicit and does not select new policy.",
        ));
    }
    // Input-selection operations deliberately have no fabricated locator or baseline.
    let preparation = [
        "configured",
        "policy",
        "choices",
        "approval",
        "implementation",
        "cleanup",
        "source",
    ];
    actions.push(action(facts,"prepare_review","review", &preparation, None, "Use adoption prepare-evidence --complete with the exact provider-observed target --base when requirements are selected. Resolve substantive review slots, then native evidence preparation/export. Never synthesize passing judgments."));
    facts.push(Fact {id:"review_selection".into(),kind:"observation",ready:false,details:vec!["Select the exact reviewed comment and independent acceptance identity; inspection does not retrieve or certify it.".into()]});
    actions.push(action(facts,"preflight","observation", &preparation, None, "Run adoption check --preflight with the actual review locator and acceptance identity before costly suites. Missing or stale inputs cannot execute project checks."));
    actions.push(action(facts,"verify","execution", &["configured","policy","choices","approval","implementation","cleanup","source","review_selection","design_documents"], None, "adoption check revalidates the selection and runs required suites itself. This inspection never permits bypassing its guards or reusing a saved preflight/report."));
    actions.push(action(facts,"reconcile_integration","observation", &["configured"], None, "After required local/pre-merge checks and authorized integration, observe required integrated checks with ci qualify. Missing integrated evidence remains incomplete; artifact delivery and release are separate."));
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_requires_every_current_dependency_and_never_a_previous_result() {
        for mask in 0..8 {
            let facts: Vec<_> = ["policy", "approval", "source"]
                .iter()
                .enumerate()
                .map(|(i, id)| Fact {
                    id: (*id).into(),
                    kind: "observation",
                    ready: mask & (1 << i) != 0,
                    details: vec![],
                })
                .collect();
            let next = action(
                &facts,
                "verify",
                "execution",
                &["policy", "approval", "source"],
                None,
                "test",
            );
            assert_eq!(next.ready, mask == 7);
            assert!(!action(&facts, "verify", "execution", &["missing"], None, "test").ready);
        }
    }
}
