use super::*;
use opdev_project::{AccessPrincipal, GitlabBranchPolicy, RequiredCheck};
use serde_json::json;

fn expected() -> RunExpectation {
    RunExpectation {
        revision: "a".repeat(40),
        run_id: 9,
        reference: "main".into(),
        source: "push".into(),
        workflow_id: Some(2),
    }
}
fn checks() -> Vec<RequiredCheck> {
    vec![RequiredCheck {
        name: "test".into(),
        producer_id: 7,
    }]
}
fn status(provider: CiProvider, id: u64, state: &str) -> Value {
    if provider == CiProvider::Github {
        json!({"id":id,"name":"test","app":{"id":7},"head_sha":"a".repeat(40),"status":"completed","conclusion":state})
    } else {
        json!({"id":id,"name":"test","creator":{"id":7},"sha":"a".repeat(40),"ref":"main","pipeline_id":9,"status":state})
    }
}

#[test]
fn newest_matching_run_is_selected_without_success_filtering() -> Result<(), String> {
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        let row = if provider == CiProvider::Github {
            json!({"id":1,"head_sha":"a".repeat(40),"head_branch":"main","event":"push","workflow_id":2,"conclusion":"success"})
        } else {
            json!({"id":1,"sha":"a".repeat(40),"ref":"main","source":"push","status":"success"})
        };
        let mut newer = row.clone();
        newer["id"] = 2.into();
        newer["status"] = "pending".into();
        newer["conclusion"] = Value::Null;
        let mut unrelated = row.clone();
        unrelated["id"] = 3.into();
        if provider == CiProvider::Github {
            unrelated["workflow_id"] = 99.into();
        } else {
            unrelated["source"] = "schedule".into();
        }
        let mut stale = row.clone();
        stale["id"] = 4.into();
        stale[if provider == CiProvider::Github {
            "head_sha"
        } else {
            "sha"
        }] = "b".repeat(40).into();
        assert_eq!(
            select_run(
                provider,
                &expected(),
                &[row.clone(), newer, unrelated.clone(), stale.clone()]
            )?,
            Some(2)
        );
        assert_eq!(
            select_run(provider, &expected(), &[unrelated, stale])?,
            None
        );
        assert!(select_run(provider, &expected(), &[row.clone(), row]).is_err());
    }
    Ok(())
}

#[test]
fn trusted_check_sources_pending_missing_and_failures_are_not_hidden() -> Result<(), String> {
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        let success = status(provider, 1, "success");
        assert!(
            checks::compare(
                provider,
                &expected(),
                &checks(),
                std::slice::from_ref(&success)
            )?
            .is_ok()
        );
        for verdict in ["failure", "failed", "cancelled", "timed_out"] {
            assert!(
                checks::compare(
                    provider,
                    &expected(),
                    &checks(),
                    &[success.clone(), status(provider, 2, verdict)]
                )?
                .is_err()
            );
        }
        for verdict in ["pending", "skipped", "neutral", "manual", "unknown"] {
            assert!(
                checks::compare(
                    provider,
                    &expected(),
                    &checks(),
                    &[success.clone(), status(provider, 2, verdict)]
                )
                .is_err()
            );
        }
        assert!(checks::compare(provider, &expected(), &checks(), &[]).is_err());
        let mut wrong = status(provider, 2, "success");
        wrong[if provider == CiProvider::Github {
            "app"
        } else {
            "creator"
        }]["id"] = 8.into();
        assert!(
            checks::compare(provider, &expected(), &checks(), &[success.clone(), wrong])?.is_err()
        );
        let mut wrong = status(provider, 2, "success");
        wrong[if provider == CiProvider::Github {
            "head_sha"
        } else {
            "sha"
        }] = "b".repeat(40).into();
        assert!(checks::compare(provider, &expected(), &checks(), &[wrong])?.is_err());
        assert!(
            checks::compare(
                provider,
                &expected(),
                &checks(),
                &[success.clone(), success]
            )
            .is_err()
        );
    }
    Ok(())
}

fn branch_policy() -> Value {
    json!({"allow_force_pushes":{"enabled":false},"allow_deletions":{"enabled":false},"enforce_admins":{"enabled":true},"required_pull_request_reviews":{},"required_status_checks":{"strict":true,"checks":[{"context":"test","app_id":7}]}})
}

