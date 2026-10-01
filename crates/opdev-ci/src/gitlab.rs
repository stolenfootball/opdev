//! Bounded, offline interpretation of GitLab documents and explicit local includes.

use opdev_core::Outcome;
use serde_json::Value;
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

const MAX_BYTES: u64 = 1024 * 1024;
const MAX_INCLUDES: usize = 150;
const MAX_DEPTH: usize = 32;
const MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;

/// A malformed configuration or an explicitly unsupported interpretation.
#[derive(Debug)]
pub struct ConfigurationProblem {
    /// Error for invalid/read failures; unverified for unsupported syntax or bounds.
    pub outcome: Outcome,
    /// Context without echoing repository file contents.
    pub diagnostic: String,
}

impl ConfigurationProblem {
    fn error(message: impl Into<String>) -> Self {
        Self {
            outcome: Outcome::Error,
            diagnostic: message.into(),
        }
    }
    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            outcome: Outcome::Unverified,
            diagnostic: message.into(),
        }
    }
}

/// Reads one regular, contained CI file without following linked children.
///
/// # Errors
/// Returns a contextual error for missing/linked/oversized/unreadable files.
pub fn read_local(root: &Path, relative: &str) -> Result<String, ConfigurationProblem> {
    let path = local_path(relative)?;
    let mut current = root.to_path_buf();
    for component in Path::new(&path).components() {
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(|_| {
            ConfigurationProblem::error(format!("CI file `{path}` is missing or unreadable"))
        })?;
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let linked = metadata.file_type().is_symlink();
        if linked {
            return Err(ConfigurationProblem::error(format!(
                "CI path `{path}` contains a linked child"
            )));
        }
    }
    // Reject nonregular paths before opening: opening a FIFO can itself block.
    if !fs::symlink_metadata(&current)
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() <= MAX_BYTES)
    {
        return Err(ConfigurationProblem::error(format!(
            "CI file `{path}` must be regular and at most 1 MiB"
        )));
    }
    let file = fs::File::open(&current)
        .map_err(|_| ConfigurationProblem::error(format!("CI file `{path}` is unreadable")))?;
    if !file
        .metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() <= MAX_BYTES)
    {
        return Err(ConfigurationProblem::error(format!(
            "CI file `{path}` must be regular and at most 1 MiB"
        )));
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ConfigurationProblem::error(format!("CI file `{path}` could not be read")))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(ConfigurationProblem::error(format!(
            "CI file `{path}` exceeds 1 MiB"
        )));
    }
    String::from_utf8(bytes)
        .map_err(|_| ConfigurationProblem::error(format!("CI file `{path}` is not UTF-8")))
}

fn local_path(value: &str) -> Result<String, ConfigurationProblem> {
    // GitLab's leading slash is repository-root relative, not an OS absolute path.
    let value = value.strip_prefix('/').unwrap_or(value);
    if value.is_empty()
        || value.contains(['\\', ':', '$', '*', '?', '[', ']'])
        || Path::new(value)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ConfigurationProblem::unsupported(
            "Only explicit repository-local include paths are supported; dynamic, glob and escaping paths remain unverified",
        ));
    }
    Ok(value.into())
}

/// Parses a pipeline with an optional separate `spec` header.
///
/// # Errors
/// Rejects malformed YAML/document shapes without echoing input contents.
pub fn parse_documents(source: &str) -> Result<Value, ConfigurationProblem> {
    if source.len() as u64 > MAX_BYTES {
        return Err(ConfigurationProblem::unsupported(
            "GitLab document exceeds 1 MiB inspection limit",
        ));
    }
    let mut docs: Vec<Value> = serde_saphyr::from_multiple(source)
        .map_err(|_| ConfigurationProblem::error("Invalid GitLab YAML document"))?;
    if docs.len() == 2
        && docs[0]
            .as_object()
            .is_some_and(|map| map.len() == 1 && map.get("spec").is_some_and(Value::is_object))
    {
        docs.remove(0);
    }
    if docs.len() != 1 || !docs[0].is_object() {
        return Err(ConfigurationProblem::error(
            "Expected one GitLab pipeline document, optionally preceded by a spec header",
        ));
    }
    let mut value = docs.remove(0);
    // Headers describe input contracts; they are not executed pipeline controls.
    value.as_object_mut().map(|map| map.remove("spec"));
    if value.to_string().contains("$[[") {
        return Err(ConfigurationProblem::unsupported(
            "GitLab input interpolation is unresolved; local inspection cannot establish effective controls",
        ));
    }
    Ok(value)
}

/// Resolves explicit local includes using an injected, snapshot-recording reader.
/// Repeated includes are merged in order; only recursion-stack repetition is a cycle.
///
/// # Errors
/// Reports malformed/missing files, cycles, bounds and unsupported include forms.
pub fn resolve(
    source: &str,
    mut read: impl FnMut(&str) -> Result<String, ConfigurationProblem>,
) -> Result<Value, ConfigurationProblem> {
    resolve_inner(
        source,
        &mut read,
        &mut vec![".gitlab-ci.yml".into()],
        &mut 0,
        &mut 0,
    )
}

