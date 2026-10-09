//! Resolve a current MR/PR review using built-in CI identity and provider APIs.
//! This reads an explicit selection; it never creates reviews or consent.
use opdev_project::{CiProvider, ProjectManifest, TestStage, WorkKind, WorkSelector};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::DiscussionLocator;

const START: &str = "<!-- opdev-ci-selection:start -->";
const END: &str = "<!-- opdev-ci-selection:end -->";

#[derive(Clone, PartialEq, Eq)]
struct Context {
    provider: CiProvider,
    repository_id: u64,
    slug: String,
    trunk: String,
    revision: String,
    stage: TestStage,
    number: Option<u64>,
}

/// A provider-observed selection, not a passing review or authenticated approval.
pub struct CiReviewSelection {
    context: Context,
    locator: DiscussionLocator,
    acceptance: String,
    description_sha256: String,
}

impl CiReviewSelection {
    /// Exact independently selected discussion.
    #[must_use]
    pub const fn locator(&self) -> &DiscussionLocator {
        &self.locator
    }
    /// Independent acceptance identity selected by the MR/PR description.
    #[must_use]
    pub fn acceptance(&self) -> &str {
        &self.acceptance
    }
    /// Detect changed selection after checks; never silently select a replacement.
    /// # Errors
    /// Missing access or changed current input prevents qualification.
    pub fn recheck(&self) -> Result<(), String> {
        let current = resolve(&self.context)?;
        if current.locator != self.locator
            || current.acceptance != self.acceptance
            || current.description_sha256 != self.description_sha256
        {
            return Err("CI review selection changed during verification; inspect the current MR/PR before retrying".into());
        }
        Ok(())
    }
}

/// Discover only the actual CI change, under explicitly selected MR/PR storage.
/// # Errors
/// Refuse unsupported contexts before network access; no local-login export.
pub fn select_ci_review(
    manifest: &ProjectManifest,
    stage: TestStage,
    revision: &str,
) -> Result<CiReviewSelection, String> {
    let context = context_with(manifest, stage, revision, |key| std::env::var(key).ok())?;
    resolve(&context)
}

