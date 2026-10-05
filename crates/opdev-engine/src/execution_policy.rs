//! Opt-in reviewed producer policy; no automatic adoption or inferred inputs.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

use anyhow::{Context, Result, ensure};
use opdev_project::{ProjectManifest, TestStage};
use opdev_remote::RunExpectation;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    CheckKind, ExecutionBinding, ExecutionRecord, ProgramLocation, execute, inspect_program,
};

/// Reviewed same-run reuse policy, separate from legacy diagnostic receipts.
///
/// Explicit policy records a trust assumption; `OpDev` cannot authenticate its
/// alleged reviewer or infer complete external inputs from these fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPolicy {
    /// Independent policy version.
    pub schema: u32,
    /// Durable actual decision/reference reviewing producer configuration and inputs.
    pub review_reference: String,
    /// Explicit reviewed completeness assertion; false prevents reuse.
    pub inputs_complete: bool,
    /// Conservative default; false requires review that commands do not read the ledger.
    #[serde(default = "ledger_is_input_by_default")]
    pub ledger_is_input: bool,
    /// Nonsecret immutable environment/toolchain/dependency identity.
    pub environment: String,
    /// Pinned wrapper executable digest.
    pub executor_sha256: String,
    /// Expected GitHub workflow ID; absent for GitLab.
    pub github_workflow_id: Option<u64>,
    /// Required suite-to-producer mappings; no command inference from job names.
    pub producers: Vec<ProducerPolicy>,
}

/// One reviewed canonical suite producer in the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerPolicy {
    /// Canonical suite ID in the project contract.
    pub suite: String,
    /// Exact provider job name, including matrix suffixes.
    pub job: String,
    /// Expected resolved executable digest; runtime dependencies belong to environment review.
    pub executable_sha256: String,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

const fn ledger_is_input_by_default() -> bool {
    true
}

