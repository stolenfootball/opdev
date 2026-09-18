//! Explicit run observations, separate from branch-latest audits and core gates.

use std::io::Read;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    CiProvider, Client, Duration, Outcome, ProjectManifest, RemoteError, Repository,
    RequestBuilder, encode, first_env, github_request, gitlab_credential, gitlab_request,
};

/// Caller-selected identity, not an inferred project qualification policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunExpectation {
    /// Full expected Git commit ID.
    pub revision: String,
    /// GitHub run ID or GitLab pipeline ID.
    pub run_id: u64,
    /// Expected branch/ref as returned by the provider.
    pub reference: String,
    /// Expected GitHub event or GitLab pipeline source.
    pub source: String,
    /// Required GitHub numeric workflow ID; absent for GitLab.
    pub workflow_id: Option<u64>,
}

/// Provider-returned metadata; no job names, logs, actors or credentials.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct RunObservation {
    /// Observed revision, run, ref, event and workflow.
    pub identity: RunExpectation,
    /// Provider lifecycle status.
    pub status: String,
    /// Terminal conclusion, when exposed.
    pub conclusion: Option<String>,
    /// GitHub's current attempt, not proof of complete retry history.
    pub attempt: Option<u64>,
}

/// Snapshot of one explicitly selected remote run, not full qualification.
#[derive(Debug, Serialize)]
pub struct RunVerification {
    /// Independent output contract version.
    pub schema: u32,
    /// Validated provider repository slug.
    pub repository: String,
    /// First-class provider name.
    pub provider: CiProvider,
    /// Caller expectations retained even if the provider is unavailable.
    pub expected: RunExpectation,
    /// Observed identity if a complete response was available.
    pub observed: Option<RunObservation>,
    /// Result of identity and run-status comparison only.
    pub outcome: Outcome,
    /// Full project qualification is outside this observation.
    pub qualification: Outcome,
    /// Scope and unavailable/mismatched evidence.
    pub diagnostics: Vec<String>,
    /// Opt-in required-job snapshot; absent from the schema-1 observation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jobs: Option<super::JobVerification>,
}

/// Verifies one explicitly selected run through GET-only provider APIs.
///
/// # Errors
/// Returns an error before network access for invalid expectations or repository
/// configuration, or if the client cannot be initialized. Unavailable provider
/// evidence becomes an unverified observation, never a successful qualification.
pub fn verify_run(
    manifest: &ProjectManifest,
    expected: &RunExpectation,
) -> Result<RunVerification, RemoteError> {
    let remote = manifest
        .project
        .ci
        .remote
        .as_deref()
        .ok_or(RemoteError::MissingRemote)?;
    let repository = Repository::parse(remote)?;
    if manifest.project.ci.provider != repository.provider {
        return Err(RemoteError::ProviderMismatch {
            declared: manifest.project.ci.provider,
            detected: repository.provider,
        });
    }
    validate(&repository, expected)?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = Client::builder()
        .user_agent(concat!("opdev/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(RemoteError::Client)?;
    let slug = repository.slug();
    let observation = match repository.provider {
        CiProvider::Github => {
            let token = first_env(&["OPDEV_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"]);
            let url = format!(
                "https://api.github.com/repos/{}/{}/actions/runs/{}",
                encode(&repository.namespace),
                encode(&repository.name),
                expected.run_id
            );
            read_response(github_request(client.get(url), token.as_deref()))
                .and_then(|value| github_observation(&value, &slug))
        }
        CiProvider::Gitlab => {
            let credential = gitlab_credential();
            let base = format!("https://gitlab.com/api/v4/projects/{}", encode(&slug));
            read_response(gitlab_request(client.get(&base), credential.as_ref())).and_then(
                |project| {
                    let id = number(&project, "id")?;
                    if string(&project, "path_with_namespace")? != slug {
                        return Err(
                            "Provider repository identity differs from the requested repository"
                                .into(),
                        );
                    }
                    let url = format!("{base}/pipelines/{}", expected.run_id);
                    read_response(gitlab_request(client.get(url), credential.as_ref()))
                        .and_then(|value| gitlab_observation(&value, id))
                },
            )
        }
        _ => unreachable!("Repository parsing allows only first-class providers"),
    };
    Ok(reconcile(&repository, expected, observation))
}

fn validate(repository: &Repository, expected: &RunExpectation) -> Result<(), RemoteError> {
    let safe = |value: &str| {
        !value.is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control)
    };
    let valid_slug = repository.slug().split('/').all(|part| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    });
    if !valid_slug
        || (repository.provider == CiProvider::Github && repository.namespace.contains('/'))
        || !matches!(expected.revision.len(), 40 | 64)
        || !expected.revision.bytes().all(|b| b.is_ascii_hexdigit())
        || expected.run_id == 0
        || !safe(&expected.reference)
        || !safe(&expected.source)
        || match repository.provider {
            CiProvider::Github => expected.workflow_id.is_none_or(|id| id == 0),
            _ => expected.workflow_id.is_some(),
        }
    {
        return Err(RemoteError::InvalidExpectation("require a supported repository, full commit ID, positive run ID, ref and source; GitHub additionally requires a positive workflow ID, GitLab forbids it".into()));
    }
    Ok(())
}

