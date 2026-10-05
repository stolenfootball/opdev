//! Same-run producer observation, without waiting for the evaluator's own job.

use std::collections::BTreeMap;
use std::io::Read;
use std::time::Instant;

use reqwest::blocking::Response;
use url::Url;

use super::jobs::{collect_pages, job_outcome, reconcile_jobs, validate_required};
use super::run::read_response;
use super::{
    CiProvider, Client, Duration, JobObservation, JobVerification, Outcome, ProjectManifest,
    RemoteError, RunExpectation, RunVerification, encode, first_env, github_request,
    gitlab_credential, gitlab_request, verify_run,
};

const LOG_LIMIT: u64 = 8 * 1024 * 1024;

/// A provider-owned observation. It cannot be constructed from caller JSON.
///
/// This authenticates the channel, not the truth of project-controlled output.
/// Callers must separately validate the reviewed producer and execution binding.
/// Logs are private and deliberately excluded from Debug/serialization.
pub struct ProducerSnapshot {
    repository: String,
    provider: CiProvider,
    expected: RunExpectation,
    attempt: Option<u64>,
    jobs: JobVerification,
    logs: BTreeMap<u64, String>,
    log_errors: BTreeMap<u64, String>,
}

impl ProducerSnapshot {
    /// Provider-returned repository identity.
    #[must_use]
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// First-class provider owning this snapshot.
    #[must_use]
    pub const fn provider(&self) -> CiProvider {
        self.provider
    }

    /// Exact run identity checked before and after log retrieval.
    #[must_use]
    pub const fn expected(&self) -> &RunExpectation {
        &self.expected
    }

    /// GitHub run attempt; GitLab retries use distinct job IDs instead.
    #[must_use]
    pub const fn attempt(&self) -> Option<u64> {
        self.attempt
    }

    /// Selected current jobs and preserved observed retry history.
    #[must_use]
    pub const fn jobs(&self) -> &JobVerification {
        &self.jobs
    }

    /// Private log bytes from the selected job's provider endpoint, if retained.
    #[must_use]
    pub fn log(&self, job_id: u64) -> Option<&str> {
        self.logs.get(&job_id).map(String::as_str)
    }

    /// Retention/transport limitation for one observed producer, without log content.
    #[must_use]
    pub fn log_error(&self, job_id: u64) -> Option<&str> {
        self.log_errors.get(&job_id).map(String::as_str)
    }
}

/// Retrieve completed producer logs from one explicit run using GET requests.
///
/// The run may still be active: its evaluator is not one of its own prerequisites.
/// Missing/erased/oversized logs, changed attempts and incomplete inventories
/// return an unavailable diagnostic. No tests, repairs, retries or writes occur.
/// Provider credentials use the existing read-only remote adapter policy.
///
/// # Errors
/// Invalid identities or names fail before network access. Transport or unstable
/// evidence returns the inner error; it must never be interpreted as a pass.
pub fn observe_producers(
    manifest: &ProjectManifest,
    expected: &RunExpectation,
    required: &[String],
) -> Result<Result<ProducerSnapshot, String>, RemoteError> {
    validate_required(required)?;
    if required.is_empty() {
        return Err(RemoteError::InvalidExpectation(
            "select at least one producer job; an empty inventory cannot establish execution"
                .into(),
        ));
    }
    let initial = verify_run(manifest, expected)?;
    if let Err(reason) = matching_run(&initial) {
        return Ok(Err(reason));
    }
    Ok(collect_snapshot(manifest, &initial, required))
}

fn matching_run(report: &RunVerification) -> Result<(), String> {
    let observed = report
        .observed
        .as_ref()
        .ok_or("Run identity is unavailable")?;
    if observed.identity != report.expected {
        return Err("Provider run identity differs from the requested subject".into());
    }
    if report.provider == CiProvider::Github && observed.attempt.is_none_or(|n| n == 0) {
        return Err("Current run attempt is unavailable".into());
    }
    Ok(())
}

