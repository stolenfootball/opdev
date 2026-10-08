//! Authenticated, minimal observations of explicitly selected mutable work content.
use opdev_project::{CiProvider, WorkKind, WorkObservation, WorkSelector};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Observe one exact excerpt without treating authorship as developer approval.
/// Uses fixed-origin authenticated GETs, bounded metadata and no cached fallback.
/// # Errors
/// Refuse missing/changed identities, unavailable content and absent excerpts.
pub fn observe_work(selector: &WorkSelector, excerpt: &str) -> Result<WorkObservation, String> {
    selector
        .validate()
        .map_err(|_| "Unsupported work selection")?;
    if excerpt.trim().is_empty() || excerpt.len() > 16 * 1024 {
        return Err("Select a nonempty exact work excerpt of at most 16 KiB".into());
    }
    super::archive::authenticated_reads(selector.provider, |get| {
        observe_with(selector, excerpt, |url| get(url, false))
    })
}

/// Re-observe original authorities; unavailable/current text cannot fall back to an old body.
/// Each provider group shares the existing one-minute network budget.
/// # Errors
/// Missing, edited or reattributed body versions remain unresolved.
pub fn recheck_work(observations: &[WorkObservation]) -> Result<(), String> {
    if observations.len() > 64 {
        return Err("Too many work observations".into());
    }
    for observation in observations {
        observation
            .validate()
            .map_err(|_| "Unsupported retained work observation")?;
    }
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        let selected: Vec<_> = observations
            .iter()
            .filter(|o| o.selector.provider == provider)
            .collect();
        if selected.is_empty() {
            continue;
        }
        super::archive::authenticated_reads(provider, |get| {
            for original in selected {
                let current =
                    observe_with(&original.selector, &original.excerpt, |url| get(url, false))?;
                if current.body_sha256 != original.body_sha256
                    || current.author_id != original.author_id
                    || current.created_at != original.created_at
                    || current.updated_at != original.updated_at
                {
                    return Err("Work authority changed since the retained review. Reassess the current scope and decisions; no earlier observation substituted".into());
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn observe_with(
    selector: &WorkSelector,
    excerpt: &str,
    mut get: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<WorkObservation, String> {
    let json = |bytes: Vec<u8>| {
        serde_json::from_slice::<Value>(&bytes)
            .map_err(|_| "Work provider metadata is unsupported".to_owned())
    };
    let numeric = if selector.provider == CiProvider::Github {
        format!(
            "https://api.github.com/repositories/{}",
            selector.repository_id
        )
    } else {
        format!(
            "https://gitlab.com/api/v4/projects/{}",
            selector.repository_id
        )
    };
    let project = json(get(&numeric)?)?;
    if project["id"].as_u64() != Some(selector.repository_id) {
        return Err("Work repository identity changed".into());
    }
    let endpoint = if selector.provider == CiProvider::Github {
        let slug = text(&project, "full_name")?;
        let parts: Vec<_> = slug.split('/').collect();
        if parts.len() != 2
            || parts.iter().any(|p| {
                p.is_empty()
                    || matches!(*p, "." | "..")
                    || !p
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
        {
            return Err("Work repository name is unsupported".into());
        }
        format!("https://api.github.com/repos/{slug}")
    } else {
        numeric.clone()
    };
    let route = if selector.provider == CiProvider::Github || selector.kind == WorkKind::Issue {
        "issues"
    } else {
        "merge_requests"
    };
    let parent_url = format!("{endpoint}/{route}/{}", selector.number);
    let parent = json(get(&parent_url)?)?;
    if selector.provider == CiProvider::Github {
        if parent["number"].as_u64() != Some(selector.number)
            || (selector.kind == WorkKind::MergeRequest) != parent.get("pull_request").is_some()
        {
            return Err("Work item number or kind differs".into());
        }
    } else if parent["iid"].as_u64() != Some(selector.number)
        || parent["project_id"].as_u64() != Some(selector.repository_id)
    {
        return Err("Work item identity differs".into());
    }
    let value = if let Some(note) = selector.note_id {
        let url = if selector.provider == CiProvider::Github {
            format!("{endpoint}/issues/comments/{note}")
        } else {
            format!("{parent_url}/notes/{note}")
        };
        let note_value = json(get(&url)?)?;
        if note_value["id"].as_u64() != Some(note)
            || (selector.provider == CiProvider::Github
                && note_value["issue_url"].as_str() != Some(parent_url.as_str()))
            || (selector.provider == CiProvider::Gitlab
                && note_value["noteable_iid"].as_u64() != Some(selector.number))
            || note_value["system"] == true
        {
            return Err(
                "Work note identity differs or is a system event, not selected author text".into(),
            );
        }
        note_value
    } else {
        parent
    };
    let record = record(selector, excerpt, &value)?;
    if json(get(&endpoint)?)?["id"].as_u64() != Some(selector.repository_id) {
        return Err("Work repository changed during observation".into());
    }
    Ok(record)
}

fn record(
    selector: &WorkSelector,
    excerpt: &str,
    value: &Value,
) -> Result<WorkObservation, String> {
    let body_key = if selector.provider == CiProvider::Github || selector.note_id.is_some() {
        "body"
    } else {
        "description"
    };
    let body = text(value, body_key)?;
    if !body.contains(excerpt) {
        return Err("Selected excerpt is absent from the current work content; no earlier version substituted".into());
    }
    let author = if selector.provider == CiProvider::Github {
        "user"
    } else {
        "author"
    };
    let record = WorkObservation {
        schema: 1,
        selector: selector.clone(),
        author_id: value[author]["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or("Work author identity unavailable")?,
        created_at: text(value, "created_at")?.into(),
        updated_at: text(value, "updated_at")?.into(),
        body_sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
        excerpt: excerpt.into(),
        observed_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Observation clock unavailable")?
            .as_secs(),
    };
    record
        .validate()
        .map_err(|_| "Work observation provenance is incomplete")?;
    Ok(record)
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value[field]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Work content or provenance is missing; no private body echoed".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(
        provider: CiProvider,
        kind: WorkKind,
        note_id: Option<u64>,
    ) -> (WorkSelector, Vec<(String, Value)>) {
        let selection = WorkSelector {
            provider,
            repository_id: 7,
            kind,
            number: 3,
            note_id,
        };
        let project = json!({"id":7, "full_name":"fixture/archive"});
        let mut parent = json!({"number":3,"iid":3,"project_id":7,"body":"Decision: retain the interface. Private context omitted.",
            "description":"Decision: retain the interface. Private context omitted.","user":{"id":9},"author":{"id":9},
            "created_at":"2026-10-01T00:00:00Z","updated_at":"2026-10-02T00:00:00Z"});
        if provider == CiProvider::Github && kind == WorkKind::MergeRequest {
            parent["pull_request"] = json!({});
        }
        let numeric = if provider == CiProvider::Github {
            "https://api.github.com/repositories/7"
        } else {
            "https://gitlab.com/api/v4/projects/7"
        };
        let endpoint = if provider == CiProvider::Github {
            "https://api.github.com/repos/fixture/archive"
        } else {
            numeric
        };
        let route = if provider == CiProvider::Github || kind == WorkKind::Issue {
            "issues"
        } else {
            "merge_requests"
        };
        let parent_url = format!("{endpoint}/{route}/3");
        let mut replies = vec![
            (numeric.into(), project.clone()),
            (parent_url.clone(), parent),
        ];
        if let Some(note) = note_id {
            let url = if provider == CiProvider::Github {
                format!("{endpoint}/issues/comments/{note}")
            } else {
                format!("{parent_url}/notes/{note}")
            };
            replies.push((url, json!({"id":note,"noteable_iid":3,"system":false,"issue_url":parent_url,
                "body":"Decision: retain the interface. Private context omitted.","user":{"id":9},"author":{"id":9},
                "created_at":"2026-10-01T00:00:00Z","updated_at":"2026-10-02T00:00:00Z"})));
        }
        replies.push((endpoint.into(), project));
        (selection, replies)
    }

    fn run_fixture(
        selection: &WorkSelector,
        replies: Vec<(String, Value)>,
        excerpt: &str,
    ) -> Result<WorkObservation, String> {
        let mut replies = replies.into_iter();
        observe_with(selection, excerpt, |url| {
            let (expected, value) = replies.next().ok_or("Unexpected additional request")?;
            assert_eq!(url, expected);
            serde_json::to_vec(&value).map_err(|_| "Fixture serialization failed".into())
        })
    }

    #[test]
    fn both_providers_capture_only_selected_excerpt_with_original_body_identity()
    -> Result<(), Box<dyn std::error::Error>> {
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            for kind in [WorkKind::Issue, WorkKind::MergeRequest] {
                for note in [None, Some(11)] {
                    let (selection, replies) = fixture(provider, kind, note);
                    let record = run_fixture(&selection, replies, "retain the interface")?;
                    assert_eq!(record.selector, selection);
                    assert_eq!(record.author_id, 9);
                    assert_eq!(
                        record.body_sha256,
                        format!(
                            "{:x}",
                            Sha256::digest(
                                b"Decision: retain the interface. Private context omitted."
                            )
                        )
                    );
                    let value = serde_json::to_value(&record)?;
                    let schema: Value = serde_json::from_str(include_str!(
                        "../../../schema/work-observation.schema.json"
                    ))?;
                    assert!(jsonschema::validator_for(&schema)?.is_valid(&value));
                    let bytes = serde_json::to_string(&record)?;
                    assert!(!bytes.contains("Private context omitted"));
                    assert!(!bytes.contains("approval"));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn wrong_item_note_repo_kind_missing_excerpt_and_changed_slug_do_not_fall_back()
    -> Result<(), Box<dyn std::error::Error>> {
        for provider in [CiProvider::Github, CiProvider::Gitlab] {
            for fault in 0..7 {
                let (selection, mut replies) = fixture(provider, WorkKind::Issue, Some(11));
                match fault {
                    0 => replies[0].1["id"] = json!(8),
                    1 => {
                        replies[1].1["number"] = json!(4);
                        replies[1].1["iid"] = json!(4);
                    }
                    2 => replies[2].1["id"] = json!(12),
                    3 => {
                        replies[2].1["issue_url"] =
                            json!("https://api.github.com/repos/fixture/archive/issues/99");
                        replies[2].1["noteable_iid"] = json!(99);
                    }
                    4 => {
                        replies[2].1["body"] =
                            json!("Changed to reject this decision; private context");
                    }
                    5 => replies[3].1["id"] = json!(8),
                    _ => replies[2].1["system"] = json!(true),
                }
                let error = run_fixture(&selection, replies, "retain the interface")
                    .err()
                    .ok_or("fault must be detected")?;
                assert!(!error.contains("private context"));
            }
        }
        let (selection, mut replies) = fixture(CiProvider::Github, WorkKind::MergeRequest, None);
        replies[1]
            .1
            .as_object_mut()
            .ok_or("object")?
            .remove("pull_request");
        assert!(run_fixture(&selection, replies, "retain the interface").is_err());
        assert!(
            observe_with(
                &selection,
                "excerpt",
                |_| Err("provider unavailable".into())
            )
            .is_err()
        );
        Ok(())
    }
}
