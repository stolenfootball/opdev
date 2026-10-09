//! Consumer-boundary regressions for bounded GitHub configuration inspection.
use opdev_ci::{TemplateContext, adapter_for};
use opdev_core::Outcome;
use opdev_project::CiProvider;
use std::fs;

#[test]
fn a_custom_workflow_filename_preserves_the_generated_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let adapter = adapter_for(CiProvider::Github)?;
    fs::create_dir_all(root.path().join(".github/workflows"))?;
    let source = adapter.render(&TemplateContext {
        opdev_version: "0.4.0".into(),
        trunk: "main".into(),
        job_image: None,
    })?;
    fs::write(root.path().join(".github/workflows/quality.yaml"), source)?;
    let observed = adapter.inspect(root.path())?;
    assert_eq!(observed.configuration.outcome, Outcome::Passed);
    assert_eq!(observed.pre_merge.outcome, Outcome::Passed);
    assert_eq!(observed.post_merge.outcome, Outcome::Passed);
    assert_eq!(observed.integrity.outcome, Outcome::Passed);
    Ok(())
}

const SOURCE_WORKFLOW: &str = r"
on:
  pull_request:
  push:
    branches: [main]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
        with:
          ref: ${{ github.event.pull_request.head.sha || github.sha }}
      - run: cargo build --locked -p opdev-cli
      - if: github.event_name == 'pull_request'
        run: ./target/debug/opdev check --ci --review-ci
      - if: github.event_name == 'push'
        run: ./target/debug/opdev check --ci --post-merge --review-ci
";

fn fixture(source: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    fs::create_dir_all(root.path().join(".github/workflows"))?;
    fs::write(
        root.path().join(".github/workflows/project-ci.yaml"),
        source,
    )?;
    Ok(root)
}

fn observe(
    value: &serde_json::Value,
) -> Result<opdev_ci::CiInspection, Box<dyn std::error::Error>> {
    // JSON is valid YAML; fixtures mutate actual structure, not a detector's strings.
    let root = fixture(&serde_json::to_string(value)?)?;
    Ok(adapter_for(CiProvider::Github)?.inspect(root.path())?)
}

#[test]
fn source_build_and_windows_commands_need_no_release_installer_text()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = adapter_for(CiProvider::Github)?;
    for windows in [false, true] {
        let source = if windows {
            SOURCE_WORKFLOW
                .replace("ubuntu-latest", "windows-latest")
                .replace("./target/debug/opdev", "& .\\target\\debug\\opdev.exe")
                .replace("run: & ", "run: |\n          & ")
        } else {
            SOURCE_WORKFLOW.into()
        };
        assert!(!source.contains("SHA256SUMS"));
        let root = fixture(&source)?;
        let found = adapter.inspect(root.path())?;
        assert_eq!(found.pre_merge.outcome, Outcome::Passed, "{found:?}");
        assert_eq!(found.post_merge.outcome, Outcome::Passed, "{found:?}");
        assert_eq!(found.integrity.outcome, Outcome::Passed, "{found:?}");
    }
    Ok(())
}

#[test]
fn declared_non_main_trunk_is_used_instead_of_a_default_guess()
-> Result<(), Box<dyn std::error::Error>> {
    let root = fixture(&SOURCE_WORKFLOW.replace("[main]", "[develop]"))?;
    let adapter = adapter_for(CiProvider::Github)?;
    assert_ne!(
        adapter.inspect(root.path())?.post_merge.outcome,
        Outcome::Passed
    );
    assert_eq!(
        adapter
            .inspect_for_trunk(root.path(), "develop")?
            .post_merge
            .outcome,
        Outcome::Passed
    );
    Ok(())
}

#[test]
fn an_external_source_build_is_bound_to_its_checkout_and_exact_ref()
-> Result<(), Box<dyn std::error::Error>> {
    let mut value: serde_json::Value = serde_saphyr::from_str(SOURCE_WORKFLOW)?;
    let steps = value["jobs"]["verify"]["steps"]
        .as_array_mut()
        .ok_or("steps")?;
    steps.insert(1, serde_json::json!({
        "uses":"actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683",
        "with":{"repository":"example/opdev-source","ref":"1234567890123456789012345678901234567890","path":"tool"}
    }));
    steps[2]["working-directory"] = "tool".into();
    for index in [3, 4] {
        steps[index]["run"] = steps[index]["run"]
            .as_str()
            .ok_or("run")?
            .replace("./target", "./tool/target")
            .into();
    }
    assert_eq!(observe(&value)?.integrity.outcome, Outcome::Passed);
    value["jobs"]["verify"]["steps"][1]["with"]["ref"] = "main".into();
    assert_eq!(observe(&value)?.integrity.outcome, Outcome::Unverified);
    Ok(())
}