fn context_with(
    manifest: &ProjectManifest,
    stage: TestStage,
    revision: &str,
    mut env: impl FnMut(&str) -> Option<String>,
) -> Result<Context, String> {
    let policy = manifest
        .assurance
        .review_storage
        .as_ref()
        .filter(|p| p.version == 2)
        .ok_or("CI review selection requires the project's reviewed MR/PR storage policy")?;
    if !matches!(stage, TestStage::PreMerge | TestStage::PostMerge) || !hex(revision, 40) {
        return Err(
            "CI review selection requires an exact pre-merge or post-merge checkout".into(),
        );
    }
    let repository = crate::Repository::parse(
        manifest
            .project
            .ci
            .remote
            .as_deref()
            .ok_or("Project remote is missing")?,
    )
    .map_err(|_| "Project remote is unsupported")?;
    if repository.provider != policy.provider || manifest.project.ci.provider != policy.provider {
        return Err("CI review provider differs from the project contract".into());
    }
    let pre = stage == TestStage::PreMerge;
    let number = match policy.provider {
        CiProvider::Gitlab => {
            if env("GITLAB_CI").as_deref() != Some("true")
                || env("CI_SERVER_URL").as_deref() != Some("https://gitlab.com")
                || env("CI_PROJECT_ID").as_deref() != Some(&policy.repository_id.to_string())
                || env("CI_COMMIT_SHA").as_deref() != Some(revision)
                || env("CI_JOB_TOKEN").is_none_or(|v| v.trim().is_empty())
            {
                return Err("GitLab CI review needs this project's built-in job identity and exact checkout; no personal login substituted".into());
            }
            if pre {
                if env("CI_PIPELINE_SOURCE").as_deref() != Some("merge_request_event")
                    || env("CI_MERGE_REQUEST_EVENT_TYPE")
                        .as_deref()
                        .is_some_and(|v| v != "detached")
                {
                    return Err("Pre-merge CI review currently requires an MR source-head pipeline, not a synthetic merge checkout".into());
                }
                Some(number_value(env("CI_MERGE_REQUEST_IID").as_deref())?)
            } else {
                None
            }
        }
        CiProvider::Github => {
            if env("GITHUB_ACTIONS").as_deref() != Some("true")
                || env("GITHUB_SERVER_URL").as_deref() != Some("https://github.com")
                || env("GITHUB_REPOSITORY_ID").as_deref() != Some(&policy.repository_id.to_string())
                || env("GITHUB_REPOSITORY").as_deref() != Some(repository.slug().as_str())
                || env("GITHUB_TOKEN").is_none_or(|v| v.trim().is_empty())
            {
                return Err(
                    "GitHub CI review needs this repository's built-in workflow token and identity"
                        .into(),
                );
            }
            if pre {
                if env("GITHUB_EVENT_NAME").as_deref() != Some("pull_request") {
                    return Err("Pre-merge CI review requires a pull_request workflow; privileged target workflows are not supported".into());
                }
                let reference = env("GITHUB_REF").unwrap_or_default();
                Some(number_value(
                    reference
                        .strip_prefix("refs/pull/")
                        .and_then(|v| v.strip_suffix("/merge")),
                )?)
            } else {
                if env("GITHUB_SHA").as_deref() != Some(revision) {
                    return Err("CI checkout differs from the workflow revision".into());
                }
                None
            }
        }
        _ => return Err("CI review provider is unsupported".into()),
    };
    Ok(Context {
        provider: policy.provider,
        repository_id: policy.repository_id,
        slug: repository.slug(),
        trunk: manifest.project.trunk.clone(),
        revision: revision.into(),
        stage,
        number,
    })
}

fn resolve(context: &Context) -> Result<CiReviewSelection, String> {
    super::archive::authenticated_reads(context.provider, |get| {
        resolve_with(context, |url| get(url, false))
    })
}

