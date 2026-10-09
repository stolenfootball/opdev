//! Bounded current-change reviews in existing MR/PR discussions, not an archive.
use opdev_project::{CiProvider, ReviewRecord, WorkKind, WorkSelector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Exact independently selected discussion body and source head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscussionLocator {
    /// Format 1; the kind distinguishes it from legacy Git archive locators.
    pub schema: u32,
    /// Must be `discussion_review`.
    pub kind: String,
    /// Existing MR/PR description or exact comment. Issues are not merge reviews.
    pub selector: WorkSelector,
    /// Full source-head commit at review; post-merge checks still bind actual staged bytes.
    pub source_commit: String,
    /// Complete current body identity; surrounding edits invalidate the selection too.
    pub body_sha256: String,
}

impl DiscussionLocator {
    /// # Errors
    /// Refuse ambiguous selections before credentials or requests.
    pub fn validate(&self) -> Result<(), String> {
        self.selector
            .validate()
            .map_err(|_| "Unsupported review selection")?;
        if self.schema != 1
            || self.kind != "discussion_review"
            || self.selector.kind != WorkKind::MergeRequest
            || !hex(&self.source_commit, 40)
            || !hex(&self.body_sha256, 64)
        {
            return Err("MR/PR review needs an exact item, full source commit and whole-body SHA-256; no provider request made".into());
        }
        Ok(())
    }
}

/// Sealed authenticated observation; not deserializable or convertible from local JSON.
pub struct DiscussionObservation {
    locator: DiscussionLocator,
    record: ReviewRecord,
    author_id: u64,
    created_at: String,
    updated_at: String,
    head: HeadSnapshot,
}

#[derive(PartialEq, Eq)]
struct HeadSnapshot {
    repository_name: String,
    merged: bool,
    merge_commit: Option<String>,
}

impl DiscussionObservation {
    /// Observed exact selection, not evidence of human consent.
    #[must_use]
    pub const fn locator(&self) -> &DiscussionLocator {
        &self.locator
    }
    /// Semantic input only; current execution remains required.
    #[must_use]
    pub const fn record(&self) -> &ReviewRecord {
        &self.record
    }
    /// Ensure discussion policy points to the project's own declared code repository.
    /// # Errors
    /// Reject another project, even if a locator and policy agree with each other.
    pub fn verify_project(&self, manifest: &opdev_project::ProjectManifest) -> Result<(), String> {
        let remote = manifest
            .project
            .ci
            .remote
            .as_deref()
            .ok_or("Project remote is missing")?;
        let repository =
            super::Repository::parse(remote).map_err(|_| "Project remote is unsupported")?;
        if repository.provider != self.locator.selector.provider
            || repository.slug() != self.head.repository_name
        {
            return Err("MR/PR review must belong to the project's declared code repository, not a separate evidence project".into());
        }
        Ok(())
    }
    /// Require the staged tree to be the observed source or actual integrated commit.
    /// # Errors
    /// Refuse unrelated local source, absent objects and unmerged post-merge claims.
    pub fn verify_source(
        &self,
        root: &std::path::Path,
        stage: opdev_project::TestStage,
    ) -> Result<(), String> {
        let post_merge = stage == opdev_project::TestStage::PostMerge;
        if post_merge && !self.head.merged {
            return Err("Post-merge review needs an actually merged MR/PR; a source-branch check is not integrated verification".into());
        }
        let commit = if post_merge {
            self.head
                .merge_commit
                .as_deref()
                .unwrap_or(&self.locator.source_commit)
        } else {
            &self.locator.source_commit
        };
        verify_commit_tree(root, commit)
    }
    /// Re-read mutable content and provenance before accepting a qualification.
    /// # Errors
    /// Changed, missing or unavailable data never falls back to saved observations.
    pub fn recheck(&self) -> Result<(), String> {
        let current = retrieve_discussion(&self.locator)?;
        if current.author_id != self.author_id
            || current.created_at != self.created_at
            || current.updated_at != self.updated_at
            || current.head != self.head
        {
            return Err(
                "MR/PR review attribution changed; inspect the current review before retrying"
                    .into(),
            );
        }
        Ok(())
    }
}

/// Authenticated fixed-origin GETs only. No publication, approval or cached fallback.
/// # Errors
/// Refuse unavailable, stale, wrong-item, wrong-head or malformed reviews.
pub fn retrieve_discussion(locator: &DiscussionLocator) -> Result<DiscussionObservation, String> {
    locator.validate()?;
    super::archive::authenticated_reads(locator.selector.provider, |get| {
        retrieve_with(locator, |url| get(url, false))
    })
}

fn retrieve_with(
    locator: &DiscussionLocator,
    mut get: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<DiscussionObservation, String> {
    locator.validate()?;
    let head = check_head(locator, &mut get)?;
    let value = super::work_observation::read_body_with(&locator.selector, &mut get)?;
    let github = locator.selector.provider == CiProvider::Github;
    let body = text(
        &value,
        if github || locator.selector.note_id.is_some() {
            "body"
        } else {
            "description"
        },
    )?;
    if format!("{:x}", Sha256::digest(body.as_bytes())) != locator.body_sha256 {
        return Err("MR/PR review changed since selection. Inspect its current conditions and tests; no earlier review substituted.".into());
    }
    let record = ReviewRecord::from_discussion_body(body).map_err(|_| "MR/PR review section is missing, ambiguous, oversized or malformed; no private content echoed")?;
    let author_id = value[if github { "user" } else { "author" }]["id"]
        .as_u64()
        .filter(|id| *id > 0)
        .ok_or("Review author identity unavailable")?;
    let observation = DiscussionObservation {
        locator: locator.clone(),
        record,
        author_id,
        head,
        created_at: text(&value, "created_at")?.into(),
        updated_at: text(&value, "updated_at")?.into(),
    };
    if check_head(locator, &mut get)? != observation.head {
        return Err("Review repository changed during retrieval".into());
    }
    Ok(observation)
}

fn check_head(
    locator: &DiscussionLocator,
    get: &mut impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<HeadSnapshot, String> {
    let selection = &locator.selector;
    let github = selection.provider == CiProvider::Github;
    let numeric = if github {
        format!(
            "https://api.github.com/repositories/{}",
            selection.repository_id
        )
    } else {
        format!(
            "https://gitlab.com/api/v4/projects/{}",
            selection.repository_id
        )
    };
    let project: Value = serde_json::from_slice(&get(&numeric)?)
        .map_err(|_| "Review repository metadata is malformed")?;
    if project["id"].as_u64() != Some(selection.repository_id) {
        return Err("Review repository identity differs".into());
    }
    let endpoint = if github {
        let slug = text(&project, "full_name")?;
        if slug.split('/').count() != 2
            || slug.split('/').any(|p| {
                p.is_empty()
                    || matches!(p, "." | "..")
                    || !p
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
        {
            return Err("Review repository name is unsupported".into());
        }
        format!(
            "https://api.github.com/repos/{slug}/pulls/{}",
            selection.number
        )
    } else {
        format!("{numeric}/merge_requests/{}", selection.number)
    };
    let item: Value = serde_json::from_slice(&get(&endpoint)?)
        .map_err(|_| "Review change metadata is malformed")?;
    let (number, repository, head) = if github {
        (
            &item["number"],
            &item["base"]["repo"]["id"],
            &item["head"]["sha"],
        )
    } else {
        (&item["iid"], &item["target_project_id"], &item["sha"])
    };
    if number.as_u64() != Some(selection.number)
        || repository.as_u64() != Some(selection.repository_id)
        || head.as_str() != Some(&locator.source_commit)
    {
        return Err("MR/PR source changed or belongs to another project. Review the current change before verification.".into());
    }
    let repository_name = text(
        &project,
        if github {
            "full_name"
        } else {
            "path_with_namespace"
        },
    )?
    .into();
    let merged = if github {
        item["merged"] == true
    } else {
        item["state"] == "merged"
    };
    let merge_commit = item
        .get("merge_commit_sha")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if merge_commit.as_ref().is_some_and(|sha| !hex(sha, 40)) {
        return Err("Integrated commit identity is unsupported".into());
    }
    Ok(HeadSnapshot {
        repository_name,
        merged,
        merge_commit,
    })
}

fn verify_commit_tree(root: &std::path::Path, commit: &str) -> Result<(), String> {
    let result = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "--no-replace-objects",
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--ignore-submodules=none",
            "--cached",
            "--quiet",
            commit,
            "--",
        ])
        .output()
        .map_err(|_| "Could not compare the reviewed commit with staged source")?;
    if !result.status.success() {
        return Err("Staged source differs from the MR/PR commit or that exact commit is unavailable locally. Fetch the identified commit through the normal workflow and review the actual change; no older source substituted.".into());
    }
    Ok(())
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Review content or provenance is unavailable".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn fixture(
        provider: CiProvider,
        note: Option<u64>,
    ) -> Result<(DiscussionLocator, Vec<(String, Value)>)> {
        // Deliberately incomplete semantic inventory: the reader authenticates storage,
        // not acceptance or execution. The engine must reject this as qualification.
        let record: ReviewRecord = serde_json::from_value(
            json!({"schema":1,"kind":"semantic_review",
            "source_sha256":"a".repeat(64),"configuration_sha256":"b".repeat(64),"stage":"pre_merge",
            "acceptance_sha256":"c".repeat(64),"ledger":{"schema":2,"project":[],"changes":[]}}),
        )?;
        let body = record.discussion_body()?;
        let locator = DiscussionLocator {
            schema: 1,
            kind: "discussion_review".into(),
            selector: WorkSelector {
                provider,
                repository_id: 7,
                kind: WorkKind::MergeRequest,
                number: 3,
                note_id: note,
            },
            source_commit: "d".repeat(40),
            body_sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
        };
        let github = provider == CiProvider::Github;
        let project =
            json!({"id":7,"full_name":"fixture/product","path_with_namespace":"fixture/product"});
        let numeric = if github {
            "https://api.github.com/repositories/7"
        } else {
            "https://gitlab.com/api/v4/projects/7"
        };
        let endpoint = if github {
            "https://api.github.com/repos/fixture/product"
        } else {
            numeric
        };
        let head_url = format!(
            "{endpoint}/{}/3",
            if github { "pulls" } else { "merge_requests" }
        );
        let head = json!({"number":3,"iid":3,"target_project_id":7,"base":{"repo":{"id":7}},"sha":locator.source_commit,"head":{"sha":locator.source_commit}});
        let parent_url = format!(
            "{endpoint}/{}/3",
            if github { "issues" } else { "merge_requests" }
        );
        let parent = json!({"number":3,"iid":3,"project_id":7,"pull_request":{},
            "body":body,"description":body,"user":{"id":9},"author":{"id":9},"created_at":"2026-10-08T01:00:00Z","updated_at":"2026-10-08T01:00:00Z"});
        let mut replies = vec![
            (numeric.into(), project.clone()),
            (head_url.clone(), head.clone()),
            (numeric.into(), project.clone()),
            (parent_url.clone(), parent.clone()),
        ];
        if let Some(id) = note {
            let mut comment = parent;
            comment["id"] = json!(id);
            comment["noteable_iid"] = json!(3);
            comment["issue_url"] = json!(parent_url);
            comment["system"] = json!(false);
            replies.push((
                if github {
                    format!("{endpoint}/issues/comments/{id}")
                } else {
                    format!("{parent_url}/notes/{id}")
                },
                comment,
            ));
        }
        replies.extend([
            (endpoint.into(), project.clone()),
            (numeric.into(), project),
            (head_url, head),
        ]);
        Ok((locator, replies))
    }
    fn read(
        locator: &DiscussionLocator,
        replies: Vec<(String, Value)>,
    ) -> std::result::Result<DiscussionObservation, String> {
        let mut replies = replies.into_iter();
        retrieve_with(locator, |url| {
            let (expected, value) = replies.next().ok_or("Unexpected extra request")?;
            assert_eq!(url, expected);
            serde_json::to_vec(&value).map_err(|_| "fixture".into())
        })
    }
    #[test]
    fn both_providers_bind_exact_description_or_note_and_code_repository() -> Result {
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            for note in [None, Some(11)] {
                let (locator, replies) = fixture(provider, note)?;
                let schema: Value = serde_json::from_str(include_str!(
                    "../../../schema/discussion-review-locator.schema.json"
                ))?;
                assert!(
                    jsonschema::validator_for(&schema)?.is_valid(&serde_json::to_value(&locator)?)
                );
                let observed = read(&locator, replies)?;
                assert_eq!(observed.locator(), &locator);
                assert_eq!(observed.author_id, 9);
                assert_eq!(observed.record().stage, opdev_project::TestStage::PreMerge);
                let temp = tempfile::tempdir()?;
                assert!(
                    std::process::Command::new("git")
                        .args(["init", "--quiet"])
                        .arg(temp.path())
                        .status()?
                        .success()
                );
                let mut manifest = opdev_project::discover(temp.path())?.manifest;
                manifest.project.ci.remote = Some(format!(
                    "https://{}.com/fixture/product",
                    if provider == CiProvider::Github {
                        "github"
                    } else {
                        "gitlab"
                    }
                ));
                observed.verify_project(&manifest)?;
                manifest.project.ci.remote = Some("https://gitlab.com/another/project".into());
                assert!(observed.verify_project(&manifest).is_err());
            }
        }
        Ok(())
    }
    #[test]
    fn edited_wrong_missing_and_concurrently_changed_reviews_fail_without_fallback() -> Result {
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            for fault in 0..10 {
                let (mut locator, mut replies) = fixture(provider, Some(11))?;
                let last = replies.len() - 1;
                match fault {
                    0 => locator.source_commit = "e".repeat(40),
                    1 => replies[0].1["id"] = json!(8),
                    2 => replies[4].1["body"] = json!("edited private text"),
                    3 => replies[4].1["id"] = json!(12),
                    4 => {
                        replies[4].1["noteable_iid"] = json!(4);
                        replies[4].1["issue_url"] = json!("wrong-parent");
                    }
                    5 => {
                        replies[last].1["sha"] = json!("e".repeat(40));
                        replies[last].1["head"]["sha"] = json!("e".repeat(40));
                    }
                    6 => replies[4].1["system"] = json!(true),
                    7 => {
                        replies[4].1["author"] = json!({});
                        replies[4].1["user"] = json!({});
                    }
                    8 => replies[4].1["updated_at"] = json!(null),
                    _ => {
                        locator.body_sha256 = "z".repeat(64);
                    }
                }
                let error = read(&locator, replies).err().ok_or("fault was accepted")?;
                assert!(!error.contains("private text"));
            }
            let (locator, _) = fixture(provider, None)?;
            assert!(retrieve_with(&locator, |_| Err("provider unavailable".into())).is_err());
        }
        Ok(())
    }
    #[test]
    fn qualification_requires_the_actual_provider_commit_tree_and_merged_boundary() -> Result {
        use opdev_project::TestStage;
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let git = |args: &[&str]| -> Result<String> {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()?;
            assert!(output.status.success());
            Ok(String::from_utf8(output.stdout)?.trim().into())
        };
        git(&["init", "-q"])?;
        git(&["config", "user.name", "Review fixture"])?;
        git(&["config", "user.email", "fixture@example.invalid"])?;
        std::fs::write(root.join("product"), "reviewed source")?;
        git(&["add", "."])?;
        git(&["commit", "-qm", "source"])?;
        let first = git(&["rev-parse", "HEAD"])?;
        let (mut locator, mut replies) = fixture(CiProvider::Gitlab, None)?;
        locator.source_commit.clone_from(&first);
        let last = replies.len() - 1;
        for index in [1, last] {
            replies[index].1["sha"] = json!(first);
        }
        let mut observed = read(&locator, replies)?;
        observed.verify_source(root, TestStage::PreMerge)?;
        assert!(
            observed.verify_source(root, TestStage::PostMerge).is_err(),
            "unmerged is not post-merge"
        );
        std::fs::write(root.join("product"), "integrated combination")?;
        git(&["add", "."])?;
        assert!(
            observed.verify_source(root, TestStage::PreMerge).is_err(),
            "same claimed review but different tree"
        );
        git(&["commit", "-qm", "integration"])?;
        let integrated = git(&["rev-parse", "HEAD"])?;
        observed.head.merged = true;
        observed.head.merge_commit = Some(integrated);
        observed.verify_source(root, TestStage::PostMerge)?;
        assert!(
            observed.verify_source(root, TestStage::PreMerge).is_err(),
            "integrated tree cannot replace reviewed source-head stage"
        );
        assert!(verify_commit_tree(root, &"f".repeat(40)).is_err());
        Ok(())
    }

    #[test]
    fn body_parser_refuses_duplicates_missing_records_reports_and_oversize() -> Result {
        let (locator, replies) = fixture(CiProvider::Gitlab, None)?;
        let observed = read(&locator, replies)?;
        let body = observed.record().discussion_body()?;
        assert_eq!(
            ReviewRecord::from_discussion_body(&body)?,
            *observed.record()
        );
        for invalid in [
            format!("{body}{body}"),
            body.replace("opdev-review:end", "missing"),
            "{\"passed\":true}".into(),
            "x".repeat(60 * 1024 + 1),
        ] {
            assert!(ReviewRecord::from_discussion_body(&invalid).is_err());
        }
        Ok(())
    }
}
