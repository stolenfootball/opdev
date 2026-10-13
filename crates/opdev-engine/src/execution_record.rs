//! Versioned canonical execution records, accepted only through a provider channel.

use std::collections::BTreeSet;

use opdev_core::Outcome;
use opdev_project::{CiProvider, TestStage};
use opdev_remote::ProducerSnapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::CheckResult;

/// Exact prefix emitted by the canonical wrapper after it has captured output.
pub(crate) const RECORD_PREFIX: &str = "OPDEV_EXECUTION_V1 ";

/// Complete reviewed identity required for a same-run canonical result.
///
/// Digests bind bytes, not the truth or completeness of review claims. Unknown
/// environment/input scope cannot be represented as an empty matching identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionBinding {
    /// Provider-owned repository slug.
    pub repository: String,
    /// GitLab or GitHub.
    pub provider: CiProvider,
    /// Exact pipeline/run, not a latest-ref artifact lookup.
    pub run_id: u64,
    /// GitHub run attempt; GitLab uses provider-owned job IDs.
    pub run_attempt: Option<u64>,
    /// Full source commit.
    pub revision: String,
    /// Exact execution stage; cross-boundary equivalence needs separate native-candidate validation.
    pub stage: TestStage,
    /// Canonical suite ID.
    pub suite: String,
    /// Exact selected producer job name, including matrix suffixes.
    pub producer: String,
    /// Serialized canonical command specification, including argv/wdir/timeout.
    pub command_sha256: String,
    /// Complete source/input identity under the reviewed policy.
    pub inputs_sha256: String,
    /// Effective project and CI configuration identity.
    pub configuration_sha256: String,
    /// Reviewed environment identity, never a dump or hash of secret values.
    pub environment: String,
    /// Actual canonical executable bytes, not merely its PATH name.
    pub executable_sha256: String,
    /// Canonical wrapper executable bytes.
    pub executor_sha256: String,
    /// Exact reviewed producer/input policy bytes.
    pub policy_sha256: String,
    /// False only under explicit review that commands and dependencies do not read the ledger.
    pub ledger_is_input: bool,
}

impl ExecutionBinding {
    pub(crate) fn validate(&self) -> Result<(), String> {
        let digest =
            |value: &str| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit());
        let text = |value: &str| {
            !value.trim().is_empty() && value.len() <= 1024 && !value.chars().any(char::is_control)
        };
        if !matches!(self.provider, CiProvider::Github | CiProvider::Gitlab)
            || self.run_id == 0
            || (self.provider == CiProvider::Github && self.run_attempt.is_none_or(|n| n == 0))
            || (self.provider == CiProvider::Gitlab && self.run_attempt.is_some())
            || !matches!(self.revision.len(), 40 | 64)
            || !self.revision.bytes().all(|b| b.is_ascii_hexdigit())
            || [
                &self.repository,
                &self.suite,
                &self.producer,
                &self.environment,
            ]
            .iter()
            .any(|v| !text(v))
            || [
                &self.command_sha256,
                &self.inputs_sha256,
                &self.configuration_sha256,
                &self.executable_sha256,
                &self.executor_sha256,
                &self.policy_sha256,
            ]
            .iter()
            .any(|v| !digest(v))
        {
            return Err("Execution identity is incomplete or unsupported; establish reviewed inputs and producer policy or use fresh execution".into());
        }
        Ok(())
    }
}

/// One observed canonical execution. Deserialization alone never qualifies it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRecord {
    /// Independent schema; legacy diagnostic receipts are not this protocol.
    pub schema: u32,
    /// Schema-2 trusted producer completion time; absent preserves legacy records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
    /// Identity observed by the reviewed canonical wrapper.
    pub binding: ExecutionBinding,
    /// Whether relevant inputs remained stable during execution.
    pub inputs_unchanged: bool,
    /// Actual suite result; policy fields are cross-checked, not trusted verbatim.
    pub result: CheckResult,
}

/// Results that have passed provider-channel and exact-binding validation.
/// No public constructor or Deserialize implementation accepts caller assertions.
pub struct ValidatedExecutions {
    pub(crate) checks: Vec<CheckResult>,
    pub(crate) bindings: Vec<ExecutionBinding>,
    pub(crate) validated_at: std::time::Instant,
}