fn resolve_with(
    context: &Context,
    mut get: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<CiReviewSelection, String> {
    let hub = context.provider == CiProvider::Github;
    let base = if hub {
        format!("https://api.github.com/repos/{}", context.slug)
    } else {
        format!(
            "https://gitlab.com/api/v4/projects/{}",
            context.repository_id
        )
    };
    if !hub {
        let job = read(&mut get, "https://gitlab.com/api/v4/job")?;
        if job["pipeline"]["project_id"].as_u64() != Some(context.repository_id)
            || job["commit"]["id"].as_str() != Some(&context.revision)
        {
            return Err("Authenticated CI job belongs to another project or commit".into());
        }
    }
    let route = if hub { "pulls" } else { "merge_requests" };
    let number = if let Some(number) = context.number {
        number
    } else {
        let url = if hub {
            format!("{base}/commits/{}/{route}?per_page=100", context.revision)
        } else {
            format!(
                "{base}/repository/commits/{}/{route}?per_page=100",
                context.revision
            )
        };
        let values = read(&mut get, &url)?;
        let items = values
            .as_array()
            .filter(|a| a.len() < 100)
            .ok_or("CI change discovery is malformed or exceeds its bound; no partial selection")?;
        let matches: Vec<_> = items
            .iter()
            .filter(|v| {
                v["merge_commit_sha"].as_str() == Some(&context.revision)
                    && if hub {
                        !v["merged_at"].is_null()
                    } else {
                        v["state"] == "merged"
                    }
            })
            .collect();
        if matches.len() != 1 {
            return Err(
                "Integrated source needs exactly one merged MR/PR; no latest-success fallback"
                    .into(),
            );
        }
        matches[0][if hub { "number" } else { "iid" }]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("Merged change identity unavailable")?
    };
    let item = read(&mut get, &format!("{base}/{route}/{number}"))?;
    select(context, number, &item)
}

fn read(get: &mut impl FnMut(&str) -> Result<Vec<u8>, String>, url: &str) -> Result<Value, String> {
    serde_json::from_slice(&get(url)?).map_err(|_| "CI provider metadata is malformed".into())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    schema: u32,
    stages: Stages,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stages {
    pre_merge: Option<Choice>,
    post_merge: Option<Choice>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Choice {
    revision: String,
    note_id: u64,
    body_sha256: String,
    acceptance_sha256: String,
}

fn select(context: &Context, number: u64, item: &Value) -> Result<CiReviewSelection, String> {
    let hub = context.provider == CiProvider::Github;
    let (id, target, source, head, branch, merged, description) = if hub {
        (
            &item["number"],
            &item["base"]["repo"]["id"],
            &item["head"]["repo"]["id"],
            &item["head"]["sha"],
            &item["base"]["ref"],
            item["merged"] == true,
            &item["body"],
        )
    } else {
        (
            &item["iid"],
            &item["target_project_id"],
            &item["source_project_id"],
            &item["sha"],
            &item["target_branch"],
            item["state"] == "merged",
            &item["description"],
        )
    };
    if id.as_u64() != Some(number)
        || target.as_u64() != Some(context.repository_id)
        || source.as_u64() != Some(context.repository_id)
        || branch.as_str() != Some(&context.trunk)
        || (!hub && super::discussion_review::gitlab_mr_repository(item, number)? != context.slug)
    {
        return Err("CI review must belong to this repository and its declared trunk; fork execution needs a separately reviewed path".into());
    }
    let head = head
        .as_str()
        .filter(|v| hex(v, 40))
        .ok_or("CI change source identity is unavailable")?;
    let pre = context.stage == TestStage::PreMerge;
    let actual = if pre {
        Some(head)
    } else {
        item["merge_commit_sha"].as_str().filter(|_| merged)
    };
    if actual != Some(&context.revision) {
        return Err("CI review does not describe this exact source and verification stage".into());
    }
    let body = description
        .as_str()
        .ok_or("MR/PR review selection is missing")?;
    if body.len() > 1024 * 1024
        || body.matches(START).count() != 1
        || body.matches(END).count() != 1
    {
        return Err("MR/PR needs exactly one explicit CI review selection".into());
    }
    let section = body
        .split_once(START)
        .and_then(|(_, tail)| tail.split_once(END))
        .map(|(section, _)| section)
        .ok_or("CI review selection markers are out of order")?;
    let data: Selection = serde_json::from_str(section)
        .map_err(|_| "CI review selection is malformed; no private content echoed")?;
    let choice = if pre {
        data.stages.pre_merge
    } else {
        data.stages.post_merge
    }
    .ok_or("Select a reviewed comment for this verification stage before running CI")?;
    if data.schema != 1
        || choice.revision != context.revision
        || choice.note_id == 0
        || !hex(&choice.body_sha256, 64)
        || !hex(&choice.acceptance_sha256, 64)
    {
        return Err(
            "CI review needs the exact revision, comment and review identities for this stage"
                .into(),
        );
    }
    let locator = DiscussionLocator {
        schema: 1,
        kind: "discussion_review".into(),
        selector: WorkSelector {
            provider: context.provider,
            repository_id: context.repository_id,
            kind: WorkKind::MergeRequest,
            number,
            note_id: Some(choice.note_id),
        },
        source_commit: head.into(),
        body_sha256: choice.body_sha256,
    };
    locator.validate()?;
    Ok(CiReviewSelection {
        context: context.clone(),
        locator,
        acceptance: choice.acceptance_sha256,
        description_sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
    })
}

fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn number_value(value: Option<&str>) -> Result<u64, String> {
    value
        .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .ok_or("CI change number is missing or invalid".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn fixture(provider: CiProvider, stage: TestStage) -> (Context, Value) {
        let context = Context {
            provider,
            repository_id: 7,
            slug: "fixture/product".into(),
            trunk: "main".into(),
            revision: "a".repeat(40),
            stage,
            number: (stage == TestStage::PreMerge).then_some(3),
        };
        let key = if stage == TestStage::PreMerge {
            "pre_merge"
        } else {
            "post_merge"
        };
        let selection = json!({"schema":1,"stages":{key:{"revision":context.revision,"note_id":11,
            "body_sha256":"b".repeat(64),"acceptance_sha256":"c".repeat(64)}}});
        let body = format!("{START}\n{selection}\n{END}");
        let head = if stage == TestStage::PreMerge {
            context.revision.clone()
        } else {
            "d".repeat(40)
        };
        let item = json!({"iid":3,"number":3,"target_project_id":7,"source_project_id":7,
            "web_url":"https://gitlab.com/fixture/product/-/merge_requests/3",
            "target_branch":"main","sha":head,"state":"merged","merged":true,"merged_at":"2026-10-09T00:00:00Z",
            "merge_commit_sha":context.revision,"description":body,"body":body,
            "head":{"sha":head,"repo":{"id":7}},"base":{"ref":"main","repo":{"id":7}}});
        (context, item)
    }

    #[test]
    fn both_providers_resolve_exact_stages_without_general_gitlab_project_reads() -> Result {
        for provider in [CiProvider::Gitlab, CiProvider::Github] {
            for stage in [TestStage::PreMerge, TestStage::PostMerge] {
                let (context, item) = fixture(provider, stage);
                let mut urls = Vec::new();
                let result = resolve_with(&context, |url| {
                    urls.push(url.to_owned());
                    let value = if url.ends_with("/job") {
                        json!({"pipeline":{"project_id":7},"commit":{"id":context.revision}})
                    } else if url.contains("?per_page") {
                        json!([item])
                    } else if url.ends_with("/3") {
                        item.clone()
                    } else {
                        return Err("unexpected API endpoint".into());
                    };
                    serde_json::to_vec(&value).map_err(|_| "fixture".into())
                })?;
                assert_eq!(result.locator.selector.note_id, Some(11));
                assert_eq!(result.acceptance, "c".repeat(64));
                assert_eq!(
                    result.locator.source_commit,
                    if stage == TestStage::PreMerge {
                        "a".repeat(40)
                    } else {
                        "d".repeat(40)
                    }
                );
                assert!(!urls.iter().any(|url| url.ends_with("/projects/7")));
            }
        }
        Ok(())
    }

    #[test]
    fn wrong_project_fork_branch_revision_stage_and_mutable_selection_never_pass() -> Result {
        for provider in [CiProvider::Gitlab, CiProvider::Github] {
            for fault in 0..12 {
                let (mut context, mut item) = fixture(provider, TestStage::PreMerge);
                match fault {
                    0 => {
                        item["target_project_id"] = json!(8);
                        item["base"]["repo"]["id"] = json!(8);
                    }
                    1 => {
                        item["source_project_id"] = json!(8);
                        item["head"]["repo"]["id"] = json!(8);
                    }
                    2 => {
                        item["target_branch"] = json!("other");
                        item["base"]["ref"] = json!("other");
                    }
                    3 => {
                        item["sha"] = json!("e".repeat(40));
                        item["head"]["sha"] = json!("e".repeat(40));
                    }
                    4 => context.stage = TestStage::PostMerge,
                    _ => {
                        let original = item["body"].as_str().ok_or("body")?;
                        let body = match fault {
                            5 => format!("{original}{original}"),
                            6 => original.replace("\"note_id\":11", "\"note_id\":0"),
                            7 => original.replace("\"schema\":1", "\"schema\":1,\"schema\":1"),
                            8 => original.replace(&"b".repeat(64), "bad-hash"),
                            9 => original.replace("\"schema\":1", "\"schema\":99"),
                            10 => original.replace(&"a".repeat(40), &"f".repeat(40)),
                            _ => "private text without selection".into(),
                        };
                        item["body"] = json!(body);
                        item["description"] = json!(body);
                    }
                }
                let error = select(&context, 3, &item)
                    .err()
                    .ok_or(format!("fault {fault} accepted"))?;
                assert!(!error.contains("private text"));
            }
        }
        Ok(())
    }

    #[test]
    fn missing_ambiguous_truncated_discovery_and_wrong_authenticated_job_fail() {
        let (context, item) = fixture(CiProvider::Gitlab, TestStage::PostMerge);
        for fault in 0..5 {
            assert!(resolve_with(&context, |url| {
                if fault == 4 { return Err("access denied".into()); }
                let value = if url.ends_with("/job") {
                    json!({"pipeline":{"project_id":if fault == 3 {8} else {7}},"commit":{"id":context.revision}})
                } else { match fault {
                    0 => json!([]), 1 => json!([item, item]), _ => json!(vec![item.clone(); 100]),
                }};
                serde_json::to_vec(&value).map_err(|_| "fixture".into())
            }).is_err(), "fault {fault}");
        }
    }

    #[test]
    fn changed_selection_changes_snapshot_even_when_comment_identity_is_unchanged() -> Result {
        let (context, mut item) = fixture(CiProvider::Gitlab, TestStage::PreMerge);
        let before = select(&context, 3, &item)?;
        item["description"] = json!(format!(
            "Changed scope\n{}",
            item["description"].as_str().ok_or("description")?
        ));
        let after = select(&context, 3, &item)?;
        assert_eq!(before.locator, after.locator);
        assert_ne!(before.description_sha256, after.description_sha256);
        Ok(())
    }

    #[test]
    fn context_requires_job_identity_before_network_and_rejects_synthetic_merge_jobs() -> Result {
        let root = tempfile::tempdir()?;
        std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(root.path())
            .status()?;
        let mut manifest = opdev_project::discover(root.path())?.manifest;
        manifest.project.ci.provider = CiProvider::Gitlab;
        manifest.project.ci.remote = Some("https://gitlab.com/fixture/product".into());
        manifest.assurance.review_storage = Some(serde_json::from_value(
            json!({"version":2,"provider":"gitlab","repository_id":7,"review_reference":"fixture","retention_authority":"contracts","report_retention_days":30}),
        )?);
        let values = std::collections::BTreeMap::from([
            ("GITLAB_CI", "true".into()),
            ("CI_SERVER_URL", "https://gitlab.com".into()),
            ("CI_PROJECT_ID", "7".into()),
            ("CI_COMMIT_SHA", "a".repeat(40)),
            ("CI_JOB_TOKEN", "fixture".into()),
            ("CI_PIPELINE_SOURCE", "merge_request_event".into()),
            ("CI_MERGE_REQUEST_IID", "3".into()),
        ]);
        let stage = TestStage::PreMerge;
        assert!(
            context_with(&manifest, stage, &"a".repeat(40), |k| values
                .get(k)
                .cloned())
            .is_ok()
        );
        for key in [
            "CI_JOB_TOKEN",
            "CI_PROJECT_ID",
            "GITLAB_CI",
            "CI_COMMIT_SHA",
        ] {
            assert!(
                context_with(&manifest, stage, &"a".repeat(40), |k| if k == key {
                    None
                } else {
                    values.get(k).cloned()
                })
                .is_err()
            );
        }
        assert!(
            context_with(&manifest, stage, &"a".repeat(40), |k| {
                if k == "CI_MERGE_REQUEST_EVENT_TYPE" {
                    Some("merged_result".into())
                } else {
                    values.get(k).cloned()
                }
            })
            .is_err()
        );
        Ok(())
    }
}
