# Native integration-candidate source checks

Engineering policy 3 can reuse reviewed equivalent source checks from a native
integration candidate instead of executing them again after merge. This is an
opt-in optimization, not a requirement to buy a provider feature or redesign CI.
The ordinary fresh pre/post-merge path remains supported.

## What must be established

The provider must identify a successful native candidate and its actual merge.
The checkout must be the observed integrated commit, and the candidate and
integrated Git trees must match. This is necessary, not sufficient: canonical
commands, complete source/configuration inputs, environment/toolchain/dependencies,
wrapper and executable digests, reviewed reuse policy, current producer attempt
and bounded execution age must also match.

The selected suites must be source-only and independent of commit, branch/ref,
event and stage metadata. Review the actual commands and transitive dependencies,
including generated version metadata and conditional CI configuration. Merely
setting the booleans below does not prove those properties. The immutable
environment identity must cover external/effective CI inputs, dependency resolution
and the trusted producer clock. Unknown or mutable external inputs are not eligible.
This protocol trusts the reviewed producer and its environment; it does not attest
to a compromised runner or prove a review assertion true.

Do not reuse deployment, live-environment, artifact-dependent, recovery or statistical
checks whose inputs/decision procedures differ. Artifact qualifications retain their
separate immutable artifact identity and required checks. The current source-bound
post-merge acceptance review is still necessary; an earlier review is not reused.

## Provider boundaries

- GitLab requires the selected MR's completed native merge train, exact successful
  pipeline, project and target. Stale, fresh, merging and `skip_merged` entries do
  not qualify. Missing subscription capability or job-token access means use fresh
  execution, not a new personal-token workaround.
- GitHub requires a `merge_group` run for the declared target and **the exact commit
  reported as integrated**. Rewritten commits are deliberately unsupported by this
  first implementation; tree equality alone cannot supply the missing relationship.
  Ordinary PR/push runs cannot masquerade as a merge group.

Provider APIs and logs are observed again after evaluation. Changed or unavailable
evidence at that point invalidates reuse instead of silently running checks again.
An initial unavailable/equivalence observation falls back to fresh canonical checks
and remains visible in the report. Current observed failures are not replaced by
an older green attempt.

## Select explicitly

Require `ci.native-candidate-reuse.v1` in both producer and evaluator, engineering
policy 3 and execution-policy schema 2. Retain all ordinary execution-policy fields
and exact producer mappings from the [same-run protocol](../spec/execution-reuse.md).
Add the following to the reviewed tracked policy (this is a fragment, not a complete
working policy):

```json
{
  "schema": 2,
  "candidate_reuse": {
    "review_reference": "actual decision and command/input review",
    "suites": ["unit"],
    "commit_ref_stage_independent": true,
    "source_only": true,
    "maximum_age_seconds": 3600
  }
}
```

Choose an appropriate age between one second and one day. Configure the trusted
producer with `opdev ci execute --policy <file> --suite <id> --environment <identity>`
in the native candidate job. Schema-2 production includes its completion time;
legacy schema-1 records cannot supply missing freshness. The `OPDEV_EXECUTION_V1`
log envelope carries the explicit record schema; older readers reject schema 2.

After integration, identify the earlier native run in an ephemeral selector outside
the checkout. This file contains selectors, never trusted evidence:

```json
{
  "run": {
    "revision": "FULL_NATIVE_CANDIDATE_COMMIT",
    "run_id": 123,
    "reference": "PROVIDER_RETURNED_REF",
    "source": "PROVIDER_RETURNED_SOURCE",
    "workflow_id": null
  },
  "change": 7,
  "integrated_revision": "FULL_PROVIDER_REPORTED_MERGED_COMMIT"
}
```

GitHub needs the numeric workflow ID; GitLab forbids one. Then run the normal
post-merge check with its current review selection and these additional options:

```text
opdev check --ci --post-merge --reuse-ci-policy <tracked-policy.json> --execution-environment <identity> --reuse-native-candidate <external-selector.json>
```

Only selected eligible suites are reused. Other required suites/extensions execute
normally. Reports identify the actual earlier run/source and explicitly distinguish
equivalent verification from execution after merge. The separately selected literal
MinimumCD post-merge obligation does not pass from this optimization.

## Rationale and limits

[GitLab merge trains](https://docs.gitlab.com/ci/pipelines/merge_trains/) and
[GitHub merge queues](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue)
test native integration candidates; they are not generic caches for any green run.
[GitLab's train API](https://docs.gitlab.com/api/merge_trains/) supplies exact
train/pipeline/MR correspondence. Feature availability and credential scope vary.
No provider setting, credential, release permission or project policy changes merely
because this capability is installed. API-contract fixtures are not live-provider
qualification or measured speed savings.
