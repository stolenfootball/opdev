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
    match gates_with_review(value, post, None, trunk(path).as_deref()) {
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
    }) || value.get("default").is_some_and(|defaults| {
        defaults.as_object().is_none_or(|map| {
            map.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "before_script" | "image" | "tags" | "artifacts"
                )
            })
        })
    }) {
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
        if job.get("inherit").is_some() || job.get("hooks").is_some() {
            return gap("Runtime inheritance or hooks need effective setup review");
        }
        let setup = job
            .get("before_script")
            .unwrap_or(&value["default"]["before_script"]);
        let build = setup
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
            _ if signed_install(setup, value) => executable.as_deref() == Some("opdev"),
            _ if pinned_source_build(setup, executable.as_deref()) => true,
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
    let trunk = trunk(path);
    if count == 2
        && gates_with_review(value, false, None, trunk.as_deref()) == Ok(1)
        && gates_with_review(value, true, None, trunk.as_deref()) == Ok(1)
    {
        passed(
            path,
            "Both native stage jobs declare same-job signed installation or locked source builds and their exact executable; actual CI checkout identity, runtime capabilities and execution still need verification",
        )
    } else {
        gap("Both required native stages and their runtime setup need review")
    }
}

fn signed_install(setup: &Value, workflow: &Value) -> bool {
    let Some(version) = workflow["variables"]["OPDEV_VERSION"].as_str() else {
        return false;
    };
    let Some(reference) = reference_install(version) else {
        return false;
    };
    setup.is_array() && *setup == reference["opdev"]["before_script"]
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

fn pinned_source_build(setup: &Value, executable: Option<&str>) -> bool {
    let Some(steps) = setup
        .as_array()
        .filter(|steps| (3..=128).contains(&steps.len()))
    else {
        return false;
    };
    // As with the GitHub adapter, prerequisite setup can precede the runtime
    // checkout. Nothing may intervene after the exact checkout/build sequence.
    // This describes wiring, not trust in arbitrary prerequisite programs.
    let tail = &steps[steps.len() - 3..];
    let Some(clone) = tail[0].as_str() else {
        return false;
    };
    let Some(checkout) = tail[1].as_str() else {
        return false;
    };
    let Some(build) = tail[2].as_str() else {
        return false;
    };
    let clone: Vec<_> = clone.split_whitespace().collect();
    let checkout: Vec<_> = checkout.split_whitespace().collect();
    let build: Vec<_> = build.split_whitespace().collect();
    let ["git", "clone", "--no-checkout", remote, source] = clone.as_slice() else {
        return false;
    };
    if !matches!(
        *remote,
        "https://gitlab.com/stolenfootball-tools/opdev.git"
            | "https://github.com/stolenfootball/opdev.git"
    ) || !relative_directory(source)
    {
        return false;
    }
    let ["git", "-C", directory, "checkout", "--detach", revision] = checkout.as_slice() else {
        return false;
    };
    if directory != source
        || revision.len() != 40
        || !revision.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return false;
    }
    let [
        "cargo",
        "build",
        "--locked",
        "--manifest-path",
        manifest,
        "-p",
        "opdev-cli",
        "--target-dir",
        target,
    ] = build.as_slice()
    else {
        return false;
    };
    if *manifest != format!("{source}/Cargo.toml") || !relative_directory(target) {
        return false;
    }
    executable.is_some_and(|command| {
        command == format!("{target}/debug/opdev") || command == format!("{target}/debug/opdev.exe")
    })
}

