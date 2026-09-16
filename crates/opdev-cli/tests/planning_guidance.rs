//! Distribution and bootstrap invariants, not a semantic model benchmark.

use std::{fs, path::Path};

#[test]
fn planning_routes_are_local_and_resolve() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let skill = root.join("plugins/opdev/skills/opdev");
    let planning = skill.join("references/planning.md").canonicalize()?;
    for relative in [
        "SKILL.md",
        "references/workflow.md",
        "references/adoption.md",
    ] {
        let source = skill.join(relative);
        let text = fs::read_to_string(&source)?;
        let targets: Vec<_> = text
            .split("](")
            .skip(1)
            .filter_map(|part| part.split_once(')').map(|(target, _)| target))
            .filter(|target| target.ends_with("planning.md"))
            .collect();
        assert!(
            !targets.is_empty(),
            "missing planning route from {relative}"
        );
        for target in targets {
            let resolved = source
                .parent()
                .ok_or("parent")?
                .join(target)
                .canonicalize()?;
            assert_eq!(resolved, planning);
            assert!(resolved.starts_with(skill.canonicalize()?));
        }
    }
    Ok(())
}

#[test]
fn adoption_decision_review_routes_survive_plugin_copy() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let skill = root.join("plugins/opdev/skills/opdev");
    let directory = tempfile::tempdir()?;
    let copied = directory.path().join("skills/opdev");
    fs::create_dir_all(copied.join("references"))?;
    for relative in [
        "SKILL.md",
        "references/adoption.md",
        "references/decision-review.md",
    ] {
        fs::copy(skill.join(relative), copied.join(relative))?;
    }
    let expected = copied
        .join("references/decision-review.md")
        .canonicalize()?;
    for relative in ["SKILL.md", "references/adoption.md"] {
        let source = copied.join(relative);
        let text = fs::read_to_string(&source)?;
        let targets: Vec<_> = text
            .split("](")
            .skip(1)
            .filter_map(|part| part.split_once(')').map(|(target, _)| target))
            .filter(|target| target.ends_with("decision-review.md"))
            .collect();
        assert!(
            !targets.is_empty(),
            "missing decision review from {relative}"
        );
        for target in targets {
            assert_eq!(
                source
                    .parent()
                    .ok_or("parent")?
                    .join(target)
                    .canonicalize()?,
                expected
            );
        }
    }
    Ok(())
}

#[test]
fn fresh_and_upgraded_guidance_matches_repository_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let maintained = fs::read_to_string(root.join("AGENTS.md"))?.replace("\r\n", "\n");
    let start = "<!-- opdev:start -->";
    let end = "<!-- opdev:end -->";
    let expected = maintained
        .split_once(start)
        .ok_or("start")?
        .1
        .split_once(end)
        .ok_or("end")?
        .0;
    let directory = tempfile::tempdir()?;
    for original in [
        "project-owned guidance\n",
        "project-owned guidance\n<!-- opdev:start -->\nold\n<!-- opdev:end -->\n",
    ] {
        fs::write(directory.path().join("AGENTS.md"), original)?;
        opdev_project::reconcile_agent_files(directory.path())?;
        let generated =
            fs::read_to_string(directory.path().join("AGENTS.md"))?.replace("\r\n", "\n");
        assert!(generated.starts_with("project-owned guidance\n"));
        assert_eq!(
            generated
                .split_once(start)
                .ok_or("start")?
                .1
                .split_once(end)
                .ok_or("end")?
                .0,
            expected
        );
        let claude = fs::read_to_string(directory.path().join("CLAUDE.md"))?;
        assert_eq!(claude.matches("@AGENTS.md").count(), 1);
    }
    Ok(())
}
