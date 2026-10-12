//! Native integration-candidate correspondence, never a caller-supplied green receipt.
use std::{path::Path, process::Command};

use opdev_project::{CiProvider, ProjectManifest};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Outcome, Repository, RunExpectation,
    qualification::Api,
    run::{number, string},
};

/// A selector only; these fields provide no evidence or permission by themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateSelection {
    /// Native pipeline/run being proposed for source-check reuse.
    pub run: RunExpectation,
    /// Exact MR/PR which has actually integrated.
    pub change: u64,
    /// Exact integrated commit, not a caller-selected tree alias.
    pub integrated_revision: String,
}

/// Sealed GET observation. Cannot be deserialized from local reports.
pub struct IntegratedCandidate {
    selection: CandidateSelection,
    repository: String,
    provider: CiProvider,
    attempt: Option<u64>,
}

impl IntegratedCandidate {
    /// Exact native source run observed at the provider.
    #[must_use]
    pub const fn selection(&self) -> &CandidateSelection {
        &self.selection
    }
    /// Provider-verified repository slug.
    #[must_use]
    pub fn repository(&self) -> &str {
        &self.repository
    }
    /// Current run attempt; GitLab retries have separate producer job IDs.
    #[must_use]
    pub const fn attempt(&self) -> Option<u64> {
        self.attempt
    }
    /// Re-observe mutable provider correspondence; unavailable is not unchanged.
    /// # Errors
    /// Refuse changed run, merge/train correspondence or attempt.
    pub fn recheck(&self, manifest: &ProjectManifest) -> Result<(), String> {
        let current = observe_integrated_candidate(manifest, &self.selection)?;
        if current.repository != self.repository
            || current.provider != self.provider
            || current.attempt != self.attempt
        {
            return Err(
                "Native candidate identity changed during verification; run fresh checks".into(),
            );
        }
        Ok(())
    }
    /// Verify local checkout correspondence without fetching or changing source.
    /// # Errors
    /// Missing objects, different bytes or a checkout other than the merged commit fail.
    pub fn verify_checkout(&self, root: &Path) -> Result<(), String> {
        let git = |args: &[&str]| -> Result<String, String> {
            let out = Command::new("git")
                .arg("-C")
                .arg(root)
                .arg("--no-replace-objects")
                .args(args)
                .output()
                .map_err(|_| "Candidate source inspection failed")?;
            if !out.status.success() {
                return Err("Candidate source objects are unavailable; use fresh execution".into());
            }
            String::from_utf8(out.stdout)
                .map(|v| v.trim().to_owned())
                .map_err(|_| "Invalid source identity".into())
        };
        let selected = &self.selection;
        if git(&["rev-parse", "--verify", "HEAD^{commit}"])? != selected.integrated_revision {
            return Err("Checkout is not the provider-observed integrated commit; run this check on the intended source".into());
        }
        let candidate = git(&[
            "rev-parse",
            "--verify",
            &format!("{}^{{tree}}", selected.run.revision),
        ])?;
        let integrated = git(&[
            "rev-parse",
            "--verify",
            &format!("{}^{{tree}}", selected.integrated_revision),
        ])?;
        if candidate != integrated {
            return Err(
                "Integrated source differs from the native candidate; run fresh checks".into(),
            );
        }
        Ok(())
    }
}

/// Observe completed native merge correspondence using existing read-only credentials.
/// This proves neither test execution nor equivalence of commands/environments.
/// # Errors
/// Unsupported plans/access, ordinary branch jobs, skipped trains and changed merges fail closed.
pub fn observe_integrated_candidate(
    manifest: &ProjectManifest,
    selection: &CandidateSelection,
) -> Result<IntegratedCandidate, String> {
    let repository = Repository::parse(
        manifest
            .project
            .ci
            .remote
            .as_deref()
            .ok_or("Project remote is missing")?,
    )
    .map_err(|_| "Project remote is unsupported")?;
    crate::run::validate(&repository, &selection.run)
        .map_err(|_| "Native run selector is invalid")?;
    if repository.provider != manifest.project.ci.provider
        || selection.change == 0
        || !sha(&selection.integrated_revision)
    {
        return Err("Native candidate selector does not match the project".into());
    }
    let run = crate::verify_run(manifest, &selection.run)
        .map_err(|_| "Native run could not be inspected")?;
    if run.outcome != Outcome::Passed {
        return Err("Native candidate run is not currently successful; inspect its current failure or run fresh checks".into());
    }
    let api = Api::new(&repository).map_err(|_| "Native candidate client unavailable")?;
    let github = repository.provider == CiProvider::Github;
    let change = api.get(&format!(
        "/{}/{}",
        if github { "pulls" } else { "merge_requests" },
        selection.change
    ))?;
    let train = if github {
        Value::Null
    } else {
        api.get(&format!(
            "/merge_trains/merge_requests/{}",
            selection.change
        ))?
    };
    correspondence(
        &repository,
        &manifest.project.trunk,
        selection,
        &change,
        &train,
    )?;
    Ok(IntegratedCandidate {
        selection: selection.clone(),
        repository: repository.slug(),
        provider: repository.provider,
        attempt: run.observed.and_then(|r| r.attempt),
    })
}

