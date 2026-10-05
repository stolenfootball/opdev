//! Versioned attributed references. A journal is not an approval or a gate.
use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};
use opdev_core::Outcome;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub schema: u32,
    pub source_sha256: String,
    pub configuration_sha256: String,
    pub stage: String,
    pub artifact_sha256: Option<String>,
}

impl Subject {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == 1,
            "unsupported workflow subject version; do not reinterpret it as current evidence"
        );
        ensure!(
            digest(&self.source_sha256) && digest(&self.configuration_sha256),
            "source and configuration need exact SHA-256 identities; obtain a current observation first"
        );
        ensure!(
            !self.stage.trim().is_empty(),
            "the workflow stage is missing"
        );
        ensure!(
            self.artifact_sha256.as_deref().is_none_or(digest),
            "artifact identity is not a SHA-256 digest"
        );
        Ok(())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, clap::ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    AcceptanceReview,
    Execution,
    ArtifactQualification,
    PolicyDecision,
    ImplementationApproval,
    ExecutionPermission,
    UserFeedback,
    ReleaseAuthorization,
}

impl Kind {
    pub const fn requires_human(self) -> bool {
        matches!(
            self,
            Self::PolicyDecision
                | Self::ImplementationApproval
                | Self::UserFeedback
                | Self::ReleaseAuthorization
        )
    }

