//! Exact current-trunk qualification from reviewed policy; GET-only and fail closed.
use std::collections::BTreeSet;
use std::time::Instant;

use opdev_project::{ProtectionPolicy, QualificationPolicy};
use serde::Serialize;
use serde_json::Value;

use super::run::{number, read_response, string};
use super::{
    CiProvider, Client, Duration, GitlabCredential, Outcome, ProjectManifest, RemoteCapability,
    RemoteError, Repository, RunExpectation, RunVerification, encode, failed, first_env,
    github_request, gitlab_credential, gitlab_request, passed, unverified, verify_run,
    verify_run_with_jobs,
};

mod checks;
mod protection;
#[cfg(test)]
mod tests;

/// Provider-neutral evidence for the exact current trunk revision.
#[derive(Debug, Serialize)]
pub struct QualifiedTrunk {
    /// Revision supplied by the caller, never substituted with another SHA.
    pub revision: String,
    /// Reviewed expectations, not an approval manufactured by this verifier.
    pub policy: Option<QualificationPolicy>,
    /// Selected exact-run and required-job observations.
    pub run: Option<RunVerification>,
    /// Required run/jobs outcome.
    pub pipeline: RemoteCapability,
    /// Producer-bound additional checks outcome.
    pub checks: RemoteCapability,
    /// Reviewed merge-policy comparison outcome.
    pub protection: RemoteCapability,
    /// Combined CI qualification, not artifact qualification.
    pub outcome: Outcome,
}

impl QualifiedTrunk {
    fn unavailable(revision: &str, reason: &str) -> Self {
        Self {
            revision: revision.into(),
            policy: None,
            run: None,
            pipeline: unverified(reason),
            checks: unverified(reason),
            protection: unverified(reason),
            outcome: Outcome::Unverified,
        }
    }

    fn aggregate(&mut self) {
        self.outcome = combine([
            self.pipeline.outcome,
            self.checks.outcome,
            self.protection.outcome,
        ]);
    }
}

/// Qualify the requested revision only if it is still the remote trunk head.
///
/// # Errors
/// Invalid repository/run policy inputs or HTTP-client startup errors prevent
/// execution. Missing policy, permissions and incomplete snapshots are unverified.
pub fn qualify_trunk(
    manifest: &ProjectManifest,
    revision: &str,
) -> Result<QualifiedTrunk, RemoteError> {
    let Some(policy) = &manifest.project.ci.qualification else {
        return Ok(QualifiedTrunk::unavailable(
            revision,
            "Reviewed remote qualification policy is missing; explicitly migrate the project contract to schema 2 after developer review",
        ));
    };
    let (repository, mut expected) = expected_identity(manifest, revision, policy)?;
    let api = Api::new(&repository)?;
    let mut result =
        QualifiedTrunk::unavailable(revision, "Remote qualification has not completed");
    result.policy = Some(policy.clone());
    let branch = match api.branch_revision(&expected.reference) {
        Ok(branch) => branch,
        Err(reason) => return Ok(QualifiedTrunk::unavailable(revision, &reason)),
    };
    if !branch.eq_ignore_ascii_case(revision) {
        return Ok(QualifiedTrunk::unavailable(
            revision,
            "Requested revision is not the current remote trunk head; a historical or change-branch run cannot qualify current trunk",
        ));
    }
    result.protection = capability(
        api.protection(policy, &expected.reference),
        "Reviewed merge policy matches the observed provider settings",
    );
    let selection = api.select_run(&expected);
    match selection {
        Err(reason) => result.pipeline = unverified(&reason),
        Ok(None) => {
            result.pipeline = unverified(
                "No matching run exists for the requested revision, trunk, source and workflow",
            );
        }
        Ok(Some(id)) => {
            expected.run_id = id;
            let run = verify_run_with_jobs(manifest, &expected, &policy.required_jobs)?;
            result.pipeline = RemoteCapability {
                outcome: combine([
                    run.outcome,
                    run.jobs
                        .as_ref()
                        .map_or(Outcome::Unverified, |jobs| jobs.outcome),
                ]),
                // The full typed run is retained separately in this report.
                evidence: Vec::new(),
                diagnostic: Some(format!(
                    "Selected run {id} for revision {revision}; required jobs are explicit policy inputs"
                )),
            };
            result.checks = api.checks(policy, &expected);
            refresh(&mut result.checks, api.checks(policy, &expected));
            // Recheck after all observations; never silently adopt a newer run/attempt.
            let final_run = verify_run(manifest, &expected)?;
            if final_run.outcome == Outcome::Failed {
                result.pipeline = failed("The selected run failed during qualification");
            } else if final_run.outcome != Outcome::Passed || final_run.observed != run.observed {
                invalidate(
                    &mut result.pipeline,
                    "Run status or attempt changed or became unavailable during qualification",
                );
            }
            match api.select_run(&expected) {
                Ok(Some(latest)) if latest == id => {}
                _ => invalidate(
                    &mut result.pipeline,
                    "Latest matching run changed or could not be rechecked",
                ),
            }
            result.run = Some(run);
        }
    }
    if api.branch_revision(&expected.reference).ok().as_deref() != Some(branch.as_str()) {
        invalidate(
            &mut result.pipeline,
            "Remote trunk changed or became unavailable during qualification",
        );
    }
    refresh(
        &mut result.protection,
        capability(
            api.protection(policy, &expected.reference),
            "Reviewed merge policy matches the observed provider settings",
        ),
    );
    result.aggregate();
    Ok(result)
}

