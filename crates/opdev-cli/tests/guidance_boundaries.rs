//! Packaged routing regression, not proof of semantic agent compliance.
use std::{fs, path::Path};

#[test]
fn boundary_routes_survive_an_isolated_skill_copy() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("plugins/opdev/skills/opdev");
    let temp = tempfile::tempdir()?;
    fs::create_dir(temp.path().join("references"))?;
    for relative in [
        "SKILL.md",
        "references/testing.md",
        "references/workflow.md",
        "references/results.md",
    ] {
        fs::copy(source.join(relative), temp.path().join(relative))?;
    }
    let entry = fs::read_to_string(temp.path().join("SKILL.md"))?;
    for (file, anchor, heading) in [
        (
            "testing.md",
            "feedback-and-required-checks",
            "## Feedback and required checks",
        ),
        ("workflow.md", "merge-authority", "### Merge authority"),
        ("results.md", "completion-claims", "### Completion claims"),
    ] {
        let routes: Vec<_> = entry
            .split("](")
            .skip(1)
            .filter_map(|part| part.split_once(')').map(|(target, _)| target))
            .filter_map(|target| target.strip_suffix(&format!("#{anchor}")))
            .collect();
        assert_eq!(routes, [format!("references/{file}")]);
        let resolved = temp.path().join(routes[0]).canonicalize()?;
        assert!(resolved.starts_with(temp.path().canonicalize()?));
        assert_eq!(fs::read(&resolved)?, fs::read(source.join(routes[0]))?);
        assert!(
            fs::read_to_string(resolved)?
                .lines()
                .any(|line| line == heading)
        );
    }
    Ok(())
}
