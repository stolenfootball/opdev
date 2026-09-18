//! Bounded, read-only observations of explicitly requested provider-owned jobs.

use std::collections::BTreeSet;
use std::time::Instant;

use serde::Serialize;
use serde_json::Value;

use super::run::{number, read_response, string};
use super::{
    CiProvider, Client, Duration, Outcome, ProjectManifest, RemoteError, RunExpectation,
    RunVerification, encode, first_env, github_request, gitlab_credential, gitlab_request,
    verify_run,
};

#[cfg(test)]
mod tests;

/// Minimal job evidence, excluding logs, actors, runner details and secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JobObservation {
    /// Provider job identity.
    pub id: u64,
    /// Exact provider-returned name (including any matrix suffix).
    pub name: String,
    /// Provider lifecycle status.
    pub status: String,
    /// GitHub conclusion; absent for GitLab.
    pub conclusion: Option<String>,
}

/// Required-job snapshot, not a declaration of sufficient project policy.
#[derive(Debug, Serialize)]
pub struct JobVerification {
    /// Caller-selected names, never inferred from existing jobs.
    pub required: Vec<String>,
    /// Current matching jobs; duplicate names remain visible and unverified.
    pub current: Vec<JobObservation>,
    /// GitLab's superseded matching jobs; never used to satisfy requirements.
    pub prior: Vec<JobObservation>,
    /// Whether all requested names uniquely identify successful current jobs.
    pub outcome: Outcome,
    /// Explicit incompleteness, retry and scope information.
    pub diagnostics: Vec<String>,
}

/// Observe one run and optionally its explicitly requested job inventory.
///
/// Empty names preserve the original schema-1 run observation. Nonempty names
/// opt into schema 2; this never establishes source trust or full qualification.
///
/// # Errors
/// Invalid or duplicate job names and invalid run expectations fail before any
/// network request. Other startup errors follow [`verify_run`].
pub fn verify_run_with_jobs(
    manifest: &ProjectManifest,
    expected: &RunExpectation,
    required: &[String],
) -> Result<RunVerification, RemoteError> {
    validate_required(required)?;
    let mut report = verify_run(manifest, expected)?;
    if required.is_empty() {
        return Ok(report);
    }
    report.schema = 2;
    report.diagnostics[0] = "Run and required-job observations only. Trusted check sources, merge protection, newer runs and artifact qualification are not established. No gate or provider setting was changed.".into();
    let mut jobs = JobVerification {
        required: required.to_vec(),
        current: Vec::new(),
        prior: Vec::new(),
        outcome: Outcome::Unverified,
        diagnostics: vec!["Names are caller-selected policy inputs, not proof of an adequate test inventory. This is a bounded snapshot, not an atomic provider attestation.".into()],
    };
    if report.outcome == Outcome::Passed {
        match fetch_jobs(manifest, &report) {
            Ok((current, history)) => {
                jobs = reconcile_jobs(report.provider, required, current, history);
            }
            Err(reason) => jobs.diagnostics.push(reason),
        }
    } else {
        jobs.diagnostics
            .push("Jobs were not queried because run identity/status did not pass".into());
    }
    report.jobs = Some(jobs);
    Ok(report)
}

fn validate_required(required: &[String]) -> Result<(), RemoteError> {
    let mut unique = BTreeSet::new();
    if required.len() > 100
        || required.iter().any(|name| {
            name.trim().is_empty()
                || name.len() > 1024
                || name.chars().any(char::is_control)
                || !unique.insert(name)
        })
    {
        return Err(RemoteError::InvalidExpectation(
            "require at most 100 distinct nonempty job names without control characters".into(),
        ));
    }
    Ok(())
}

type Inventory = Vec<JobObservation>;

