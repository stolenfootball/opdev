use super::*;
use serde_json::json;

fn expected() -> RunExpectation {
    RunExpectation {
        revision: "a".repeat(40),
        run_id: 1,
        reference: "main".into(),
        source: "push".into(),
        workflow_id: Some(2),
    }
}

fn row(provider: CiProvider, id: u64, name: &str, state: &str) -> Value {
    if provider == CiProvider::Github {
        json!({"id":id,"name":name,"run_id":1,"head_sha":"a".repeat(40),"status":"completed","conclusion":state})
    } else {
        json!({"id":id,"name":name,"pipeline":{"id":1,"sha":"a".repeat(40),"ref":"main"},"status":state})
    }
}

fn page(provider: CiProvider, rows: Vec<Value>, total: u64) -> Value {
    let rows = Value::Array(rows);
    if provider == CiProvider::Github {
        json!({"total_count":total,"jobs":rows})
    } else {
        json!(rows)
    }
}

#[test]
fn paginates_to_find_later_required_job() -> Result<(), String> {
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        let jobs = collect_pages(provider, &expected(), |n| {
            Ok(if n == 1 {
                page(
                    provider,
                    (1..=100)
                        .map(|id| row(provider, id, "other", "success"))
                        .collect(),
                    101,
                )
            } else {
                page(
                    provider,
                    vec![row(provider, 101, "required", "success")],
                    101,
                )
            })
        })?;
        assert_eq!(jobs.len(), 101);
        let result = reconcile_jobs(provider, &["required".into()], jobs, vec![]);
        assert_eq!(result.outcome, Outcome::Passed);
        assert_eq!(result.current[0].id, 101);
    }
    Ok(())
}

#[test]
fn missing_ambiguous_pending_or_failed_required_jobs_do_not_pass() -> Result<(), String> {
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        for status in [
            "success", "failed", "failure", "canceled", "pending", "skipped", "neutral", "manual",
            "unknown",
        ] {
            let job = parse_job(provider, &expected(), &row(provider, 1, "test", status))?;
            let result = reconcile_jobs(provider, &["test".into()], vec![job], vec![]);
            assert_eq!(
                result.outcome,
                match status {
                    "success" => Outcome::Passed,
                    "failed" | "failure" | "canceled" => Outcome::Failed,
                    _ => Outcome::Unverified,
                }
            );
        }
        let success = parse_job(provider, &expected(), &row(provider, 1, "test", "success"))?;
        assert_eq!(
            reconcile_jobs(provider, &["absent".into()], vec![success.clone()], vec![]).outcome,
            Outcome::Unverified
        );
        let mut duplicate = success.clone();
        duplicate.id = 2;
        assert_eq!(
            reconcile_jobs(
                provider,
                &["test".into()],
                vec![success.clone(), duplicate],
                vec![]
            )
            .outcome,
            Outcome::Unverified
        );
        let failed = parse_job(provider, &expected(), &row(provider, 2, "failed", "failed"))?;
        assert_eq!(
            reconcile_jobs(
                provider,
                &["failed".into(), "absent".into()],
                vec![failed, success],
                vec![]
            )
            .outcome,
            Outcome::Failed
        );
    }
    Ok(())
}

#[test]
fn current_failure_never_falls_back_to_prior_success_and_retries_remain_visible()
-> Result<(), String> {
    let provider = CiProvider::Gitlab;
    let old = parse_job(provider, &expected(), &row(provider, 1, "test", "success"))?;
    let current = parse_job(provider, &expected(), &row(provider, 2, "test", "failed"))?;
    let result = reconcile_jobs(
        provider,
        &["test".into()],
        vec![current.clone()],
        vec![old, current],
    );
    assert_eq!(result.outcome, Outcome::Failed);
    assert_eq!(result.prior.len(), 1);
    assert_eq!(result.current[0].id, 2);
    Ok(())
}