/// Load a strictly versioned policy from a reviewed tracked project file.
impl ExecutionPolicy {
    /// Read without creating or migrating policy. Unknown fields/versions fail.
    ///
    /// # Errors
    /// Rejects traversal, links, oversized input, unsupported fields and unresolved review.
    pub fn load(root: &Path, relative: &Path) -> Result<Self> {
        ensure!(
            !relative.as_os_str().is_empty()
                && relative
                    .components()
                    .all(|c| matches!(c, Component::Normal(_))),
            "execution policy must be a relative project file without traversal"
        );
        let mut path = root.to_path_buf();
        for part in relative.components() {
            path.push(part);
            ensure!(
                !fs::symlink_metadata(&path)?.file_type().is_symlink(),
                "execution policy must not traverse a symbolic link"
            );
        }
        ensure!(
            fs::metadata(&path)?.len() <= 65_536,
            "execution policy exceeds 64 KiB"
        );
        let tracked = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["ls-files", "--error-unmatch", "--"])
            .arg(relative)
            .output()?;
        ensure!(
            tracked.status.success(),
            "execution policy must be tracked in the reviewed source"
        );
        let policy: Self = serde_json::from_slice(&fs::read(path)?).context(
            "execution policy must be strict JSON; no automatic conversion was attempted",
        )?;
        policy.validate()?;
        Ok(policy)
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == 1,
            "execution policy version is unsupported; keep fresh execution or use a compatible CLI"
        );
        ensure!(
            self.inputs_complete && !self.review_reference.trim().is_empty(),
            "execution input/producer review is incomplete; no result can be reused"
        );
        ensure!(
            !self.environment.trim().is_empty() && !self.environment.chars().any(char::is_control),
            "a nonsecret reviewed environment identity is required"
        );
        ensure!(
            valid_digest(&self.executor_sha256),
            "the reviewed OpDev executable digest must contain exactly 64 hexadecimal characters"
        );
        ensure!(
            self.github_workflow_id != Some(0),
            "the reviewed GitHub workflow ID must be positive"
        );
        let mut suites = BTreeSet::new();
        ensure!(
            !self.producers.is_empty() && self.producers.len() <= 100,
            "select between one and 100 canonical producers"
        );
        for producer in &self.producers {
            ensure!(
                suites.insert(&producer.suite),
                "duplicate canonical suite in execution policy"
            );
            ensure!(
                !producer.job.trim().is_empty()
                    && !producer.job.chars().any(char::is_control)
                    && !producer.suite.trim().is_empty(),
                "producer suite and job identities must be nonempty, with no control characters in the job name"
            );
            ensure!(
                valid_digest(&producer.executable_sha256),
                "the reviewed canonical executable digest must contain exactly 64 hexadecimal characters"
            );
        }
        Ok(())
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Bind reviewed execution inputs from clean committed source, not receipt fields.
///
/// `environment` must be obtained from the reviewed producer's actual environment
/// configuration, not copied from untrusted result JSON. The current CI run is
/// verified separately through the provider; this function performs no network IO.
///
/// # Errors
/// Unknown inputs, dirty source, incompatible stage/policy or different wrapper fail.
#[allow(clippy::too_many_arguments)] // Explicit independent identities, not ambient global context.
pub fn prepare_execution_bindings(
    root: &Path,
    manifest: &ProjectManifest,
    policy: &ExecutionPolicy,
    run: &RunExpectation,
    repository: &str,
    attempt: Option<u64>,
    stage: TestStage,
    environment: &str,
) -> Result<Vec<ExecutionBinding>> {
    policy.validate()?;
    for producer in &policy.producers {
        ensure!(
            manifest
                .testing
                .suites
                .iter()
                .any(|suite| suite.id == producer.suite),
            "execution policy names an undeclared suite {:?}",
            producer.suite
        );
    }
    ensure!(
        environment == policy.environment,
        "current environment differs from the reviewed execution policy; use fresh execution"
    );
    ensure!(
        policy.github_workflow_id == run.workflow_id,
        "workflow identity differs from reviewed producer policy"
    );
    ensure!(
        stage != TestStage::PostMerge
            || (run.reference == manifest.project.trunk && run.source == "push"),
        "post-merge execution requires this project's integration trunk push, not a feature or merge-request subject"
    );
    let revision = crate::test_execution::clean_execution_revision(root, policy.ledger_is_input)?;
    ensure!(
        revision == run.revision,
        "execution source differs from the current CI run"
    );
    let inputs = opdev_project::staged_fingerprint(root)?;
    let executor = hash(&fs::read(std::env::current_exe()?)?);
    ensure!(
        executor == policy.executor_sha256,
        "current OpDev executable differs from the reviewed producer; no saved checks were accepted"
    );
    let configuration = hash(&serde_json::to_vec(manifest)?);
    let policy_hash = hash(&serde_json::to_vec(policy)?);
    let mut bindings = Vec::new();
    for suite in crate::plan::selected_suites(manifest, stage) {
        let Some(producer) = policy.producers.iter().find(|p| p.suite == suite.id) else {
            continue;
        };
        let binding = ExecutionBinding {
            repository: repository.into(),
            provider: manifest.project.ci.provider,
            run_id: run.run_id,
            run_attempt: attempt,
            revision: revision.clone(),
            stage,
            suite: suite.id.clone(),
            producer: producer.job.clone(),
            command_sha256: hash(&serde_json::to_vec(&manifest.commands[&suite.command])?),
            inputs_sha256: inputs.clone(),
            configuration_sha256: configuration.clone(),
            environment: environment.into(),
            executable_sha256: producer.executable_sha256.clone(),
            executor_sha256: executor.clone(),
            policy_sha256: policy_hash.clone(),
            ledger_is_input: policy.ledger_is_input,
        };
        binding.validate().map_err(anyhow::Error::msg)?;
        bindings.push(binding);
    }
    ensure!(
        !bindings.is_empty(),
        "no canonical suites are selected for this stage"
    );
    Ok(bindings)
}