fn resolve_inner(
    source: &str,
    read: &mut impl FnMut(&str) -> Result<String, ConfigurationProblem>,
    stack: &mut Vec<String>,
    count: &mut usize,
    bytes: &mut usize,
) -> Result<Value, ConfigurationProblem> {
    *bytes += source.len();
    if *bytes > MAX_TOTAL_BYTES {
        return Err(ConfigurationProblem::unsupported(
            "GitLab total input exceeds 8 MiB inspection limit",
        ));
    }
    let mut current = parse_documents(source)?;
    let includes = current
        .as_object_mut()
        .and_then(|map| map.remove("include"));
    let mut merged = serde_json::json!({});
    if let Some(includes) = includes {
        let includes = match includes {
            Value::Array(items) => items,
            item => vec![item],
        };
        for include in includes {
            let path = match &include {
                Value::String(path) => local_path(path)?,
                Value::Object(map) if map.len() == 1 => local_path(map.get("local").and_then(Value::as_str).ok_or_else(|| ConfigurationProblem::unsupported("External/template/component includes are unresolved by local inspection"))?)?,
                _ => return Err(ConfigurationProblem::unsupported("Conditional/parameterized or unsupported GitLab includes remain unverified")),
            };
            *count += 1;
            if *count > MAX_INCLUDES || stack.len() >= MAX_DEPTH {
                return Err(ConfigurationProblem::unsupported(
                    "GitLab include count/depth limit reached; effective configuration remains unverified",
                ));
            }
            if stack.contains(&path) {
                return Err(ConfigurationProblem::error(format!(
                    "GitLab include cycle at `{path}`"
                )));
            }
            let source_text = read(&path)?;
            stack.push(path.clone());
            let value =
                resolve_inner(&source_text, read, stack, count, bytes).map_err(|mut error| {
                    error.diagnostic = format!("Included `{path}`: {}", error.diagnostic);
                    error
                })?;
            stack.pop();
            merge(&mut merged, value);
        }
    }
    merge(&mut merged, current);
    Ok(merged)
}

fn merge(target: &mut Value, source: Value) {
    match (target, source) {
        (Value::Object(target), Value::Object(source)) => {
            for (key, value) in source {
                if let Some(existing) = target.get_mut(&key) {
                    merge(existing, value);
                } else {
                    target.insert(key, value);
                }
            }
        }
        (target, value) => *target = value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_only_contained_regular_bounded_utf8_files() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        fs::create_dir(root.path().join("ci"))?;
        fs::write(root.path().join("ci/check.yml"), "job: {}")?;
        assert_eq!(
            read_local(root.path(), "/ci/check.yml").map_err(|e| e.diagnostic)?,
            "job: {}"
        );
        for path in [
            "missing.yml",
            "ci",
            "../outside.yml",
            "C:/outside.yml",
            "ci/*.yml",
        ] {
            assert!(read_local(root.path(), path).is_err(), "{path}");
        }
        fs::write(root.path().join("invalid.yml"), [0xff])?;
        assert!(read_local(root.path(), "invalid.yml").is_err());
        let large = fs::File::create(root.path().join("large.yml"))?;
        large.set_len(MAX_BYTES + 1)?;
        assert!(read_local(root.path(), "large.yml").is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.path().join("ci"), root.path().join("link"))?;
            assert!(read_local(root.path(), "link/check.yml").is_err());
        }
        Ok(())
    }
    #[test]
    fn depth_and_total_bytes_limits_remain_unverified() {
        let mut depth = 0;
        let result = resolve("include: a.yml", |_| {
            depth += 1;
            Ok(format!("include: file{depth}.yml"))
        });
        assert!(matches!(
            result,
            Err(ConfigurationProblem {
                outcome: Outcome::Unverified,
                ..
            })
        ));
        let large = format!("padding: {}", "x".repeat(1024 * 1024 - 10));
        let result = resolve(
            "include: [a.yml,a.yml,a.yml,a.yml,a.yml,a.yml,a.yml,a.yml,a.yml]",
            |_| Ok(large.clone()),
        );
        assert!(matches!(
            result,
            Err(ConfigurationProblem {
                outcome: Outcome::Unverified,
                ..
            })
        ));
    }
    #[test]
    fn unsupported_missing_cycles_and_documents_remain_explicit() {
        for source in [
            "include: {remote: https://example.test/ci.yml}",
            "include: {local: ci.yml, rules: []}",
            "include: '../ci.yml'",
            "job: {script: '$[[ inputs.run ]]'}",
        ] {
            assert!(matches!(
                resolve(source, |_| unreachable!()),
                Err(ConfigurationProblem {
                    outcome: Outcome::Unverified,
                    ..
                })
            ));
        }
        assert!(matches!(
            resolve("include: ci.yml", |_| Ok("include: ci.yml".into())),
            Err(ConfigurationProblem {
                outcome: Outcome::Error,
                ..
            })
        ));
        assert!(parse_documents("a: 1\n---\nb: 2").is_err());
        assert!(parse_documents("spec: {}\n---\na: 1\n---\nb: 2").is_err());
    }
    #[test]
    fn includes_are_bounded_and_maps_merge_while_arrays_replace() -> Result<(), String> {
        let source = "include: a.yml\njob: {script: [new], variables: {B: root}}";
        let value = resolve(source, |_| {
            Ok("job: {script: [old], variables: {A: included, B: included}}".into())
        })
        .map_err(|error| error.diagnostic)?;
        assert_eq!(value["job"]["script"], serde_json::json!(["new"]));
        assert_eq!(
            value["job"]["variables"],
            serde_json::json!({"A":"included", "B":"root"})
        );
        let source = format!("include: [{}]", vec!["a.yml"; MAX_INCLUDES + 1].join(","));
        assert!(matches!(
            resolve(&source, |_| Ok("job: {}".into())),
            Err(ConfigurationProblem {
                outcome: Outcome::Unverified,
                ..
            })
        ));
        Ok(())
    }
}
