//! Keep benchmark accounting and acceptance tests in the canonical CI suite.

#[cfg(unix)]
#[test]
fn token_efficiency_benchmark_regressions() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::process::Command::new("python3")
        .arg(root.join("tests/benchmark_test.py"))
        .current_dir(&root)
        .output()?;
    assert!(
        output.status.success(),
        "benchmark regressions failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