#[test]
fn local_read_bounds_and_nonregular_paths_are_not_partial_successes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = fixture(SOURCE_WORKFLOW)?;
    let adapter = adapter_for(CiProvider::Github)?;
    fs::create_dir(root.path().join(".github/workflows/directory.yml"))?;
    assert_eq!(
        adapter.inspect(root.path())?.configuration.outcome,
        Outcome::Error
    );
    let bounded = fixture(SOURCE_WORKFLOW)?;
    for index in 0..64 {
        fs::write(
            bounded
                .path()
                .join(format!(".github/workflows/file-{index}.yml")),
            SOURCE_WORKFLOW,
        )?;
    }
    assert_eq!(
        adapter.inspect(bounded.path())?.configuration.outcome,
        Outcome::Unverified
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn linked_workflows_cannot_read_outside_the_repository() -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture(SOURCE_WORKFLOW)?;
    let external = tempfile::tempdir()?;
    fs::write(external.path().join("secret.yml"), "PRIVATE_SENTINEL")?;
    std::os::unix::fs::symlink(
        external.path().join("secret.yml"),
        root.path().join(".github/workflows/linked.yml"),
    )?;
    let found = adapter_for(CiProvider::Github)?.inspect(root.path())?;
    assert_eq!(found.configuration.outcome, Outcome::Error);
    assert!(!format!("{found:?}").contains("PRIVATE_SENTINEL"));
    Ok(())
}

#[test]
fn misleading_strings_optional_or_skipped_checks_never_pass()
-> Result<(), Box<dyn std::error::Error>> {
    let baseline: serde_json::Value = serde_saphyr::from_str(SOURCE_WORKFLOW)?;
    for case in [
        "job_optional",
        "step_optional",
        "job_false",
        "step_false",
        "comment",
        "echo",
        "masked",
        "inspection",
        "manual",
        "wrong_event",
        "wrong_branch",
        "wrong_checkout",
        "wrong_stage",
        "missing_runtime",
        "unlocked",
        "build_after_gate",
        "action_after_build",
    ] {
        let mut value = baseline.clone();
        match case {
            "job_optional" => value["jobs"]["verify"]["continue-on-error"] = true.into(),
            "step_optional" => value["jobs"]["verify"]["steps"][2]["continue-on-error"] = true.into(),
            "job_false" => value["jobs"]["verify"]["if"] = false.into(),
            "step_false" => value["jobs"]["verify"]["steps"][2]["if"] = false.into(),
            "comment" => value["jobs"]["verify"]["steps"][2]["run"] = "# opdev check --ci SHA256SUMS sha256sum -c verify-blob --certificate-identity --certificate-oidc-issuer\necho harmless".into(),
            "echo" => value["jobs"]["verify"]["steps"][2]["run"] = "echo opdev check --ci".into(),
            "masked" => value["jobs"]["verify"]["steps"][2]["run"] = "./target/debug/opdev check --ci || true".into(),
            "inspection" => value["jobs"]["verify"]["steps"][2]["run"] = "./target/debug/opdev check --ci --no-exec".into(),
            "manual" => value["on"] = "workflow_dispatch".into(),
            "wrong_event" => value["on"] = serde_json::json!({"pull_request_target":null}),
            "wrong_branch" => value["on"]["pull_request"] = serde_json::json!({"branches":["other"]}),
            "wrong_checkout" => value["jobs"]["verify"]["steps"][0]["with"]["ref"] = "main".into(),
            "wrong_stage" => value["jobs"]["verify"]["steps"][2]["run"] = "./target/debug/opdev check --ci --post-merge".into(),
            "missing_runtime" => value["jobs"]["verify"]["steps"][1]["run"] = "echo no runtime".into(),
            "unlocked" => value["jobs"]["verify"]["steps"][1]["run"] = "cargo build -p opdev-cli".into(),
            "build_after_gate" => value["jobs"]["verify"]["steps"].as_array_mut().ok_or("steps")?.swap(1,2),
            "action_after_build" => value["jobs"]["verify"]["steps"].as_array_mut().ok_or("steps")?.insert(2, serde_json::json!({"uses":"example/replace-runtime@main"})),
            _ => unreachable!(),
        }
        let result = observe(&value)?;
        assert_ne!(
            result.pre_merge.outcome,
            Outcome::Passed,
            "{case}: {result:?}"
        );
    }
    Ok(())
}