pub(super) fn read_response(request: RequestBuilder) -> Result<Value, String> {
    let response = request
        .send()
        .map_err(|_| "Provider request could not complete".to_owned())?;
    if !response.status().is_success() {
        return Err(format!(
            "Provider returned HTTP {}; run evidence is unavailable",
            response.status()
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|_| "Provider response could not be read".to_owned())?;
    if bytes.len() > 1_048_576 {
        return Err("Provider response exceeded the 1 MiB limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Provider response was not valid JSON".into())
}

pub(super) fn string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control))
        .map(str::to_owned)
        .ok_or_else(|| format!("Provider omitted or invalidated {key}"))
}

pub(super) fn number(value: &Value, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)
        .ok_or_else(|| format!("Provider omitted or invalidated {key}"))
}

fn github_observation(value: &Value, slug: &str) -> Result<RunObservation, String> {
    if string(&value["repository"], "full_name")? != slug {
        return Err("Provider repository identity differs from the requested repository".into());
    }
    Ok(RunObservation {
        identity: RunExpectation {
            revision: string(value, "head_sha")?,
            run_id: number(value, "id")?,
            reference: string(value, "head_branch")?,
            source: string(value, "event")?,
            workflow_id: Some(number(value, "workflow_id")?),
        },
        status: string(value, "status")?,
        conclusion: if value["conclusion"].is_null() {
            None
        } else {
            Some(string(value, "conclusion")?)
        },
        attempt: Some(number(value, "run_attempt")?),
    })
}

fn gitlab_observation(value: &Value, project_id: u64) -> Result<RunObservation, String> {
    if number(value, "project_id")? != project_id {
        return Err("Pipeline project identity differs from the requested repository".into());
    }
    Ok(RunObservation {
        identity: RunExpectation {
            revision: string(value, "sha")?,
            run_id: number(value, "id")?,
            reference: string(value, "ref")?,
            source: string(value, "source")?,
            workflow_id: None,
        },
        status: string(value, "status")?,
        conclusion: None,
        attempt: None,
    })
}