impl ValidatedExecutions {
    /// Recheck the current local subject without running suites or contacting CI.
    ///
    /// # Errors
    /// Changed inputs or an expired immediate-observation window are rejected.
    pub fn verify_subject(
        &self,
        root: &std::path::Path,
        manifest: &opdev_project::ProjectManifest,
        stage: TestStage,
    ) -> Result<(), String> {
        current_subject(root, manifest, stage, self, true)
    }
    /// Inspect actual outcomes, including failed and errored attempts.
    #[must_use]
    pub fn checks(&self) -> &[CheckResult] {
        &self.checks
    }

    /// Inspect the exact execution subjects without changing them.
    #[must_use]
    pub fn bindings(&self) -> &[ExecutionBinding] {
        &self.bindings
    }
}

/// Validate records fetched from provider-owned current job logs. This function
/// performs no network requests, filesystem writes or command execution.
///
/// Expected identities must come from the current source and reviewed policy,
/// never copied from the receipt being checked. The provider channel authenticates
/// job ownership only, not policy adequacy or a compromised trusted producer.
///
/// # Errors
/// Missing, ambiguous, stale or unsupported records cannot qualify. A current
/// producer failure is preserved; an old successful attempt is never substituted.
pub fn validate_producer_records(
    snapshot: &ProducerSnapshot,
    expected: &[ExecutionBinding],
) -> Result<ValidatedExecutions, String> {
    let mut suites = BTreeSet::new();
    let mut checks = Vec::new();
    if expected.is_empty() {
        return Err("No canonical executions were selected for reuse".into());
    }
    for binding in expected {
        binding.validate()?;
        if !suites.insert(&binding.suite) {
            return Err(
                "Duplicate expected suite identity; reuse requires one exact result per suite"
                    .into(),
            );
        }
        if snapshot.repository() != binding.repository
            || snapshot.provider() != binding.provider
            || snapshot.expected().run_id != binding.run_id
            || snapshot.expected().revision != binding.revision
            || snapshot.attempt() != binding.run_attempt
        {
            return Err(
                "Execution belongs to another repository, source, run or current attempt".into(),
            );
        }
        let jobs: Vec<_> = snapshot
            .jobs()
            .current
            .iter()
            .filter(|job| job.name == binding.producer)
            .collect();
        if jobs.len() != 1 {
            checks.push(unavailable_result(
                binding,
                Outcome::Unverified,
                format!(
                    "Required producer {:?} is missing or ambiguous; no saved result was used",
                    binding.producer
                ),
            ));
            continue;
        }
        let job = jobs[0];
        let provider_outcome = match (
            binding.provider,
            job.status.as_str(),
            job.conclusion.as_deref(),
        ) {
            (CiProvider::Github, "completed", Some("success"))
            | (CiProvider::Gitlab, "success", _) => Outcome::Passed,
            (
                CiProvider::Github,
                "completed",
                Some("failure" | "cancelled" | "timed_out" | "startup_failure"),
            )
            | (CiProvider::Gitlab, "failed" | "canceled", _) => Outcome::Failed,
            _ => {
                checks.push(unavailable_result(
                    binding,
                    Outcome::Unverified,
                    format!(
                        "Required producer {:?} has not completed a qualifying attempt",
                        binding.producer
                    ),
                ));
                continue;
            }
        };
        let log = snapshot.log(job.id).ok_or_else(|| {
            format!(
                "Retained execution log for producer {:?} is unavailable (provider status {:?}, conclusion {:?}): {}",
                binding.producer, job.status, job.conclusion,
                snapshot.log_error(job.id).unwrap_or("no retained execution log")
            )
        });
        let mut result = observed_result(binding, provider_outcome, log);
        if provider_outcome == Outcome::Failed && result.outcome == Outcome::Passed {
            result.outcome = Outcome::Failed;
            result.summary = "The canonical command reported success but its current producer job failed; this attempt cannot qualify".into();
        }
        result.evidence.push(opdev_core::Evidence {
            kind: "same_run_producer".into(),
            summary: format!("Provider-owned job {} in run {}; current attempt {:?}; result states whether canonical execution was verified", job.id, binding.run_id, binding.run_attempt),
            location: Some(format!("{}:{}:job:{}", match binding.provider { CiProvider::Github => "github", _ => "gitlab" }, binding.repository, job.id)),
        });
        checks.push(result);
    }
    Ok(ValidatedExecutions {
        checks,
        bindings: expected.to_vec(),
        validated_at: std::time::Instant::now(),
    })
}

