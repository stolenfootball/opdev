# Design and improve CI with evidence

Use for new CI, adoption discovery, substantial pipeline changes or observed CI
friction. Ordinary coding does not require a pipeline audit. Keep recommendations
proportional to the current outcome; reuse accepted decisions and current evidence.
This applies to any software ecosystem. GitHub and GitLab have first-class OpDev
adapters; other providers need their own verified semantics, not translated guesses.

## Start with the project's path

For new CI, inspect project commands, acceptance/risk coverage, supported targets,
runner capacity, dependencies, artifact consumers and delivery requirements. Use
the simplest arrangement that meets them. Reuse project tooling; do not prescribe
a language, test runner, image distribution or four-workflow architecture.
Resolve only material choices, such as cost versus latency or additional supported
platforms. Do not add a questionnaire, mandatory document or configuration schema.

For existing CI, inspect effective configuration, includes, invoked scripts and
available representative run evidence before proposing changes. Trace wrappers:
one command may already run another job's entire suite. Unknown remote/dynamic
includes remain unknown. Reuse provider-native validation/expanded configuration
where available; local YAML inspection is not a provider compiler or proof of runs.
Prefer a small reviewed diff to replacing adequate project-owned workflows.

Keep focused feedback, required pre-merge verification, integrated-trunk checks
and explicitly authorized publication distinct. These are purposes, not mandatory
separate workflows. Feedback may precede full checks; it cannot satisfy missing
merge requirements. A candidate need does not authorize a release, installation
in a consumer project or provider-policy change. Follow [planning](planning.md)
and the [release boundary](workflow.md#release-authorization).

## Choose changes around evidence

- **Execution ownership:** identify one owner for genuinely equivalent execution.
  Compare source, effective configuration, command/wrapper inputs, toolchain,
  environment, artifact, trust and freshness, not just matching argv. Preserve
  separate OS/architecture/configuration coverage and pre/post-merge verification.
  Prefer one fresh canonical execution to redundant standalone suites plus an
  OpDev job that reruns them. Optional authenticated same-run reuse follows
  [testing](testing.md); never invent receipts, reuse across revisions or expand
  credentials to make reuse work. Inspect `check --plan` when supported; it lists
  selected commands, not provider scheduling or execution equivalence.
- **Scheduling:** trace the critical path (dependent work determining completion),
  not just the number of jobs. Group cheap checks sharing expensive setup when
  useful; parallelize independent expensive work only within runner capacity and
  the project's cost preference. Fast failure can save compute but serial barriers
  can delay successful runs. Prevent accidental push/PR duplication, including
  first-push-before-MR races. Branch-only feedback is a legitimate reviewed choice.
  Scope cancellation to safely replaceable work; preserve deliberate long trials,
  publication and cleanup. Do not make every job non-interruptible.
- **Setup and caches:** measure checkout, image startup, dependency installation,
  cache download/extraction and check execution separately. Consider suitable
  existing images before creating a maintained image service. Preserve product
  base images; tooling compatibility is not permission to migrate the product.
  Cache only reconstructible data, with appropriate dependency/toolchain,
  OS/libc/architecture and trust boundaries. Test both cold and warm paths; cache
  absence must not hide required work. Cache hits are not test evidence or artifact
  identity. Do not share privileged caches with untrusted writers for speed.
- **Test signal:** preserve meaningful assertions and supported environments.
  Review hidden retries, filtering and allowed failures. Keep retries visible and
  distinguish product failures, unavailable required results and broken tooling.
  Path filters need dependency-aware justification, including shared code,
  build scripts and lockfiles. Moving required tests to a schedule is a policy
  change, not an optimization shortcut. Predictive selection requires calibrated
  project evidence and reviewed policy; filename guesses cannot establish safety.
- **Enforcement:** test what happens when required work fails, is skipped,
  cancelled or missing, including matrix legs. Check actual branch protection,
  event/source, revision and required-job identities. A green aggregate, preview
  job or successful syntax check is insufficient. A collector that runs `always()`
  must explicitly reject non-successful required results; running it is not enough.
  Keep optional jobs separate from the required inventory without treating their
  failures as successful experiments.
- **Artifacts and trust:** build a candidate once, identify it immutably and pass
  those bytes to downstream qualification/promotion. Keep trusted execution and
  secrets away from unreviewed contributions; use least-privilege tokens and
  appropriate immutable dependency/action pins. Reuse existing delivery controls;
  do not add publication or release recovery work merely to complete a milestone.
- **Maintenance:** prefer existing commands and native provider features over
  bespoke wrappers or a new optimizer. Retain diagnostic logs with safe exposure
  and bounded retention. A more elaborate pipeline needs a concrete benefit.

For practical contracts, use the native check exit status, actual stage flag and
`--expected-revision` with the provider's exact checkout SHA. Do not duplicate
OpDev's schema, command inventory or result validation in a project-specific JSON
adapter. Preserve separate artifact/host qualification that OpDev does not perform.
Manual evaluations reference a real procedure through `manual`, not a deliberately
failing placeholder command. They remain `not_run` and cannot satisfy a required
boundary through the CLI. Keep their actual product-review requirements intact.

During long CI waits, report meaningful changes, completion or actionable blockers.
Avoid repeating unchanged job state or reconstructing the full plan on each poll.
Follow host-required communication intervals with a concise update when necessary.

## Impact-based verification

During new CI design, adoption or authorized improvement, establish a proportionate
path for changes that affect only prose. It should run the relevant documentation
checks without unrelated product compilation, native suites or dependency audits
on every wording edit. Keep full coverage for changes that can affect the product,
and retain independently justified scheduled dependency/security checks.

Inspect the actual consumers of changed files before classifying them. Markdown
may contain executable agent instructions, tested examples, generated build inputs
or behavioral contracts. Mixed changes take the union of required checks; unknown
impact, selector changes, shared build inputs and CI changes take the conservative
path. Do not infer safety solely from a docs directory or file extension.

Prefer existing commands and provider-native change rules over a custom optimizer.
Review the contract's stage commands together with job rules and protection: a
path filter does not change what an unconditional OpDev command executes. No new
OpDev selector flag is implied. Where the current runtime cannot express the
required selection, report the limitation and propose a supported arrangement;
never manufacture a successful check, hide a required missing job, or weaken
protection to accommodate skipped jobs.

Test prose-only, executable-documentation, product-only, mixed and unknown-input
changes, plus required-job failure/missing cases and actual MR/trunk comparison
bases. A documentation-qualified result must not be reported as product runtime
verification. Changing a project's required inventory or provider scheduling needs
actual authorization; new guidance alone does not migrate its policy. Until an
approved path exists, preserve existing merge gates while still presenting useful
documentation/design results promptly.

## Provider checks, not a universal template

With explicit engineering policy 3, `ci.native-candidate-reuse.v1` can avoid
repeating eligible source checks after a native integration candidate. Propose it
only after reviewing command/input completeness, immutable effective CI and
environment inputs, commit/ref/event/stage independence, current producer identity
and a bounded age. Use execution policy 2 and an exact native-run selector;
provider-owned logs and actual merge correspondence are required. A matching tree
alone is insufficient. Unknown correspondence falls back to fresh execution;
changed evidence after evaluation invalidates reuse. Keep deployment, live-system,
artifact-dependent and non-equivalent checks fresh. Do not enable paid features,
expand credentials, migrate policy or change CI merely to obtain reuse.
GitHub currently requires the exact integrated merge-group commit; rewritten
commits remain unsupported. GitLab requires a completed non-skipped merge train.
Reports must distinguish earlier execution from equivalent integrated verification.
This does not establish the separate literal MinimumCD post-merge obligation.

For GitHub, verify PR/head versus synthetic merge subjects and `merge_group` when
a merge queue is used. A skipped job may report success; a skipped workflow may
leave checks pending. An always-running required collector must inspect every
required result, including matrix coverage, and itself be required by protection.
Do not assume a manual diagnostic run can satisfy PR checks. Inspect the expected
check producer as well as its name. See [required-check semantics](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks).

For GitLab, inspect `workflow:rules`, job rules, `needs`, inherited setup,
`allow_failure`, manual jobs and child pipelines. `CI_OPEN_MERGE_REQUESTS` cannot
cancel a duplicate run already started before the MR existed. Require successful
pipelines and do not allow skipped pipelines to qualify. CI Lint and simulation
help, but verify their supported event/ref scope and use actual isolated runs for
scheduling/protection claims. See [pipeline efficiency](https://docs.gitlab.com/ci/pipelines/pipeline_efficiency/)
and [CI Lint](https://docs.gitlab.com/ci/yaml/lint/).

Use the repository's existing examples or reviewed OpDev provider patterns as
starting points, not automatic replacements. Never fetch or execute unpinned
remote scripts just to follow this guide. Unsupported includes, inaccessible
protection settings and unavailable required observations remain unverified.

## Recommend, verify, learn

Offer a small ranked set of recommendations with observation/source, uncertainty,
tradeoff, proposed change, validation and reversal. Include keeping a compliant
setup. Advice does not authorize edits, new issues, cancelling jobs, spending on
trials or changing runners/permissions. Reuse actual scoped authorization when
present; no extra permission round is needed for routine approved implementation.
Honor declined advice without a waiver file or repeated pitch unless evidence
materially changes. Report actual core failures separately; they cannot be waived.

Measure useful-feedback latency, time to required verification, queue/setup time,
summed runner compute, false alarms/missed findings and maintenance burden as
applicable. Separate observations from estimates; use comparable repeated runs
and cold/warm cache conditions for performance claims. Report sample size and
variability; a single fast run or YAML inspection does not prove a speedup.
Choose the next small change from evidence, including no change when appropriate.
Keep work in the existing tracker and enduring decisions in their routed authority.
No new ledger, automatic gate, ten-minute deadline or optimization score is added.

### Review readiness without manufactured failures

When qualification needs a review selected after source publication or merge,
avoid routinely launching expensive jobs with knowingly missing inputs. Prefer a
provider-native blocking readiness hold where supported. For GitLab, a manual
required job must explicitly set `allow_failure: false`; the authorized controller
selects the exact review and starts that job. This is scheduling, not an extra
human approval or permission to leave required work unfinished. Pending/manual,
missing, skipped, canceled and failed checks must still prevent qualification.
Validate actual provider behavior and downstream dependencies before adopting it.
Do not translate this blindly to GitHub, where skipped-job and workflow-trigger
semantics differ. Preserve adequate existing pipelines and developer choices.

Recheck source/review readiness in the job itself to detect edits after dispatch.
Missing or stale inputs must fail closed; a hold is not a reusable readiness receipt.
Use opt-in fail-fast canonical execution when supported to avoid spending on full
suites after cheap failures. Preserve failed attempts and do not add broad tokens,
polling services or custom schedulers merely to coordinate a review.
