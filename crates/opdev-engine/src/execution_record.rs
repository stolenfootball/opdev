//! Versioned canonical execution records, accepted only through a provider channel.

use std::collections::BTreeSet;

use opdev_core::Outcome;
use opdev_project::{CiProvider, TestStage};
use opdev_remote::ProducerSnapshot;
use serde::{Deserialize, Serialize};

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
    /// Exact suite stage; pre-merge never qualifies post-merge.
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
    use sha2::{Digest, Sha256};
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
            if record.schema != 1 {
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