fn unavailable_result(binding: &ExecutionBinding, outcome: Outcome, reason: String) -> CheckResult {
    CheckResult {
        id: binding.suite.clone(),
        kind: crate::CheckKind::Suite,
        blocking: true,
        gates: crate::evaluator::gates_for_test_stage(binding.stage),
        outcome,
        summary: reason,
        evidence: vec![],
        stdout: None,
        stderr: None,
        duration_ms: None,
    }
}

/// Reuse only equivalent source checks from an authenticated native candidate.
/// Current post-merge acceptance, deployment and artifact qualification stay separate.
/// # Errors
/// Missing correspondence, review, complete inputs, time bounds or canonical records fails closed.
pub fn validate_candidate_records(
    root: &std::path::Path,
    manifest: &opdev_project::ProjectManifest,
    policy: &crate::ExecutionPolicy,
    candidate: &opdev_remote::IntegratedCandidate,
    snapshot: &ProducerSnapshot,
    destination: &[ExecutionBinding],
) -> Result<ValidatedExecutions, String> {
    policy
        .validate()
        .map_err(|_| "Native candidate reuse policy is incomplete or unsupported")?;
    if manifest
        .assurance
        .engineering
        .as_ref()
        .is_none_or(|p| p.version != "3")
        || policy.schema != 2
    {
        return Err("Cross-boundary candidate reuse needs explicitly selected engineering policy 3 and execution policy 2".into());
    }
    let selected = policy
        .candidate_reuse
        .as_ref()
        .ok_or("Candidate reuse policy is absent")?;
    candidate.verify_checkout(root)?;
    let selection = candidate.selection();
    let (targets, sources) = candidate_bindings(
        manifest,
        policy,
        selection,
        candidate.repository(),
        candidate.attempt(),
        destination,
    )?;
    let mut executions = validate_producer_records(snapshot, &sources)?;
    if executions
        .checks
        .iter()
        .any(|c| c.outcome == Outcome::Unverified)
    {
        return Err(
            "A native candidate producer has no verified current execution; use fresh checks"
                .into(),
        );
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "Current clock is unavailable")?
        .as_secs();
    for source in &sources {
        let job = snapshot
            .jobs()
            .current
            .iter()
            .find(|job| job.name == source.producer)
            .ok_or("Candidate producer is missing")?;
        let log = snapshot
            .log(job.id)
            .ok_or("Candidate producer log is unavailable")?;
        current_candidate_time(log, source, now, selected.maximum_age_seconds)?;
    }
    for check in &mut executions.checks {
        check.gates = crate::evaluator::gates_for_test_stage(TestStage::PostMerge);
        check.evidence.push(opdev_core::Evidence { kind:"native_candidate_execution".into(),
            summary:format!("Source check executed before integration in native run {} at {}; equivalent to integrated {} under the reviewed source-only policy. This is not a post-merge execution or artifact/deployment qualification.", selection.run.run_id, selection.run.revision, selection.integrated_revision),
            location:Some(format!("run:{}", selection.run.run_id)) });
    }
    // Destination bindings protect the current qualification subject; the actual
    // earlier execution identity remains explicit in provider evidence above.
    executions.bindings = targets;
    executions.verify_subject(root, manifest, TestStage::PostMerge)?;
    Ok(executions)
}

