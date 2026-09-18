//! Reviewed remote expectations; never inferred or approved by discovery.
use crate::{CiProvider, ManifestError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Policy inputs for current-trunk CI qualification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualificationPolicy {
    /// Existing authority containing the actual developer decision.
    pub review_reference: String,
    /// Expected run event/pipeline source.
    pub source: String,
    /// GitHub workflow identity; absent for GitLab.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<u64>,
    /// Exact provider-owned job names (matrix names included).
    pub required_jobs: Vec<String>,
    /// Additional checks bound to a provider producer identity.
    pub required_checks: Vec<RequiredCheck>,
    /// Reviewed provider merge-policy expectations.
    pub protection: ProtectionPolicy,
}

/// A name alone cannot identify a trusted check producer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredCheck {
    /// Exact check name.
    pub name: String,
    /// GitHub App ID or GitLab commit-status creator ID.
    pub producer_id: u64,
}

/// Provider-specific configuration behind a shared qualification result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProtectionPolicy {
    /// Classic GitHub protection with source-pinned required checks.
    GithubBranch {
        /// Reviewed requirement for up-to-date checks.
        strict: bool,
    },
    /// Explicitly selected active rulesets applying to trunk.
    GithubRulesets {
        /// Exact selected ruleset identities.
        ids: Vec<u64>,
        /// Reviewed requirement for up-to-date checks.
        strict: bool,
    },
    /// All matching GitLab protected-branch rules, including wildcards.
    Gitlab {
        /// Reviewed matching rules and access identities.
        rules: Vec<GitlabBranchPolicy>,
    },
}

/// A GitLab protected-branch rule whose name may contain `*`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitlabBranchPolicy {
    /// Exact protection rule name.
    pub name: String,
    /// Approved push principals, including explicit no-access role 0.
    pub push: Vec<AccessPrincipal>,
    /// Approved merge principals.
    pub merge: Vec<AccessPrincipal>,
}

/// Normalized principal identity, excluding display names and row IDs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AccessPrincipal {
    /// Numeric GitLab access level; zero explicitly means no access.
    Role(u64),
    /// Specific user.
    User(u64),
    /// Specific group.
    Group(u64),
    /// Specific deploy key.
    DeployKey(u64),
    /// Specific custom member role.
    MemberRole(u64),
}

impl QualificationPolicy {
    pub(crate) fn validate(&self, provider: CiProvider) -> Result<(), ManifestError> {
        let text =
            |s: &str| !s.trim().is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control);
        let unique = |names: &[String]| {
            names.len() <= 100
                && names.iter().all(|s| text(s))
                && names.iter().collect::<BTreeSet<_>>().len() == names.len()
        };
        let valid_principals = |values: &[AccessPrincipal]| {
            !values.is_empty()
                && values.len() <= 100
                && values.iter().collect::<BTreeSet<_>>().len() == values.len()
                && values.iter().all(|p| match p {
                    AccessPrincipal::Role(id) => matches!(id, 0 | 30 | 40 | 60),
                    AccessPrincipal::User(id)
                    | AccessPrincipal::Group(id)
                    | AccessPrincipal::DeployKey(id)
                    | AccessPrincipal::MemberRole(id) => *id > 0,
                })
        };
        let protection_valid = match (&self.protection, provider) {
            (ProtectionPolicy::GithubBranch { .. }, CiProvider::Github) => true,
            (ProtectionPolicy::GithubRulesets { ids, .. }, CiProvider::Github) => {
                !ids.is_empty()
                    && ids.len() <= 20
                    && ids.iter().all(|id| *id > 0)
                    && ids.iter().collect::<BTreeSet<_>>().len() == ids.len()
            }
            (ProtectionPolicy::Gitlab { rules }, CiProvider::Gitlab) => {
                !rules.is_empty()
                    && rules.len() <= 100
                    && unique(&rules.iter().map(|r| r.name.clone()).collect::<Vec<_>>())
                    && rules
                        .iter()
                        .all(|r| valid_principals(&r.push) && valid_principals(&r.merge))
            }
            _ => false,
        };
        if !text(&self.review_reference)
            || !text(&self.source)
            || self.required_jobs.is_empty()
            || !unique(&self.required_jobs)
            || !unique(
                &self
                    .required_checks
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>(),
            )
            || self.required_checks.iter().any(|c| c.producer_id == 0)
            || !protection_valid
            || match provider {
                CiProvider::Github => {
                    self.workflow_id.is_none_or(|id| id == 0) || self.required_checks.is_empty()
                }
                CiProvider::Gitlab => self.workflow_id.is_some(),
                _ => true,
            }
        {
            return Err(ManifestError::Semantic("remote qualification needs a review reference, distinct required jobs/checks, valid producer/workflow identities and matching provider protection policy".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_rejects_missing_review_ambiguous_names_and_wrong_provider_identity() {
        let base = QualificationPolicy {
            review_reference: "review:45".into(),
            source: "push".into(),
            workflow_id: Some(1),
            required_jobs: vec!["test".into()],
            required_checks: vec![RequiredCheck {
                name: "test".into(),
                producer_id: 7,
            }],
            protection: ProtectionPolicy::GithubBranch { strict: true },
        };
        assert!(base.validate(CiProvider::Github).is_ok());
        assert!(base.validate(CiProvider::Gitlab).is_err());
        assert!(base.validate(CiProvider::Unconfigured).is_err());
        let mut invalid = base.clone();
        invalid.review_reference.clear();
        assert!(invalid.validate(CiProvider::Github).is_err());
        invalid = base.clone();
        invalid.required_checks[0].producer_id = 0;
        assert!(invalid.validate(CiProvider::Github).is_err());
        invalid = base.clone();
        invalid
            .required_checks
            .push(invalid.required_checks[0].clone());
        assert!(invalid.validate(CiProvider::Github).is_err());
        invalid = base.clone();
        invalid.required_jobs.clear();
        assert!(invalid.validate(CiProvider::Github).is_err());
        invalid = base.clone();
        invalid.workflow_id = None;
        assert!(invalid.validate(CiProvider::Github).is_err());
        invalid = base;
        invalid.protection = ProtectionPolicy::GithubRulesets {
            ids: vec![1, 1],
            strict: true,
        };
        assert!(invalid.validate(CiProvider::Github).is_err());
    }
}