fn relative_directory(value: &str) -> bool {
    !value.is_empty()
        && value.split('/').all(|part| {
            !matches!(part, "" | "." | ".." | ".git")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
}

fn gap(message: &str) -> Capability {
    Capability {
        outcome: Outcome::Unverified,
        evidence: vec![],
        diagnostic: Some(message.into()),
    }
}

#[cfg(test)]
fn gates(value: &Value, post: bool) -> Result<usize, &'static str> {
    gates_with_review(value, post, None, None)
}

fn trunk(path: &Path) -> Option<String> {
    opdev_project::ProjectManifest::load(&path.parent()?.join(opdev_project::MANIFEST_PATH))
        .ok()
        .map(|manifest| manifest.project.trunk)
}

pub(crate) fn review_handoff(path: &Path, value: &Value, storage: u32) -> Capability {
    for post in [false, true] {
        match gates_with_review(value, post, Some(storage), trunk(path).as_deref()) {
            Ok(1) => {}
            Ok(_) => {
                return gap(
                    "The selected review needs exactly one supported required invocation per stage; inspect the missing or duplicate handoff. No checks ran",
                );
            }
            Err(message) => {
                return gap(&format!(
                    "The selected review handoff is unverified: {message}. No checks ran"
                ));
            }
        }
    }
    passed(
        path,
        "Selected review inputs are wired to both stage invocations; live token access, review identity and execution remain unverified",
    )
}

fn gates_with_review(
    value: &Value,
    post: bool,
    storage: Option<u32>,
    trunk: Option<&str>,
) -> Result<usize, &'static str> {
    let jobs = value.as_object().ok_or("CI jobs must be a mapping")?;
    if value["default"].get("allow_failure").is_some() {
        return Err(
            "Global admission/default overrides need effective provider review; native job text alone cannot establish required execution",
        );
    }
    if let Some(workflow) = value.get("workflow") {
        if workflow
            .as_object()
            .is_none_or(|map| map.keys().any(|key| key != "rules"))
        {
            return Err("Unsupported workflow overrides need effective provider review");
        }
        if selected_rule(&workflow["rules"], post, trunk, true)?.is_none() {
            return Err("Workflow rules do not admit this required stage");
        }
    }
    let mut count = 0;
    for (name, job) in jobs {
        if name.starts_with('.') {
            continue;
        }
        let Some(scripts) = job["script"].as_array() else {
            continue;
        };
        if !scripts.iter().any(|s| {
            s.as_str().is_some_and(|s| {
                s.contains("ci verify")
                    || (storage.is_some() && s.contains("opdev") && s.contains("check"))
            })
        }) {
            continue;
        }
        if [
            "extends",
            "only",
            "except",
            "trigger",
            "parallel",
            "variables",
            "inherit",
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
        let Some(selected) = selected_rule(&job["rules"], post, trunk, false)? else {
            continue;
        };
        let rule = selected
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
        let invocation = match storage {
            Some(version) => crate::github::review_gate(script, post, version),
            None => crate::github::gate(script, post),
        };
        match invocation {
            Ok(Some(_)) => count += 1,
            _ => {
                return Err(
                    "Stage, review arguments or exit propagation is unresolved; preflight cannot replace execution",
                );
            }
        }
    }
    Ok(count)
}

// GitLab evaluates rules in order and stops at the first match. Deliberately
// support only statically decidable boundary expressions, never assume unknown
// variables, path filters or an earlier exclusion cannot affect admission.
fn selected_rule<'a>(
    rules: &'a Value,
    post: bool,
    trunk: Option<&str>,
    workflow: bool,
) -> Result<Option<&'a Value>, &'static str> {
    let rules = rules
        .as_array()
        .ok_or("Explicit admission rules are required")?;
    if rules.len() > 128 {
        return Err("Admission rules exceed the inspection bound");
    }
    for rule in rules {
        let fields = rule.as_object().ok_or("Admission rule must be a mapping")?;
        if fields.keys().any(|key| {
            !(matches!(key.as_str(), "if" | "when") || key == "allow_failure" && !workflow)
        }) {
            return Err("Filtered or variable-changing admission needs effective provider review");
        }
        let applies = match fields.get("if") {
            None => true,
            Some(Value::String(condition)) => boundary_condition(condition, post, trunk)?,
            Some(_) => return Err("Admission condition must be text"),
        };
        if applies {
            return match rule.get("when").and_then(Value::as_str) {
                Some("never") => Ok(None),
                None if rule.get("when").is_some() => Err("Admission timing must be text"),
                None | Some("always") if workflow => Ok(Some(rule)),
                None | Some("on_success" | "manual") if !workflow => Ok(Some(rule)),
                _ => Err("Unsupported admission timing needs effective provider review"),
            };
        }
    }
    Ok(None)
}

fn boundary_condition(
    condition: &str,
    post: bool,
    trunk: Option<&str>,
) -> Result<bool, &'static str> {
    if condition.len() > 4096 {
        return Err("Admission condition exceeds the inspection bound");
    }
    // AND binds more tightly than OR. Parentheses, regexes and arbitrary shell
    // syntax are not evaluated by this bounded declarative inspector.
    let mut result = false;
    for disjunction in condition.split("||") {
        let mut conjunction = true;
        for atom in disjunction.split("&&") {
            conjunction &= boundary_atom(atom.trim(), post, trunk)?;
        }
        result |= conjunction;
    }
    Ok(result)
}

