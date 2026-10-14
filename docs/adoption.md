# Adoption without late handoff surprises

Adoption connects a project's existing development path to reviewed OpDev policy.
It is not complete because configuration files exist, and it does not authorize
a release. Keep decisions and migration progress in the existing work tracker;
keep enduring project instructions at their declared owners.

## Start with the whole path, implement a small useful slice

For an empty project, settle the first useful behavior and material tooling and
policy choices, then connect that behavior to runnable verification. For an
existing codebase, preserve adequate tools and inspect actual CI inheritance and
consumer commands. For an older OpDev project, preserve settled choices and
compare the selected destination with both local and CI runtime capabilities.
Reopen changed gaps, not every previous decision.

Before broad migration or full test runs, trace these boundaries:

| Boundary | What to establish early |
| --- | --- |
| Preparation | Current source, selected policy, real test mappings, catalog/safeguard inputs and adoption assertions can form one valid review. |
| MR/PR CI | The selected posted comment is accessible with the job identity and the actual check command receives the selection. |
| Integrated trunk | The review describes the integrated source and correct stage, with the actual prior-stage observation where required. |
| Applicable delivery | The delivery-stage review identifies the exact artifact/inventory, and publication depends on qualification in the supported trigger path. |

Use the smallest safe connected test for uncertain boundaries. A provider mock
can verify request and identity handling; it cannot prove live CI permissions.
CI lint and flag-presence assertions cannot prove the generated inputs work in
the reader. Keep untested live boundaries visible. This is an early connection
check, not another full qualification or a new permanent adoption artifact.

## Prepare once, inspect cheaply, execute at the right boundary

With `adoption.coordination.v1`, start with `opdev adoption next`. It reads the
current contract, choices, approval claims, cleanup and staged source and returns
dependent actions alongside independent work. It does not execute those actions,
authenticate consent, cache a completed step or claim adoption passed. Reinspect
after changes or interrupted work. A ready planning action is not permission to
apply its result. `init --project` owns new scaffolding; reviewed `upgrade
--migration` preview/apply/resume owns changes to existing policy. Keep its recovery
file outside source until verification, then remove it. Resume the original plan,
not a newly inferred replacement; unexpected later edits must survive.

Use native preparation rather than constructing identifiers and hashes by hand.
With `adoption.complete-review.v1`, use `adoption prepare-evidence --complete`
(add `--base` with the full provider-observed target commit for requirements
policy). It generates the whole unresolved bootstrap questionnaire: selected rule
IDs, adoption/cleanup/pipeline evidence slots, catalog identities and capability
impact/objective slots. Resolve the actual judgments and feed it to `evidence
bootstrap --adoption --answers FILE`. Null impacts and empty summaries intentionally
cannot become completed review. No rule is marked passed by preparation.
The older command without `--complete` remains a partial worksheet, not a full
answers file. Requirements and safeguard declarations still need substantive review.

Use `evidence prepare` and discussion export for the reviewed current record;
post through the authorized provider workflow and inspect the actual posted body.
For CI, `ci prepare-review` supplies the locator and proposed
description change. Recheck the original description before applying it. A
literal filename or stale comment selection is not the intended review.

Run `adoption check --preflight` with the same exact review arguments planned
for verification. Repair missing or stale inputs before expensive suites. This
checks input readiness only. Then `adoption check` runs the referenced pre-merge
suites itself; do not run them immediately beforehand merely to produce another
report. Distinct remote, platform and integrated-trunk checks still apply.
Exit 0 verifies the evaluated controls, not unobserved integrated CI. JSON reports
`controls_verified` separately from `integration_qualified`; `complete` requires
both. After local verification, reconcile the exact current trunk with `ci qualify`
and retain the two observations in the existing work item. Do not rerun local
suites merely to obtain a combined completion label. Explicit `adoption check
--remote` can include that live observation when a combined run is actually needed.

With `ci.verify.v1`, a pipeline can use `ci verify --stage pre-merge`,
`--stage post-merge`, or `--stage delivery`. Each invocation selects and validates
its own native review and runs the existing required gate once, stopping remaining
suites on failure. Add `--preflight` for input inspection before expensive setup;
the later verifier rechecks everything rather than consuming cached readiness.
`adoption check --review-ci` connects the adoption-specific gate to the same reader;
it is not a suite to register recursively. Local adoption keeps explicit locator
and acceptance arguments. Neither adapter publishes or changes policy.

Existing `ci review-check` does not export its selection to a subsequent command.
The low-level gate still needs `check --ci --review-ci` and its stage flag, or
explicit locator/acceptance arguments. Keep project setup, tests and runtime
installation unchanged unless separately reviewed. Add only the integration call
to the existing required job; do not generate a replacement pipeline merely to use
the adapter. A blocking manual job may wait for selection without requiring an
extra human approval. GitHub PR retries must still check out the actual PR head,
not the synthetic merge commit. Built-in job credentials stay in CI; never export
a developer's local login. Local GitHub preparation can reuse the existing `gh`
login; Actions uses its own built-in token and never that local fallback.
Unsupported event contexts fail closed.

Static adapter inspection recognizes the reviewed signed-release bootstrap as
well as same-job locked source builds; consumer projects do not need to contain
OpDev's Rust source. Changed installers, inheritance or wrappers may remain
unverified until their effective setup is reviewed. Do not rewrite a product's
toolchain or pipeline merely to fit a recognized example. Static recognition is
not proof that the selected runtime has the required capabilities or ran in CI.

## Source head, integrated source and artifact are different identities

The discussion locator's `source_commit` always identifies the provider's MR/PR
head. Do not fill it from a tag or merge checkout merely because that is the
current commit. Source-head and merge commit IDs can differ while their files
are identical. OpDev compares the staged tree with the selected provider commit;
it does not require all Git commit IDs to be equal.

With `evidence.merged-delivery.v1`, delivery for a merged change uses its
provider-observed integration commit. Missing integration metadata or objects
fails closed; the old branch is not a fallback. Unmerged candidates still use
their source head. Neither case substitutes a pre-merge review for a delivery
review: current source, configuration, stage, acceptance and artifact identity
must still match. With `evidence.delivery-handoff.v1`, `ci prepare-review --delivery`
adds a separate delivery selection without replacing pre/post selections. Native
delivery discovery in CI uses the associated merged change and exact integrated
checkout. An unmerged candidate uses an explicit local locator; it is not silently
treated as an integrated CI delivery. Older runtimes need explicit delivery
selection or an approved compatible upgrade, never guessed flags or a changed pin.

Rehearse the handoff without publishing. Adoption tests the controls; an actual
release later requires developer authorization and fresh artifact qualification.
Do not tag, publish or run release-specific rollback solely to finish adoption.

## Diagnose the failed boundary

Before declaring an OpDev defect, retain the failing command, expected behavior
and smallest safe reproduction in the work item. Separate bad inputs, unsupported
capabilities, unavailable access and product defects. Trace actual comparison
semantics rather than interpreting a helper name or a different SHA as proof.
If reproduction is unavailable, report a hypothesis with the discriminating
check still needed. Preserve failures and continue independent authorized work.

See the [adoption contract](../spec/adoption.md),
[review handoff](../spec/evidence-lifetimes.md) and
[requirements guide](requirements-and-verification.md) for precise contracts.