fn expected_identity(
    manifest: &ProjectManifest,
    revision: &str,
    policy: &QualificationPolicy,
) -> Result<(Repository, RunExpectation), RemoteError> {
    let repository = Repository::parse(
        manifest
            .project
            .ci
            .remote
            .as_deref()
            .ok_or(RemoteError::MissingRemote)?,
    )?;
    if repository.provider != manifest.project.ci.provider {
        return Err(RemoteError::ProviderMismatch {
            declared: manifest.project.ci.provider,
            detected: repository.provider,
        });
    }
    let expected = RunExpectation {
        revision: revision.into(),
        run_id: 1,
        reference: manifest.project.trunk.clone(),
        source: policy.source.clone(),
        workflow_id: policy.workflow_id,
    };
    super::run::validate(&repository, &expected)?;
    Ok((repository, expected))
}

fn invalidate(value: &mut RemoteCapability, reason: &str) {
    if value.outcome != Outcome::Failed {
        *value = unverified(reason);
    }
}

fn refresh(previous: &mut RemoteCapability, next: RemoteCapability) {
    if next.outcome == Outcome::Failed && previous.outcome != Outcome::Failed {
        *previous = next;
    } else if previous.outcome != next.outcome || previous.evidence != next.evidence {
        invalidate(
            previous,
            "Provider evidence changed or became unavailable during qualification",
        );
    }
}

fn combine(outcomes: impl IntoIterator<Item = Outcome>) -> Outcome {
    let outcomes: Vec<_> = outcomes.into_iter().collect();
    if outcomes.contains(&Outcome::Failed) {
        Outcome::Failed
    } else if outcomes.iter().all(|outcome| *outcome == Outcome::Passed) {
        Outcome::Passed
    } else {
        Outcome::Unverified
    }
}

/// A mismatch is a known failure; absent/unsupported evidence is not.
type Verdict = Result<Result<(), String>, String>;

fn capability(result: Verdict, summary: &str) -> RemoteCapability {
    match result {
        Ok(Ok(())) => passed("remote_qualification", summary, None),
        Ok(Err(reason)) => failed(&reason),
        Err(reason) => unverified(&reason),
    }
}

struct Api {
    client: Client,
    repository: Repository,
    base: String,
    token: Option<String>,
    credential: Option<GitlabCredential>,
    started: Instant,
}