fn boundary_atom(atom: &str, post: bool, trunk: Option<&str>) -> Result<bool, &'static str> {
    match atom {
        "$CI_COMMIT_BRANCH" => return Ok(post),
        "$CI_COMMIT_TAG" => return Ok(false),
        _ => {}
    }
    let (left, right) = atom
        .split_once("==")
        .ok_or("Unsupported admission expression; inspect effective provider configuration")?;
    let right = right.trim();
    let literal = right
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| right.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')));
    match left.trim() {
        "$CI_PIPELINE_SOURCE" => literal
            .map(|value| value == if post { "push" } else { "merge_request_event" })
            .ok_or("Pipeline source must be a literal for static admission review"),
        "$CI_COMMIT_BRANCH" if right == "$CI_DEFAULT_BRANCH" => Ok(post),
        "$CI_COMMIT_BRANCH" => {
            let branch =
                literal.ok_or("Branch comparison needs a literal or default-branch identity")?;
            let trunk = trunk.ok_or("Literal branch admission needs the declared project trunk")?;
            Ok(post && branch == trunk)
        }
        _ => Err("Unknown admission input; no unconditional execution assumed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workflow_first_match_and_default_runtime_are_both_inspected() {
        let good = serde_json::json!({
            "default":{"tags":["linux"],"before_script":["cargo build --locked -p opdev-cli"]},
            "workflow":{"rules":[{"if":"$CI_PIPELINE_SOURCE == \"merge_request_event\""},{"if":"$CI_PIPELINE_SOURCE == \"push\" && $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH"}]},
            "pre":{"script":["./target/debug/opdev ci verify --stage pre-merge"],"rules":[{"if":"$CI_PIPELINE_SOURCE == \"merge_request_event\""}]},
            "post":{"script":["./target/debug/opdev ci verify --stage post-merge"],"rules":[{"if":"$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH && $CI_PIPELINE_SOURCE == \"push\""}]}
        });
        let path = Path::new(".gitlab-ci.yml");
        assert_eq!(integrity(path, &good).outcome, Outcome::Passed);
        assert_eq!(review_handoff(path, &good, 2).outcome, Outcome::Passed);
        for rule in [
            serde_json::json!({"if":"$CI_PIPELINE_SOURCE == \"merge_request_event\"","when":"never"}),
            serde_json::json!({"if":"$UNKNOWN == \"true\"","when":"never"}),
            serde_json::json!({"changes":["src/**"],"when":"always"}),
        ] {
            let mut changed = good.clone();
            changed["workflow"]["rules"][0] = rule;
            assert_ne!(integrity(path, &changed).outcome, Outcome::Passed);
            assert_ne!(review_handoff(path, &changed, 2).outcome, Outcome::Passed);
        }
        let mut changed = good.clone();
        changed["post"]["before_script"] = serde_json::json!([]);
        assert_ne!(integrity(path, &changed).outcome, Outcome::Passed);
        changed = good.clone();
        changed["default"]["hooks"] =
            serde_json::json!({"pre_get_sources_script":["echo altered"]});
        assert_ne!(integrity(path, &changed).outcome, Outcome::Passed);
        assert_eq!(
            boundary_condition(
                "$CI_COMMIT_BRANCH == 'develop' && $CI_PIPELINE_SOURCE == 'push'",
                true,
                Some("develop")
            ),
            Ok(true)
        );
        assert_eq!(
            boundary_condition("$CI_COMMIT_BRANCH == 'develop'", true, Some("main")),
            Ok(false)
        );
        assert!(boundary_condition("$CI_COMMIT_BRANCH == 'develop'", true, None).is_err());
    }

    #[test]
    fn pinned_runtime_requires_exact_checkout_locked_build_and_direct_output() {
        let setup = serde_json::json!([
            "prerequisite-tool-setup",
            "git clone --no-checkout https://gitlab.com/stolenfootball-tools/opdev.git .tools/source",
            "git -C .tools/source checkout --detach 1111111111111111111111111111111111111111",
            "cargo build --locked --manifest-path .tools/source/Cargo.toml -p opdev-cli --target-dir .tools/target"
        ]);
        assert!(pinned_source_build(
            &setup,
            Some(".tools/target/debug/opdev")
        ));
        for executable in [
            None,
            Some("opdev"),
            Some("target/debug/opdev"),
            Some("../.tools/target/debug/opdev"),
        ] {
            assert!(!pinned_source_build(&setup, executable));
        }
        for (index, replacement) in [
            (
                1,
                "git clone --no-checkout https://example.test/other.git .tools/source",
            ),
            (2, "git -C .tools/source checkout --detach main"),
            (
                2,
                "git -C .tools/other checkout --detach 1111111111111111111111111111111111111111",
            ),
            (
                3,
                "cargo build --manifest-path .tools/source/Cargo.toml -p opdev-cli --target-dir .tools/target",
            ),
        ] {
            let mut changed = setup.clone();
            changed[index] = replacement.into();
            assert!(!pinned_source_build(
                &changed,
                Some(".tools/target/debug/opdev")
            ));
        }
        let mut changed = setup;
        if let Some(steps) = changed.as_array_mut() {
            steps.push("echo changed after build".into());
        }
        assert!(!pinned_source_build(
            &changed,
            Some(".tools/target/debug/opdev")
        ));
    }
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
