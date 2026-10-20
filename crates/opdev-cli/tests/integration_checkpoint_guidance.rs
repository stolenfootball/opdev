//! Packaging regression only: this does not establish live-agent compliance.
use std::{fs, path::Path};

#[test]
fn all_work_entry_points_share_one_packaged_boundary_rule() -> Result<(), Box<dyn std::error::Error>>
{
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/opdev/skills/opdev");
    let temp = tempfile::tempdir()?;
    fs::create_dir(temp.path().join("references"))?;
    let routes = [
        "SKILL.md",
        "references/practical-workflow.md",
        "references/legacy-workflow.md",
        "references/planning.md",
        "references/testing.md",
        "references/workflow.md",
        "references/results.md",
        "references/ci-design.md",
    ];
    for relative in routes.into_iter().chain(["references/work-boundaries.md"]) {
        fs::copy(source.join(relative), temp.path().join(relative))?;
    }
    let expected = temp
        .path()
        .join("references/work-boundaries.md")
        .canonicalize()?;
    for relative in routes {
        let path = temp.path().join(relative);
        let text = fs::read_to_string(&path)?;
        let links: Vec<_> = text
            .split("](")
            .skip(1)
            .filter_map(|part| part.split_once(')').map(|(target, _)| target))
            .filter(|target| {
                target
                    .split('#')
                    .next()
                    .is_some_and(|file| file.ends_with("work-boundaries.md"))
            })
            .collect();
        assert!(!links.is_empty(), "missing shared rule from {relative}");
        for link in links {
            let (file, anchor) = link
                .split_once('#')
                .map_or((link, None), |(file, anchor)| (file, Some(anchor)));
            let resolved = path.parent().ok_or("parent")?.join(file).canonicalize()?;
            assert_eq!(resolved, expected);
            assert!(resolved.starts_with(temp.path().canonicalize()?));
            assert_eq!(
                fs::read(&resolved)?,
                fs::read(source.join("references/work-boundaries.md"))?
            );
            if let Some(anchor) = anchor {
                assert!(
                    fs::read_to_string(&resolved)?
                        .lines()
                        .filter_map(|line| line.strip_prefix("## "))
                        .any(|heading| heading.to_lowercase().replace(' ', "-") == anchor)
                );
            }
        }
    }
    Ok(())
}

#[test]
fn both_policy_routes_reach_packaged_checkpoint_guidance() -> Result<(), Box<dyn std::error::Error>>
{
    check_routes(
        "choose-an-integration-checkpoint",
        "### Choose an integration checkpoint",
    )
}

#[test]
fn both_policy_routes_reach_packaged_needs_discovery() -> Result<(), Box<dyn std::error::Error>> {
    check_routes(
        "understand-the-need-before-narrowing-the-design",
        "## Understand the need before narrowing the design",
    )
}

#[test]
fn both_policy_routes_reach_packaged_design_acceptance() -> Result<(), Box<dyn std::error::Error>> {
    check_routes(
        "agree-on-the-design-before-implementation",
        "## Agree on the design before implementation",
    )
}

#[test]
fn both_policy_routes_reach_packaged_deliverable_boundary() -> Result<(), Box<dyn std::error::Error>>
{
    check_routes(
        "deliver-the-requested-result-without-unrelated-qualification",
        "### Deliver the requested result without unrelated qualification",
    )
}

fn check_routes(anchor: &str, heading: &str) -> Result<(), Box<dyn std::error::Error>> {
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
            .filter_map(|target| target.strip_suffix(&format!("#{anchor}")))
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
                .any(|line| line == heading)
        );
    }
    Ok(())
}
