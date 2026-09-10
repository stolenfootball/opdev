//! Execute offline bootstrap regression tests through the canonical workspace suite.

#[cfg(unix)]
#[test]
fn managed_runtime_bootstrap_regressions() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for suite in ["tests/runtime_test.py", "tests/dist_test.py"] {
        let output = std::process::Command::new("python3")
            .arg(root.join(suite))
            .current_dir(&root)
            .output()?;
        assert!(
            output.status.success(),
            "installer regressions failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn runtime_pin_is_compatible_and_covers_supported_platforms()
-> Result<(), Box<dyn std::error::Error>> {
    let lock = include_str!("../../../plugins/opdev/runtime.lock");
    let rows: Vec<Vec<&str>> = lock
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|line| line.split_whitespace().collect())
        .collect();
    let value = |key: &str| -> Result<&str, Box<dyn std::error::Error>> {
        let matching: Vec<_> = rows.iter().filter(|row| row[0] == key).collect();
        if matching.len() != 1 || matching[0].len() != 2 {
            return Err(format!("expected one scalar runtime lock field: {key}").into());
        }
        Ok(matching[0][1])
    };
    let version = semver::Version::parse(value("version")?)?;
    assert_eq!(value("tag")?, format!("v{version}"));
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../plugins/opdev/opdev-compatibility.json"
    ))?;
    let range = contract["requires"]["cli"]
        .as_str()
        .ok_or("missing CLI compatibility range")?;
    assert!(semver::VersionReq::parse(range)?.matches(&version));
    let mut targets = std::collections::BTreeSet::new();
    for row in rows.iter().filter(|row| row[0] == "target") {
        assert_eq!(row.len(), 6);
        assert!(targets.insert(row[3]), "duplicate runtime target");
        assert_eq!(row[5].len(), 64);
        assert!(row[5].bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
    assert_eq!(
        targets,
        [
            "x86_64-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin"
        ]
        .into_iter()
        .collect()
    );
    Ok(())
}