#[test]
fn incomplete_invalid_duplicate_and_unavailable_pages_fail_closed() {
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        assert!(collect_pages(provider, &expected(), |_| Err("HTTP 403".into())).is_err());
        assert!(collect_pages(provider, &expected(), |_| Ok(json!({}))).is_err());
        assert!(
            collect_pages(provider, &expected(), |_| Ok(page(
                provider,
                vec![row(provider, 1, "x", "success"); 100],
                101
            )))
            .is_err()
        );
        // A failure after a complete first page cannot be mistaken for completeness.
        assert!(
            collect_pages(provider, &expected(), |n| if n == 1 {
                Ok(page(
                    provider,
                    (1..=100)
                        .map(|id| row(provider, id, "x", "success"))
                        .collect(),
                    101,
                ))
            } else {
                Err("HTTP 403".into())
            })
            .is_err()
        );
        assert!(
            collect_pages(provider, &expected(), |n| {
                Ok(page(
                    provider,
                    (1..=100)
                        .map(|id| row(provider, u64::from(n) * 100 + id, "x", "success"))
                        .collect(),
                    1001,
                ))
            })
            .is_err()
        );
        for key in ["id", "name", "status"] {
            let mut invalid = row(provider, 1, "x", "success");
            invalid[key] = Value::Null;
            assert!(parse_job(provider, &expected(), &invalid).is_err());
        }
        let mut wrong = expected();
        wrong.run_id = 9;
        assert!(parse_job(provider, &wrong, &row(provider, 1, "x", "success")).is_err());
        wrong = expected();
        wrong.revision = "b".repeat(40);
        assert!(parse_job(provider, &wrong, &row(provider, 1, "x", "success")).is_err());
    }
    assert!(
        collect_pages(CiProvider::Github, &expected(), |_| Ok(page(
            CiProvider::Github,
            vec![],
            1
        )))
        .is_err()
    );
}

#[test]
fn validates_names_before_network() {
    for names in [
        vec![String::new()],
        vec!["x\n".into()],
        vec!["x".into(), "x".into()],
        vec!["x".into(); 101],
    ] {
        assert!(validate_required(&names).is_err());
    }
    assert!(validate_required(&[]).is_ok());
    assert!(validate_required(&["matrix (Linux, x86_64)".into()]).is_ok());
}

fn report(provider: CiProvider) -> RunVerification {
    let mut identity = expected();
    if provider == CiProvider::Gitlab {
        identity.workflow_id = None;
    }
    RunVerification {
        schema: 2,
        repository: "team/project".into(),
        provider,
        observed: Some(super::super::RunObservation {
            identity: identity.clone(),
            status: "success".into(),
            conclusion: None,
            attempt: (provider == CiProvider::Github).then_some(1),
        }),
        expected: identity,
        outcome: Outcome::Passed,
        qualification: Outcome::Unverified,
        diagnostics: vec!["Scope is limited".into()],
        jobs: None,
    }
}

#[test]
fn snapshot_changes_or_missing_history_are_unverified() -> Result<(), String> {
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        let before = report(provider);
        let mut after = report(provider);
        let jobs = vec![parse_job(
            provider,
            &expected(),
            &row(provider, 1, "test", "success"),
        )?];
        assert!(validate_snapshot(&before, &after, &jobs, &jobs, &jobs).is_ok());
        assert!(validate_snapshot(&before, &after, &jobs, &[], &jobs).is_err());
        after.outcome = Outcome::Unverified;
        assert!(validate_snapshot(&before, &after, &jobs, &jobs, &jobs).is_err());
        after = report(provider);
        after
            .observed
            .as_mut()
            .ok_or("fixture observation")?
            .attempt = Some(2);
        assert!(validate_snapshot(&before, &after, &jobs, &jobs, &jobs).is_err());
        if provider == CiProvider::Gitlab {
            assert!(validate_snapshot(&before, &report(provider), &jobs, &jobs, &[]).is_err());
        }
    }
    Ok(())
}

#[test]
fn opt_in_schema_requires_job_evidence_and_default_schema_forbids_it()
-> Result<(), Box<dyn std::error::Error>> {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../schema/ci-run-verification.schema.json"
    ))?;
    let mut report = report(CiProvider::Gitlab);
    report.jobs = Some(reconcile_jobs(
        CiProvider::Gitlab,
        &["test".into()],
        vec![],
        vec![],
    ));
    let mut value = serde_json::to_value(&report)?;
    assert!(jsonschema::is_valid(&schema, &value));
    value["schema"] = 1.into();
    assert!(!jsonschema::is_valid(&schema, &value));
    value
        .as_object_mut()
        .ok_or("fixture object")?
        .remove("jobs");
    assert!(jsonschema::is_valid(&schema, &value));
    value["schema"] = 2.into();
    assert!(!jsonschema::is_valid(&schema, &value));
    Ok(())
}
