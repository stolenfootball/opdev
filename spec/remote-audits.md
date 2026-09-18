# Read-only remote audits

`opdev check --remote` and `opdev doctor --remote` query GitHub or GitLab with GET requests only. OpDev does not create pipelines, change branch protection, modify merge settings, delete branches, post statuses, or otherwise mutate a remote repository. Local CI generation remains a separate explicit operation.

Remote URLs are accepted only for the first-class `github.com` and `gitlab.com` hosts in 0.1. This prevents a repository-controlled remote from turning the audit into a request to an arbitrary host. Self-managed instances and additional providers require a future adapter with an explicit trusted API origin.

Authentication is optional. GitHub reads `OPDEV_GITHUB_TOKEN`, `GITHUB_TOKEN`,
then `GH_TOKEN`, and sends the selected value as a bearer credential. GitLab
uses the first non-empty source in this exact order:

1. `OPDEV_GITLAB_OAUTH_TOKEN`, sent as `Authorization: Bearer`;
2. `OPDEV_GITLAB_PRIVATE_TOKEN`, sent as `PRIVATE-TOKEN` for explicit
   compatibility;
3. `OPDEV_GITLAB_TOKEN`, `GITLAB_TOKEN`, then `GLAB_TOKEN`, each sent as
   `Authorization: Bearer`; and
4. the `gitlab.com` token returned by an authenticated
   `glab config get token --host gitlab.com`, also sent as Bearer.

GitLab documents Bearer authentication for OAuth tokens and for personal,
project, and group access tokens, so generic credentials do not need secret
shape detection. OpDev does not retry a rejected credential under another
header. The optional `glab` lookup keeps browser or device OAuth login seamless
without reading credential files or keyrings directly. A missing `glab`
executable or credential simply leaves the audit unauthenticated.

Tokens are added only to request headers, captured in memory only as long as
needed, and never copied into evidence or diagnostics. Missing permission,
unavailable fields, HTTP failures, and non-definitive pipeline states produce
`unverified`; they do not become passes. In particular, HTTP 401 never falls
back to public evidence. A missing trunk pipeline is `migration_required`,
while a definitive failing trunk pipeline is `failed`.

`doctor --remote` observes the provider default branch, protection visibility,
pipeline existence and merged-branch cleanup settings. A latest green pipeline
is supporting information, **not qualification**; its capability is `unverified`.
Provider settings alone cannot prove branch origin, lifetime, daily integration,
or deletion. `check --remote` additionally requires the reviewed exact-trunk
qualification below; generic passing ledger assertions cannot hide its absence.

## Explicit CI run observations

Development CLIs additionally expose `opdev ci verify-run`. This is separate from
the existing branch-latest audit and does not update its gates or a ledger:

```sh
opdev ci verify-run --revision FULL_SHA --run RUN_ID --ref main --source push
# GitHub also requires the numeric workflow identity:
opdev ci verify-run --revision FULL_SHA --run RUN_ID --ref main --source push --workflow WORKFLOW_ID --format json
```

Repository/provider come from the existing project contract. The caller must
select a full commit ID, positive run/pipeline ID, provider-returned ref and
event/source. GitHub requires a positive workflow ID; GitLab rejects that option.
There is no implicit HEAD, branch-latest fallback, guessed workflow, policy
selection or provider mutation. Review expectations in the existing work/CI
authority; this command does not establish that the caller chose adequate policy.

The verifier uses direct run-detail GET endpoints, not paginated lists or
success-filtered searches. It checks the returned repository/project identity,
run ID, revision, ref, source and (GitHub) workflow identity. A matching completed
GitHub success or GitLab success yields `passed`. Identity mismatch or a failed,
cancelled or timed-out run yields `failed`. Pending/skipped/neutral/unknown states,
missing fields, inaccessible/missing runs, malformed data and HTTP failures yield
`unverified`; they never fall back to another green run. Existing credentials are
reused without header retries. Redirects are disabled, responses are limited to
1 MiB, and response bodies, actors, job logs and credentials are not echoed.

Human and [JSON schema-1 output](../schema/ci-run-verification.schema.json) retain
expected and observed identity, lifecycle status and GitHub's observed attempt.
Exit 0 means the explicit identity/status observation passed; exit 1 means failed
or unverified; invalid inputs/configuration or inability to start the verifier
exit 2. No local test command is executed and no evidence file is written.