    pub const fn next_action(self) -> &'static str {
        match self {
            Self::AcceptanceReview => {
                "Review the current acceptance conditions and actual test assertions"
            }
            Self::Execution => {
                "Inspect the required check evidence; run only missing required checks"
            }
            Self::ArtifactQualification => {
                "Locate the exact artifact and its required qualification evidence"
            }
            Self::PolicyDecision => {
                "Read the original policy decision; ask only if its scope is unresolved"
            }
            Self::ImplementationApproval => {
                "Read the original implementation approval and its limits"
            }
            Self::ExecutionPermission => {
                "Check the current host permission for the intended action"
            }
            Self::UserFeedback => {
                "Obtain the required actual product feedback, not permission to perform work"
            }
            Self::ReleaseAuthorization => {
                "Do not publish without a current explicit release request"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub reference: String,
    pub actor: String,
    pub human_attributed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentReference {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub id: String,
    pub kind: Kind,
    pub subject: Subject,
    pub scope: String,
    pub origin: Origin,
    pub evidence: Vec<ContentReference>,
    pub observed_at: u64,
    pub expires_at: Option<u64>,
    pub outcome: Outcome,
    pub supersedes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revocation {
    pub id: String,
    pub target: String,
    pub origin: Origin,
    pub reason: String,
    pub observed_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "event",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Event {
    Record(Record),
    Revoke(Revocation),
}

impl Event {
    fn id(&self) -> &str {
        match self {
            Self::Record(r) => &r.id,
            Self::Revoke(r) => &r.id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub schema: u32,
    pub protocol: String,
    pub work: String,
    pub events: Vec<Event>,
}

impl Journal {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == 1 && self.protocol == "workflow.v1",
            "unsupported workflow record version; preserve this journal and use a compatible CLI"
        );
        ensure!(
            !self.work.trim().is_empty(),
            "the original work authority is missing"
        );
        ensure!(
            self.events.len() <= 10_000,
            "workflow journal exceeds 10,000 events"
        );
        let mut ids = BTreeSet::new();
        let mut records = BTreeMap::new();
        let mut retired = BTreeSet::new();
        for event in &self.events {
            ensure!(
                !event.id().trim().is_empty() && ids.insert(event.id()),
                "duplicate or empty workflow event identity"
            );
            match event {
                Event::Record(record) => {
                    record.subject.validate()?;
                    validate_origin(&record.origin)?;
                    ensure!(!record.scope.trim().is_empty(), "record scope is missing");
                    ensure!(
                        record.expires_at.is_none_or(|end| end > record.observed_at),
                        "record expires before its observation"
                    );
                    ensure!(
                        !record.kind.requires_human() || record.origin.human_attributed,
                        "this decision requires an actual human source; an agent cannot approve itself"
                    );
                    ensure!(
                        record
                            .evidence
                            .iter()
                            .all(|r| !r.path.trim().is_empty() && digest(&r.sha256)),
                        "evidence references need a path and exact SHA-256 digest"
                    );
                    ensure!(
                        record.kind != Kind::ArtifactQualification
                            || record
                                .subject
                                .artifact_sha256
                                .as_ref()
                                .is_some_and(|digest| record
                                    .evidence
                                    .iter()
                                    .any(|reference| reference.sha256 == *digest)
                                    && record
                                        .evidence
                                        .iter()
                                        .any(|reference| reference.sha256 != *digest)),
                        "artifact qualification needs retained artifact bytes and separate qualification evidence"
                    );
                    if let Some(previous) = &record.supersedes {
                        let prior: &&Record = records.get(previous.as_str()).ok_or_else(|| {
                            anyhow::anyhow!("superseded record does not precede this event")
                        })?;
                        ensure!(
                            prior.kind == record.kind
                                && prior.scope == record.scope
                                && !retired.contains(previous.as_str()),
                            "supersession must replace one active record of the same kind and scope; permission cannot become feedback or release authority"
                        );
                        retired.insert(previous.as_str());
                    }
                    records.insert(record.id.as_str(), record);
                }
                Event::Revoke(revocation) => {
                    validate_origin(&revocation.origin)?;
                    ensure!(
                        records.contains_key(revocation.target.as_str())
                            && !revocation.reason.trim().is_empty(),
                        "revocation needs an earlier record and a reason"
                    );
                    retired.insert(revocation.target.as_str());
                }
            }
        }
        Ok(())
    }
}

pub fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn validate_origin(origin: &Origin) -> Result<()> {
    ensure!(
        !origin.reference.trim().is_empty() && !origin.actor.trim().is_empty(),
        "the original decision or observation reference and attributed actor are required"
    );
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub id: String,
    pub kind: Kind,
    pub state: &'static str,
    pub explanation: String,
    pub next_action: &'static str,
    pub original_reference: String,
}

/// Pure projection. File/provider observations are supplied separately, not performed here.
pub fn project(
    journal: &Journal,
    subject: &Subject,
    now: u64,
    mut content_matches: impl FnMut(&ContentReference) -> bool,
) -> Result<Vec<Finding>> {
    journal.validate()?;
    subject.validate()?;
    let retired: BTreeSet<&str> = journal
        .events
        .iter()
        .filter_map(|event| match event {
            Event::Revoke(r) => Some(r.target.as_str()),
            Event::Record(r) => r.supersedes.as_deref(),
        })
        .collect();
    let mut findings = Vec::new();
    let mut active = BTreeMap::new();
    for event in &journal.events {
        if let Event::Record(record) = event
            && !retired.contains(record.id.as_str())
        {
            *active
                .entry((record.kind, record.scope.as_str()))
                .or_insert(0) += 1;
        }
    }
    for event in &journal.events {
        let Event::Record(record) = event else {
            continue;
        };
        let (state, explanation) = if retired.contains(record.id.as_str()) {
            (
                "retired",
                "This record was revoked or superseded; history is retained",
            )
        } else if active
            .get(&(record.kind, record.scope.as_str()))
            .is_some_and(|count| *count > 1)
        {
            (
                "conflicting",
                "Multiple active records describe this scope; resolve their actual authority rather than selecting a favorable result",
            )
        } else if record.subject != *subject {
            (
                "stale",
                "The supplied source, configuration, stage or artifact changed after this record",
            )
        } else if record.observed_at > now || record.expires_at.is_some_and(|end| now >= end) {
            (
                "expired",
                "This observation is expired or dated in the future; obtain a current observation",
            )
        } else if record.evidence.is_empty() || !record.evidence.iter().all(&mut content_matches) {
            (
                "unresolved",
                "Original supporting evidence is missing, changed or unavailable; do not infer approval or repeat tests solely from this index",
            )
        } else if record.outcome != Outcome::Passed {
            (
                "not_satisfied",
                "The retained observation did not pass; inspect its actual result before continuing",
            )
        } else {
            (
                "recorded",
                "Evidence bytes match the supplied subject; attribution and scope still require the original authority, not this journal",
            )
        };
        findings.push(Finding {
            id: record.id.clone(),
            kind: record.kind,
            state,
            explanation: explanation.into(),
            next_action: record.kind.next_action(),
            original_reference: record.origin.reference.clone(),
        });
    }
    Ok(findings)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn record(id: &str, kind: Kind) -> Record {
        Record {
            id: id.into(),
            kind,
            subject: Subject {
                schema: 1,
                source_sha256: "a".repeat(64),
                configuration_sha256: "b".repeat(64),
                stage: "pre_merge".into(),
                artifact_sha256: None,
            },
            scope: "neutral accepted outcome".into(),
            origin: Origin {
                reference: "tracker:neutral/decision/1".into(),
                actor: "Fixture developer".into(),
                human_attributed: true,
            },
            evidence: vec![ContentReference {
                path: "decision.txt".into(),
                sha256: "c".repeat(64),
            }],
            observed_at: 10,
            expires_at: Some(100),
            outcome: Outcome::Passed,
            supersedes: None,
        }
    }

    fn journal(records: Vec<Record>) -> Journal {
        Journal {
            schema: 1,
            protocol: "workflow.v1".into(),
            work: "tracker:neutral/work/1".into(),
            events: records.into_iter().map(Event::Record).collect(),
        }
    }

    #[test]
    fn changed_subject_missing_evidence_and_expiry_never_become_readiness() -> Result<()> {
        let record = record("execution-1", Kind::Execution);
        let subject = record.subject.clone();
        let journal = journal(vec![record]);
        assert_eq!(
            project(&journal, &subject, 20, |_| true)?[0].state,
            "recorded"
        );
        assert_eq!(
            project(&journal, &subject, 100, |_| true)?[0].state,
            "expired"
        );
        assert_eq!(
            project(&journal, &subject, 5, |_| true)?[0].state,
            "expired"
        );
        assert_eq!(
            project(&journal, &subject, 20, |_| false)?[0].state,
            "unresolved"
        );
        for field in [
            "source_sha256",
            "configuration_sha256",
            "stage",
            "artifact_sha256",
        ] {
            let mut changed = serde_json::to_value(&subject)?;
            changed[field] = if field == "stage" {
                "post_merge".into()
            } else {
                "d".repeat(64).into()
            };
            assert_eq!(
                project(&journal, &serde_json::from_value(changed)?, 20, |_| true)?[0].state,
                "stale",
                "{field}"
            );
        }
        Ok(())
    }

    #[test]
    fn decisions_do_not_change_kind_and_conflicts_need_explicit_resolution() -> Result<()> {
        let permission = record("permission", Kind::ExecutionPermission);
        let subject = permission.subject.clone();
        let mut feedback = record("feedback", Kind::UserFeedback);
        feedback.supersedes = Some("permission".into());
        assert!(
            journal(vec![permission.clone(), feedback])
                .validate()
                .is_err()
        );
        let mut agent = record("self-approval", Kind::ImplementationApproval);
        agent.origin.human_attributed = false;
        assert!(journal(vec![agent]).validate().is_err());
        let mut next = permission.clone();
        next.id = "permission-2".into();
        let conflict = journal(vec![permission.clone(), next.clone()]);
        assert!(
            project(&conflict, &subject, 20, |_| true)?
                .iter()
                .all(|r| r.state == "conflicting")
        );
        next.supersedes = Some(permission.id.clone());
        let mut resolved = journal(vec![permission, next]);
        assert_eq!(
            project(&resolved, &subject, 20, |_| true)?[0].state,
            "retired"
        );
        assert_eq!(
            project(&resolved, &subject, 20, |_| true)?[1].state,
            "recorded"
        );
        resolved.events.push(Event::Revoke(Revocation {
            id: "revocation".into(),
            target: "permission-2".into(),
            origin: record("origin", Kind::ExecutionPermission).origin,
            reason: "User narrowed authorization".into(),
            observed_at: 21,
        }));
        assert!(
            project(&resolved, &subject, 22, |_| true)?
                .iter()
                .all(|r| r.state == "retired")
        );
        assert_eq!(resolved.events.len(), 3);
        Ok(())
    }

    #[test]
    fn unsupported_fields_and_versions_preserve_semantics() -> Result<()> {
        let journal = journal(vec![record("execution", Kind::Execution)]);
        let mut value = serde_json::to_value(&journal)?;
        value["automatic_approval"] = true.into();
        assert!(serde_json::from_value::<Journal>(value).is_err());
        let mut future = journal.clone();
        future.schema = 2;
        assert!(future.validate().is_err());
        future = journal.clone();
        future.events.push(future.events[0].clone());
        assert!(future.validate().is_err());
        Ok(())
    }
}
