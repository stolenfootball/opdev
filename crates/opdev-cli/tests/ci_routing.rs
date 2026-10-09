//! CI routing must preserve platform-specific qualification requirements.

use serde_json::{Value, json};

#[test]
fn diagnostic_examples_are_explicit_and_do_not_publish_candidates()
-> Result<(), Box<dyn std::error::Error>> {
    let lab: Value = serde_saphyr::from_str(include_str!(
        "../../../examples/feedback/diagnostic-gitlab.yml"
    ))?;
    assert!(
        lab["stages"]
            .as_array()
            .ok_or("stages")?
            .contains(&lab["diagnose"]["stage"])
    );
    assert_eq!(lab["workflow"]["rules"].as_array().ok_or("rules")?.len(), 1);
    assert_eq!(
        lab["workflow"]["rules"][0]["if"],
        "$CI_PIPELINE_SOURCE == \"web\" || $CI_PIPELINE_SOURCE == \"api\""
    );
    assert_eq!(
        lab["diagnose"]["script"],
        json!(["python3 examples/feedback/fixture.py diagnose Linux"])
    );
    assert!(lab.get("candidate").is_none());
    let hub: Value = serde_saphyr::from_str(include_str!(
        "../../../examples/feedback/diagnostic-github.yml"
    ))?;
    assert_eq!(hub["on"], "workflow_dispatch");
    assert_eq!(hub["permissions"], json!({"contents":"read"}));
    assert_eq!(hub["jobs"].as_object().ok_or("jobs")?.len(), 1);
    Ok(())
}

#[test]
fn quality_has_one_canonical_execution_and_no_feature_push_pipeline()
-> Result<(), Box<dyn std::error::Error>> {
    let ci: Value = serde_saphyr::from_str(include_str!("../../../.gitlab-ci.yml"))?;
    let scripts = ci["quality"]["script"].as_array().ok_or("quality script")?;
    assert_eq!(
        scripts
            .iter()
            .filter(|s| s.as_str().is_some_and(|s| s.contains("check --ci")))
            .count(),
        1
    );
    assert!(!scripts.iter().any(|s| s.as_str().is_some_and(|s| {
        ["cargo fmt", "cargo clippy", "cargo test"]
            .iter()
            .any(|command| s.contains(command))
    })));
    let body = scripts
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(body.contains("cargo llvm-cov show-env --export-prefix"));
    assert!(body.contains("opdev check --ci --review-ci"));
    assert!(body.contains("--post-merge"));
    assert!(!body.contains("ci_review.py"));
    assert!(!body.contains("GITLAB_TOKEN="));
    assert!(body.contains("cargo llvm-cov report --lcov"));
    assert!(!body.contains("cargo llvm-cov test"));
    assert_eq!(ci["quality"]["artifacts"]["expire_in"], "30 days");
    assert_eq!(ci["quality"]["artifacts"]["when"], "always");
    assert_ne!(ci["quality"]["allow_failure"], true);
    let rules = ci["workflow"]["rules"].as_array().ok_or("workflow rules")?;
    assert_eq!(rules.len(), 3);
    assert!(
        rules
            .iter()
            .any(|rule| rule["if"] == "$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH")
    );
    assert!(
        rules
            .iter()
            .any(|rule| rule["if"] == "$CI_PIPELINE_SOURCE == \"merge_request_event\"")
    );
    assert!(rules.iter().any(|rule| rule["if"] == "$CI_COMMIT_TAG"));
    Ok(())
}

#[test]
fn native_ci_review_requires_explicit_ci_context_and_does_not_run_on_plan()
-> Result<(), Box<dyn std::error::Error>> {
    for args in [
        vec!["check", "--review-ci"],
        vec!["check", "--ci", "--review-ci", "--plan"],
        vec!["check", "--ci", "--review-ci", "--remote"],
        vec!["check", "--ci", "--review-ci", "--delivery"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_opdev"))
            .args(args)
            .output()?;
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("error:"));
    }
    Ok(())
}