`qualification` remains `unverified`. A single run snapshot does not prove the
required job inventory ran, check producers were trusted, merge protection matches
reviewed policy, no newer/retried run exists, or an artifact is qualified. A passed
observation for a historical run does not imply current branch health. Do not use
this command alone as a replacement for required gates or the red-trunk policy.

This observation command remains separate from the policy-backed core audit.
It does not select project policy or migrate the project contract.

## Explicit required-job observations

Add one `--require-job NAME` per required provider-owned job to `ci verify-run`:

```sh
opdev ci verify-run --revision FULL_SHA --run RUN_ID --ref main --source push --require-job test --require-job lint --format json
```

GitHub still requires `--workflow WORKFLOW_ID`. Names match exactly, including
matrix suffixes. Declare expectations in the existing work/CI authority; the CLI
does not infer required names, create a policy file or approve their sufficiency.
At most 100 distinct names may be supplied. Missing or duplicate current names
are `unverified`; a failed current match is `failed` even if another match passed.
Only a unique successful current job satisfies its requested name. Skipped,
neutral, manual, pending and unknown states do not satisfy it. `allow_failure`
does not waive a caller's explicit requirement.

GitHub enumerates the observed run attempt's jobs and validates their run and
revision. The attempt number remains visible; prior attempts are not collected.
GitLab enumerates the pipeline's current jobs and retry history, validating
run/revision/ref. Superseded matching jobs are retained under `prior` but never
satisfy a current requirement. Child/downstream pipelines, trigger bridges,
external checks and third-party producer trust are outside this inventory; they
cannot be silently substituted for missing jobs.

Enumeration constructs page URLs on the fixed provider API origin, never follows
response links or redirects, and does not filter by success. Each response is
limited to 1 MiB, each collection to 10 pages of 100 jobs. Enumeration stops
starting new page requests after one minute; an in-flight request may consume its
20-second timeout and the final run recheck has its own existing bounds. Duplicate
IDs, inconsistent totals, incomplete collections, HTTP errors and limits produce
`unverified`, not a partial-inventory pass. The current inventory is read again,
then run identity/status/attempt are rechecked. Observed changes or missing retry
history invalidate the job snapshot. These checks detect observed races, not an
atomic provider attestation or proof that no future run/retry occurs.

Without `--require-job`, schema-1 output and exit behavior remain unchanged. With
it, [schema-2 output](../schema/ci-run-verification.schema.json) adds `jobs` with
requested names, current/prior observations, outcome and diagnostics. Run
`outcome` retains its original meaning; exit 0 additionally requires a passed job
observation. `qualification` remains `unverified`. No core gate, provider setting,
project manifest, ledger or policy is changed. Consumers must opt into and support
the new report shape; older CLIs reject the new argument rather than migrate data.