/// Run one canonical suite once and capture a record for its provider job log.
/// It does not evaluate acceptance, update a ledger, or authenticate its own log.
///
/// # Errors
/// Refuses changed source/configuration/executable before launching any command.
pub fn run_canonical_producer(
    root: &Path,
    manifest: &ProjectManifest,
    binding: &ExecutionBinding,
) -> Result<ExecutionRecord> {
    binding.validate().map_err(anyhow::Error::msg)?;
    let suite = crate::plan::selected_suites(manifest, binding.stage)
        .find(|s| s.id == binding.suite)
        .context("selected suite is not declared for this stage")?;
    let command = &manifest.commands[&suite.command];
    ensure!(
        hash(&serde_json::to_vec(command)?) == binding.command_sha256
            && hash(&serde_json::to_vec(manifest)?) == binding.configuration_sha256,
        "canonical command or project configuration changed before execution"
    );
    let program = command.argv.first().context("empty canonical command")?;
    ensure!(
        Path::new(program).is_absolute(),
        "execution reuse currently requires an absolute canonical executable; keep the unchanged fresh-execution path when lookup cannot be bound exactly"
    );
    let ProgramLocation::Located(executable) = inspect_program(program) else {
        anyhow::bail!("canonical executable cannot be identified; no command was run");
    };
    let stable = || -> Result<bool> {
        Ok(
            crate::test_execution::clean_execution_revision(root, binding.ledger_is_input)?
                == binding.revision
                && opdev_project::staged_fingerprint(root)? == binding.inputs_sha256
                && hash(&fs::read(&executable)?) == binding.executable_sha256
                && hash(&fs::read(std::env::current_exe()?)?) == binding.executor_sha256,
        )
    };
    ensure!(
        stable()?,
        "source or executable identity differs from reviewed producer inputs; no command was run"
    );
    let result = crate::evaluator::execution_result(
        binding.suite.clone(),
        CheckKind::Suite,
        true,
        crate::evaluator::gates_for_test_stage(binding.stage),
        execute(root, command, None),
    );
    Ok(ExecutionRecord {
        schema: 1,
        binding: binding.clone(),
        inputs_unchanged: stable().unwrap_or(false),
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy_value() -> serde_json::Value {
        serde_json::json!({
            "schema": 1, "review_reference": "neutral fixture review",
            "inputs_complete": true, "environment": "immutable-neutral-image",
            "executor_sha256": "a".repeat(64), "github_workflow_id": null,
            "producers": [{"suite": "unit", "job": "unit-job", "executable_sha256": "b".repeat(64)}]
        })
    }

    #[test]
    fn ledger_exclusion_is_explicit_and_unknown_policy_is_not_migrated()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut value = policy_value();
        value
            .as_object_mut()
            .ok_or("policy object")?
            .remove("github_workflow_id");
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/execution-policy.schema.json"))?;
        assert!(jsonschema::is_valid(&schema, &value));
        let default: ExecutionPolicy = serde_json::from_value(value.clone())?;
        assert!(default.ledger_is_input);
        assert_eq!(default.github_workflow_id, None);
        default.validate()?;
        value["ledger_is_input"] = false.into();
        let reviewed: ExecutionPolicy = serde_json::from_value(value.clone())?;
        assert!(!reviewed.ledger_is_input);
        reviewed.validate()?;
        value["implicit_exclusions"] = serde_json::json!(["docs/"]);
        assert!(serde_json::from_value::<ExecutionPolicy>(value).is_err());
        Ok(())
    }

    #[test]
    fn invalid_policies_fail_before_execution_binding() -> Result<(), Box<dyn std::error::Error>> {
        for (key, invalid) in [
            ("schema", serde_json::json!(99)),
            ("inputs_complete", serde_json::json!(false)),
            ("review_reference", serde_json::json!(" ")),
            ("executor_sha256", serde_json::json!("not-a-digest")),
            ("github_workflow_id", serde_json::json!(0)),
            ("environment", serde_json::json!("bad\nidentity")),
        ] {
            let mut value = policy_value();
            value[key] = invalid;
            let policy: ExecutionPolicy = serde_json::from_value(value)?;
            assert!(policy.validate().is_err(), "accepted invalid {key}");
        }
        let mut value = policy_value();
        value["producers"][0]["executable_sha256"] = "bad".into();
        assert!(
            serde_json::from_value::<ExecutionPolicy>(value)?
                .validate()
                .is_err()
        );
        Ok(())
    }
}
