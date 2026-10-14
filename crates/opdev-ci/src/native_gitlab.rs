//! Narrow static inspection of the native verifier adapter; never provider proof.
use crate::{Capability, passed};
use opdev_core::Outcome;
use serde_json::Value;
use std::path::Path;

pub(crate) fn present(value: &Value) -> bool {
    value.as_object().is_some_and(|jobs| {
        jobs.values().any(|job| {
            job["script"].as_array().is_some_and(|scripts| {
                scripts
                    .iter()
                    .any(|s| s.as_str().is_some_and(|s| s.contains("ci verify")))
            })
        })
    })
}

pub(crate) fn inspect(path: &Path, value: &Value, post: bool) -> Capability {
    match gates(value, post) {
        Ok(1) => passed(
            path,
            "One required native verifier declares the selected stage; live identity, runtime and execution still require verification",
        ),
        Ok(_) => gap(
            "Expected exactly one required native verifier for this boundary; missing or duplicate jobs need review",
        ),
        Err(message) => gap(message),
    }
}

pub(crate) fn integrity(path: &Path, value: &Value) -> Capability {
    if value.get("variables").is_some_and(|variables| {
        variables
            .as_object()
            .is_none_or(|v| v.keys().any(|k| k != "OPDEV_VERSION"))
    }) || value.get("default").is_some()
    {
        return gap(
            "Native verifier source/runtime has global overrides; inspect its effective setup without changing it automatically",
        );
    }
    let Some(jobs) = value.as_object() else {
        return gap("CI jobs must be a mapping");
    };
    let mut count = 0;
    for (name, job) in jobs {
        if name.starts_with('.') {
            continue;
        }
        let Some(script) = job["script"]
            .as_array()
            .filter(|s| s.len() == 1)
            .and_then(|s| s[0].as_str())
        else {
            continue;
        };
        if !script.contains("ci verify") {
            continue;
        }
        // A separately admitted delivery job is not an integration gate. Its
        // own reader/gate still verifies source, stage and artifact at execution.
        if [false, true]
            .into_iter()
            .all(|post| crate::github::gate(script, post) == Ok(None))
            && script
                .split_whitespace()
                .collect::<Vec<_>>()
                .windows(2)
                .any(|pair| pair == ["--stage", "delivery"])
        {
            continue;
        }
        let build = job["before_script"]
            .as_array()
            .filter(|s| s.len() == 1)
            .and_then(|s| s[0].as_str());
        let executable = [false, true]
            .into_iter()
            .find_map(|post| crate::github::gate(script, post).ok().flatten());
        let matches_runtime = match build {
            Some("cargo build --locked -p opdev-cli") => executable
                .as_deref()
                .is_some_and(|e| matches!(e, "target/debug/opdev" | "target/debug/opdev.exe")),
            Some("cargo build --release --locked -p opdev-cli") => executable
                .as_deref()
                .is_some_and(|e| matches!(e, "target/release/opdev" | "target/release/opdev.exe")),
            _ if signed_install(job, value) => executable.as_deref() == Some("opdev"),
            _ => {
                return gap(
                    "Native verifier needs a same-job reviewed runtime setup. Unsupported setup remains unverified; installer text in another job cannot supply it",
                );
            }
        };
        if !matches_runtime {
            return gap(
                "Native verifier executable is not the output of its same-job verified runtime setup",
            );
        }
        count += 1;
    }
    if count == 2 && gates(value, false) == Ok(1) && gates(value, true) == Ok(1) {
        passed(
            path,
            "Both native stage jobs declare same-job signed installation or locked source builds and their exact executable; actual CI checkout identity, runtime capabilities and execution still need verification",
        )
    } else {
        gap("Both required native stages and their runtime setup need review")
    }
}

fn signed_install(job: &Value, workflow: &Value) -> bool {
    let Some(version) = workflow["variables"]["OPDEV_VERSION"].as_str() else {
        return false;
    };
    let Some(reference) = reference_install(version) else {
        return false;
    };
    job["before_script"].is_array() && job["before_script"] == reference["opdev"]["before_script"]
}