Sources: [GitHub workflow jobs](https://docs.github.com/en/rest/actions/workflow-jobs)
and [GitLab jobs](https://docs.gitlab.com/api/jobs/).

## Reviewed current-trunk qualification

Development CLIs support optional `project.ci.qualification` in **project schema
2**. Ordinary initialization still emits schema 1, and upgrades do not select or
write qualification policy. Before opting in, present the project's actual CI
jobs, sources and protection choices to its developer. Record the actual decision
in an existing authority, then explicitly review a manifest diff changing `schema`
and adding this section. Preserve all unrelated fields. A review reference is a
pointer to that decision, not proof of consent manufactured by the CLI.

Example GitHub section (replace all example identities with reviewed provider
values; this is not a recommended policy for every project):

```yaml
schema: 2
project:
  # Preserve existing kind, trunk, provider and remote.
  ci:
    qualification:
      review_reference: "existing work item containing the developer decision"
      source: push
      workflow_id: 1234
      required_jobs: [test, lint]
      required_checks:
        - name: test
          producer_id: 5678
      protection:
        kind: github_branch
        strict: true
```

GitHub requires a numeric workflow ID, at least one exact job name and at least
one check name bound to its GitHub App ID. Classic branch protection must enforce
pull requests and administrator restrictions, disallow force-push/deletion and
configured pull-request bypass, and expose the exact reviewed check/producer set
and `strict` value. Legacy commit-status contexts without App-bound check runs
are not supported by this adapter and cannot produce a pass.

Alternatively use `protection: {kind: github_rulesets, ids: [123], strict: true}`.
All selected rulesets must actively apply to trunk, expose an empty bypass list,
and collectively enforce pull requests, no force-push/deletion and the exact
producer-bound check set. Active branch rules must agree with ruleset details.
Unselected additional restrictions are outside this selected-policy comparison.
Missing bypass visibility (including insufficient API permissions) is unverified;
OpDev never requests a paid feature or changes settings to obtain a pass.

GitLab omits `workflow_id`. Required jobs belong to the selected pipeline;
`required_checks` may be empty, or pin additional commit-status names to numeric
creator IDs. Its protection section lists **every** matching protected-branch
rule, including overlapping `*` wildcard rules:

```yaml
protection:
  kind: gitlab
  rules:
    - name: main
      push: [{kind: role, id: 0}]
      merge: [{kind: role, id: 40}]
```

Access principals may be `role`, `user`, `group`, `deploy_key` or `member_role`.
Roles support explicit no-access (0), Developer (30), Maintainer (40) and Admin
(60). Compare exact identities, not display names or API row IDs. New matching
rules, force-push, missing rules or changed access are drift. The project must
require successful pipelines and explicitly disallow skipped ones; absent/null
booleans remain unverified. A reviewed direct-push permission is not proof that
all delivery uses CI; the other core delivery/trunk requirements still apply.

### Selection, freshness and result meaning

`check --remote` captures clean local HEAD before canonical checks, rechecks it
before/after remote observation, and requires it to equal current remote trunk.
Dirty, historical and change-branch sources cannot qualify current trunk. Among
runs matching revision/ref/source/workflow, select the newest numeric provider ID
without filtering for success. Direct run detail and required jobs must agree.
Newer pending/failed evidence never falls back to an old green run. Checks use
the newest identity for each required name, not a success/trusted-producer filter.

Remote collections use fixed-origin GETs, bounded pagination (10 pages of 100,
1 MiB per response), no redirects and 20-second request timeouts. Qualification
metadata stops starting requests after two minutes; the existing run/job verifier
has its own documented bounds. Recheck check identities/statuses, protection,
latest run, selected attempt and trunk. Changes, truncated collections, missing
fields, HTTP errors and unsupported provider capabilities remain unverified.
These observations detect visible races, not an atomic provider attestation or
a promise that policy, runs or source cannot change afterward.

Human output identifies the revision and separates CI qualification from artifact
qualification. Full check JSON retains reviewed expectations, selected run/jobs,
check identities/producers/statuses and policy uncertainty inside
`remote_ci_qualification` evidence. The embedded standalone run's `qualification`
remains unverified by design; the enclosing combined result carries the stronger
CI-only verdict. No new check-report schema is needed: evidence remains in its
existing extensible envelope.

Missing reviewed policy or remote evidence blocks the requested remote
qualification for `MCD-CI-001`, `MCD-TEST-002` and the red-trunk decision, even when
generic local evidence passed. Existing failed/error/migration-required rules
are preserved. This does not qualify artifacts, delivery/recovery, all branch
history, policy adequacy or full compliance. Checks without `--remote` retain
their existing local behavior. See [schema migration](compatibility.md).

### Verification

Offline fixtures cover run selection, wrong producers, pending/missing/failed
checks, pagination, permissions, classic/ruleset distinctions, wildcard/access
drift, snapshot disagreement, migration and preservation of existing failures.
The ignored `live_remote_qualification` test accepts explicit environment inputs
(`OPDEV_TEST_QUALIFICATION_ROOT`, `TRUNK`, `REVISION`, `CI`, `OUTCOME`, each with
the same `OPDEV_TEST_QUALIFICATION_` prefix). `CI` is JSON for an in-memory
`project.ci` override; this is a read-only canary, never an adoption write. It
requires passed run/jobs/check observations and the caller's expected overall
outcome. Ordinary tests never access a live provider.

Sources: [GitHub workflow-run API](https://docs.github.com/en/rest/actions/workflow-runs#get-a-workflow-run)
and [GitLab pipeline API](https://docs.gitlab.com/api/pipelines/).

Policy sources: [GitHub branch protection](https://docs.github.com/en/rest/branches/branch-protection),
[rules and rulesets](https://docs.github.com/en/rest/repos/rules),
[check runs](https://docs.github.com/en/rest/checks/runs),
[GitLab projects](https://docs.gitlab.com/api/projects/),
[protected branches](https://docs.gitlab.com/api/protected_branches/) and
[commit statuses](https://docs.gitlab.com/api/commits/#list-commit-statuses).
