# Result and aggregation semantics

Every OpDev rule evaluation produces exactly one result.

## Results

### `passed`

The rule applied, its verifier completed successfully, and all evidence required
by that verifier is present and valid for the evaluated subject.

### `failed`

The rule applied and evidence demonstrates that its requirement was not met.
A failure cannot be overridden by an extension or converted to a warning.

### `unverified`

The rule appears applicable, but OpDev lacks sufficient evidence or permission
to decide whether it passed. Missing remote permissions, missing live evidence,
and stale qualification evidence are unverified rather than passed.

### `not_applicable`

The rule's declared applicability condition is demonstrably false for the
evaluated project or change. A reason and the evidence used to decide
applicability are required.

### `error`

OpDev could not complete the evaluation because the verifier, project command,
provider, or evidence parser failed. Errors are not project failures, but they
block any verdict that depends on the affected rule.

### `migration_required`

A brownfield project does not yet implement the rule and has recorded the gap as
migration work. This permits incremental development but is not compliance and
cannot qualify a delivery governed by that rule.

## Required result record

Every result MUST identify:

- Rule ID and rule-catalog version.
- Result.
- Subject, such as project, revision, artifact digest, environment, or change.
- Verifier and verifier version.
- Evaluation time.
- Evidence references or a reason no evidence was available.
- A concise diagnostic.

Results that execute a command also record the argument vector, exit
classification, duration, and redacted output reference. Secrets and sensitive
values MUST NOT be embedded in result records.

## Reporting execution

Agent reports MUST distinguish unattempted actions and predicted permission
requirements from observed denials and completed execution. A denial claim MUST
be supported by the matching invocation's tool result. An incomplete invocation
has an unknown result, not an inferred pass or failure. Before reporting actions
or blockers, reconcile claims with available execution evidence; preserve retry
history and qualify missing context. This is a reporting obligation, not a new
rule outcome, user approval step or project ledger. Do not execute mutations or
bypass permissions merely to obtain evidence for a report.

## Aggregation

A required rule contributes to an aggregate verdict as follows:

| Rule result | Aggregate effect |
| --- | --- |
| `passed` | Satisfies the rule. |
| `not_applicable` | Satisfies aggregation only when applicability evidence is valid. |
| `failed` | Blocks the verdict. |
| `unverified` | Blocks the verdict. |
| `error` | Blocks the verdict. |
| `migration_required` | Blocks compliance and qualified delivery, but may permit local development. |

An aggregate verdict passes only when every applicable required rule is either
`passed` or validly `not_applicable`.

## Gates

Rules may participate in one or more gates:

- `development`: whether ordinary local implementation may proceed.
- `integration`: whether a change may enter trunk.
- `delivery`: whether an artifact may be delivered through OpDev.
- `compliance`: whether the project may claim the selected OpDev or external
  profile.

A broken required trunk pipeline blocks new feature development even when local
development checks would otherwise pass. Repair, diagnosis, rollback, and work
needed to restore the pipeline remain permitted.

## Freshness and identity

### Daily integration target (catalog 2)

Daily integration remains the MinimumCD compliance requirement MCD-TRUNK-003,
but it no longer participates in development or integration decisions. A missed
or unverified day prompts a review of the blocker and the next small tested
change, not a retrospective merge prohibition or an approval exception.
Its actual result remains visible and still blocks a compliance claim when
unsatisfied. OpDev's operational gates therefore do not enforce every MinimumCD
practice: a successful merge gate is not proof of MinimumCD compliance.

Keep one integration branch, small changes and short-lived branches. Do not use
MCD-TRUNK-002 as a substitute one-day timer; assess branch origin, scope, drift,
reintegration and cleanup. All other applicable checks retain their blocking
behavior. This change neither executes a merge nor authorizes delivery.

This corrects a deadlock where historical delay prevented the integration needed
to recover. A generic override mechanism was rejected because it would introduce
approval and exception machinery without improving test confidence. Revisit the
policy if observed branch drift or conflicts increase; do not infer improvement
merely from fewer blocked reports.

The CLI's default exit follows development; `check --ci` follows integration.
With capability `check.post-merge-integration.v1`, `check --ci --post-merge`
executes the selected integrated-trunk suites/extensions and follows integration,
not delivery. Selected blocking post-merge checks contribute to both integration
and delivery, including unavailable reuse evidence. Delivery-only rules remain
visible and blocking for delivery, without preventing otherwise verified
integration. Pre-merge and post-merge executions are not interchangeable.
In post-merge evaluation-only mode, selected checks without verified execution
are reported as `unverified`, not omitted, even without a reuse policy.
`check --ci --delivery` executes the delivery stage and follows the delivery gate;
it cannot be combined with `--no-exec`. Inspect all reported gates before making
broader claims. An integration-only baseline does not enforce the release path.
Gate success is evidence, not developer authorization to release.

This separates integrated-source verification from an artifact's readiness to
ship: coupling both exits previously forced delivery work into ordinary milestone
completion. Reusing the integration aggregate with stage-selected checks keeps
failed post-merge suites and extensions blocking without creating a fifth gate
or weakening delivery rules. Ignoring the nonzero exit or adding a blanket waiver
would also hide genuine failures and was rejected. Regression tests cover exit
selection, all check outcomes, missing execution and rejected reuse; shared
guidance separately requires explicit release authorization. Revisit this split
if a required integrated-source risk cannot be represented by the existing
integration rules and selected post-merge checks, not merely because a delivery
gate remains blocked when release was not requested.

Declaring a trunk name or matching a provider default branch does not establish
the single-trunk workflow. Review branch roles and release source. A known
contradiction in the adoption workflow remains `migration_required` despite a
generic evidence assertion. Likewise project kind or omitted effectiveness risk
metadata cannot establish accessibility, operations or effectiveness exclusions;
absent reviewed capability evidence these rules remain `unverified`.

Evidence is valid only for the subject it identifies. Artifact qualification
MUST identify the artifact digest. Revision-only evidence cannot be silently
reused for a different artifact. Time-sensitive or external evidence MUST state
its freshness policy.

## Extensions

Extensions may produce additional results and may strengthen a gate. They MUST
NOT:

- Replace a core result.
- Change a core rule's applicability.
- convert `failed`, `unverified`, `error`, or `migration_required` to `passed`.
- Suppress required evidence.

