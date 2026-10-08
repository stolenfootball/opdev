//! Exact Git-backed evidence reads; storage origin is not execution or consent.

use std::io::Read;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{CiProvider, encode, first_env, github_request, gitlab_credential, gitlab_request};

const META_LIMIT: u64 = 1024 * 1024;
const BLOB_LIMIT: u64 = 8 * 1024 * 1024;

/// Explicit immutable selection, obtained from the existing authorized evidence authority.
/// This locator neither approves retention policy nor authenticates its own selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveLocator {
    /// Locator format (1).
    pub schema: u32,
    /// First-class hosted provider; self-hosted origins are unsupported.
    pub provider: CiProvider,
    /// Stable numeric provider repository/project ID, not a mutable name.
    pub repository_id: u64,
    /// Full lowercase Git SHA-1 commit ID; refs and abbreviated IDs are unsupported.
    pub commit: String,
    /// Exact case-sensitive Git path, at most 16 components and 1024 bytes.
    pub path: String,
    /// Independently expected SHA-256 of the raw envelope bytes.
    pub sha256: String,
}

impl ArchiveLocator {
    /// Reject unsupported or ambiguous selections before credentials or network access.
    ///
    /// # Errors
    /// Returns a content-free diagnostic for invalid schema, provider, identity or path.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1
            || !matches!(self.provider, CiProvider::Github | CiProvider::Gitlab)
            || self.repository_id == 0
            || !hex(&self.commit, 40)
            || !hex(&self.sha256, 64)
            || self.path.len() > 1024
            || self.path.split('/').count() > 16
            || self.path.split('/').any(|s| {
                s.is_empty()
                    || matches!(s, "." | "..")
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            })
        {
            return Err("Unsupported archive locator; select a numeric repository, full commit, portable Git path and exact SHA-256".into());
        }
        Ok(())
    }
}

/// Bytes observed through authenticated fixed-origin provider GETs.
/// Not deserializable and deliberately not Debug: contents can be private.
/// No conversion to qualifying execution or developer approval exists.
pub struct ArchiveObservation {
    locator: ArchiveLocator,
    blob: String,
    bytes: Vec<u8>,
}

impl ArchiveObservation {
    /// Exact selection matched by this retrieval.
    #[must_use]
    pub const fn locator(&self) -> &ArchiveLocator {
        &self.locator
    }
    /// Provider-observed regular Git blob identity.
    #[must_use]
    pub fn blob(&self) -> &str {
        &self.blob
    }
    /// Private bytes, bounded to 8 MiB. Do not log them automatically.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Retrieve one exact regular file without redirects, writes, cache fallback or publication.
/// Uses existing remote-adapter credentials, but requires a credential instead of
/// falling back to anonymous reads. The provider authenticates storage origin,
/// not the truth, approval, retention or execution claims inside stored bytes.
///
/// # Errors
/// Invalid selection, missing credentials, HTTP/permission/retention failures,
/// ambiguous/truncated trees, special files and changed bytes all return errors.
/// Diagnostics exclude credentials, response bodies and private content.
pub fn retrieve_archive(locator: &ArchiveLocator) -> Result<ArchiveObservation, String> {
    locator.validate()?;
    authenticated_reads(locator.provider, |get| retrieve_with(locator, get))
}

pub(crate) fn authenticated_reads<T>(
    provider: CiProvider,
    operation: impl FnOnce(&mut dyn FnMut(&str, bool) -> Result<Vec<u8>, String>) -> Result<T, String>,
) -> Result<T, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = Client::builder()
        .user_agent(concat!("opdev/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not initialize archive retrieval")?;
    let github = (provider == CiProvider::Github)
        .then(|| first_env(&["OPDEV_GITHUB_TOKEN", "GITHUB_TOKEN", "GH_TOKEN"]))
        .flatten();
    let gitlab = (provider == CiProvider::Gitlab)
        .then(gitlab_credential)
        .flatten();
    if github.is_none() && gitlab.is_none() {
        return Err(
            "Archive credentials unavailable; no anonymous or cached result substituted".into(),
        );
    }
    let started = Instant::now();
    operation(&mut |url, raw| {
        let origin = if provider == CiProvider::Github {
            "https://api.github.com/"
        } else {
            "https://gitlab.com/api/v4/"
        };
        if !url.starts_with(origin) {
            return Err("Provider request left its fixed origin".into());
        }
        let remaining = Duration::from_mins(1)
            .checked_sub(started.elapsed())
            .ok_or("Archive retrieval exceeded its one-minute budget")?;
        let request = client
            .get(url)
            .timeout(remaining.min(Duration::from_secs(20)));
        let request = if provider == CiProvider::Github {
            github_request(request, github.as_deref()).header(
                "Accept",
                if raw {
                    "application/vnd.github.raw+json"
                } else {
                    "application/vnd.github+json"
                },
            )
        } else {
            gitlab_request(request, gitlab.as_ref())
        };
        let response = request
            .send()
            .map_err(|_| "Archive request could not complete; no cached result substituted")?;
        if !response.status().is_success() {
            return Err(format!(
                "Archive provider returned HTTP {}; access, retention or location may have changed. No earlier result substituted",
                response.status().as_u16()
            ));
        }
        bounded_read(response, if raw { BLOB_LIMIT } else { META_LIMIT })
    })
}

fn bounded_read(reader: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Archive response could not be read")?;
    if bytes.len() as u64 > limit {
        return Err("Archive response exceeds supported size; no partial result accepted".into());
    }
    Ok(bytes)
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn json(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|_| "Archive metadata is not supported JSON".into())
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| "Archive metadata is incomplete".into())
}