fn reconcile(
    repository: &Repository,
    expected: &RunExpectation,
    observed: Result<RunObservation, String>,
) -> RunVerification {
    let mut result = RunVerification {
        schema: 1, repository: repository.slug(), provider: repository.provider,
        expected: expected.clone(), observed: None, outcome: Outcome::Unverified,
        qualification: Outcome::Unverified, jobs: None,
        diagnostics: vec!["Run identity/status observation only. Required jobs, trusted check sources, merge protection, latest attempt freshness and artifact qualification are not established. No gate or provider setting was changed.".into()],
    };
    match observed {
        Err(reason) => result.diagnostics.push(reason),
        Ok(run) => {
            let identity = &run.identity;
            let matches = identity.revision.eq_ignore_ascii_case(&expected.revision)
                && identity.run_id == expected.run_id
                && identity.reference == expected.reference
                && identity.source == expected.source
                && identity.workflow_id == expected.workflow_id;
            result.outcome = if matches {
                let verdict = if repository.provider == CiProvider::Github {
                    if run.status == "completed" {
                        run.conclusion.as_deref()
                    } else {
                        None
                    }
                } else {
                    Some(run.status.as_str())
                };
                match verdict {
                    Some("success") => Outcome::Passed,
                    Some(
                        "failure" | "failed" | "cancelled" | "canceled" | "timed_out"
                        | "startup_failure",
                    ) => Outcome::Failed,
                    _ => Outcome::Unverified,
                }
            } else {
                result.diagnostics.push("Observed run identity does not match the requested revision/run/ref/source/workflow".into());
                Outcome::Failed
            };
            result.observed = Some(run);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(provider: CiProvider) -> (Repository, RunExpectation, Value) {
        let repo = Repository {
            provider,
            namespace: "team".into(),
            name: "project".into(),
        };
        let github = provider == CiProvider::Github;
        let expected = RunExpectation {
            revision: "a".repeat(40),
            run_id: 123,
            reference: "main".into(),
            source: "push".into(),
            workflow_id: github.then_some(456),
        };
        let value = if github {
            json!({"id":123,"head_sha":"a".repeat(40),"head_branch":"main","event":"push",
                "workflow_id":456,"status":"completed","conclusion":"success","run_attempt":2,
                "repository":{"full_name":"team/project"}})
        } else {
            json!({"id":123,"sha":"a".repeat(40),"ref":"main","source":"push","project_id":99,"status":"success"})
        };
        (repo, expected, value)
    }

    fn observe(provider: CiProvider, value: &Value) -> Result<RunObservation, String> {
        if provider == CiProvider::Github {
            github_observation(value, "team/project")
        } else {
            gitlab_observation(value, 99)
        }
    }

    #[test]
    fn both_providers_require_the_requested_identity_before_passing()
    -> Result<(), Box<dyn std::error::Error>> {
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            let (repository, expected, value) = fixture(provider);
            validate(&repository, &expected)?;
            let report = reconcile(&repository, &expected, observe(provider, &value));
            assert_eq!(report.outcome, Outcome::Passed);
            assert_eq!(report.qualification, Outcome::Unverified);
            let schema: Value = serde_json::from_str(include_str!(
                "../../../schema/ci-run-verification.schema.json"
            ))?;
            assert!(jsonschema::is_valid(
                &schema,
                &serde_json::to_value(&report)?
            ));
            for field in ["revision", "run", "ref", "source", "workflow"] {
                let mut wrong = expected.clone();
                match field {
                    "revision" => wrong.revision = "b".repeat(40),
                    "run" => wrong.run_id += 1,
                    "ref" => wrong.reference = "different-branch".into(),
                    "source" => wrong.source = "schedule".into(),
                    _ if provider == CiProvider::Github => wrong.workflow_id = Some(789),
                    _ => continue,
                }
                let report = reconcile(&repository, &wrong, observe(provider, &value));
                assert_eq!(report.outcome, Outcome::Failed, "{provider:?}: {field}");
                assert!(report.observed.is_some());
            }
        }
        Ok(())
    }

    #[test]
    fn pending_missing_and_unsupported_conclusions_never_pass()
    -> Result<(), Box<dyn std::error::Error>> {
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            let (repository, expected, base) = fixture(provider);
            for status in [
                "running", "pending", "queued", "skipped", "neutral", "manual", "unknown",
            ] {
                let mut value = base.clone();
                if provider == CiProvider::Github {
                    value["conclusion"] = status.into();
                } else {
                    value["status"] = status.into();
                }
                assert_eq!(
                    reconcile(&repository, &expected, observe(provider, &value)).outcome,
                    Outcome::Unverified
                );
            }
            for status in ["failed", "failure", "canceled", "cancelled", "timed_out"] {
                let mut value = base.clone();
                if provider == CiProvider::Github {
                    value["conclusion"] = status.into();
                } else {
                    value["status"] = status.into();
                }
                assert_eq!(
                    reconcile(&repository, &expected, observe(provider, &value)).outcome,
                    Outcome::Failed
                );
            }
            for field in base.as_object().ok_or("fixture object")?.keys() {
                if field == "conclusion" {
                    continue;
                }
                let mut value = base.clone();
                value.as_object_mut().ok_or("fixture object")?.remove(field);
                assert_eq!(
                    reconcile(&repository, &expected, observe(provider, &value)).outcome,
                    Outcome::Unverified,
                    "{field}"
                );
            }
        }
        let (repo, expected, mut github) = fixture(CiProvider::Github);
        github["status"] = "in_progress".into();
        assert_eq!(
            reconcile(&repo, &expected, observe(CiProvider::Github, &github)).outcome,
            Outcome::Unverified
        );
        Ok(())
    }

    #[test]
    fn invalid_arguments_are_rejected_before_network_or_credential_lookup() {
        let (repo, base, _) = fixture(CiProvider::Github);
        for revision in ["HEAD", "abc123", "not-a-full-commit-id"] {
            let mut invalid = base.clone();
            invalid.revision = revision.into();
            assert!(validate(&repo, &invalid).is_err());
        }
        let mut expected = base.clone();
        expected.run_id = 0;
        assert!(validate(&repo, &expected).is_err());
        expected = base.clone();
        expected.workflow_id = None;
        assert!(validate(&repo, &expected).is_err());
        expected = base.clone();
        expected.reference = "main\ncontrol".into();
        assert!(validate(&repo, &expected).is_err());
        let (gitlab, _, _) = fixture(CiProvider::Gitlab);
        assert!(validate(&gitlab, &base).is_err());
        let unsafe_repo = Repository {
            provider: CiProvider::Github,
            namespace: "..".into(),
            name: "project".into(),
        };
        assert!(validate(&unsafe_repo, &base).is_err());
    }

    #[test]
    fn wrong_repository_and_project_responses_never_pass() {
        let (repo, expected, mut value) = fixture(CiProvider::Github);
        value["repository"]["full_name"] = "unrelated/project".into();
        assert_eq!(
            reconcile(&repo, &expected, observe(CiProvider::Github, &value)).outcome,
            Outcome::Unverified
        );
        let (repo, expected, mut value) = fixture(CiProvider::Gitlab);
        value["project_id"] = 100.into();
        assert_eq!(
            reconcile(&repo, &expected, observe(CiProvider::Gitlab, &value)).outcome,
            Outcome::Unverified
        );
    }

    #[test]
    fn http_errors_and_invalid_bodies_are_unverified_without_echoing_secrets()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        use std::net::TcpListener;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        for (status, body) in [
            (401, "secret-fixture"),
            (403, "secret-fixture"),
            (404, "missing"),
            (429, "rate limited"),
            (302, "redirect"),
            (200, "secret-invalid-json"),
        ]
        .into_iter()
        .map(|(status, body)| (status, body.to_owned()))
        .chain(std::iter::once((200, "x".repeat(1_048_577))))
        {
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let address = listener.local_addr()?;
            let server = std::thread::spawn(move || -> std::io::Result<()> {
                let (mut stream, _) = listener.accept()?;
                let mut request = [0; 4096];
                let length = stream.read(&mut request)?;
                assert!(String::from_utf8_lossy(&request[..length]).starts_with("GET "));
                write!(
                    stream,
                    "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )?;
                Ok(())
            });
            let observation = read_response(client.get(format!("http://{address}/run")))
                .and_then(|value| github_observation(&value, "team/project"));
            server.join().map_err(|_| "server panicked")??;
            let (repo, expected, _) = fixture(CiProvider::Github);
            let report = reconcile(&repo, &expected, observation);
            assert_eq!(report.outcome, Outcome::Unverified);
            assert!(!serde_json::to_string(&report)?.contains("secret"));
        }
        Ok(())
    }
}
