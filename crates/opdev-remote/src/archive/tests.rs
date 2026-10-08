use std::collections::BTreeMap;

use serde_json::json;

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Fixture {
    locator: ArchiveLocator,
    responses: BTreeMap<String, Vec<u8>>,
    project: String,
    commit: String,
    tree: String,
    child: String,
    raw: String,
    recheck: String,
}

impl Fixture {
    fn new(provider: CiProvider) -> Result<Self, Box<dyn std::error::Error>> {
        let locator = ArchiveLocator {
            schema: 1,
            provider,
            repository_id: 42,
            commit: "a".repeat(40),
            path: "attempts/001.json".into(),
            sha256: format!("{:x}", Sha256::digest(b"failed attempt bytes")),
        };
        let (project, commit, tree, child, raw, recheck) = if provider == CiProvider::Github {
            let base = "https://api.github.com/repos/test/archive";
            (
                "https://api.github.com/repositories/42".into(),
                format!("{base}/git/commits/{}", locator.commit),
                format!("{base}/git/trees/{}", "b".repeat(40)),
                format!("{base}/git/trees/{}", "c".repeat(40)),
                format!("{base}/git/blobs/{}", "d".repeat(40)),
                base.into(),
            )
        } else {
            let base = "https://gitlab.com/api/v4/projects/42";
            (
                base.into(),
                format!("{base}/repository/commits/{}", locator.commit),
                format!(
                    "{base}/repository/tree?ref={}&path=&per_page=100&page=1",
                    locator.commit
                ),
                format!(
                    "{base}/repository/tree?ref={}&path=attempts&per_page=100&page=1",
                    locator.commit
                ),
                format!("{base}/repository/blobs/{}/raw", "d".repeat(40)),
                base.into(),
            )
        };
        let mut result = Self {
            locator,
            responses: BTreeMap::new(),
            project,
            commit,
            tree,
            child,
            raw,
            recheck,
        };
        for key in [result.project.clone(), result.recheck.clone()] {
            result.set(&key, &json!({"id":42,"full_name":"test/archive"}))?;
        }
        result.set(
            &result.commit.clone(),
            &json!({"id":"a".repeat(40),"sha":"a".repeat(40),"tree":{"sha":"b".repeat(40)}}),
        )?;
        result.set_tree(false, "040000", "tree")?;
        result.set_tree(true, "100644", "blob")?;
        result
            .responses
            .insert(result.raw.clone(), b"failed attempt bytes".to_vec());
        Ok(result)
    }
    fn set(&mut self, url: &str, value: &Value) -> TestResult {
        self.responses
            .insert(url.into(), serde_json::to_vec(value)?);
        Ok(())
    }
    fn set_tree(&mut self, leaf: bool, mode: &str, kind: &str) -> TestResult {
        let github = self.locator.provider == CiProvider::Github;
        let name = if leaf { "001.json" } else { "attempts" };
        let path = if leaf && !github {
            "attempts/001.json"
        } else {
            name
        };
        let id = if leaf { "d" } else { "c" }.repeat(40);
        let entry = json!({"path":path,"name":name,"mode":mode,"type":kind,"sha":id,"id":id});
        let value = if github {
            json!({"sha":if leaf {"c"} else {"b"}.repeat(40),"truncated":false,"tree":[entry]})
        } else {
            json!([entry])
        };
        self.set(
            &if leaf {
                self.child.clone()
            } else {
                self.tree.clone()
            },
            &value,
        )
    }
    fn retrieve(&self) -> Result<ArchiveObservation, String> {
        retrieve_with(&self.locator, |url, raw| {
            if raw != (url == self.raw) {
                return Err("Unexpected raw/metadata request".into());
            }
            self.responses
                .get(url)
                .cloned()
                .ok_or_else(|| "Missing fixture endpoint; no fallback allowed".into())
        })
    }
}

#[test]
fn both_providers_bind_repository_commit_tree_regular_file_and_exact_failed_bytes() -> TestResult {
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        let f = Fixture::new(provider)?;
        let result = f.retrieve()?;
        assert_eq!(result.locator(), &f.locator);
        assert_eq!(result.blob(), "d".repeat(40));
        assert_eq!(result.bytes(), b"failed attempt bytes");
    }
    Ok(())
}

#[test]
fn invalid_expectations_fail_before_any_provider_request() -> TestResult {
    let f = Fixture::new(CiProvider::Github)?;
    for path in [
        "",
        "/001",
        "a//b",
        "a/../b",
        "a/./b",
        "a\\b",
        "https://evil.test/b",
        "a?b",
        "a%b",
    ] {
        let mut locator = f.locator.clone();
        locator.path = path.into();
        let mut calls = 0;
        assert!(
            retrieve_with(&locator, |_, _| {
                calls += 1;
                Err("must not request".into())
            })
            .is_err()
        );
        assert_eq!(calls, 0, "{path}");
    }
    for (field_name, replacement) in [
        ("schema", json!(99)),
        ("repository_id", json!(0)),
        ("provider", json!("other")),
        ("commit", json!("main")),
        ("sha256", json!("abcd")),
    ] {
        let mut value = serde_json::to_value(&f.locator)?;
        value[field_name] = replacement;
        let locator: ArchiveLocator = serde_json::from_value(value)?;
        assert!(locator.validate().is_err(), "{field_name}");
    }
    Ok(())
}