fn object_id(value: &Value, name: &str) -> Result<String, String> {
    let id = field(value, name)?;
    if !hex(id, 40) {
        return Err("Archive returned an unsupported Git identity".into());
    }
    Ok(id.into())
}

fn retrieve_with<F>(locator: &ArchiveLocator, mut get: F) -> Result<ArchiveObservation, String>
where
    F: FnMut(&str, bool) -> Result<Vec<u8>, String>,
{
    locator.validate()?;
    let project_url = match locator.provider {
        CiProvider::Github => format!(
            "https://api.github.com/repositories/{}",
            locator.repository_id
        ),
        _ => format!(
            "https://gitlab.com/api/v4/projects/{}",
            locator.repository_id
        ),
    };
    let project = json(&get(&project_url, false)?)?;
    if project["id"].as_u64() != Some(locator.repository_id) {
        return Err("Archive repository identity differs; no name-based fallback".into());
    }
    let (blob, blob_url, recheck) = if locator.provider == CiProvider::Github {
        github_blob(locator, &project, &mut get)?
    } else {
        let blob = gitlab_blob(locator, &project_url, &mut get)?;
        let url = format!("{project_url}/repository/blobs/{blob}/raw");
        (blob, url, project_url.clone())
    };
    let bytes = get(&blob_url, true)?;
    if bytes.len() as u64 > BLOB_LIMIT || format!("{:x}", Sha256::digest(&bytes)) != locator.sha256
    {
        return Err(
            "Archive bytes differ from the expected SHA-256; no earlier result substituted".into(),
        );
    }
    // Recheck the actual slug endpoint used for GitHub objects, not only the numeric alias.
    let checked = json(&get(&recheck, false)?)?;
    if checked["id"].as_u64() != Some(locator.repository_id) {
        return Err("Archive repository changed during retrieval".into());
    }
    Ok(ArchiveObservation {
        locator: locator.clone(),
        blob,
        bytes,
    })
}

fn github_blob<F>(
    locator: &ArchiveLocator,
    project: &Value,
    get: &mut F,
) -> Result<(String, String, String), String>
where
    F: FnMut(&str, bool) -> Result<Vec<u8>, String>,
{
    let slug = field(project, "full_name")?;
    let components: Vec<_> = slug.split('/').collect();
    if components.len() != 2
        || components.iter().any(|s| {
            s.is_empty()
                || matches!(*s, "." | "..")
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
    {
        return Err("Archive repository name is unsupported".into());
    }
    let base = format!("https://api.github.com/repos/{slug}");
    let commit = json(&get(
        &format!("{base}/git/commits/{}", locator.commit),
        false,
    )?)?;
    if field(&commit, "sha")? != locator.commit {
        return Err("Archive commit differs".into());
    }
    let mut tree = object_id(&commit["tree"], "sha")?;
    let parts: Vec<_> = locator.path.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        let response = json(&get(&format!("{base}/git/trees/{tree}"), false)?)?;
        if field(&response, "sha")? != tree || response["truncated"].as_bool() != Some(false) {
            return Err("Archive tree differs or is truncated; no partial result accepted".into());
        }
        let entries = response["tree"]
            .as_array()
            .ok_or("Archive tree entries unavailable")?;
        let matches: Vec<_> = entries
            .iter()
            .filter(|e| e["path"].as_str() == Some(part))
            .collect();
        if matches.len() != 1 {
            return Err("Archive file is missing or ambiguous".into());
        }
        let entry = matches[0];
        validate_entry(entry, i + 1 == parts.len())?;
        tree = object_id(entry, "sha")?;
    }
    let blob_url = format!("{base}/git/blobs/{tree}");
    Ok((tree, blob_url, base))
}

fn gitlab_blob<F>(
    locator: &ArchiveLocator,
    project_url: &str,
    get: &mut F,
) -> Result<String, String>
where
    F: FnMut(&str, bool) -> Result<Vec<u8>, String>,
{
    let commit = json(&get(
        &format!("{project_url}/repository/commits/{}", locator.commit),
        false,
    )?)?;
    if field(&commit, "id")? != locator.commit {
        return Err("Archive commit differs".into());
    }
    let parts: Vec<_> = locator.path.split('/').collect();
    let mut parent = String::new();
    let mut selected = String::new();
    for (i, part) in parts.iter().enumerate() {
        let full = if parent.is_empty() {
            (*part).to_owned()
        } else {
            format!("{parent}/{part}")
        };
        let mut matches = Vec::new();
        let mut complete = false;
        for page in 1..=10 {
            let response = json(&get(
                &format!(
                    "{project_url}/repository/tree?ref={}&path={}&per_page=100&page={page}",
                    locator.commit,
                    encode(&parent)
                ),
                false,
            )?)?;
            let entries = response
                .as_array()
                .ok_or("Archive tree entries unavailable")?;
            matches.extend(
                entries
                    .iter()
                    .filter(|e| e["path"].as_str() == Some(&full))
                    .cloned(),
            );
            if entries.len() < 100 {
                complete = true;
                break;
            }
        }
        if !complete || matches.len() != 1 {
            return Err("Archive file missing, ambiguous or tree exceeds enumeration bound".into());
        }
        validate_entry(&matches[0], i + 1 == parts.len())?;
        selected = object_id(&matches[0], "id")?;
        parent = full;
    }
    Ok(selected)
}

fn validate_entry(entry: &Value, last: bool) -> Result<(), String> {
    let mode = field(entry, "mode")?;
    let kind = field(entry, "type")?;
    if (last && kind == "blob" && matches!(mode, "100644" | "100755"))
        || (!last && kind == "tree" && mode == "040000")
    {
        Ok(())
    } else {
        Err("Archive path is not a regular Git file; links and submodules are unsupported".into())
    }
}

#[cfg(test)]
mod tests;
