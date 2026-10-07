# Same-run canonical execution

Execution-record schema 1 and execution-policy schema 1 are independent of the
legacy diagnostic receipt and check-report formats. Parsing a record, matching
a hash or locating a green job does not authenticate canonical execution.
Existing `test-execution` receipts remain diagnostic and cannot supply this API.

## Trust and input boundary

Reuse is opt-in. A reviewed, tracked policy identifies canonical suites, exact
producer job names, wrapper and command executable digests, and a nonsecret
immutable environment/toolchain/dependency identity. Its actual decision
reference must establish producer ownership, effective CI configuration, all
relevant ignored/external inputs, failure propagation and retention. Unknown
inputs leave `inputs_complete` false and prevent reuse. The boolean records a
reviewed assertion; it is not automatic discovery or authenticated human consent.
Never infer completeness from a matching commit or dump/hash secret values.

The policy file may live at an existing suitable authority or under `.opdev/`.
No file is generated automatically and no existing project decision is migrated.
Omitting reuse preserves the ordinary canonical execution path. Keeping a
compliant pipeline with deliberate repeated checks is supported.

The canonical wrapper captures child output and emits one escaped JSON record
per suite. The evaluator retrieves the exact provider-owned job log, not a local
receipt or latest-ref artifact. A completed producer can be consumed while the
evaluator and overall run are still running. Provider identity, current attempt,
selected job inventory and source are checked again after retrieval. No older
green attempt substitutes for a newer failed, pending, skipped or missing job.
Prior observed GitLab jobs remain visible; GitHub run-attempt identity is retained,
without claiming complete history of every prior attempt.

This establishes a provider-owned channel, not resistance to a compromised
trusted producer, CI administrator or project-controlled code. A reviewed
producer must not print arbitrary caller records or allow unreviewed configuration
to select its inputs. Captured command output cannot inject an ordinary separate
record line; duplicate/malformed records fail closed. Artifact upload metadata
alone does not prove the uploading job on both providers, which is why job logs
are the common first transport.

The consuming process obtains its run identity from the CI environment. This is
an operational trust boundary, not cryptographic proof that the consumer runs
inside that provider: a local process able to reproduce the environment and
read the provider could invoke it too. Deployments must restrict qualification
and delivery to their reviewed CI path; this command cannot enforce the location
or permissions of an arbitrary caller.

## Supported subject

The initial path requires clean committed execution inputs and exact same-run
identity. By default the ledger is an execution input too. Only an explicit
reviewed `ledger_is_input: false` policy permits same-run ledger corrections
without rerunning a command: review must establish that the command and its
dependencies do not read that file. This narrow exception does not approve the
new review facts; acceptance still validates their current payload and sources.
All other tracked/untracked source changes invalidate reuse. The commit cannot
change, even for an administrative-looking edit. There is no cross-revision
cache, inferred path exclusion or pre/post-merge equivalence. As with ordinary
source observations, edit-and-revert changes during execution cannot be detected
by before/after snapshots alone.

Canonical producer commands currently require an absolute executable, preserving
their declared argv, working directory and timeout. This avoids claiming a PATH
lookup identifies the actual child on platforms with different search order.
The executable and wrapper bytes are checked before and after execution; their
runtime dependencies and external configuration still require environment review.
Unbound PATH/shim invocations retain the ordinary fresh-execution path.

GitLab uses its current pipeline identity and exact job name. GitHub currently
supports `push` and `workflow_dispatch`; pull-request synthetic merge subjects
must use fresh execution until separately modeled. The reviewed numeric workflow
ID is checked against the provider. Same-run does not mean identical environments:
each policy's environment and executable identity must match its own producers.

## Interfaces

- `opdev ci execute --policy PATH --suite ID --environment ID [--post-merge]`
  runs one reviewed canonical producer once. It emits `OPDEV_EXECUTION_V1 ` plus
  its JSON record to the job log. Exit 0 requires a successful unchanged execution;
  command failure or changed inputs exits 1; launcher/timeout errors exit 2.
- `opdev check --ci --reuse-ci-policy PATH --execution-environment ID` explicitly
  observes provider evidence, validates bindings, executes selected missing
  checks and evaluates. It does not silently fall back after rejected evidence.
  Omit both reuse options to request the existing fresh-execution path.
- Adding `--no-exec` prevents canonical/extension execution. The explicit reuse
  option still performs provider observation; this is not offline evaluation.
  The engine's separate evaluation API takes already validated, non-deserializable
  results and performs no provider calls. Missing checks remain unverified.
- `--post-merge` selects the declared integrated-trunk suites/extensions and follows
  the integration aggregate on CLIs with `check.post-merge-integration.v1`.
  Selected blocking checks affect integration and delivery, including unverified
  results after rejected reuse; delivery-only gaps remain separately visible.
  This does not treat pre-merge execution as interchangeable.
  It does not authorize release, publication, tagging or consumer migration.

No saved file can construct the engine's validated provider result. The immediate
evaluation must start within 30 seconds after record validation; expiry requires
renewed observation or explicit fresh execution. Additional checks may be long:
the CLI observes producer attempts again after evaluation, without rerunning
tests. Changed/unavailable attempts or changed review data invalidate readiness;
newly observed failures remain failures. All selected results remain
inspectable in the ordinary report, including failures and bounded output.

## Transport and limits

Requests use the existing read-only credential policy. GitLab job tokens do not
generally grant arbitrary job-inventory access; missing access stays unavailable,
not an automatic credential-scope change. Inventories are bounded to ten pages
of 100 jobs, and the observation budget is two minutes plus the final in-flight
request timeout. Logs are UTF-8 and at most 8 MiB. Missing or expired logs do not
establish execution. Retain provider logs long enough for the required evaluation
and independent review; no cleanup/retention setting is changed by OpDev.

GitHub's log API returns a short-lived download location. Follow only HTTPS
Actions log-storage locations, without forwarding API credentials, printing
signed URLs, accepting alternate ports or following another redirect. No tokens
or raw logs are included in provider snapshot serialization. Full check output
can contain private command output and must be reviewed before sharing.

Sources: [GitLab job logs and inventory](https://docs.gitlab.com/api/jobs/),
[GitLab job-token permissions](https://docs.gitlab.com/ci/jobs/ci_job_token/),
[GitHub job logs and run attempts](https://docs.github.com/en/rest/actions/workflow-jobs),
[GitHub Actions storage domains](https://docs.github.com/en/actions/reference/runners/self-hosted-runners).