// Private derivation only: selectors never become authenticated observations.
fn candidate_bindings(
    manifest: &opdev_project::ProjectManifest,
    policy: &crate::ExecutionPolicy,
    selection: &opdev_remote::CandidateSelection,
    repository: &str,
    attempt: Option<u64>,
    destination: &[ExecutionBinding],
) -> Result<(Vec<ExecutionBinding>, Vec<ExecutionBinding>), String> {
    let selected = policy
        .candidate_reuse
        .as_ref()
        .ok_or("Candidate reuse policy is absent")?;
    let policy_digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(policy).map_err(|_| "Reuse policy cannot be encoded")?)
    );
    let mut targets = Vec::new();
    let mut sources = Vec::new();
    for target in destination
        .iter()
        .filter(|b| selected.suites.contains(&b.suite))
    {
        if target.stage != TestStage::PostMerge
            || target.revision != selection.integrated_revision
            || target.repository != repository
            || target.environment != policy.environment
            || target.policy_sha256 != policy_digest
            || !selected.source_only
            || !selected.commit_ref_stage_independent
            || !policy.inputs_complete
            || selected.review_reference.trim().is_empty()
            || !(1..=86_400).contains(&selected.maximum_age_seconds)
            || !crate::plan::selected_suites(manifest, TestStage::PreMerge)
                .any(|s| s.id == target.suite)
        {
            return Err("Candidate command is not a reviewed equivalent source check at both boundaries; execute it freshly".into());
        }
        let mut source = target.clone();
        source.stage = TestStage::PreMerge;
        source.revision.clone_from(&selection.run.revision);
        source.run_id = selection.run.run_id;
        source.run_attempt = attempt;
        targets.push(target.clone());
        sources.push(source);
    }
    if sources.is_empty() {
        return Err("No eligible native-candidate suite is selected at this boundary".into());
    }
    Ok((targets, sources))
}

fn current_candidate_time(
    log: &str,
    binding: &ExecutionBinding,
    now: u64,
    maximum_age: u64,
) -> Result<(), String> {
    validate_log(log, binding)?;
    let record = log
        .lines()
        .map(strip_timestamp)
        .filter_map(|line| line.strip_prefix(RECORD_PREFIX))
        .filter_map(|payload| serde_json::from_str::<ExecutionRecord>(payload).ok())
        .find(|record| record.binding == *binding)
        .ok_or("Candidate execution record is absent")?;
    let completed = record
        .completed_at
        .filter(|_| record.schema == 2)
        .ok_or("Candidate producer has no supported completion time; execute fresh checks")?;
    if !(1..=86_400).contains(&maximum_age)
        || completed == 0
        || completed > now
        || now - completed > maximum_age
    {
        return Err(
            "Candidate execution is expired or its clock is invalid; execute fresh checks".into(),
        );
    }
    Ok(())
}

fn observed_result(
    binding: &ExecutionBinding,
    provider_outcome: Outcome,
    log: Result<&str, String>,
) -> CheckResult {
    match log.and_then(|log| validate_log(log, binding)) {
        Ok(result) => result,
        Err(reason) => unavailable_result(
            binding,
            if provider_outcome == Outcome::Failed {
                Outcome::Failed
            } else {
                Outcome::Unverified
            },
            if provider_outcome == Outcome::Failed {
                format!(
                    "The current producer job failed; its canonical test result is unknown: {reason}. Inspect that job; an older green attempt cannot replace it"
                )
            } else {
                format!(
                    "Canonical execution could not be verified: {reason}. Restore current evidence or explicitly run fresh checks"
                )
            },
        ),
    }
}

pub(crate) fn current_subject(
    root: &std::path::Path,
    manifest: &opdev_project::ProjectManifest,
    stage: TestStage,
    executions: &ValidatedExecutions,
    require_recent: bool,
) -> Result<(), String> {
    if require_recent && executions.validated_at.elapsed() > std::time::Duration::from_secs(30) {
        return Err("Provider observations are older than this immediate evaluation window; re-observe the current attempts or use fresh execution".into());
    }
    let inputs = opdev_project::staged_fingerprint(root)
        .map_err(|_| "Current source inputs cannot be identified")?;
    let configuration = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(manifest).map_err(|_| "Current configuration cannot be encoded")?
        )
    );
    for binding in &executions.bindings {
        let revision =
            crate::test_execution::clean_execution_revision(root, binding.ledger_is_input)
                .map_err(|_| "Current execution inputs are not clean and committed")?;
        let suite = crate::plan::selected_suites(manifest, stage)
            .find(|suite| suite.id == binding.suite)
            .ok_or("Reused execution is not selected in this stage")?;
        let command = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&manifest.commands[&suite.command])
                    .map_err(|_| "Canonical command cannot be encoded")?
            )
        );
        if binding.revision != revision
            || binding.inputs_sha256 != inputs
            || binding.configuration_sha256 != configuration
            || binding.command_sha256 != command
            || binding.stage != stage
            || binding.provider != manifest.project.ci.provider
        {
            return Err("Source, configuration, canonical command or stage changed after execution verification; no saved pass was applied".into());
        }
    }
    Ok(())
}

