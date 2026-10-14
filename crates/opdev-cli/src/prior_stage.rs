//! Prepare prior-stage observations for review; never reuse them as current execution.
use anyhow::{Context, Result, ensure};
use clap::Args;
use opdev_core::{Evidence, Outcome};
use opdev_project::{CiProvider, EvidenceAssertion, TestStage};
use std::{path::PathBuf, process::ExitCode};

#[derive(Debug, Args)]
pub(crate) struct ObserveArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Exact pre-merge review, not a report or latest-green lookup.
    #[arg(long)]
    review_locator: PathBuf,
    #[arg(long)]
    acceptance_sha256: String,
    /// Explicit pipeline/workflow run to inspect using the existing developer login.
    #[arg(long)]
    run: u64,
    #[arg(long = "ref")]
    reference: String,
    #[arg(long)]
    workflow: Option<u64>,
    /// Complete reviewed pre-merge job inventory when it differs from trunk jobs.
    #[arg(long = "required-job", requires = "job_review_reference")]
    required_jobs: Vec<String>,
    /// Actual review reference for the explicitly selected pre-merge inventory.
    #[arg(long, requires = "required_jobs")]
    job_review_reference: Option<String>,
    #[arg(long)]
    output: PathBuf,
}

pub(crate) fn observe(args: &ObserveArgs) -> Result<ExitCode> {
    let (root, manifest) = crate::load_project(&args.root)?;
    ensure!(
        std::env::var_os("CI_JOB_TOKEN").is_none()
            && std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true"),
        "Prepare prior-stage observations locally with existing provider access; do not add developer credentials to CI"
    );
    let revision = crate::clean_remote_revision(&root)
        .context("Prior-stage handoff needs a clean integrated checkout")?;
    let output = crate::evidence_bundle::export_destination(&root, &args.output)?;
    let locator: opdev_remote::DiscussionLocator = serde_json::from_slice(
        &crate::local_state::read(&std::path::absolute(&args.review_locator)?)?
            .context("Review locator missing")?,
    )?;
    let policy = manifest
        .assurance
        .review_storage
        .as_ref()
        .context("MR/PR review policy required")?;
    ensure!(
        policy.version == 2
            && locator.selector.provider == policy.provider
            && locator.selector.repository_id == policy.repository_id,
        "Prior review belongs to another storage policy; no provider request made"
    );
    let observation = opdev_remote::retrieve_discussion(&locator).map_err(anyhow::Error::msg)?;
    observation
        .verify_source(&root, TestStage::PostMerge)
        .map_err(anyhow::Error::msg)?;
    // Both trees must match. A changed merge result requires fresh impact review,
    // never automatic transfer of the earlier source's qualification.
    let review = opdev_engine::ValidatedReview::from_discussion(
        &root,
        &manifest,
        TestStage::PreMerge,
        &args.acceptance_sha256,
        observation,
    )
    .map_err(anyhow::Error::msg)?;
    let defaults = manifest
        .project
        .ci
        .qualification
        .as_ref()
        .map(|policy| policy.required_jobs.as_slice());
    let jobs = select_jobs(args, defaults)?;
    let expected = opdev_remote::RunExpectation {
        revision: locator.source_commit.clone(),
        run_id: args.run,
        reference: args.reference.clone(),
        source: match policy.provider {
            CiProvider::Gitlab => "merge_request_event",
            CiProvider::Github => "pull_request",
            _ => anyhow::bail!("Unsupported provider"),
        }
        .into(),
        workflow_id: args.workflow,
    };
    let result = opdev_remote::verify_run_with_jobs(&manifest, &expected, jobs)?;
    let gaps = review
        .preflight(&root, &manifest, TestStage::PreMerge)
        .map_err(anyhow::Error::msg)?;
    ensure!(
        gaps.is_empty(),
        "Prior review inputs are not ready: {}",
        gaps.join("; ")
    );
    ensure!(
        crate::clean_remote_revision(&root).as_deref() == Some(&revision),
        "Source changed during prior-stage observation; nothing written"
    );
    let prepared = prepare_observation(
        &result,
        policy.repository_id,
        &revision,
        args.job_review_reference.as_deref(),
    )?;
    let observed_success = prepared["observed_success"] == true;
    crate::local_state::write_new(&output, &serde_json::to_vec_pretty(&prepared)?)?;
    Ok(if observed_success {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn select_jobs<'a>(args: &'a ObserveArgs, defaults: Option<&'a [String]>) -> Result<&'a [String]> {
    let jobs = if args.required_jobs.is_empty() {
        ensure!(
            args.job_review_reference.is_none(),
            "An inventory review needs explicit pre-merge job names"
        );
        defaults.context("Reviewed required jobs are missing")?
    } else {
        ensure!(
            args.job_review_reference
                .as_deref()
                .is_some_and(|reference| !reference.trim().is_empty()
                    && reference.len() <= 4096
                    && !reference.chars().any(char::is_control)),
            "Explicit pre-merge jobs need their actual inventory review reference; do not select only passing jobs"
        );
        &args.required_jobs
    };
    ensure!(
        !jobs.is_empty(),
        "Prior-stage handoff needs reviewed required job names; a green aggregate alone is insufficient"
    );
    Ok(jobs)
}

fn prepare_observation(
    result: &opdev_remote::RunVerification,
    repository_id: u64,
    revision: &str,
    inventory_reference: Option<&str>,
) -> Result<serde_json::Value> {
    let expected = &result.expected;
    let observed_success = result.outcome == Outcome::Passed
        && result
            .jobs
            .as_ref()
            .is_some_and(|j| j.outcome == Outcome::Passed);
    let mut assertion = EvidenceAssertion {
        rule_id: "MCD-TEST-001".parse()?,
        outcome: Outcome::Unverified,
        summary: format!(
            "Prior-stage provider observations for source {} and run {} require review; integrated execution remains separate",
            expected.revision, expected.run_id
        ),
        evidence: vec![Evidence {
            kind: "prior_stage_observation".into(),
            summary: serde_json::to_string(&result)?,
            location: Some(format!(
                "{:?}:{}:run:{}",
                result.provider, repository_id, expected.run_id
            )),
        }],
    };
    if let Some(reference) = inventory_reference {
        assertion.evidence.push(Evidence {
            kind: "prior_stage_inventory_review".into(),
            location: Some(reference.into()),
            summary: "Explicit pre-merge job inventory selected for this observation only; trunk policy is unchanged. Attribution is not authenticated approval or proof of inventory completeness.".into(),
        });
    }
    Ok(serde_json::json!({
        "schema":1,"kind":"prior_stage_observation","source":expected.revision,
        "integrated_revision":revision,"matching_trees":true,"observed_success":observed_success,
        "qualification":"unverified","assertion":assertion,
        "limits":"Review actual run context, required-job inventory and the unchanged merge relationship before attributing this prior-stage finding. No current checks ran, no review was approved or posted, and no release is authorized. Failed or unavailable observations cannot support a pass."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Harness {
        #[command(flatten)]
        args: ObserveArgs,
    }

    #[test]
    fn distinct_prior_inventory_requires_review_and_preserves_trunk_policy() -> Result<()> {
        let base = [
            "opdev",
            "--review-locator",
            "fixture.json",
            "--acceptance-sha256",
            "fixture",
            "--run",
            "1",
            "--ref",
            "refs/merge-requests/1/head",
            "--output",
            "out.json",
        ];
        let defaults = vec!["native-post".into()];
        let parsed = Harness::try_parse_from(base)?;
        assert_eq!(select_jobs(&parsed.args, Some(&defaults))?, defaults);
        assert!(select_jobs(&parsed.args, None).is_err());
        assert!(select_jobs(&parsed.args, Some(&[])).is_err());
        for incomplete in [
            vec!["--required-job", "native-pre"],
            vec!["--job-review-reference", "fixture:review"],
        ] {
            assert!(Harness::try_parse_from(base.into_iter().chain(incomplete)).is_err());
        }
        let mut parsed = Harness::try_parse_from(base.into_iter().chain([
            "--required-job",
            "native-pre",
            "--required-job",
            "security-pre",
            "--job-review-reference",
            "fixture:complete-pre-inventory",
        ]))?;
        assert_eq!(
            select_jobs(&parsed.args, Some(&defaults))?,
            ["native-pre", "security-pre"]
        );
        assert_eq!(
            defaults,
            ["native-post"],
            "trunk obligations are never rewritten"
        );
        for reference in [
            None,
            Some(String::new()),
            Some("  ".into()),
            Some("invalid\nreference".into()),
        ] {
            parsed.args.job_review_reference = reference;
            assert!(select_jobs(&parsed.args, Some(&defaults)).is_err());
        }
        Ok(())
    }

    #[test]
    fn observed_run_is_never_an_approved_assertion_or_current_execution() -> Result<()> {
        let mut result = opdev_remote::RunVerification {
            schema: 2,
            repository: "fixture/product".into(),
            provider: CiProvider::Gitlab,
            expected: opdev_remote::RunExpectation {
                revision: "a".repeat(40),
                run_id: 5,
                reference: "refs/merge-requests/1/head".into(),
                source: "merge_request_event".into(),
                workflow_id: None,
            },
            observed: None,
            outcome: Outcome::Passed,
            qualification: Outcome::Unverified,
            diagnostics: vec![],
            jobs: Some(opdev_remote::JobVerification {
                required: vec!["quality".into()],
                current: vec![],
                prior: vec![],
                outcome: Outcome::Passed,
                diagnostics: vec![],
            }),
        };
        for outcome in [
            Outcome::Passed,
            Outcome::Failed,
            Outcome::Unverified,
            Outcome::Error,
        ] {
            result.outcome = outcome;
            let draft = prepare_observation(&result, 1, &"b".repeat(40), None)?;
            assert_eq!(draft["observed_success"], outcome == Outcome::Passed);
            assert_eq!(draft["assertion"]["outcome"], "unverified");
            assert_eq!(draft["qualification"], "unverified");
            assert_eq!(draft["source"], "a".repeat(40));
            assert_eq!(draft["integrated_revision"], "b".repeat(40));
        }
        result.outcome = Outcome::Passed;
        for outcome in [Outcome::Failed, Outcome::Unverified, Outcome::Error] {
            result
                .jobs
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("jobs"))?
                .outcome = outcome;
            let draft = prepare_observation(
                &result,
                1,
                &"b".repeat(40),
                Some("fixture:complete-pre-inventory"),
            )?;
            assert_eq!(
                draft["observed_success"], false,
                "review cannot erase missing or failed jobs"
            );
            assert_eq!(draft["assertion"]["outcome"], "unverified");
            assert_eq!(
                draft["assertion"]["evidence"][1]["location"],
                "fixture:complete-pre-inventory"
            );
        }
        result.jobs = None;
        assert_eq!(
            prepare_observation(&result, 1, &"b".repeat(40), Some("fixture:inventory"))?["observed_success"],
            false
        );
        Ok(())
    }
}