#[test]
fn github_branch_policy_checks_bind_sources_and_fail_closed() -> Result<(), String> {
    let base = branch_policy();
    assert!(protection::github_branch(&base, true, &checks())?.is_ok());
    for key in ["allow_force_pushes", "allow_deletions"] {
        let mut drift = base.clone();
        drift[key]["enabled"] = true.into();
        assert!(protection::github_branch(&drift, true, &checks())?.is_err());
    }
    let mut drift = base.clone();
    drift["enforce_admins"]["enabled"] = false.into();
    assert!(protection::github_branch(&drift, true, &checks())?.is_err());
    let mut drift = base.clone();
    drift["required_status_checks"]["checks"][0]["app_id"] = 9.into();
    assert!(protection::github_branch(&drift, true, &checks())?.is_err());
    drift["required_status_checks"]["checks"][0]["app_id"] = Value::Null;
    assert!(protection::github_branch(&drift, true, &checks()).is_err());
    assert!(protection::github_branch(&json!({}), true, &checks()).is_err());
    assert!(protection::github_branch(&base, false, &checks())?.is_err());
    Ok(())
}

fn active_rules() -> Vec<Value> {
    vec![
        json!({"type":"pull_request","ruleset_id":11}),
        json!({"type":"non_fast_forward","ruleset_id":11}),
        json!({"type":"deletion","ruleset_id":11}),
        json!({"type":"required_status_checks","ruleset_id":11,"parameters":{"strict_required_status_checks_policy":true,"required_status_checks":[{"context":"test","integration_id":7}]}}),
    ]
}

#[test]
fn rulesets_are_distinct_from_classic_protection_and_missing_bypass_visibility_is_unknown()
-> Result<(), String> {
    let active = active_rules();
    let detail =
        json!({"id":11,"enforcement":"active","target":"branch","bypass_actors":[],"rules":active});
    assert!(
        protection::github_rulesets(
            &[11],
            true,
            &checks(),
            &active,
            std::slice::from_ref(&detail)
        )?
        .is_ok()
    );
    assert!(
        protection::github_rulesets(
            &[12],
            true,
            &checks(),
            &active,
            std::slice::from_ref(&detail)
        )?
        .is_err()
    );
    let mut hidden = detail.clone();
    hidden
        .as_object_mut()
        .ok_or("fixture")?
        .remove("bypass_actors");
    assert!(protection::github_rulesets(&[11], true, &checks(), &active, &[hidden]).is_err());
    let mut bypass = detail.clone();
    bypass["bypass_actors"] =
        json!([{"actor_type":"Integration","actor_id":7,"bypass_mode":"always"}]);
    assert!(protection::github_rulesets(&[11], true, &checks(), &active, &[bypass])?.is_err());
    let mut disabled = detail.clone();
    disabled["enforcement"] = "evaluate".into();
    assert!(protection::github_rulesets(&[11], true, &checks(), &active, &[disabled])?.is_err());
    assert!(
        protection::github_rulesets(
            &[11],
            true,
            &checks(),
            &active[1..],
            std::slice::from_ref(&detail)
        )
        .is_err()
    );
    let mut weakened = detail;
    weakened["rules"] = json!(&active[1..]);
    assert!(
        protection::github_rulesets(&[11], true, &checks(), &active[1..], &[weakened])?.is_err()
    );
    Ok(())
}

#[test]
fn gitlab_merge_settings_and_all_matching_wildcard_access_rules_are_compared() -> Result<(), String>
{
    let project = json!({"only_allow_merge_if_pipeline_succeeds":true,"allow_merge_on_skipped_pipeline":false});
    let expected = vec![GitlabBranchPolicy {
        name: "ma*".into(),
        push: vec![AccessPrincipal::Role(0)],
        merge: vec![AccessPrincipal::Role(40)],
    }];
    let rule = json!({"name":"ma*","allow_force_push":false,"push_access_levels":[{"access_level":0}],"merge_access_levels":[{"access_level":40}]});
    assert!(protection::gitlab(&project, "main", &expected, std::slice::from_ref(&rule))?.is_ok());
    let other = json!({"name":"release/*"});
    assert!(protection::gitlab(&project, "main", &expected, &[rule.clone(), other])?.is_ok());
    let mut bypass = rule.clone();
    bypass["name"] = "*".into();
    assert!(protection::gitlab(&project, "main", &expected, &[rule.clone(), bypass])?.is_err());
    let mut drift = rule.clone();
    drift["push_access_levels"][0] = json!({"access_level":40});
    assert!(protection::gitlab(&project, "main", &expected, &[drift])?.is_err());
    let mut drift = project.clone();
    drift["allow_merge_on_skipped_pipeline"] = true.into();
    assert!(protection::gitlab(&drift, "main", &expected, std::slice::from_ref(&rule))?.is_err());
    assert!(
        protection::gitlab(&json!({}), "main", &expected, std::slice::from_ref(&rule)).is_err()
    );
    assert!(protection::gitlab(&project, "main", &expected, &[])?.is_err());
    Ok(())
}