fn validate_log(log: &str, expected: &ExecutionBinding) -> Result<CheckResult, String> {
    let mut selected = None;
    for line in log.lines() {
        // GitHub prefixes emitted lines with an ISO timestamp. Do not accept
        // arbitrary embedded markers in echoed shell commands or test output.
        let line = strip_timestamp(line);
        if let Some(payload) = line.strip_prefix(RECORD_PREFIX) {
            let record: ExecutionRecord = serde_json::from_str(payload).map_err(|_| {
                "Producer execution record is malformed or uses unsupported fields".to_owned()
            })?;
            if !((record.schema == 1 && record.completed_at.is_none())
                || (record.schema == 2 && record.completed_at.is_some_and(|n| n > 0)))
            {
                return Err("Execution record version is unsupported; use a compatible producer or fresh execution".into());
            }
            if record.binding.suite != expected.suite {
                continue;
            }
            if selected.is_some() {
                return Err("Producer log contains multiple records for the same suite; attempt history is ambiguous".into());
            }
            if record.binding != *expected || !record.inputs_unchanged {
                return Err("Execution inputs, configuration, stage, toolchain or producer changed; run this check again".into());
            }
            if record.result.id != expected.suite
                || record.result.kind != crate::CheckKind::Suite
                || !record.result.blocking
                || record.result.gates != crate::evaluator::gates_for_test_stage(expected.stage)
                || !matches!(
                    record.result.outcome,
                    Outcome::Passed | Outcome::Failed | Outcome::Error
                )
            {
                return Err(
                    "Execution result does not describe the required canonical suite and gate"
                        .into(),
                );
            }
            selected = Some(record.result);
        }
    }
    selected.ok_or_else(|| "No canonical execution record was found in the required producer log; legacy receipts remain diagnostic only".into())
}

