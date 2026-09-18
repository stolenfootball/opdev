use super::{Value, Verdict, number, string};
use opdev_project::{AccessPrincipal, GitlabBranchPolicy, RequiredCheck};
use std::collections::BTreeSet;

fn boolean(value: &Value, key: &str) -> Result<bool, String> {
    value[key]
        .as_bool()
        .ok_or_else(|| format!("Provider did not expose boolean {key}"))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value[key]
        .as_array()
        .ok_or_else(|| format!("Provider did not expose collection {key}"))
}

fn required_checks(
    value: &Value,
    context_key: &str,
    producer_key: &str,
) -> Result<BTreeSet<RequiredCheck>, String> {
    value
        .as_array()
        .ok_or("Required-check policy is unavailable")?
        .iter()
        .map(|row| {
            Ok(RequiredCheck {
                name: string(row, context_key)?,
                producer_id: number(row, producer_key)?,
            })
        })
        .collect()
}

pub(super) fn github_branch(value: &Value, strict: bool, checks: &[RequiredCheck]) -> Verdict {
    if boolean(&value["allow_force_pushes"], "enabled")?
        || boolean(&value["allow_deletions"], "enabled")?
        || !boolean(&value["enforce_admins"], "enabled")?
    {
        return Ok(Err("Reviewed branch protection drift: force-push/deletion or administrator bypass is allowed".into()));
    }
    let reviews = &value["required_pull_request_reviews"];
    if !reviews.is_object() {
        return Ok(Err(
            "Reviewed branch protection drift: pull requests are not required".into(),
        ));
    }
    if let Some(bypass) = reviews.get("bypass_pull_request_allowances") {
        for kind in ["users", "teams", "apps"] {
            if !array(bypass, kind)?.is_empty() {
                return Ok(Err(
                    "Reviewed branch protection drift: pull-request bypass is configured".into(),
                ));
            }
        }
    }
    let status = &value["required_status_checks"];
    if status.is_null() {
        return Ok(Err("Required status checks are not enforced".into()));
    }
    let actual = required_checks(&status["checks"], "context", "app_id")?;
    if actual != checks.iter().cloned().collect() || boolean(status, "strict")? != strict {
        return Ok(Err(
            "Required check names, producer bindings or up-to-date policy drifted".into(),
        ));
    }
    Ok(Ok(()))
}

pub(super) fn github_rulesets(
    ids: &[u64],
    strict: bool,
    checks: &[RequiredCheck],
    active: &[Value],
    details: &[Value],
) -> Verdict {
    let mut active_ids = BTreeSet::new();
    for rule in active {
        active_ids.insert(number(rule, "ruleset_id")?);
    }
    if ids.iter().any(|id| !active_ids.contains(id)) {
        return Ok(Err(
            "A reviewed ruleset is no longer active for trunk".into()
        ));
    }
    let mut found = BTreeSet::new();
    let mut observed_checks = BTreeSet::new();
    let mut rules = BTreeSet::new();
    let mut status_policy_seen = false;
    for detail in details {
        let id = number(detail, "id")?;
        if !ids.contains(&id) || !found.insert(id) {
            return Err("Unexpected or duplicate ruleset identity".into());
        }
        if string(detail, "enforcement")? != "active" || string(detail, "target")? != "branch" {
            return Ok(Err("Reviewed ruleset is not enforcing branch policy".into()));
        }
        // GitHub omits this field without sufficient permissions. Omission is
        // unknown, not an empty bypass list or permission to change settings.
        if !array(detail, "bypass_actors")?.is_empty() {
            return Ok(Err("Reviewed ruleset permits bypass actors".into()));
        }
        let normalized = |rows: Vec<&Value>| {
            rows.into_iter()
                .map(|rule| {
                    serde_json::json!({"type": rule["type"], "parameters": rule["parameters"]})
                        .to_string()
                })
                .collect::<BTreeSet<_>>()
        };
        if normalized(array(detail, "rules")?.iter().collect())
            != normalized(
                active
                    .iter()
                    .filter(|rule| rule["ruleset_id"].as_u64() == Some(id))
                    .collect(),
            )
        {
            return Err(
                "Active branch rules and ruleset details disagree; snapshot is unverified".into(),
            );
        }
        for rule in active
            .iter()
            .filter(|rule| rule["ruleset_id"].as_u64() == Some(id))
        {
            let kind = string(rule, "type")?;
            if kind == "required_status_checks" {
                let parameters = &rule["parameters"];
                if boolean(parameters, "strict_required_status_checks_policy")? != strict {
                    return Ok(Err("Ruleset up-to-date policy drifted".into()));
                }
                if parameters.get("do_not_enforce_on_create").is_some()
                    && boolean(parameters, "do_not_enforce_on_create")?
                {
                    return Ok(Err(
                        "Ruleset does not enforce checks on branch creation".into()
                    ));
                }
                observed_checks.extend(required_checks(
                    &parameters["required_status_checks"],
                    "context",
                    "integration_id",
                )?);
                status_policy_seen = true;
            }
            rules.insert(kind);
        }
    }
    if found != ids.iter().copied().collect() {
        return Err("Ruleset detail inventory is incomplete".into());
    }
    if !["pull_request", "non_fast_forward", "deletion"]
        .iter()
        .all(|kind| rules.contains(*kind))
        || !status_policy_seen
        || observed_checks != checks.iter().cloned().collect()
    {
        return Ok(Err(
            "Reviewed ruleset requirements or producer-bound checks drifted".into(),
        ));
    }
    Ok(Ok(()))
}