fn collect_snapshot(
    manifest: &ProjectManifest,
    initial: &RunVerification,
    required: &[String],
) -> Result<ProducerSnapshot, String> {
    let client = producer_client()?;
    let github = initial.provider == CiProvider::Github;
    let token = github
        .then(|| first_env(&["OPDEV_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"]))
        .flatten();
    let credential = (!github).then(gitlab_credential).flatten();
    let (base, inventory_url) = producer_urls(initial)?;
    let started = Instant::now();
    let budget = || {
        if started.elapsed() >= Duration::from_mins(2) {
            Err("Producer observation exceeded its bounded request budget".to_owned())
        } else {
            Ok(())
        }
    };
    let request = |url: &str| {
        if github {
            github_request(client.get(url), token.as_deref())
        } else {
            gitlab_request(client.get(url), credential.as_ref())
        }
    };
    let inventory = |history: bool| {
        collect_pages(initial.provider, &initial.expected, |page| {
            budget()?;
            let url = if github {
                format!("{inventory_url}?per_page=100&page={page}")
            } else {
                format!("{inventory_url}?per_page=100&page={page}&include_retried={history}")
            };
            read_response(request(&url))
        })
    };
    let log = |id| {
        budget()?;
        let endpoint = if github { "logs" } else { "trace" };
        request(&format!("{base}/jobs/{id}/{endpoint}"))
            .send()
            .map_err(|_| "Producer log request could not complete".to_owned())
            .and_then(|response| {
                if github {
                    github_download(&client, &response)
                } else {
                    Ok(response)
                }
            })
            .and_then(read_log)
    };
    let final_run = || {
        budget()?;
        verify_run(manifest, &initial.expected)
            .map_err(|_| "Run could not be rechecked after producer observation".to_owned())
    };
    assemble_snapshot(initial, required, inventory, log, final_run)
}

fn assemble_snapshot(
    initial: &RunVerification,
    required: &[String],
    mut inventory: impl FnMut(bool) -> Result<Vec<JobObservation>, String>,
    mut read: impl FnMut(u64) -> Result<String, String>,
    final_run: impl FnOnce() -> Result<RunVerification, String>,
) -> Result<ProducerSnapshot, String> {
    matching_run(initial)?;
    let current = inventory(false)?;
    let history = if initial.provider == CiProvider::Github {
        Vec::new()
    } else {
        inventory(true)?
    };
    let selected: Vec<_> = current
        .iter()
        .filter(|job| required.contains(&job.name))
        .cloned()
        .collect();
    let mut logs = BTreeMap::new();
    let mut log_errors = BTreeMap::new();
    for job in &selected {
        // Pending/skipped/ambiguous producers never acquire a successful receipt.
        if !matches!(
            job_outcome(initial.provider, job),
            Outcome::Passed | Outcome::Failed
        ) || selected
            .iter()
            .filter(|other| other.name == job.name)
            .count()
            != 1
        {
            continue;
        }
        match read(job.id) {
            Ok(log) => {
                logs.insert(job.id, log);
            }
            Err(reason) => {
                log_errors.insert(job.id, reason);
            }
        }
    }
    let again = inventory(false)?;
    stable_producers(initial, &final_run()?, required, &current, &again, &history)?;
    let jobs = reconcile_jobs(initial.provider, required, current, history);
    Ok(ProducerSnapshot {
        repository: initial.repository.clone(),
        provider: initial.provider,
        expected: initial.expected.clone(),
        attempt: initial.observed.as_ref().and_then(|run| run.attempt),
        jobs,
        logs,
        log_errors,
    })
}

fn stable_producers(
    initial: &RunVerification,
    final_run: &RunVerification,
    required: &[String],
    current: &[JobObservation],
    again: &[JobObservation],
    history: &[JobObservation],
) -> Result<(), String> {
    matching_run(initial)?;
    matching_run(final_run)?;
    if initial.repository != final_run.repository
        || initial.provider != final_run.provider
        || initial.expected != final_run.expected
        || initial.observed.as_ref().and_then(|run| run.attempt)
            != final_run.observed.as_ref().and_then(|run| run.attempt)
    {
        return Err("Run identity or latest attempt changed during producer observation".into());
    }
    let selected = |jobs: &[JobObservation]| -> Vec<JobObservation> {
        jobs.iter()
            .filter(|job| required.contains(&job.name))
            .cloned()
            .collect()
    };
    if selected(current) != selected(again) {
        return Err("Required producer inventory changed during observation; no historical fallback is allowed".into());
    }
    if initial.provider == CiProvider::Gitlab
        && selected(current).iter().any(|job| !history.contains(job))
    {
        return Err("Retry history did not contain the selected current producers".into());
    }
    Ok(())
}

fn producer_urls(run: &RunVerification) -> Result<(String, String), String> {
    let github = run.provider == CiProvider::Github;
    let base = if github {
        format!("https://api.github.com/repos/{}/actions", run.repository)
    } else {
        format!(
            "https://gitlab.com/api/v4/projects/{}",
            encode(&run.repository)
        )
    };
    let inventory = if github {
        let attempt = run
            .observed
            .as_ref()
            .and_then(|run| run.attempt)
            .ok_or("Missing run attempt")?;
        format!(
            "{base}/runs/{}/attempts/{attempt}/jobs",
            run.expected.run_id
        )
    } else {
        format!("{base}/pipelines/{}/jobs", run.expected.run_id)
    };
    Ok((base, inventory))
}

