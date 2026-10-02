# One verification path to a candidate

These small provider examples exercise a neutral Python text fixture, not a
production delivery pipeline. Copy the fixture and the selected YAML into an
isolated test repository; GitHub uses `.github/workflows/feedback.yml`, GitLab
uses `.gitlab-ci.yml`. Select an available project runner/image. The examples
use Python already available on the selected runner. Keep consumer fixtures and
secrets out of this repository.

MR/PR changes run verification. Trunk verifies the integrated revision, builds a
candidate once after verification, and checks those same bytes downstream. No
new API/dispatch pipeline is needed. The candidate is only this example script;
it is not an OpDev package, published release or proof of a project's recovery.
In an initialized project, the reviewed canonical OpDev job replaces the fixture
verification command and applicable delivery requirements still apply.

Feature-branch pushes are intentionally excluded: the first push before opening
an MR must not start a competing full pipeline. A project that needs branch-only
feedback must explicitly choose its own scheduling, including the first-push
race. GitLab `CI_OPEN_MERGE_REQUESTS` alone cannot suppress work that started
before an MR existed. GitHub PR checks and trunk pushes have separate purposes.

Existing workflows are project-owned. Review trigger, permission and artifact
changes before applying these patterns; do not replace custom YAML. Provider
validation/live job observations establish scheduling. Offline parsing and
dependency assertions are narrower evidence, not a provider emulator.

The check plan (`opdev check --plan --format json`) previews declared invocations.
It does not qualify a gate or reuse saved results. Keep raw argument/path output
private when it contains project details.

## Early environment observations

The standalone `diagnostic-gitlab.yml` and `diagnostic-github.yml` examples run
only on explicit web/API/dispatch requests. In an isolated project, select a reviewed
ref and invoke the provider's existing manual pipeline mechanism. The probe
reports OS, Python and one observable text operation, creates no resources, and
fails visibly for a mismatched OS expectation. It never creates verification or
candidate artifacts. A successful diagnostic is still **unverified** for gates.

Before use, review project/group variables, runner identity and privileges,
allowed refs, fork/untrusted-branch behavior and log visibility. These minimal
YAML files cannot override inherited runner permissions or remove inherited
secrets. Use a secret-free isolated project/runner for this example; do not expose
trusted credentials or protected execution to unreviewed source. No need to
change a production project's pipeline, branch protection or runner configuration.

For a real diagnostic, state the question, target, effort/time bound, actual
side effects and expected next decision in existing work context. Prefer
observation. If disposable resources are necessary, use a unique owned namespace,
record identifiers, check current ownership before cleanup and verify removal;
never infer ownership from a name or delete a shared resource. Retain failed
attempts and sanitized observations. Promotion of a resulting fix uses normal
reviewed source, tests and integration; distributing bytes uses declared CI.