fn principals(rows: &[Value]) -> Result<BTreeSet<AccessPrincipal>, String> {
    let mut result = BTreeSet::new();
    for row in rows {
        let specific = ["user_id", "group_id", "deploy_key_id", "member_role_id"]
            .into_iter()
            .filter(|key| !row[*key].is_null())
            .collect::<Vec<_>>();
        let principal = match specific.as_slice() {
            [] => {
                AccessPrincipal::Role(row["access_level"].as_u64().ok_or("Missing access level")?)
            }
            ["user_id"] => AccessPrincipal::User(number(row, "user_id")?),
            ["group_id"] => AccessPrincipal::Group(number(row, "group_id")?),
            ["deploy_key_id"] => AccessPrincipal::DeployKey(number(row, "deploy_key_id")?),
            ["member_role_id"] => AccessPrincipal::MemberRole(number(row, "member_role_id")?),
            _ => return Err("Ambiguous protected-branch principal".into()),
        };
        if !result.insert(principal) {
            return Err("Duplicate protected-branch principal".into());
        }
    }
    if result.is_empty() {
        return Err("Empty protected-branch access policy is unverified".into());
    }
    Ok(result)
}

pub(super) fn gitlab(
    project: &Value,
    branch: &str,
    expected: &[GitlabBranchPolicy],
    observed: &[Value],
) -> Verdict {
    if !boolean(project, "only_allow_merge_if_pipeline_succeeds")?
        || boolean(project, "allow_merge_on_skipped_pipeline")?
    {
        return Ok(Err(
            "GitLab merge policy no longer requires successful non-skipped pipelines".into(),
        ));
    }
    let mut found = BTreeSet::new();
    for rule in observed {
        let name = string(rule, "name")?;
        if !matches_branch(&name, branch) {
            continue;
        }
        if !found.insert(name.clone()) {
            return Err("Duplicate matching protection rule".into());
        }
        let Some(wanted) = expected.iter().find(|rule| rule.name == name) else {
            return Ok(Err(
                "An unreviewed protected-branch rule now applies to trunk".into(),
            ));
        };
        if boolean(rule, "allow_force_push")? {
            return Ok(Err(
                "A matching protected-branch rule permits force-push".into()
            ));
        }
        if principals(array(rule, "push_access_levels")?)? != wanted.push.iter().cloned().collect()
            || principals(array(rule, "merge_access_levels")?)?
                != wanted.merge.iter().cloned().collect()
        {
            return Ok(Err(
                "Protected-branch push or merge access drifted from reviewed identities".into(),
            ));
        }
    }
    if found != expected.iter().map(|rule| rule.name.clone()).collect() {
        return Ok(Err(
            "Reviewed protected-branch rules are missing or no longer match trunk".into(),
        ));
    }
    Ok(Ok(()))
}

/// GitLab's documented star wildcard, without interpreting regex or shell syntax.
fn matches_branch(pattern: &str, branch: &str) -> bool {
    let text: Vec<_> = branch.chars().collect();
    let mut matches = vec![false; text.len() + 1];
    matches[0] = true;
    for ch in pattern.chars() {
        if ch == '*' {
            for i in 1..matches.len() {
                matches[i] |= matches[i - 1];
            }
        } else {
            for i in (1..matches.len()).rev() {
                matches[i] = matches[i - 1] && text[i - 1] == ch;
            }
            matches[0] = false;
        }
    }
    matches[text.len()]
}