#[test]
fn published_locator_schema_agrees_with_strict_runtime_selection() -> TestResult {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../schema/evidence-archive.schema.json"
    ))?;
    let validator = jsonschema::validator_for(&schema)?;
    let f = Fixture::new(CiProvider::Gitlab)?;
    let mut value = serde_json::to_value(&f.locator)?;
    assert!(validator.is_valid(&value));
    for path in ["a/../b", "../a", "a/.", "a//b", "/a", "a\\b", "a?b"] {
        value["path"] = json!(path);
        assert!(!validator.is_valid(&value), "{path}");
        assert!(
            serde_json::from_value::<ArchiveLocator>(value.clone())?
                .validate()
                .is_err()
        );
    }
    value = serde_json::to_value(&f.locator)?;
    value["approval"] = json!(true);
    assert!(!validator.is_valid(&value));
    assert!(serde_json::from_value::<ArchiveLocator>(value).is_err());
    Ok(())
}

#[test]
fn symlinks_submodules_missing_and_duplicate_entries_never_become_archives() -> TestResult {
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        for leaf in [false, true] {
            for (mode, kind) in [
                ("120000", "blob"),
                ("160000", "commit"),
                ("unknown", "blob"),
            ] {
                let mut f = Fixture::new(provider)?;
                f.set_tree(leaf, mode, kind)?;
                assert!(f.retrieve().is_err(), "{provider:?} {leaf} {mode}");
            }
        }
        for duplicate in [false, true] {
            let mut f = Fixture::new(provider)?;
            let mut response: Value =
                serde_json::from_slice(f.responses.get(&f.child).ok_or("fixture")?)?;
            let entries = if provider == CiProvider::Github {
                response["tree"].as_array_mut()
            } else {
                response.as_array_mut()
            }
            .ok_or("entries")?;
            if duplicate {
                entries.push(entries[0].clone());
            } else {
                entries.clear();
            }
            f.set(&f.child.clone(), &response)?;
            assert!(f.retrieve().is_err());
        }
    }
    Ok(())
}

#[test]
fn changed_repository_revision_blob_missing_bytes_and_access_denial_never_fall_back() -> TestResult
{
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        for key in ["project", "recheck", "commit", "raw", "missing", "denied"] {
            let mut f = Fixture::new(provider)?;
            match key {
                "project" => f.set(
                    &f.project.clone(),
                    &json!({"id":43,"full_name":"test/archive"}),
                )?,
                "recheck" => f.set(
                    &f.recheck.clone(),
                    &json!({"id":43,"full_name":"test/archive"}),
                )?,
                "commit" => f.set(
                    &f.commit.clone(),
                    &json!({"id":"f".repeat(40),"sha":"f".repeat(40)}),
                )?,
                "raw" => {
                    f.responses
                        .insert(f.raw.clone(), b"new green bytes".to_vec());
                }
                _ => {
                    f.responses.remove(&f.raw);
                }
            }
            assert!(f.retrieve().is_err(), "{provider:?} {key}");
        }
        // A failure at any individual request stops there, including the final identity recheck.
        let f = Fixture::new(provider)?;
        let mut count = 0;
        retrieve_with(&f.locator, |url, _| {
            count += 1;
            f.responses.get(url).cloned().ok_or("missing".into())
        })?;
        for failed_call in 1..=count {
            let mut actual = 0;
            assert!(
                retrieve_with(&f.locator, |url, _| {
                    actual += 1;
                    if actual == failed_call {
                        Err("HTTP 403".into())
                    } else {
                        f.responses.get(url).cloned().ok_or("missing".into())
                    }
                })
                .is_err()
            );
            assert_eq!(actual, failed_call);
        }
    }
    Ok(())
}

#[test]
fn truncated_and_oversized_metadata_or_bytes_cannot_be_partial_successes() -> TestResult {
    let mut f = Fixture::new(CiProvider::Github)?;
    let mut tree: Value = serde_json::from_slice(f.responses.get(&f.tree).ok_or("tree")?)?;
    tree["truncated"] = json!(true);
    f.set(&f.tree.clone(), &tree)?;
    assert!(f.retrieve().is_err());
    let f = Fixture::new(CiProvider::Gitlab)?;
    let mut pages = 0;
    let result = retrieve_with(&f.locator, |url, _| {
        if url.contains("/tree?") {
            pages += 1;
            Ok(serde_json::to_vec(&vec![json!({"path":"ignored"}); 100]).map_err(|_| "json")?)
        } else {
            f.responses.get(url).cloned().ok_or("missing".into())
        }
    });
    assert!(result.is_err());
    assert_eq!(pages, 10);
    assert_eq!(bounded_read(&b"1234"[..], 4)?, b"1234");
    assert!(bounded_read(&b"12345"[..], 4).is_err());
    Ok(())
}
