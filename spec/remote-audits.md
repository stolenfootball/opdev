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

The audit verifies the provider default branch, trunk protection visibility, the existence and latest verdict of a trunk pipeline, and merged-branch cleanup settings. Provider settings alone do not prove branch origin, lifetime, daily integration, or deletion in every case, so branch-lifecycle evidence remains `unverified` until history supplies the missing facts.

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

The existing `--remote` branch-latest audit is not upgraded by this addition: it
does not establish the exact-run claims above. Beyond the opt-in job inventory
below, external required-check/source inventories, rulesets/protection comparison,
pagination for those collections and connection of reviewed expectations to core
qualification remain tracked in issue #45. No
project/check schema migration is introduced in this increment.

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

### Design decision and follow-up criterion

Prefer explicitly selected run-detail endpoints over arbitrary latest-success
selection. This gives a bounded consumer outcome without choosing every project's
workflow policy or depending on paid provider features. Do not add a parallel
policy language. Revisit persistent configuration and automatic selection when
the next increment establishes how reviewed required checks and policy drift can
be represented without weakening existing requirements.

Sources: [GitHub workflow-run API](https://docs.github.com/en/rest/actions/workflow-runs#get-a-workflow-run)
and [GitLab pipeline API](https://docs.gitlab.com/api/pipelines/).