#[test]
fn pagination_bounds_missing_pages_and_permission_errors_never_qualify() -> Result<(), String> {
    let result = pages(None, |n| {
        Ok(if n == 1 {
            json!((1..=100).collect::<Vec<_>>())
        } else {
            json!([101])
        })
    })?;
    assert_eq!(result.len(), 101);
    assert!(pages(None, |_| Err("HTTP 403".into())).is_err());
    assert!(
        pages(None, |n| if n == 1 {
            Ok(json!((1..=100).collect::<Vec<_>>()))
        } else {
            Err("HTTP 401".into())
        })
        .is_err()
    );
    assert!(pages(None, |_| Ok(json!(vec![1; 100]))).is_err());
    assert!(
        pages(Some("checks"), |_| Ok(
            json!({"total_count":2,"checks":[1]})
        ))
        .is_err()
    );
    assert!(
        pages(None, |n| Ok(json!(
            (1..=100).map(|id| n * 100 + id).collect::<Vec<_>>()
        )))
        .is_err()
    );
    Ok(())
}

#[test]
fn missing_observations_never_erase_known_failures() {
    let mut known = failed("known failure");
    invalidate(&mut known, "unavailable");
    assert_eq!(known.outcome, Outcome::Failed);
    assert_eq!(
        combine([Outcome::Passed, Outcome::Unverified]),
        Outcome::Unverified
    );
    assert_eq!(
        combine([Outcome::Failed, Outcome::Unverified]),
        Outcome::Failed
    );
    refresh(&mut known, unverified("permission lost"));
    assert_eq!(known.outcome, Outcome::Failed);
    let mut good = passed("policy", "matches", None);
    refresh(&mut good, unverified("permission lost"));
    assert_eq!(good.outcome, Outcome::Unverified);
    refresh(&mut good, failed("drift"));
    assert_eq!(good.outcome, Outcome::Failed);
}

#[test]
fn malformed_check_does_not_hide_another_required_failure() -> Result<(), String> {
    let mut required = checks();
    required.push(RequiredCheck {
        name: "broken".into(),
        producer_id: 7,
    });
    let mut malformed = status(CiProvider::Github, 1, "success");
    malformed["app"] = Value::Null;
    let mut failure = status(CiProvider::Github, 2, "failure");
    failure["name"] = "broken".into();
    assert!(
        checks::compare(
            CiProvider::Github,
            &expected(),
            &required,
            &[malformed, failure]
        )?
        .is_err()
    );
    Ok(())
}

/// Read-only opt-in canary. Overrides are test inputs, never project adoption.
#[test]
#[ignore = "requires explicit public remote policy/revision inputs and provider access"]
fn live_remote_qualification() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var("OPDEV_TEST_QUALIFICATION_ROOT")?;
    let mut manifest =
        ProjectManifest::load(&std::path::Path::new(&root).join(opdev_project::MANIFEST_PATH))?;
    manifest.schema = 2;
    manifest.project.trunk = std::env::var("OPDEV_TEST_QUALIFICATION_TRUNK")?;
    manifest.project.ci = serde_json::from_str(&std::env::var("OPDEV_TEST_QUALIFICATION_CI")?)?;
    let manifest = ProjectManifest::from_yaml(&manifest.to_yaml()?)?;
    let result = qualify_trunk(
        &manifest,
        &std::env::var("OPDEV_TEST_QUALIFICATION_REVISION")?,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    let expected: Outcome = serde_json::from_str(&format!(
        "\"{}\"",
        std::env::var("OPDEV_TEST_QUALIFICATION_OUTCOME")?
    ))?;
    assert_eq!(result.outcome, expected);
    assert_eq!(result.pipeline.outcome, Outcome::Passed);
    assert_eq!(result.checks.outcome, Outcome::Passed);
    Ok(())
}