#[test]
fn unsupported_control_flow_and_ambiguous_candidates_are_explicit()
-> Result<(), Box<dyn std::error::Error>> {
    let baseline: serde_json::Value = serde_saphyr::from_str(SOURCE_WORKFLOW)?;
    for case in [
        "paths",
        "condition",
        "needs",
        "matrix",
        "shell",
        "root",
        "runtime_override",
        "missing_runner",
        "duplicate",
        "remote",
    ] {
        let mut value = baseline.clone();
        match case {
            "paths" => value["on"]["pull_request"] = serde_json::json!({"paths":["src/**"]}),
            "condition" => value["jobs"]["verify"]["if"] = "inputs.run_checks".into(),
            "needs" => value["jobs"]["verify"]["needs"] = "build".into(),
            "matrix" => {
                value["jobs"]["verify"]["strategy"] =
                    serde_json::json!({"matrix":{"os":["ubuntu-latest"]}});
            }
            "shell" => {
                value["jobs"]["verify"]["defaults"] =
                    serde_json::json!({"run":{"shell":"bash {0}"}});
            }
            "root" => {
                value["jobs"]["verify"]["steps"][2]["working-directory"] =
                    "different-project".into();
            }
            "runtime_override" => {
                value["jobs"]["verify"]["env"] = serde_json::json!({"CARGO_TARGET_DIR":"other"});
            }
            "missing_runner" => {
                value["jobs"]["verify"]
                    .as_object_mut()
                    .ok_or("job")?
                    .remove("runs-on");
            }
            "duplicate" => value["jobs"]["another"] = value["jobs"]["verify"].clone(),
            "remote" => {
                value["jobs"]["verify"] =
                    serde_json::json!({"uses":"owner/repo/.github/workflows/check.yml@main"});
            }
            _ => unreachable!(),
        }
        let found = observe(&value)?;
        assert_eq!(
            found.pre_merge.outcome,
            Outcome::Unverified,
            "{case}: {found:?}"
        );
        assert!(found.pre_merge.diagnostic.is_some());
    }
    Ok(())
}

#[test]
fn same_commit_reusable_workflow_is_resolved_without_network()
-> Result<(), Box<dyn std::error::Error>> {
    let root = fixture(
        "on: [pull_request, push]\njobs:\n  required:\n    uses: ./.github/workflows/checks.yml\n",
    )?;
    let mut child: serde_json::Value = serde_saphyr::from_str(SOURCE_WORKFLOW)?;
    child["on"] = "workflow_call".into();
    fs::write(
        root.path().join(".github/workflows/checks.yml"),
        serde_json::to_string(&child)?,
    )?;
    let adapter = adapter_for(CiProvider::Github)?;
    let found = adapter.inspect(root.path())?;
    assert_eq!(found.pre_merge.outcome, Outcome::Passed, "{found:?}");
    assert_eq!(found.post_merge.outcome, Outcome::Passed, "{found:?}");
    child["jobs"]["verify"] = serde_json::json!({"uses":"./.github/workflows/checks.yml"});
    fs::write(
        root.path().join(".github/workflows/checks.yml"),
        serde_json::to_string(&child)?,
    )?;
    assert_eq!(
        adapter.inspect(root.path())?.pre_merge.outcome,
        Outcome::Error
    );
    Ok(())
}

#[test]
fn runtime_setup_in_a_different_job_cannot_supply_the_gate()
-> Result<(), Box<dyn std::error::Error>> {
    let mut value: serde_json::Value = serde_saphyr::from_str(SOURCE_WORKFLOW)?;
    value["jobs"]["build"] = serde_json::json!({
        "runs-on":"ubuntu-latest", "steps":[
            {"uses":"actions/checkout@v4", "with":{"ref":"${{ github.event.pull_request.head.sha || github.sha }}"}},
            {"run":"cargo build --locked -p opdev-cli"}
        ]
    });
    value["jobs"]["verify"]["steps"]
        .as_array_mut()
        .ok_or("steps")?
        .remove(1);
    assert_eq!(observe(&value)?.pre_merge.outcome, Outcome::Unverified);
    Ok(())
}

#[test]
fn generated_stage_steps_select_exact_source_and_separate_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let rendered = adapter_for(CiProvider::Github)?.render(&TemplateContext {
        opdev_version: "0.4.0".into(),
        trunk: "develop".into(),
        job_image: None,
    })?;
    let value: serde_json::Value = serde_saphyr::from_str(&rendered)?;
    let steps = &value["jobs"]["opdev"]["steps"];
    assert_eq!(
        steps[0]["with"]["ref"],
        "${{ github.event.pull_request.head.sha || github.sha }}"
    );
    assert_eq!(steps[2]["if"], "github.event_name == 'pull_request'");
    assert_eq!(steps[3]["if"], "github.event_name == 'push'");
    assert!(
        !steps[2]["run"]
            .as_str()
            .ok_or("pre")?
            .contains("--post-merge")
    );
    assert!(
        steps[3]["run"]
            .as_str()
            .ok_or("post")?
            .contains("--post-merge")
    );
    assert_eq!(value["on"]["push"]["branches"][0], "develop");
    Ok(())
}

#[test]
fn malformed_missing_and_oversized_workflows_cannot_hide_behind_a_valid_one()
-> Result<(), Box<dyn std::error::Error>> {
    let root = fixture(SOURCE_WORKFLOW)?;
    let adapter = adapter_for(CiProvider::Github)?;
    for source in ["jobs: [SECRET_SENTINEL", "on: push\njobs: []"] {
        fs::write(root.path().join(".github/workflows/extra.yml"), source)?;
        let found = adapter.inspect(root.path())?;
        assert_eq!(found.configuration.outcome, Outcome::Error);
        assert!(!format!("{found:?}").contains("SECRET_SENTINEL"));
    }
    fs::write(
        root.path().join(".github/workflows/extra.yml"),
        "a".repeat(1024 * 1024 + 1),
    )?;
    assert_eq!(
        adapter.inspect(root.path())?.configuration.outcome,
        Outcome::Error
    );
    Ok(())
}
