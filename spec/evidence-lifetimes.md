# Evidence lifetimes and conservative reuse

## Current-change review and bounded retention

Explicit `assurance.review_storage.version: 2` selects acceptance review in the
project's existing GitLab MR or GitHub PR, not a separate evidence repository or
an append-only source ledger. It requires `evidence.discussion-review.v1` in local
and CI runtimes. Version 1 retains its Git-archive meaning; legacy projects are
not migrated by a runtime update. Current adoption recommends version 2.

Preserve the reason a change was accepted in the existing work/review authority.
Keep code and meaningful tests in ordinary Git history. A review identifies
conditions, actual assertions, discriminating examples and limitations; a green
suite alone is insufficient. Detailed routine reports/logs have a bounded
project-selected lifetime (`report_retention_days`, 1-3650; recommended 30), not
permanent storage. The declaration does not configure or prove provider retention.
Keep required inputs through remaining active verification, retain unresolved
failure/retry findings and reproductions in their work item, and do not relabel a
failed attempt after repair. There is no mandatory archive of every invocation,
agent conversation, old draft or diagnostic. Remove owned temporary drafts and
migration rollback files after their purpose is complete, with normal path and
authorization safety; do not accumulate a replacement evidence store.

After expiry, an old detailed result is unavailable, not newly failed and not
fresh proof for another check. Existing merges are not retroactively revoked.
Missing inputs needed for current qualification remain unverified/error as
appropriate; recheck current source or retrieve the required current record.
Never fall back to an older green run. Stronger selected regulatory/retention
requirements remain explicit and are not silently shortened.

Actual releases retain artifact identity, provenance and applicable qualification
at the existing artifact owner for their support/obligation lifetime. Routine
milestone completion does not authorize a release or require release archiving.
No infinite detailed historical reconstruction is claimed. MR/PR bodies are
mutable/deletable; concise review history is not an immutable attestation service.

CI templates retain both successful and failed reports for 30 days. Review actual
provider limits and keep-latest settings (GitLab can keep per-ref artifacts beyond
`expire_in`), including active handoffs, before selecting a different lifetime.
Select exact job/run/attempt identities, not a mutable latest-success link. Do not
rerun equivalent tests solely to manufacture a second report.