#[test]
fn standard_script_checks_cover_maintained_sources_without_rewriting_fixtures()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new(if cfg!(windows) { "python" } else { "python3" })
        .arg(root.join("tests/style_tools_test.py"))
        .current_dir(root)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ci: Value = serde_saphyr::from_str(include_str!("../../../.gitlab-ci.yml"))?;
    let windows = ci["installer-windows"]["script"]
        .as_array()
        .ok_or("Windows script")?;
    for mode in ["format", "lint"] {
        assert!(windows.contains(&json!(format!(
            "powershell -NoProfile -File scripts/check_powershell.ps1 -Mode {mode}"
        ))));
    }
    assert!(
        ci["quality"]["before_script"]
            .as_array()
            .ok_or("quality setup")?
            .contains(&json!("python scripts/setup_style.py"))
    );
    Ok(())
}

#[test]
fn generic_linux_jobs_use_group_runners_and_platform_jobs_keep_overrides()
-> Result<(), Box<dyn std::error::Error>> {
    let ci: Value = serde_saphyr::from_str(include_str!("../../../.gitlab-ci.yml"))?;
    let linux = json!(["linux", "docker", "proxmox"]);
    assert_eq!(ci["default"]["tags"], linux);
    for name in [
        "quality",
        "plugin-validation",
        "installer-linux",
        "installer-arm64",
        "package-linux-x86_64",
        "qualify-dist",
        "fetch-cross-platform-packages",
        "release-evidence",
        "sign-release",
        "publish-github",
        "create-gitlab-release",
    ] {
        let tags = ci[name].get("tags").unwrap_or(&ci["default"]["tags"]);
        assert_eq!(tags, &linux, "{name}");
    }
    for name in ["installer-windows", "qualify-dist-windows"] {
        assert_eq!(ci[name]["tags"], json!(["windows", "powershell", "hyperv"]));
    }
    assert_eq!(
        ci["package-linux-aarch64"]["tags"],
        json!(["saas-linux-small-arm64"])
    );
    assert_eq!(
        ci["package-windows"]["tags"],
        json!(["saas-windows-medium-amd64"])
    );
    assert_eq!(ci["package-macos"]["tags"], json!(["saas-macos-medium-m1"]));
    Ok(())
}

#[test]
fn native_arm_handoff_remains_a_required_dependency() -> Result<(), Box<dyn std::error::Error>> {
    let ci: Value = serde_saphyr::from_str(include_str!("../../../.gitlab-ci.yml"))?;
    let github: Value = serde_saphyr::from_str(include_str!(
        "../../../.github/workflows/arm64-installer.yml"
    ))?;
    assert_eq!(
        github["jobs"]["installer-arm64"]["runs-on"],
        "ubuntu-24.04-arm"
    );
    assert_eq!(github["permissions"]["contents"], "read");
    assert_eq!(
        ci["installer-arm64"]["script"],
        json!(["python3 scripts/await_arm64.py --revision \"$CI_COMMIT_SHA\""])
    );
    assert_ne!(ci["installer-arm64"]["allow_failure"], true);
    assert!(
        ci["installer-linux"]["script"]
            .as_array()
            .ok_or("script")?
            .contains(&json!("test \"$(uname -m)\" = x86_64"))
    );
    assert_ne!(ci["installer-linux"]["allow_failure"], true);
    for name in [
        "package-linux-x86_64",
        "qualify-dist",
        "fetch-cross-platform-packages",
    ] {
        assert!(
            ci[name]["needs"]
                .as_array()
                .ok_or("needs")?
                .contains(&json!("installer-linux")),
            "{name}"
        );
        assert!(
            ci[name]["needs"]
                .as_array()
                .ok_or("needs")?
                .contains(&json!("installer-arm64")),
            "{name}"
        );
    }
    Ok(())
}