fn sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn correspondence(
    repository: &Repository,
    trunk: &str,
    selected: &CandidateSelection,
    change: &Value,
    train: &Value,
) -> Result<(), String> {
    let github = repository.provider == CiProvider::Github;
    if string(change, "merge_commit_sha")? != selected.integrated_revision {
        return Err(
            "Selected commit is not this change's observed integration; no candidate reused".into(),
        );
    }
    if github {
        if change["merged"] != true
            || number(change, "number")? != selected.change
            || string(&change["base"]["repo"], "full_name")? != repository.slug()
            || string(&change["base"], "ref")? != trunk
            || selected.run.source != "merge_group"
            || !selected
                .run
                .reference
                .starts_with(&format!("gh-readonly-queue/{trunk}/"))
            || selected.run.revision != selected.integrated_revision
        {
            return Err("GitHub reuse needs the exact integrated merge-group commit and target. A matching tree or ordinary PR job is insufficient; use fresh checks for rewritten commits".into());
        }
    } else if change["state"] != "merged"
        || number(change, "iid")? != selected.change
        || string(change, "target_branch")? != trunk
        || string(change, "web_url")?
            != format!(
                "https://gitlab.com/{}/-/merge_requests/{}",
                repository.slug(),
                selected.change
            )
        || train["status"] != "merged"
        || string(train, "target_branch")? != trunk
        || number(&train["merge_request"], "iid")? != selected.change
        || train["merge_request"]["state"] != "merged"
        || number(&train["merge_request"], "project_id")? != number(change, "project_id")?
        || number(&train["pipeline"], "project_id")? != number(change, "project_id")?
        || number(&train["pipeline"], "id")? != selected.run.run_id
        || string(&train["pipeline"], "sha")? != selected.run.revision
        || string(&train["pipeline"], "ref")? != selected.run.reference
        || string(&train["pipeline"], "source")? != selected.run.source
        || train["pipeline"]["status"] != "success"
    {
        return Err("GitLab reuse needs this change's completed, non-skipped native train and exact successful pipeline; use fresh checks when that correspondence is unavailable".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn checkout_requires_exact_head_and_real_equivalent_trees()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let root = directory.path();
        let git = |args: &[&str]| -> Result<String, Box<dyn std::error::Error>> {
            let output = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()?;
            assert!(
                output.status.success(),
                "{args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(String::from_utf8(output.stdout)?.trim().to_owned())
        };
        git(&["init", "--quiet"])?;
        git(&["config", "user.name", "Neutral fixture"])?;
        git(&["config", "user.email", "fixture@example.invalid"])?;
        std::fs::write(root.join("source.txt"), "first\n")?;
        git(&["add", "source.txt"])?;
        git(&["commit", "--quiet", "-m", "candidate"])?;
        let original = git(&["rev-parse", "HEAD"])?;
        git(&[
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "integrated identity",
        ])?;
        let integrated = git(&["rev-parse", "HEAD"])?;
        let mut candidate = IntegratedCandidate {
            repository: "neutral/project".into(),
            provider: CiProvider::Gitlab,
            attempt: None,
            selection: CandidateSelection {
                change: 1,
                integrated_revision: integrated.clone(),
                run: RunExpectation {
                    revision: original,
                    run_id: 1,
                    reference: "refs/merge-requests/1/train".into(),
                    source: "merge_request_event".into(),
                    workflow_id: None,
                },
            },
        };
        candidate.verify_checkout(root)?;
        std::fs::write(root.join("source.txt"), "changed\n")?;
        git(&["add", "source.txt"])?;
        git(&["commit", "--quiet", "-m", "changed content"])?;
        let different = git(&["rev-parse", "HEAD"])?;
        assert!(
            candidate
                .verify_checkout(root)
                .is_err_and(|e| e.contains("Checkout"))
        );
        candidate
            .selection
            .integrated_revision
            .clone_from(&different);
        assert!(
            candidate
                .verify_checkout(root)
                .is_err_and(|e| e.contains("differs"))
        );
        // Replacement refs cannot manufacture content equivalence.
        git(&["replace", &different, &integrated])?;
        assert!(
            candidate
                .verify_checkout(root)
                .is_err_and(|e| e.contains("differs"))
        );
        candidate.selection.run.revision = "0".repeat(40);
        assert!(
            candidate
                .verify_checkout(root)
                .is_err_and(|e| e.contains("unavailable"))
        );
        Ok(())
    }

    #[test]
    fn github_accepts_only_exact_native_integrated_identity() -> Result<(), String> {
        let repository =
            Repository::parse("https://github.com/team/project").map_err(|e| e.to_string())?;
        let selected = CandidateSelection {
            change: 7,
            integrated_revision: "a".repeat(40),
            run: RunExpectation {
                revision: "a".repeat(40),
                run_id: 10,
                reference: "gh-readonly-queue/main/pr-7-a".into(),
                source: "merge_group".into(),
                workflow_id: Some(8),
            },
        };
        let change = json!({"number":7,"merged":true,"merge_commit_sha":"a".repeat(40),"base":{"ref":"main","repo":{"full_name":"team/project"}}});
        correspondence(&repository, "main", &selected, &change, &Value::Null)?;
        for mode in [
            "ordinary-job",
            "other-target",
            "rewritten-commit",
            "wrong-change",
            "unmerged",
            "wrong-repo",
        ] {
            let mut selected = selected.clone();
            let mut change = change.clone();
            match mode {
                "ordinary-job" => selected.run.source = "push".into(),
                "other-target" => selected.run.reference = "gh-readonly-queue/other/pr-7-a".into(),
                "rewritten-commit" => selected.run.revision = "b".repeat(40),
                "wrong-change" => change["number"] = json!(8),
                "unmerged" => change["merged"] = json!(false),
                _ => change["base"]["repo"]["full_name"] = json!("elsewhere/project"),
            }
            assert!(
                correspondence(&repository, "main", &selected, &change, &Value::Null).is_err(),
                "{mode}"
            );
        }
        Ok(())
    }

    #[test]
    fn gitlab_refuses_skipped_stale_other_or_failed_train_correspondence() -> Result<(), String> {
        let repository =
            Repository::parse("https://gitlab.com/team/project").map_err(|e| e.to_string())?;
        let selected = CandidateSelection {
            change: 7,
            integrated_revision: "a".repeat(40),
            run: RunExpectation {
                revision: "b".repeat(40),
                run_id: 10,
                reference: "refs/merge-requests/7/train".into(),
                source: "merge_request_event".into(),
                workflow_id: None,
            },
        };
        let change = json!({"iid":7,"project_id":22,"state":"merged","merge_commit_sha":"a".repeat(40),"target_branch":"main","web_url":"https://gitlab.com/team/project/-/merge_requests/7"});
        let train = json!({"status":"merged","target_branch":"main","merge_request":{"iid":7,"project_id":22,"state":"merged"},"pipeline":{"project_id":22,"id":10,"sha":"b".repeat(40),"ref":selected.run.reference,"source":"merge_request_event","status":"success"}});
        correspondence(&repository, "main", &selected, &change, &train)?;
        for mode in [
            "skip_merged",
            "stale",
            "fresh",
            "other-run",
            "other-source",
            "other-project",
            "failed",
            "missing-pipeline",
        ] {
            let mut train = train.clone();
            match mode {
                "other-run" => train["pipeline"]["id"] = json!(11),
                "other-source" => train["pipeline"]["sha"] = json!("c".repeat(40)),
                "other-project" => train["pipeline"]["project_id"] = json!(23),
                "failed" => train["pipeline"]["status"] = json!("failed"),
                "missing-pipeline" => train["pipeline"] = Value::Null,
                _ => train["status"] = json!(mode),
            }
            assert!(
                correspondence(&repository, "main", &selected, &change, &train).is_err(),
                "{mode}"
            );
        }
        Ok(())
    }
}