fn fetch_jobs(
    manifest: &ProjectManifest,
    report: &RunVerification,
) -> Result<(Inventory, Inventory), String> {
    let client = Client::builder()
        .user_agent(concat!("opdev/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "Job observation client could not start".to_owned())?;
    let expected = &report.expected;
    let token = (report.provider == CiProvider::Github)
        .then(|| first_env(&["OPDEV_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"]))
        .flatten();
    let credential = (report.provider == CiProvider::Gitlab)
        .then(gitlab_credential)
        .flatten();
    let base = if report.provider == CiProvider::Github {
        let attempt = report
            .observed
            .as_ref()
            .and_then(|run| run.attempt)
            .ok_or("Run attempt is unavailable")?;
        format!(
            "https://api.github.com/repos/{}/actions/runs/{}/attempts/{attempt}/jobs",
            report.repository, expected.run_id
        )
    } else {
        format!(
            "https://gitlab.com/api/v4/projects/{}/pipelines/{}/jobs",
            encode(&report.repository),
            expected.run_id
        )
    };
    let started = Instant::now();
    let collect = |history: bool| {
        collect_pages(report.provider, expected, |page| {
            if started.elapsed() >= Duration::from_mins(1) {
                return Err("Job observation exceeded the request time budget".into());
            }
            // Construct page URLs ourselves; never follow provider-supplied links.
            let mut url = format!("{base}?per_page=100&page={page}");
            if report.provider == CiProvider::Gitlab {
                use std::fmt::Write;
                write!(url, "&include_retried={history}")
                    .map_err(|_| "Could not construct job request".to_owned())?;
            }
            let request = client.get(url);
            read_response(if report.provider == CiProvider::Github {
                github_request(request, token.as_deref())
            } else {
                gitlab_request(request, credential.as_ref())
            })
        })
    };
    let current = collect(false)?;
    let history = if report.provider == CiProvider::Gitlab {
        collect(true)?
    } else {
        Vec::new()
    };
    let again = collect(false)?;
    let final_run = verify_run(manifest, expected)
        .map_err(|_| "Run could not be rechecked after job observation".to_owned())?;
    validate_snapshot(report, &final_run, &current, &again, &history)?;
    Ok((current, history))
}

fn validate_snapshot(
    report: &RunVerification,
    final_run: &RunVerification,
    current: &[JobObservation],
    again: &[JobObservation],
    history: &[JobObservation],
) -> Result<(), String> {
    if current != again {
        return Err("Job inventory changed during observation; retry with a stable run".into());
    }
    if final_run.outcome != Outcome::Passed || final_run.observed != report.observed {
        return Err(
            "Run identity, attempt or status changed or became unavailable during observation"
                .into(),
        );
    }
    if report.provider == CiProvider::Gitlab && current.iter().any(|job| !history.contains(job)) {
        return Err("Retry history did not contain the observed current jobs".into());
    }
    Ok(())
}

fn collect_pages<F>(
    provider: CiProvider,
    expected: &RunExpectation,
    mut get: F,
) -> Result<Inventory, String>
where
    F: FnMut(u32) -> Result<Value, String>,
{
    let mut jobs = Vec::new();
    let mut ids = BTreeSet::new();
    let mut total = None;
    for page in 1..=10 {
        let value = get(page)?;
        let rows = if provider == CiProvider::Github {
            let count = value["total_count"]
                .as_u64()
                .ok_or("Job count is unavailable")?;
            if count > 1000 || total.is_some_and(|prior| prior != count) {
                return Err("Job count changed or exceeded the inventory limit".into());
            }
            total = Some(count);
            value["jobs"].as_array()
        } else {
            value.as_array()
        }
        .ok_or("Job inventory is not an array")?;
        if rows.len() > 100 {
            return Err("Job page exceeded the requested page size".into());
        }
        for row in rows {
            let job = parse_job(provider, expected, row)?;
            if !ids.insert(job.id) {
                return Err("Duplicate job ID across inventory pages".into());
            }
            jobs.push(job);
        }
        if let Some(count) = total
            && (jobs.len() as u64 > count || (rows.len() < 100 && (jobs.len() as u64) < count))
        {
            return Err("Job inventory is inconsistent with its total count".into());
        }
        if total == Some(jobs.len() as u64) || (total.is_none() && rows.len() < 100) {
            jobs.sort_by_key(|job| job.id);
            return Ok(jobs);
        }
    }
    Err("Job inventory exceeded the 10-page limit; completeness is unverified".into())
}

fn parse_job(
    provider: CiProvider,
    expected: &RunExpectation,
    value: &Value,
) -> Result<JobObservation, String> {
    let (run, revision, reference) = if provider == CiProvider::Github {
        (number(value, "run_id")?, string(value, "head_sha")?, None)
    } else {
        (
            number(&value["pipeline"], "id")?,
            string(&value["pipeline"], "sha")?,
            Some(string(&value["pipeline"], "ref")?),
        )
    };
    if run != expected.run_id
        || !revision.eq_ignore_ascii_case(&expected.revision)
        || reference.is_some_and(|reference| reference != expected.reference)
    {
        return Err("Job belongs to a different run, revision or ref".into());
    }
    Ok(JobObservation {
        id: number(value, "id")?,
        name: string(value, "name")?,
        status: string(value, "status")?,
        conclusion: if provider == CiProvider::Github && !value["conclusion"].is_null() {
            Some(string(value, "conclusion")?)
        } else {
            None
        },
    })
}

fn job_outcome(provider: CiProvider, job: &JobObservation) -> Outcome {
    let verdict = if provider == CiProvider::Github {
        if job.status != "completed" {
            return Outcome::Unverified;
        }
        job.conclusion.as_deref()
    } else {
        Some(job.status.as_str())
    };
    match verdict {
        Some("success") => Outcome::Passed,
        Some("failure" | "failed" | "cancelled" | "canceled" | "timed_out" | "startup_failure") => {
            Outcome::Failed
        }
        _ => Outcome::Unverified,
    }
}

fn reconcile_jobs(
    provider: CiProvider,
    required: &[String],
    current: Inventory,
    history: Inventory,
) -> JobVerification {
    let mut result = JobVerification {
        required: required.to_vec(),
        prior: history.into_iter().filter(|job| required.contains(&job.name) && !current.iter().any(|current| current.id == job.id)).collect(),
        current: current.into_iter().filter(|job| required.contains(&job.name)).collect(),
        outcome: Outcome::Passed,
        diagnostics: vec!["Only explicitly requested provider-owned jobs were compared. External check producers, policy sufficiency and merge enforcement are not verified.".into()],
    };
    if provider == CiProvider::Github {
        result.diagnostics.push("Jobs are pinned to the observed run attempt; the run attempt number remains visible. Complete prior-attempt history is not collected.".into());
    } else {
        result.diagnostics.push("Superseded matching jobs are retained as prior evidence and never satisfy a current requirement.".into());
    }
    for name in required {
        let matching: Vec<_> = result
            .current
            .iter()
            .filter(|job| &job.name == name)
            .collect();
        let failed = matching
            .iter()
            .any(|job| job_outcome(provider, job) == Outcome::Failed);
        let outcome = if failed {
            Outcome::Failed
        } else if matching.len() == 1 {
            job_outcome(provider, matching[0])
        } else {
            Outcome::Unverified
        };
        if matching.len() != 1 {
            result
                .diagnostics
                .push(format!("Required job {name:?} is missing or ambiguous"));
        }
        if outcome == Outcome::Failed
            || (result.outcome == Outcome::Passed && outcome != Outcome::Passed)
        {
            result.outcome = outcome;
        }
    }
    result
}