fn strip_timestamp(line: &str) -> &str {
    let Some((prefix, body)) = line.split_once(' ') else {
        return line;
    };
    if (20..=35).contains(&prefix.len())
        && prefix.ends_with('Z')
        && prefix.as_bytes().get(10) == Some(&b'T')
        && prefix
            .bytes()
            .all(|b| b.is_ascii_digit() || b"-:.TZ".contains(&b))
    {
        body
    } else {
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)] // One unchanged fixture contrasts every identity field.
    fn candidate_mapping_changes_only_native_execution_identity_and_rejects_input_drift()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(root.path())
                .args(["init", "--quiet"])
                .status()?
                .success()
        );
        let mut manifest = opdev_project::discover(root.path())?.manifest;
        manifest.testing.suites = vec![opdev_project::TestSuite {
            id: "unit".into(),
            command: "check".into(),
            stages: vec![TestStage::PreMerge, TestStage::PostMerge],
        }];
        let mut original = record();
        original.schema = 2;
        original.completed_at = Some(1940);
        let policy: crate::ExecutionPolicy = serde_json::from_value(serde_json::json!({
            "schema":2, "review_reference":"fixture input review", "inputs_complete":true,
            "environment":original.binding.environment,"executor_sha256":original.binding.executor_sha256,
            "producers":[{"suite":"unit","job":"unit-job","executable_sha256":original.binding.executable_sha256}],
            "candidate_reuse":{"review_reference":"fixture equivalence review","suites":["unit"],
                "source_only":true,"commit_ref_stage_independent":true,"maximum_age_seconds":60}
        }))?;
        policy.validate()?;
        original.binding.policy_sha256 =
            format!("{:x}", Sha256::digest(serde_json::to_vec(&policy)?));
        let selector = opdev_remote::CandidateSelection {
            change: 1,
            integrated_revision: "9".repeat(40),
            run: opdev_remote::RunExpectation {
                revision: original.binding.revision.clone(),
                run_id: original.binding.run_id,
                reference: "refs/merge-requests/1/train".into(),
                source: "merge_request_event".into(),
                workflow_id: None,
            },
        };
        let mut target = original.binding.clone();
        target.revision.clone_from(&selector.integrated_revision);
        target.run_id = 20;
        target.stage = TestStage::PostMerge;
        let derive = |target: &ExecutionBinding| {
            candidate_bindings(
                &manifest,
                &policy,
                &selector,
                "neutral/project",
                None,
                std::slice::from_ref(target),
            )
        };
        let (targets, sources) = derive(&target)?;
        assert_eq!(targets, vec![target.clone()]);
        assert_eq!(sources, vec![original.binding.clone()]);
        let bytes = log(&original)?;
        current_candidate_time(&bytes, &sources[0], 2000, 60)?;
        // Even when native trees match, no input may be copied from the old log
        // to disguise a changed current command, environment or producer.
        for field in [
            "command_sha256",
            "inputs_sha256",
            "configuration_sha256",
            "environment",
            "executable_sha256",
            "executor_sha256",
            "policy_sha256",
            "producer",
            "ledger_is_input",
            "repository",
            "revision",
            "stage",
        ] {
            let mut changed = serde_json::to_value(&target)?;
            changed[field] = match field {
                "ledger_is_input" => serde_json::json!(false),
                "stage" => serde_json::json!("pre_merge"),
                "revision" => serde_json::json!("8".repeat(40)),
                "environment" | "producer" | "repository" => serde_json::json!("different"),
                _ => serde_json::json!("8".repeat(64)),
            };
            let changed = serde_json::from_value(changed)?;
            if let Ok((_, mapped)) = derive(&changed) {
                assert!(
                    current_candidate_time(&bytes, &mapped[0], 2000, 60).is_err(),
                    "accepted {field}"
                );
            }
        }
        manifest.testing.suites[0].stages = vec![TestStage::PostMerge];
        assert!(
            candidate_bindings(
                &manifest,
                &policy,
                &selector,
                "neutral/project",
                None,
                &[target]
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn candidate_freshness_requires_exact_current_supported_record()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut record = record();
        assert!(current_candidate_time(&log(&record)?, &record.binding, 2000, 60).is_err());
        record.schema = 2;
        record.completed_at = Some(1940);
        let bytes = log(&record)?;
        current_candidate_time(&bytes, &record.binding, 2000, 60)?;
        assert!(current_candidate_time(&bytes, &record.binding, 2001, 60).is_err());
        assert!(current_candidate_time(&bytes, &record.binding, 1939, 60).is_err());
        assert!(current_candidate_time(&bytes, &record.binding, 2000, 0).is_err());
        assert!(current_candidate_time(&bytes, &record.binding, 2000, 86401).is_err());
        assert!(
            current_candidate_time(&format!("{bytes}\n{bytes}"), &record.binding, 2000, 60)
                .is_err()
        );
        let mut other = record.binding.clone();
        other.command_sha256 = "9".repeat(64);
        assert!(current_candidate_time(&bytes, &other, 2000, 60).is_err());
        record.inputs_unchanged = false;
        assert!(current_candidate_time(&log(&record)?, &record.binding, 2000, 60).is_err());
        Ok(())
    }

    fn record() -> ExecutionRecord {
        let binding = ExecutionBinding {
            repository: "neutral/project".into(),
            provider: CiProvider::Gitlab,
            run_id: 10,
            run_attempt: None,
            revision: "a".repeat(40),
            stage: TestStage::PreMerge,
            suite: "unit".into(),
            producer: "unit-job".into(),
            command_sha256: "a".repeat(64),
            inputs_sha256: "b".repeat(64),
            configuration_sha256: "c".repeat(64),
            environment: "image@sha256:fixed".into(),
            executable_sha256: "d".repeat(64),
            executor_sha256: "e".repeat(64),
            policy_sha256: "f".repeat(64),
            ledger_is_input: true,
        };
        ExecutionRecord {
            schema: 1,
            completed_at: None,
            binding,
            inputs_unchanged: true,
            result: CheckResult {
                id: "unit".into(),
                kind: crate::CheckKind::Suite,
                blocking: true,
                gates: crate::evaluator::gates_for_test_stage(TestStage::PreMerge),
                outcome: Outcome::Passed,
                summary: "Canonical suite exited successfully".into(),
                evidence: vec![],
                stdout: None,
                stderr: None,
                duration_ms: Some(12),
            },
        }
    }

    fn log(record: &ExecutionRecord) -> Result<String, serde_json::Error> {
        Ok(format!("{RECORD_PREFIX}{}", serde_json::to_string(record)?))
    }

    #[test]
    fn exact_records_preserve_failures_errors_and_output() -> Result<(), Box<dyn std::error::Error>>
    {
        for outcome in [Outcome::Passed, Outcome::Failed, Outcome::Error] {
            let mut record = record();
            record.result.outcome = outcome;
            record.result.stderr = Some("bounded diagnostic".into());
            record.binding.validate()?;
            let schema: serde_json::Value =
                serde_json::from_str(include_str!("../../../schema/execution-record.schema.json"))?;
            assert!(jsonschema::is_valid(
                &schema,
                &serde_json::to_value(&record)?
            ));
            assert_eq!(
                validate_log(&log(&record)?, &record.binding)?,
                record.result
            );
            assert_eq!(
                validate_log(
                    &format!("2026-10-05T12:00:00.123Z {}", log(&record)?),
                    &record.binding
                )?,
                record.result
            );
        }
        Ok(())
    }

    #[test]
    fn changed_binding_never_reuses_a_green_record() -> Result<(), Box<dyn std::error::Error>> {
        let record = record();
        let bytes = log(&record)?;
        for field in [
            "repository",
            "producer",
            "revision",
            "suite",
            "command_sha256",
            "inputs_sha256",
            "configuration_sha256",
            "environment",
            "executable_sha256",
            "executor_sha256",
            "policy_sha256",
        ] {
            let mut value = serde_json::to_value(&record.binding)?;
            value[field] = "changed".into();
            let wrong = serde_json::from_value(value)?;
            assert!(validate_log(&bytes, &wrong).is_err(), "{field}");
        }
        let mut wrong = record.binding.clone();
        wrong.stage = TestStage::PostMerge;
        assert!(validate_log(&bytes, &wrong).is_err());
        wrong = record.binding.clone();
        wrong.run_id += 1;
        assert!(validate_log(&bytes, &wrong).is_err());
        Ok(())
    }

    #[test]
    fn ambiguous_legacy_unknown_and_policy_spoofing_do_not_qualify()
    -> Result<(), Box<dyn std::error::Error>> {
        let record = record();
        let valid = log(&record)?;
        for invalid in [
            format!("{valid}\n{valid}"),
            format!("echo '{valid}'"),
            "{\"schema\":2,\"qualification\":\"unverified\"}".into(),
        ] {
            assert!(validate_log(&invalid, &record.binding).is_err());
        }
        for field in [
            "schema",
            "inputs_unchanged",
            "blocking",
            "gates",
            "outcome",
            "extra",
        ] {
            let mut value = serde_json::to_value(&record)?;
            match field {
                "schema" => value[field] = 2.into(),
                "inputs_unchanged" => value[field] = false.into(),
                "blocking" => value["result"][field] = false.into(),
                "gates" => value["result"][field] = serde_json::json!([]),
                "outcome" => value["result"][field] = "not_applicable".into(),
                _ => value[field] = true.into(),
            }
            assert!(
                validate_log(&format!("{RECORD_PREFIX}{value}"), &record.binding).is_err(),
                "{field}"
            );
        }
        Ok(())
    }

    #[test]
    fn missing_or_malformed_logs_preserve_known_producer_failure() {
        let binding = record().binding;
        for log in [
            Err("log expired".into()),
            Ok("OPDEV_EXECUTION_V1 malformed"),
        ] {
            let failed = observed_result(&binding, Outcome::Failed, log.clone());
            assert_eq!(failed.outcome, Outcome::Failed);
            assert!(failed.summary.contains("canonical test result is unknown"));
            let unknown = observed_result(&binding, Outcome::Passed, log);
            assert_eq!(unknown.outcome, Outcome::Unverified);
        }
    }
}
