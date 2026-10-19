//! Packaging regression only: this does not establish live-agent compliance.
use std::{fs, path::Path};

#[test]
fn both_policy_routes_reach_packaged_checkpoint_guidance() -> Result<(), Box<dyn std::error::Error>>
{
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let skill = root.join("plugins/opdev/skills/opdev");
    let directory = tempfile::tempdir()?;
    let copied = directory.path().join("skills/opdev");
    fs::create_dir_all(copied.join("references"))?;
    for relative in [
        "SKILL.md",
        "references/legacy-workflow.md",
        "references/practical-workflow.md",
        "references/planning.md",
    ] {
        fs::copy(skill.join(relative), copied.join(relative))?;
    }
    let router = fs::read_to_string(copied.join("SKILL.md"))?;
    let planning = copied.join("references/planning.md").canonicalize()?;
    for route in ["legacy-workflow", "practical-workflow"] {
        let relative = format!("references/{route}.md");
        assert!(router.contains(&format!("]({relative})")));
        let source = copied.join(relative);
        let text = fs::read_to_string(&source)?;
        let links: Vec<_> = text
            .split("](")
            .skip(1)
            .filter_map(|part| part.split_once(')').map(|(target, _)| target))
            .filter_map(|target| target.strip_suffix("#choose-an-integration-checkpoint"))
            .collect();
        assert_eq!(links.len(), 1, "checkpoint route from {route}");
        let resolved = source
            .parent()
            .ok_or("parent")?
            .join(links[0])
            .canonicalize()?;
        assert_eq!(resolved, planning);
        assert!(resolved.starts_with(copied.canonicalize()?));
        assert_eq!(
            fs::read(&resolved)?,
            fs::read(skill.join("references/planning.md"))?
        );
        assert!(
            fs::read_to_string(resolved)?
                .lines()
                .any(|line| line == "### Choose an integration checkpoint")
        );
    }
    Ok(())
}
