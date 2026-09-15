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
fn installer_matrix_preserves_native_arm_and_remains_a_required_dependency()
-> Result<(), Box<dyn std::error::Error>> {
    let ci: Value = serde_saphyr::from_str(include_str!("../../../.gitlab-ci.yml"))?;
    assert_eq!(
        ci["installer-linux"]["parallel"]["matrix"],
        json!([
            {"INSTALLER_RUNNER": "proxmox", "INSTALLER_ARCH": "x86_64"},
            {"INSTALLER_RUNNER": "saas-linux-small-arm64", "INSTALLER_ARCH": "aarch64"}
        ])
    );
    assert_eq!(ci["installer-linux"]["tags"], json!(["$INSTALLER_RUNNER"]));
    assert!(
        ci["installer-linux"]["script"]
            .as_array()
            .ok_or("script")?
            .contains(&json!("test \"$(uname -m)\" = \"$INSTALLER_ARCH\""))
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
    }
    Ok(())
}