impl Api {
    fn new(repository: &Repository) -> Result<Self, RemoteError> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let github = repository.provider == CiProvider::Github;
        Ok(Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(20))
                .user_agent(concat!("opdev/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(RemoteError::Client)?,
            repository: repository.clone(),
            base: if github {
                format!("https://api.github.com/repos/{}", repository.slug())
            } else {
                format!(
                    "https://gitlab.com/api/v4/projects/{}",
                    encode(&repository.slug())
                )
            },
            token: github
                .then(|| first_env(&["OPDEV_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"]))
                .flatten(),
            credential: (!github).then(gitlab_credential).flatten(),
            started: Instant::now(),
        })
    }

    fn get(&self, path: &str) -> Result<Value, String> {
        if self.started.elapsed() >= Duration::from_mins(2) {
            return Err("Remote qualification exceeded its request-start budget".into());
        }
        let request = self.client.get(format!("{}{path}", self.base));
        read_response(if self.repository.provider == CiProvider::Github {
            github_request(request, self.token.as_deref())
        } else {
            gitlab_request(request, self.credential.as_ref())
        })
    }

    fn list(&self, path: &str, field: Option<&str>) -> Result<Vec<Value>, String> {
        pages(field, |page| {
            self.get(&format!(
                "{path}{}per_page=100&page={page}",
                if path.contains('?') { "&" } else { "?" }
            ))
        })
    }

    fn branch_revision(&self, branch: &str) -> Result<String, String> {
        let value = self.get(&if self.repository.provider == CiProvider::Github {
            format!("/branches/{}", encode(branch))
        } else {
            format!("/repository/branches/{}", encode(branch))
        })?;
        if string(&value, "name")? != branch {
            return Err("Provider returned a different branch".into());
        }
        string(
            &value["commit"],
            if self.repository.provider == CiProvider::Github {
                "sha"
            } else {
                "id"
            },
        )
    }

    fn select_run(&self, expected: &RunExpectation) -> Result<Option<u64>, String> {
        let rows = if self.repository.provider == CiProvider::Github {
            self.list(
                &format!(
                    "/actions/workflows/{}/runs?head_sha={}&branch={}&event={}",
                    expected.workflow_id.ok_or("Missing workflow")?,
                    expected.revision,
                    encode(&expected.reference),
                    encode(&expected.source)
                ),
                Some("workflow_runs"),
            )?
        } else {
            self.list(
                &format!(
                    "/pipelines?sha={}&ref={}&source={}&order_by=id&sort=desc",
                    expected.revision,
                    encode(&expected.reference),
                    encode(&expected.source)
                ),
                None,
            )?
        };
        select_run(self.repository.provider, expected, &rows)
    }

    fn checks(&self, policy: &QualificationPolicy, expected: &RunExpectation) -> RemoteCapability {
        match self.check_rows(policy, expected) {
            Err(reason) => unverified(&reason),
            Ok(rows) => {
                let mut result = capability(
                    checks::compare(
                        self.repository.provider,
                        expected,
                        &policy.required_checks,
                        &rows,
                    ),
                    "Required check names, producer identities and statuses match",
                );
                let mut observations = rows.iter()
                    .filter(|row| policy.required_checks.iter().any(|check| row["name"].as_str() == Some(check.name.as_str())))
                    .map(|row| serde_json::json!({
                        "id": row["id"], "name": row["name"],
                        "producer_id": if self.repository.provider == CiProvider::Github { &row["app"]["id"] } else { &row["creator"]["id"] },
                        "revision": if self.repository.provider == CiProvider::Github { &row["head_sha"] } else { &row["sha"] },
                        "status": row["status"], "conclusion": row["conclusion"]
                    }).to_string()).collect::<Vec<_>>();
                observations.sort();
                result.evidence.push(super::Evidence {
                    kind: "remote_check_inventory".into(),
                    summary: format!(
                        "Observed required-name check identities: {}",
                        observations.join(", ")
                    ),
                    location: None,
                });
                result
            }
        }
    }

    fn check_rows(
        &self,
        policy: &QualificationPolicy,
        expected: &RunExpectation,
    ) -> Result<Vec<Value>, String> {
        let rows = if policy.required_checks.is_empty() {
            Vec::new()
        } else if self.repository.provider == CiProvider::Github {
            self.list(
                &format!("/commits/{}/check-runs?filter=all", expected.revision),
                Some("check_runs"),
            )?
        } else {
            self.list(
                &format!(
                    "/repository/commits/{}/statuses?all=true&pipeline_id={}&ref={}",
                    expected.revision,
                    expected.run_id,
                    encode(&expected.reference)
                ),
                None,
            )?
        };
        Ok(rows)
    }

    fn protection(&self, policy: &QualificationPolicy, branch: &str) -> Verdict {
        match &policy.protection {
            ProtectionPolicy::GithubBranch { strict } => protection::github_branch(
                &self.get(&format!("/branches/{}/protection", encode(branch)))?,
                *strict,
                &policy.required_checks,
            ),
            ProtectionPolicy::GithubRulesets { ids, strict } => {
                let active = self.list(&format!("/rules/branches/{}", encode(branch)), None)?;
                let mut rulesets = Vec::new();
                for id in ids {
                    rulesets.push(self.get(&format!("/rulesets/{id}?includes_parents=true"))?);
                }
                protection::github_rulesets(
                    ids,
                    *strict,
                    &policy.required_checks,
                    &active,
                    &rulesets,
                )
            }
            ProtectionPolicy::Gitlab { rules } => protection::gitlab(
                &self.get("")?,
                branch,
                rules,
                &self.list("/protected_branches", None)?,
            ),
        }
    }
}