fn download_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "Provider log download URL is invalid".to_owned())?;
    let host = url.host_str().unwrap_or_default();
    // Documented Actions log-storage endpoints only; no arbitrary redirects,
    // credentials, alternate ports or token-bearing URL diagnostics.
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(443)
        || url.fragment().is_some()
        || !(host == "results-receiver.actions.githubusercontent.com"
            || (host.ends_with(".blob.core.windows.net") && host != ".blob.core.windows.net"))
    {
        return Err(
            "Provider log download location is unsupported; no redirect was followed".into(),
        );
    }
    Ok(url)
}

fn producer_client() -> Result<Client, String> {
    Client::builder()
        .user_agent(concat!("opdev/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "Producer observation client could not start".to_owned())
}

fn github_download(client: &Client, response: &Response) -> Result<Response, String> {
    if response.status() != reqwest::StatusCode::FOUND {
        return Err(format!(
            "Provider log lookup returned HTTP {}; log evidence is unavailable",
            response.status()
        ));
    }
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .ok_or("Provider omitted its log download location")?;
    let url = download_url(location)?;
    // Deliberately create a new unauthenticated request. Never forward the API
    // token to blob storage, and never allow a second redirect.
    client
        .get(url)
        .send()
        .map_err(|_| "Provider log download could not complete".into())
}

fn read_log(response: Response) -> Result<String, String> {
    if !response.status().is_success() {
        return Err(format!(
            "Producer log returned HTTP {}; retained evidence is unavailable",
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(LOG_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Producer log could not be read".to_owned())?;
    if bytes.len() as u64 > LOG_LIMIT {
        return Err(
            "Producer log exceeded the 8 MiB limit; execution evidence is unavailable".into(),
        );
    }
    String::from_utf8(bytes)
        .map_err(|_| "Producer log is not UTF-8; execution evidence is unavailable".into())
}

#[cfg(test)]
mod tests {
    use super::super::RunObservation;
    use super::*;

    fn run(provider: CiProvider, attempt: u64) -> RunVerification {
        let expected = RunExpectation {
            revision: "a".repeat(40),
            run_id: 1,
            reference: "main".into(),
            source: "push".into(),
            workflow_id: (provider == CiProvider::Github).then_some(9),
        };
        RunVerification {
            schema: 1,
            repository: "neutral/project".into(),
            provider,
            observed: Some(RunObservation {
                identity: expected.clone(),
                status: "running".into(),
                conclusion: None,
                attempt: (provider == CiProvider::Github).then_some(attempt),
            }),
            expected,
            outcome: Outcome::Unverified,
            qualification: Outcome::Unverified,
            diagnostics: vec![],
            jobs: None,
        }
    }

    fn job(id: u64, name: &str, status: &str) -> JobObservation {
        JobObservation {
            id,
            name: name.into(),
            status: status.into(),
            conclusion: None,
        }
    }

    #[test]
    fn evaluation_does_not_wait_for_itself_but_rejects_changed_producers() -> Result<(), String> {
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            let initial = run(provider, 1);
            let final_run = run(provider, 1);
            let required = vec!["tests".to_owned()];
            let before = vec![job(1, "tests", "success"), job(2, "evaluator", "running")];
            let after = vec![job(1, "tests", "success"), job(2, "evaluator", "success")];
            stable_producers(&initial, &final_run, &required, &before, &after, &before)?;
            for status in ["pending", "failed", "skipped"] {
                let newer = vec![job(3, "tests", status)];
                assert!(
                    stable_producers(&initial, &final_run, &required, &before, &newer, &before)
                        .is_err()
                );
            }
            assert!(
                stable_producers(&initial, &final_run, &required, &before, &[], &before).is_err()
            );
        }
        Ok(())
    }

    #[test]
    fn wrong_run_attempt_repository_and_incomplete_history_are_rejected() {
        let required = vec!["tests".to_owned()];
        let jobs = vec![job(1, "tests", "success")];
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            let initial = run(provider, 1);
            let mut wrong = run(provider, 1);
            wrong.repository = "other/project".into();
            assert!(stable_producers(&initial, &wrong, &required, &jobs, &jobs, &jobs).is_err());
            wrong = run(provider, 1);
            wrong.expected.revision = "b".repeat(40);
            assert!(stable_producers(&initial, &wrong, &required, &jobs, &jobs, &jobs).is_err());
        }
        assert!(
            stable_producers(
                &run(CiProvider::Github, 1),
                &run(CiProvider::Github, 2),
                &required,
                &jobs,
                &jobs,
                &jobs
            )
            .is_err()
        );
        assert!(
            stable_producers(
                &run(CiProvider::Gitlab, 1),
                &run(CiProvider::Gitlab, 1),
                &required,
                &jobs,
                &jobs,
                &[]
            )
            .is_err()
        );
    }

    #[test]
    fn download_locations_are_bounded_without_echoing_sensitive_urls() {
        for accepted in [
            "https://storage.blob.core.windows.net/log?sig=private",
            "https://results-receiver.actions.githubusercontent.com/log",
        ] {
            assert!(download_url(accepted).is_ok());
        }
        for rejected in [
            "http://storage.blob.core.windows.net/log",
            "https://127.0.0.1/log",
            "https://storage.blob.core.windows.net.attacker.invalid/log",
            "https://user:private@storage.blob.core.windows.net/log",
            "https://storage.blob.core.windows.net:8443/log",
            "https://unknown.invalid/?sig=private",
        ] {
            let result = download_url(rejected);
            assert!(result.is_err());
            assert!(!result.err().unwrap_or_default().contains("private"));
        }
    }

    fn api_job(provider: CiProvider, id: u64, status: &str) -> serde_json::Value {
        if provider == CiProvider::Github {
            serde_json::json!({"id":id,"name":"tests","run_id":1,"head_sha":"a".repeat(40),
                "status":if status == "pending" {"in_progress"} else {"completed"},
                "conclusion":if status == "failed" {"failure"} else {status}})
        } else {
            serde_json::json!({"id":id,"name":"tests","status":status,
                "pipeline":{"id":1,"sha":"a".repeat(40),"ref":"main"}})
        }
    }

    #[test]
    fn complete_provider_fixtures_never_select_older_green_logs() -> Result<(), String> {
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            for status in ["success", "failed", "pending", "skipped"] {
                let initial = run(provider, 2);
                let required = vec!["tests".into()];
                let mut log_reads = Vec::new();
                let snapshot = assemble_snapshot(
                    &initial,
                    &required,
                    |history| {
                        let mut rows = vec![api_job(provider, 20, status)];
                        if history {
                            rows.push(api_job(provider, 10, "success"));
                        }
                        let response = if provider == CiProvider::Github {
                            serde_json::json!({"total_count":rows.len(),"jobs":rows})
                        } else {
                            serde_json::json!(rows)
                        };
                        collect_pages(provider, &initial.expected, |_| Ok(response.clone()))
                    },
                    |id| {
                        log_reads.push(id);
                        Ok(format!("owned log {id}"))
                    },
                    || Ok(run(provider, 2)),
                )?;
                assert_eq!(snapshot.jobs().current.len(), 1);
                assert_eq!(snapshot.jobs().current[0].id, 20);
                assert!(!log_reads.contains(&10));
                assert_eq!(snapshot.log(10), None);
                let expected = match status {
                    "success" => Outcome::Passed,
                    "failed" => Outcome::Failed,
                    _ => Outcome::Unverified,
                };
                assert_eq!(snapshot.jobs().outcome, expected);
                if matches!(status, "pending" | "skipped") {
                    assert!(log_reads.is_empty());
                }
                if provider == CiProvider::Gitlab {
                    assert_eq!(snapshot.jobs().prior[0].id, 10);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn expired_log_preserves_observed_current_job_failure() -> Result<(), String> {
        let initial = run(CiProvider::Gitlab, 1);
        let jobs = vec![job(20, "tests", "failed")];
        let snapshot = assemble_snapshot(
            &initial,
            &["tests".into()],
            |_| Ok(jobs.clone()),
            |_| Err("retained log unavailable".into()),
            || Ok(run(CiProvider::Gitlab, 1)),
        )?;
        assert_eq!(snapshot.jobs().outcome, Outcome::Failed);
        assert!(snapshot.log(20).is_none());
        assert_eq!(snapshot.log_error(20), Some("retained log unavailable"));
        Ok(())
    }

    #[test]
    fn missing_ambiguous_and_racing_inventory_cannot_authenticate_a_receipt() -> Result<(), String>
    {
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            let initial = run(provider, 1);
            for jobs in [
                vec![],
                vec![job(1, "tests", "success"), job(2, "tests", "success")],
            ] {
                let mut reads = 0;
                let result = assemble_snapshot(
                    &initial,
                    &["tests".into()],
                    |_| Ok(jobs.clone()),
                    |_| {
                        reads += 1;
                        Ok("should not fetch".into())
                    },
                    || Ok(run(provider, 1)),
                )?;
                assert_eq!(reads, 0);
                assert_eq!(result.jobs().outcome, Outcome::Unverified);
            }
            let mut current_reads = 0;
            let result = assemble_snapshot(
                &initial,
                &["tests".into()],
                |history| {
                    if !history {
                        current_reads += 1;
                    }
                    Ok(vec![job(
                        if current_reads > 1 { 2 } else { 1 },
                        "tests",
                        "pending",
                    )])
                },
                |_| Err("no pending logs".into()),
                || Ok(run(provider, 1)),
            );
            assert!(result.is_err());
        }
        Ok(())
    }
}