fn reference_install(version: &str) -> Option<Value> {
    let rendered = crate::adapter_for(opdev_project::CiProvider::Gitlab)
        .ok()?
        .render(&crate::TemplateContext {
            opdev_version: version.into(),
            trunk: "main".into(),
            job_image: Some("ubuntu:24.04".into()),
        })
        .ok()?;
    serde_saphyr::from_str(&rendered).ok()
}

fn gap(message: &str) -> Capability {
    Capability {
        outcome: Outcome::Unverified,
        evidence: vec![],
        diagnostic: Some(message.into()),
    }
}

fn gates(value: &Value, post: bool) -> Result<usize, &'static str> {
    let jobs = value.as_object().ok_or("CI jobs must be a mapping")?;
    if value.get("workflow").is_some() || value["default"].get("allow_failure").is_some() {
        return Err(
            "Global admission/default overrides need effective provider review; native job text alone cannot establish required execution",
        );
    }
    let expected = if post {
        "$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH"
    } else {
        "$CI_PIPELINE_SOURCE == \"merge_request_event\""
    };
    let mut count = 0;
    for (name, job) in jobs {
        if name.starts_with('.') {
            continue;
        }
        let Some(scripts) = job["script"].as_array() else {
            continue;
        };
        if !scripts
            .iter()
            .any(|s| s.as_str().is_some_and(|s| s.contains("ci verify")))
        {
            continue;
        }
        if [
            "extends",
            "only",
            "except",
            "trigger",
            "parallel",
            "variables",
        ]
        .iter()
        .any(|key| job.get(key).is_some())
        {
            return Err(
                "Native verifier has unsupported inheritance, event or identity overrides; inspect effective provider configuration",
            );
        }
        if job.get("allow_failure").is_some_and(|v| v != false)
            || job
                .get("when")
                .is_some_and(|v| v != "on_success" && v != "manual")
        {
            return Err(
                "Native verifier must be required; optional or exceptional execution cannot qualify this boundary",
            );
        }
        let rules = job["rules"]
            .as_array()
            .ok_or("Native verifier needs explicit stage admission rules")?;
        if rules.len() != 1 || rules[0]["if"] != expected {
            continue;
        }
        let rule = rules[0]
            .as_object()
            .ok_or("Admission rule must be a mapping")?;
        if rule
            .keys()
            .any(|k| !matches!(k.as_str(), "if" | "when" | "allow_failure"))
            || rule.get("allow_failure").is_some_and(|v| v != false)
            || rule
                .get("when")
                .is_some_and(|v| v != "on_success" && v != "manual")
        {
            return Err("Filtered or optional native verifier admission is not a required check");
        }
        if job["when"] == "manual" && job["allow_failure"] != false {
            return Err("A job-level manual verifier needs explicit allow_failure: false");
        }
        if scripts.len() != 1 {
            return Err(
                "Native verifier wrapper needs explicit exit/source review; use one direct invocation",
            );
        }
        let script = scripts[0]
            .as_str()
            .ok_or("Native invocation must be text")?;
        match crate::github::gate(script, post) {
            Ok(Some(_)) => count += 1,
            _ => {
                return Err(
                    "Native verifier stage or exit propagation is unresolved; preflight cannot replace execution",
                );
            }
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_native_runtime_does_not_require_product_source_builds()
    -> Result<(), Box<dyn std::error::Error>> {
        let reference = reference_install("0.5.0").ok_or("reference installer")?;
        let mut workflow = serde_json::json!({"variables":reference["variables"],
            "pre":{"before_script":reference["opdev"]["before_script"],"script":["opdev ci verify --stage pre-merge"],"rules":[{"if":"$CI_PIPELINE_SOURCE == \"merge_request_event\""}]},
            "post":{"before_script":reference["opdev"]["before_script"],"script":["opdev ci verify --stage post-merge"],"rules":[{"if":"$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH"}]}
        });
        let path = Path::new(".gitlab-ci.yml");
        assert_eq!(integrity(path, &workflow).outcome, Outcome::Passed);
        workflow["post"]["before_script"][10] = "echo skip verification".into();
        assert_ne!(integrity(path, &workflow).outcome, Outcome::Passed);
        assert_eq!(
            workflow["pre"]["before_script"],
            reference["opdev"]["before_script"]
        );
        Ok(())
    }
    #[test]
    fn runtime_build_must_supply_both_actual_stage_executables() {
        let good = serde_json::json!({
            "pre": {"before_script":["cargo build --locked -p opdev-cli"],"script":["./target/debug/opdev ci verify --stage pre-merge"],"rules":[{"if":"$CI_PIPELINE_SOURCE == \"merge_request_event\""}]},
            "post": {"before_script":["cargo build --locked -p opdev-cli"],"script":["./target/debug/opdev ci verify --stage post-merge"],"rules":[{"if":"$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH"}]}
        });
        let path = Path::new(".gitlab-ci.yml");
        assert_eq!(integrity(path, &good).outcome, Outcome::Passed);
        for key in ["pre", "post"] {
            let mut changed = good.clone();
            changed[key]["before_script"] = serde_json::json!([]);
            assert_ne!(integrity(path, &changed).outcome, Outcome::Passed);
            changed = good.clone();
            changed[key]["script"][0] = changed[key]["script"][0]
                .as_str()
                .unwrap_or_default()
                .replace("./target", "../target")
                .into();
            assert_ne!(integrity(path, &changed).outcome, Outcome::Passed);
        }
        let mut changed = good.clone();
        changed["variables"] = serde_json::json!({"GIT_CHECKOUT":"false"});
        assert_ne!(integrity(path, &changed).outcome, Outcome::Passed);
        let mut with_delivery = good.clone();
        with_delivery["delivery"] = serde_json::json!({
            "script":["./target/debug/opdev ci verify --stage delivery"],
            "rules":[{"if":"$CI_PIPELINE_SOURCE == \"web\"", "when":"manual", "allow_failure":false}]
        });
        assert_eq!(integrity(path, &with_delivery).outcome, Outcome::Passed);
        // Delivery wired in place of integration must still fail.
        with_delivery["pre"]["script"][0] =
            "./target/debug/opdev ci verify --stage delivery".into();
        assert_ne!(integrity(path, &with_delivery).outcome, Outcome::Passed);
    }
    #[test]
    fn stage_admission_and_actual_invocation_are_both_required() {
        let good = serde_json::json!({"verify":{"script":["opdev ci verify --stage pre-merge"],"rules":[{"if":"$CI_PIPELINE_SOURCE == \"merge_request_event\"","when":"manual","allow_failure":false}]}});
        assert_eq!(gates(&good, false), Ok(1));
        assert_eq!(gates(&good, true), Ok(0));
        for replacement in [
            "opdev ci verify --stage pre-merge --preflight",
            "opdev ci verify --stage delivery",
            "opdev ci verify --stage pre-merge || true",
        ] {
            let mut candidate = good.clone();
            candidate["verify"]["script"][0] = replacement.into();
            assert!(gates(&candidate, false).is_err());
        }
        for key in ["extends", "variables", "only"] {
            let mut candidate = good.clone();
            candidate["verify"][key] = "unknown".into();
            assert!(gates(&candidate, false).is_err());
        }
        let mut candidate = good.clone();
        candidate["verify"]["rules"][0]["allow_failure"] = true.into();
        assert!(gates(&candidate, false).is_err());
        candidate = good.clone();
        candidate["duplicate"] = good["verify"].clone();
        assert_ne!(
            inspect(Path::new(".gitlab-ci.yml"), &candidate, false).outcome,
            Outcome::Passed
        );
    }
}