Rationale: current qualification and indefinite reconstruction are different
requirements. Existing review systems preserve useful intent with less duplication;
bounded CI storage limits routine detail. Alternatives are a replace-in-place
source file (offline but still bookkeeping/history growth) and no structured
review (simpler but weaker requirement/test enforcement). Revisit the selected
design if measured setup cost, provider availability or inadequate historical
diagnosis outweighs this benefit, without weakening source/current-check binding.
See [Google's change descriptions](https://google.github.io/eng-practices/review/developer/cl-descriptions.html),
[reviewed test adequacy](https://google.github.io/eng-practices/review/reviewer/looking-for.html),
[GitLab artifact retention](https://docs.gitlab.com/ci/jobs/job_artifacts/), and
[SLSA artifact-bound provenance](https://slsa.dev/spec/v1.2/distributing-provenance).

## Native CI review integration

Capability `evidence.ci-review.v1` supplies `check --ci --review-ci`, optionally
`--post-merge`, under explicit storage policy 2. This replaces per-repository
authentication/discovery scripts. It reads one explicit selection in the current
MR/PR description, between `<!-- opdev-ci-selection:start -->` and
`<!-- opdev-ci-selection:end -->`. JSON format 1 contains `schema: 1` and `stages`
with optional `pre_merge`/`post_merge` entries. Each selected entry has exactly
`revision` (full stage commit), `note_id` (positive comment ID), `body_sha256`
(whole comment), and `acceptance_sha256` (independently reviewed inventory).
Duplicate/unknown fields, ambiguous markers, missing stages and invalid identities
fail before canonical execution. The description selects the comment; the comment
cannot select its own expected acceptance identity. Attribution is not consent.

GitLab uses its automatic short-lived `CI_JOB_TOKEN` with `JOB-TOKEN` ahead of
local credentials when `GITLAB_CI=true`. Native selection requires that job token;
the authenticated `/job` response must match the project's ID and exact checkout.
Review reads use the supported MR/details/notes APIs, including the authenticated
MR's numeric target ID and web path instead of general project metadata. Local
explicit-locator reads preserve existing `glab` authentication. Never copy a human
token into CI, create a project token by default or alias a job token to a personal
token variable. GitLab issues and general policy/branch-protection audits are not
supported by this limited token: use accessible original MR scope for CI review
references, or keep unavailable authority checks visibly blocked. Do not silently
capture issue text as if it were the current original authority.

GitHub uses the supplied `GITHUB_TOKEN`; grant only needed read access (`contents`,
`pull-requests` and `issues` for the endpoints used here). Pass it to the CLI's
environment from the workflow token. Source-head PR checks must check out the
actual head, not Actions' default synthetic merge ref. Native selection rejects
`pull_request_target`, forks, synthetic GitLab merge-result/train contexts and
other unsupported contexts; these need an explicitly reviewed future path, not
broader credentials or execution of untrusted code in a privileged job.

Post-merge selection uses the commit-to-MR/PR endpoint and exactly one actual
merged change, then checks its target trunk, source project and integrated commit.
Discovery is bounded to fewer than 100 returned candidates; ambiguous/truncated
results fail, never select latest green. The current-stage record still binds
actual source/configuration/acceptance and fresh execution. The selection and
review content are re-read after execution. No saved artifact substitutes for
current review, execution or authorization. Routine reports use existing bounded
CI artifacts, not an additional source registry.

Rationale: providers already supply job authentication, change identity and report
storage. OpDev owns only its requirement/test checks and a shared provider adapter.
Retain explicit manual locators for local/legacy consumers; revisit discovery when
a real supported fork, merge-queue or large-history case needs it. No automatic
policy migration, publication or credential provisioning occurs. See
[GitLab job-token access](https://docs.gitlab.com/ci/jobs/ci_job_token/),
[glab CI authentication](https://docs.gitlab.com/cli/authentication/), and
[GitHub workflow tokens](https://docs.github.com/en/actions/tutorials/authenticate-with-github_token).

## Bounded review handoff and representation

`evidence.review-handoff.v1` provides `ci prepare-review --change NUMBER --note ID
--acceptance-sha256 ID [--post-merge]`. It reads the explicitly selected posted
comment through existing provider authentication and verifies current source,
configuration, stage, acceptance review and referenced authorities. It returns
JSON containing the locator, independent acceptance identity, original description
digest and proposed description. Only the selected stage is changed; other-stage
selection and surrounding text are preserved. Ambiguous/malformed existing blocks,
forks, wrong source and observed drift fail. No upload, approval, retry or issue
creation occurs. Inspect the proposal and recheck the original description digest
immediately before an authorized provider write; this proposal is not a write lock.

`ci review-check [--post-merge]` uses the same built-in CI identity and selection
as normal verification, rechecks mutable authorities and reports only input
readiness. Exit 0 is **not** integration qualification. Run it before expensive
check-only tool setup; actual `check --ci --review-ci` still validates current
inputs and executes required suites. The normal path never trusts a preflight
receipt. No project command is run by either preparation command.

`evidence.discussion-wire.v2` changes only the discussion representation, not
semantic review schema 1 or any acceptance digest. A `opdev-review:v2` Markdown
section contains JSON with `schema: 2`, `record`, `strings` and `references`.
Each reference has a JSON-pointer `path` to a null slot in `record` and an `index`
into `strings`. The writer interns repeated strings of at least 80 bytes only
when pointer overhead still saves bytes. Expansion restores the original typed
record before ordinary validation: no summaries, omitted claims or new consent.
References cannot overwrite values or other references, missing indexes/paths
fail, strings must be used at least twice, and unknown fields fail. Limits are
4,096 references/strings, 32 pointer segments, 1 MiB expanded JSON and the unchanged
60 KiB whole-comment bound. Strings are escaped against Markdown delimiter
injection. Old v1 sections remain readable; mixed/unknown versions fail closed.
Older readers cannot read v2: check local **and CI** capabilities before exporting.

This lossless representation was chosen over increasing the note limit, deleting
review details, or storing another evidence archive. Revisit the dictionary if
measured usability or payload savings do not justify it; the typed record and v1
reader allow a different bounded encoding without changing review semantics.

## Qualification freshness (independent of storage lifetime)

OpDev does not implement a general cross-revision test cache or infer unaffected
files from filenames, comments, documentation labels or agent judgment. Source,
configuration and stage remain whole-subject bindings by default. A newer source
or unknown dependency requires fresh qualification or an explicitly unverified
result. A cache hit is never a requirement-adequacy judgment.

The supported narrow case is [same-run CI execution](execution-reuse.md): reviewed
producer/input/environment policy can explicitly exclude an evidence ledger that
is not a command input. Execution remains bound to the exact producer and run;
new test conditions still need current assertion review. No automatic exclusion
is inferred, and the default includes the ledger. Fresh execution remains usable.

| Fact | Lifetime and invalidation |
| --- | --- |
| Assertion review | Exact source, complete acceptance inventory and current assertions; new conditions invalidate review even when execution is unchanged |
| Canonical execution | Exact reviewed input/configuration/environment identities, provider producer/run/attempt and stage; newer attempts and unavailable evidence cannot fall back to old green |
| Artifact qualification | Exact retained artifact bytes and separate qualification evidence; replacement or missing bytes cannot inherit old qualification |
| Developer decision | Original attributed scope, authority, revocation/supersession and applicable expiry; permission is not feedback or release authority |
| Remote observation | Explicit observation context and freshness, rechecked through the provider when qualification requires it; local journals cannot refresh or authenticate it |

The [workflow reference projection](workflow-records.md) applies conservative
subject and content checks independently to each record and explains the changed
identity. Review records compare a separately supplied complete acceptance
digest. A changed review need not erase unchanged execution history, but the
projection itself never qualifies that execution. Existing core evaluation and
provider validators remain responsible for gates and current source observations.

All supporting references used for current qualification must be available and unchanged. Expired, future-dated,
conflicting, revoked and superseded facts stay visible. Cooperative concurrent
updates use the journal's expected-head check; an older observation cannot replace
new facts. No derived index is required or authoritative. Historical diagnostic
receipts retain their meaning and are not upgraded into qualifying executions.

This intentionally favors broader rechecking over an unproven scoped cache.
Introduce narrower dependencies only for a demonstrably useful case with a
reviewed complete input boundary, negative/mutation tests against fresh checks,
explicit compatibility and developer policy. Neither test removal nor automatic
policy migration is a valid optimization. Qualification, delegation and inspection
remain separate; unsupported consumers use the fresh execution path.
