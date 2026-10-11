//! Repository delivery controls, not a substitute for actual artifact qualification.
use serde_json::Value;
use std::{fs, path::Path, process::Command};

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn release_path(ci: &Value) -> std::result::Result<(), String> {
    let paths: &[(&str, &[&str])] = &[
        (
            "fetch-cross-platform-packages",
            &[
                "quality",
                "plugin-validation",
                "installer-linux",
                "installer-arm64",
                "installer-windows",
            ],
        ),
        (
            "qualify-dist",
            &[
                "quality",
                "plugin-validation",
                "installer-linux",
                "installer-arm64",
                "installer-windows",
            ],
        ),
        ("qualify-dist-windows", &["qualify-dist"]),
        (
            "release-evidence",
            &["fetch-cross-platform-packages", "qualify-dist-windows"],
        ),
        ("sign-release", &["release-evidence"]),
        ("publish-github", &["sign-release"]),
        ("create-gitlab-release", &["publish-github"]),
    ];
    for (job, required) in paths {
        let needs = ci[job]["needs"]
            .as_array()
            .ok_or_else(|| format!("{job}: missing needs"))?;
        for prerequisite in *required {
            let edge = needs
                .iter()
                .find(|edge| {
                    edge.as_str() == Some(prerequisite)
                        || edge["job"].as_str() == Some(prerequisite)
                })
                .ok_or_else(|| format!("{job}: missing {prerequisite}"))?;
            if edge.get("optional").is_some_and(|v| v != false) {
                return Err(format!("{job}: optional {prerequisite}"));
            }
            for name in [*job, *prerequisite] {
                let definition = &ci[name];
                if !definition.is_object()
                    || definition.get("allow_failure").is_some_and(|v| v != false)
                    || definition.get("when").is_some_and(|v| v != "on_success")
                {
                    return Err(format!("{name}: missing or nonblocking job"));
                }
                let rules = definition["rules"]
                    .as_array()
                    .ok_or_else(|| format!("{name}: missing rules"))?;
                if rules.iter().any(|rule| {
                    rule.get("allow_failure").is_some_and(|v| v != false)
                        || rule.get("when").is_some_and(|v| v != "on_success")
                }) {
                    return Err(format!("{name}: nonblocking rule"));
                }
            }
        }
    }
    for name in ["publish-github", "create-gitlab-release"] {
        let rules = ci[name]["rules"].as_array().ok_or("publisher rules")?;
        if rules.len() != 1
            || rules[0]["if"].as_str()
                != Some(
                    r#"$CI_COMMIT_REF_PROTECTED == "true" && $CI_COMMIT_TAG =~ /^v[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$/"#,
                )
        {
            return Err(format!(
                "{name}: publication must require a protected release tag"
            ));
        }
    }
    Ok(())
}

#[test]
fn publication_requires_the_complete_controlled_dependency_path() -> Result {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ci: Value = serde_saphyr::from_str(&fs::read_to_string(root.join(".gitlab-ci.yml"))?)?;
    release_path(&ci)?;
    // Discriminating mutations: graph reachability alone must not bless a bypass.
    for name in [
        "quality",
        "installer-windows",
        "qualify-dist-windows",
        "sign-release",
    ] {
        let mut changed = ci.clone();
        changed[name]["allow_failure"] = true.into();
        assert!(release_path(&changed).is_err(), "allowed failure: {name}");
    }
    let mut changed = ci.clone();
    changed["release-evidence"]["needs"] = serde_json::json!(["fetch-cross-platform-packages"]);
    assert!(
        release_path(&changed).is_err(),
        "omitted Windows qualification"
    );
    changed = ci.clone();
    changed["publish-github"]["needs"] =
        serde_json::json!([{"job":"sign-release","optional":true}]);
    assert!(release_path(&changed).is_err(), "optional signing");
    changed = ci.clone();
    changed["publish-github"]["rules"] = serde_json::json!([{"if":"$CI_COMMIT_TAG"}]);
    assert!(release_path(&changed).is_err(), "unprotected publisher");
    changed = ci.clone();
    changed["publish-github"]["rules"][0]["if"] = format!(
        "{} || true",
        ci["publish-github"]["rules"][0]["if"].as_str().unwrap()
    )
    .into();
    assert!(
        release_path(&changed).is_err(),
        "protected prefix with bypass"
    );
    Ok(())
}

#[test]
fn publication_recovery_preserves_uploaded_bytes_and_rejects_conflicts() -> Result {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Python is already a canonical prerequisite on Windows and Linux. Running
    // this portable suite here avoids the POSIX-only installer's selection gap.
    let output = Command::new("python")
        .arg(root.join("tests/dist_test.py"))
        .current_dir(root)
        .output()?;
    assert!(
        output.status.success(),
        "publication recovery failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
