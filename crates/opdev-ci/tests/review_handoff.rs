//! Review-input wiring is distinct from generic gate presence and live access.
use opdev_core::Outcome;
use opdev_project::{CiProvider, ProjectManifest, ReviewStorage, discover};
use serde_json::json;
use std::fs;

fn fixture(
    provider: CiProvider,
    pre: &str,
    post: &str,
) -> Result<(tempfile::TempDir, ProjectManifest), Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    let root = repo.path();
    assert!(
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .arg(root)
            .status()?
            .success()
    );
    let mut manifest = discover(root)?.manifest;
    manifest.project.trunk = "main".into();
    manifest.project.ci.provider = provider;
    manifest.assurance.review_storage = Some(ReviewStorage {
        version: 2,
        provider,
        repository_id: 7,
        review_reference: "Synthetic selected policy".into(),
        retention_authority: "fixture".into(),
        report_retention_days: Some(1),
    });
    let source = if provider == CiProvider::Github {
        fs::create_dir_all(root.join(".github/workflows"))?;
        json!({"on":{"pull_request":null,"push":{"branches":["main"]}},"jobs":{"verify":{
        "runs-on":"ubuntu-24.04","steps":[
            {"uses":"actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683","with":{"ref":"${{ github.event.pull_request.head.sha || github.sha }}"}},
            {"run":"cargo build --locked -p opdev-cli"},
            {"if":"github.event_name == 'pull_request'","run":pre},
            {"if":"github.event_name == 'push'","run":post}
        ]}}})
    } else {
        json!({
            "pre":{"script":[pre],"rules":[{"if":"$CI_PIPELINE_SOURCE == \"merge_request_event\""}]},
            "post":{"script":[post],"rules":[{"if":"$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH"}]}
        })
    };
    let path = if provider == CiProvider::Github {
        ".github/workflows/quality.yml"
    } else {
        ".gitlab-ci.yml"
    };
    fs::write(root.join(path), serde_json::to_vec(&source)?)?;
    Ok((repo, manifest))
}

#[test]
fn both_providers_require_actual_complete_stage_review_arguments()
-> Result<(), Box<dyn std::error::Error>> {
    for provider in [CiProvider::Github, CiProvider::Gitlab] {
        for (pre, post) in [
            (
                "./target/debug/opdev ci verify --stage pre-merge",
                "./target/debug/opdev ci verify --stage post-merge",
            ),
            (
                "./target/debug/opdev check --ci --review-ci",
                "./target/debug/opdev check --ci --post-merge --review-ci",
            ),
            (
                "./target/debug/opdev check --ci --review-locator review.json --review-acceptance-sha256 abc",
                "./target/debug/opdev check --ci --post-merge --review-locator review.json --review-acceptance-sha256 abc",
            ),
        ] {
            let (repo, manifest) = fixture(provider, pre, post)?;
            let result = opdev_ci::inspect_review_handoff(repo.path(), &manifest);
            assert_eq!(result.outcome, Outcome::Passed, "{provider:?}: {result:?}");
            assert!(
                result
                    .evidence
                    .iter()
                    .any(|e| e.summary.contains("remain unverified"))
            );
        }
        let post = "./target/debug/opdev check --ci --post-merge --review-ci";
        for pre in [
            "./target/debug/opdev check --ci",
            "# --review-ci\n./target/debug/opdev check --ci",
            "echo opdev check --ci --review-ci",
            "./target/debug/opdev check --ci --review-locator review.json",
            "./target/debug/opdev check --ci --review-acceptance-sha256 abc",
            "./target/debug/opdev check --ci --report --review-ci",
            "./target/debug/opdev check --ci --review-locator '' --review-acceptance-sha256 abc",
            "./target/debug/opdev check --ci --review-ci --review-locator review.json --review-acceptance-sha256 abc",
            "./target/debug/opdev ci verify --stage pre-merge --preflight",
            "./target/debug/opdev ci verify --stage post-merge",
            "./target/debug/opdev check --ci --review-ci || true",
        ] {
            let (repo, manifest) = fixture(provider, pre, post)?;
            let result = opdev_ci::inspect_review_handoff(repo.path(), &manifest);
            assert_ne!(
                result.outcome,
                Outcome::Passed,
                "{provider:?} admitted {pre}: {result:?}"
            );
        }
        let (repo, mut manifest) = fixture(
            provider,
            "./target/debug/opdev ci verify --stage pre-merge",
            "./target/debug/opdev ci verify --stage post-merge",
        )?;
        manifest
            .assurance
            .review_storage
            .as_mut()
            .ok_or("storage")?
            .version = 1;
        assert_ne!(
            opdev_ci::inspect_review_handoff(repo.path(), &manifest).outcome,
            Outcome::Passed
        );
        manifest.assurance.review_storage = None;
        assert_eq!(
            opdev_ci::inspect_review_handoff(repo.path(), &manifest).outcome,
            Outcome::NotApplicable
        );
    }
    Ok(())
}