fn pages<F>(field: Option<&str>, mut get: F) -> Result<Vec<Value>, String>
where
    F: FnMut(u32) -> Result<Value, String>,
{
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    let mut total = None;
    for page in 1..=10 {
        let value = get(page)?;
        let current = if let Some(field) = field {
            let count = value["total_count"]
                .as_u64()
                .ok_or("Missing collection total")?;
            if count > 1000 || total.is_some_and(|old| old != count) {
                return Err("Collection total changed or exceeded the limit".into());
            }
            total = Some(count);
            value[field].as_array()
        } else {
            value.as_array()
        }
        .ok_or("Provider collection was not an array")?;
        if current.len() > 100 {
            return Err("Provider page exceeded its requested size".into());
        }
        for row in current {
            if !seen.insert(row.to_string()) {
                return Err("Repeated collection entry; pagination completeness is unknown".into());
            }
            rows.push(row.clone());
        }
        if let Some(count) = total
            && (rows.len() as u64 > count || (current.len() < 100 && (rows.len() as u64) < count))
        {
            return Err("Collection was incomplete or inconsistent".into());
        }
        if total == Some(rows.len() as u64) || (total.is_none() && current.len() < 100) {
            return Ok(rows);
        }
    }
    Err("Collection exceeded 10 pages; completeness is unverified".into())
}

fn select_run(
    provider: CiProvider,
    expected: &RunExpectation,
    rows: &[Value],
) -> Result<Option<u64>, String> {
    let mut ids = BTreeSet::new();
    let mut selected = None;
    for row in rows {
        let id = number(row, "id")?;
        if !ids.insert(id) {
            return Err("Duplicate run identity in provider collection".into());
        }
        let github = provider == CiProvider::Github;
        if !string(row, if github { "head_sha" } else { "sha" })?
            .eq_ignore_ascii_case(&expected.revision)
            || string(row, if github { "head_branch" } else { "ref" })? != expected.reference
            || (github
                && (string(row, "event")? != expected.source
                    || Some(number(row, "workflow_id")?) != expected.workflow_id))
            || (!github && row.get("source").is_some() && string(row, "source")? != expected.source)
        {
            continue;
        }
        // No success filter: the newest matching pending/failed run wins.
        selected = Some(selected.map_or(id, |old: u64| old.max(id)));
    }
    Ok(selected)
}
