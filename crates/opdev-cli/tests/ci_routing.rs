//! CI routing must preserve platform-specific qualification requirements.

use serde_json::{Value, json};

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
