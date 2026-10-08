//! Clean adoption checks declarations, exact retained files and absence, never deletes.
use opdev_project::clean_adoption::{CleanTarget, Retirement, retirement_gaps, validate_target};
use std::{fs, path::Path, process::Command};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn git(root: &Path, args: &[&str]) -> Result {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn target(path: &str, replacement: Option<&str>) -> CleanTarget {
    CleanTarget {
        version: 1,
        inventory_reference: "synthetic-inventory-review".into(),
        retirements: vec![Retirement {
            path: path.into(),
            reason: "Useful claims consolidated; original retained in Git history".into(),
            replacement: replacement.map(Into::into),
        }],
    }
}

#[test]
fn retirement_requires_absence_in_checkout_and_index_and_staged_replacement() -> Result {
    let repo = tempfile::tempdir()?;
    let root = repo.path();
    git(root, &["init", "-q"])?;
    let target = target("old.md", Some("design.md"));
    fs::write(root.join("old.md"), "old design")?;
    fs::write(root.join("design.md"), "retained design")?;
    git(root, &["add", "old.md"])?;
    assert_eq!(retirement_gaps(root, &target)?.len(), 2);
    fs::remove_file(root.join("old.md"))?;
    assert_eq!(
        retirement_gaps(root, &target)?.len(),
        2,
        "index still ships obsolete content"
    );
    git(root, &["add", "-A"])?;
    assert!(retirement_gaps(root, &target)?.is_empty());
    fs::write(root.join(".gitignore"), "old.md\n")?;
    fs::write(root.join("old.md"), "ignored stale active instruction")?;
    assert_eq!(retirement_gaps(root, &target)?.len(), 1);
    assert_eq!(
        fs::read_to_string(root.join("old.md"))?,
        "ignored stale active instruction",
        "inspection cannot clean automatically"
    );
    Ok(())
}

#[test]
fn unsafe_ambiguous_protected_and_overlapping_paths_are_rejected() {
    for path in [
        ".",
        "../old",
        "C:/old",
        "old\\file",
        ".git/config",
        "A/.GIT/config",
        "CON.md",
        "a/NUL",
        "LPT1.md",
        "old.",
        ".OPDEV",
        "AGENTS.md",
        ".opdev/project.yaml",
        ".opdev/adoption.yaml",
        ".opdev/guidance.md",
        "*.md",
        "a//b",
    ] {
        assert!(validate_target(&target(path, None)).is_err(), "{path}");
    }
    for replacement in ["old.md", "OLD.md", "old.md/child", "../new.md", "NUL"] {
        assert!(
            validate_target(&target("old.md", Some(replacement))).is_err(),
            "{replacement}"
        );
    }
    let mut duplicate = target("old.md", None);
    duplicate
        .retirements
        .push(target("OLD.md", None).retirements.remove(0));
    assert!(validate_target(&duplicate).is_err());
    let mut overlap = target("old", None);
    overlap
        .retirements
        .push(target("old/child.md", None).retirements.remove(0));
    assert!(validate_target(&overlap).is_err());
    let mut later_removed = target("old.md", Some("new.md"));
    later_removed
        .retirements
        .push(target("new.md", None).retirements.remove(0));
    assert!(validate_target(&later_removed).is_err());
}

#[test]
fn directories_and_untracked_replacements_do_not_establish_retained_content() -> Result {
    let repo = tempfile::tempdir()?;
    let root = repo.path();
    git(root, &["init", "-q"])?;
    fs::create_dir(root.join("design"))?;
    fs::write(root.join("design/one.md"), "retained")?;
    git(root, &["add", "."])?;
    assert_eq!(
        retirement_gaps(root, &target("old", Some("design")))?.len(),
        1
    );
    fs::write(root.join("new.md"), "untracked")?;
    assert_eq!(
        retirement_gaps(root, &target("old", Some("new.md")))?.len(),
        1
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn linked_retirement_or_replacement_never_passes() -> Result {
    let repo = tempfile::tempdir()?;
    let external = tempfile::tempdir()?;
    let root = repo.path();
    git(root, &["init", "-q"])?;
    std::os::unix::fs::symlink(external.path(), root.join("old"))?;
    assert!(retirement_gaps(root, &target("old/absent", None)).is_err());
    assert!(retirement_gaps(root, &target("absent", Some("old/absent"))).is_err());
    Ok(())
}
