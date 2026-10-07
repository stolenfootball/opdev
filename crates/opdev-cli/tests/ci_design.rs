//! Distribution and executable example boundaries, not a provider emulator or agent evaluation.
use std::{fs, path::Path, process::Command};

#[test]
fn ci_design_reference_survives_isolated_plugin_copy() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let skill = root.join("plugins/opdev/skills/opdev");
    let temp = tempfile::tempdir()?;
    fs::create_dir_all(temp.path().join("references"))?;
    let routes = [
        "SKILL.md",
        "references/planning.md",
        "references/adoption.md",
    ];
    for relative in routes.into_iter().chain(["references/ci-design.md"]) {
        fs::copy(skill.join(relative), temp.path().join(relative))?;
    }
    let expected = temp.path().join("references/ci-design.md").canonicalize()?;
    for relative in routes {
        let source = temp.path().join(relative);
        let text = fs::read_to_string(&source)?;
        let links: Vec<_> = text
            .split("](")
            .skip(1)
            .filter_map(|part| part.split_once(')').map(|(target, _)| target))
            .filter(|target| target.ends_with("ci-design.md"))
            .collect();
        assert!(!links.is_empty(), "missing CI route from {relative}");
        for link in links {
            let resolved = source.parent().ok_or("parent")?.join(link).canonicalize()?;
            assert_eq!(resolved, expected);
            assert!(resolved.starts_with(temp.path().canonicalize()?));
            assert_eq!(
                fs::read(resolved)?,
                fs::read(skill.join("references/ci-design.md"))?
            );
        }
    }
    Ok(())
}

#[test]
fn github_required_gate_executes_and_rejects_every_non_success()
-> Result<(), Box<dyn std::error::Error>> {
    let config: serde_json::Value =
        serde_saphyr::from_str(include_str!("../../../examples/feedback/github.yml"))?;
    let gate = &config["jobs"]["integration"];
    assert_eq!(gate["needs"], "verify");
    assert_eq!(gate["if"], "${{ always() }}");
    let step = &gate["steps"][0];
    assert_eq!(step["shell"], "python");
    assert_eq!(step["env"]["VERIFY_RESULT"], "${{ needs.verify.result }}");
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("gate.py");
    fs::write(
        &script,
        step["run"].as_str().ok_or("executable gate missing")?,
    )?;
    for result in [
        None,
        Some(""),
        Some("success"),
        Some("failure"),
        Some("skipped"),
        Some("cancelled"),
        Some("neutral"),
        Some("pending"),
        Some("unknown"),
    ] {
        let mut command = Command::new(if cfg!(windows) { "python" } else { "python3" });
        command.arg(&script).env_remove("VERIFY_RESULT");
        if let Some(value) = result {
            command.env("VERIFY_RESULT", value);
        }
        let output = command.output()?;
        assert_eq!(
            output.status.success(),
            result == Some("success"),
            "result {result:?}: {output:?}"
        );
        if result != Some("success") {
            assert!(
                String::from_utf8_lossy(&output.stdout)
                    .contains("Required verification did not succeed")
            );
        }
    }
    Ok(())
}

#[test]
fn example_topology_preserves_required_checks_and_exact_candidate_flow()
-> Result<(), Box<dyn std::error::Error>> {
    let gh: serde_json::Value =
        serde_saphyr::from_str(include_str!("../../../examples/feedback/github.yml"))?;
    assert_eq!(gh["on"]["push"]["branches"], serde_json::json!(["main"]));
    assert!(gh["on"].get("pull_request").is_some());
    assert!(gh["on"].get("merge_group").is_some());
    assert!(gh["on"].get("workflow_dispatch").is_none());
    for event in ["push", "pull_request", "merge_group"] {
        assert!(gh["on"][event].get("paths").is_none());
        assert!(gh["on"][event].get("paths-ignore").is_none());
    }
    assert!(gh["jobs"]["verify"].get("if").is_none());
    assert!(gh["jobs"]["verify"].get("continue-on-error").is_none());
    assert_eq!(gh["jobs"]["candidate"]["needs"], "integration");
    assert_eq!(gh["jobs"]["consume"]["needs"], "candidate");
    let gl: serde_json::Value =
        serde_saphyr::from_str(include_str!("../../../examples/feedback/gitlab.yml"))?;
    let rules = gl["workflow"]["rules"].as_array().ok_or("rules")?;
    assert_eq!(rules.len(), 2);
    assert_eq!(
        rules[0]["if"],
        "$CI_PIPELINE_SOURCE == \"merge_request_event\""
    );
    assert_eq!(
        rules[1]["if"],
        "$CI_PIPELINE_SOURCE == \"push\" && $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH"
    );
    assert!(gl["verify"].get("rules").is_none());
    assert!(gl["verify"].get("allow_failure").is_none());
    assert!(gl["verify"].get("when").is_none());
    assert_eq!(
        gl["candidate"]["needs"],
        serde_json::json!([{"job":"verify","artifacts":true}])
    );
    assert_eq!(
        gl["consume"]["needs"],
        serde_json::json!([{"job":"candidate","artifacts":true}])
    );
    for job in ["candidate", "consume"] {
        assert_eq!(
            gl[job]["rules"],
            serde_json::json!([{"if":"$CI_PIPELINE_SOURCE == \"push\" && $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH"}])
        );
    }
    Ok(())
}

#[test]
fn design_scenarios_keep_review_answers_out_of_model_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../benchmarks/ci-review/design/cases.json"
    ))?;
    let review: serde_json::Map<String, serde_json::Value> = serde_json::from_str(include_str!(
        "../../../benchmarks/ci-review/design/review.json"
    ))?;
    let mut ids = std::collections::BTreeSet::new();
    for case in &cases {
        let fields = case.as_object().ok_or("case object")?;
        assert_eq!(fields.len(), 5);
        for field in ["id", "provider", "kind", "request", "facts"] {
            assert!(
                !fields[field]
                    .as_str()
                    .ok_or("string field")?
                    .trim()
                    .is_empty()
            );
        }
        let id = case["id"].as_str().ok_or("id")?;
        assert!(ids.insert(id), "duplicate scenario");
        assert!(review.get(id).and_then(|value| value.as_str()).is_some());
    }
    assert_eq!(ids.len(), review.len());
    for provider in ["github", "gitlab"] {
        for kind in ["new", "existing"] {
            assert!(
                cases
                    .iter()
                    .any(|case| case["provider"] == provider && case["kind"] == kind)
            );
        }
    }
    Ok(())
}
