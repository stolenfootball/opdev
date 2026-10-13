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

Use native preparation rather than constructing identifiers and hashes by hand.
`adoption prepare-evidence` supplies an unresolved partial worksheet with the
selected policy's rule IDs. Its adoption/cleanup/pipeline assertions supplement,
not replace, complete acceptance conditions, requirement changes and safeguards.
Both the work and testing assertions need their actual adoption review evidence.

Use `evidence prepare` and discussion export for the reviewed current record;
post through the authorized provider workflow and inspect the actual posted body.
For pre/post-merge CI, `ci prepare-review` supplies the locator and proposed
description change. Recheck the original description before applying it. A
literal filename or stale comment selection is not the intended review.

Run `adoption check --preflight` with the same exact review arguments planned
for verification. Repair missing or stale inputs before expensive suites. This
checks input readiness only. Then `adoption check` runs the referenced pre-merge
suites itself; do not run them immediately beforehand merely to produce another
report. Distinct remote, platform and integrated-trunk checks still apply.

In CI, `ci review-check` does not export its selection to a subsequent command.
The actual gate must still use `check --ci --review-ci` (and `--post-merge` at
that boundary), or explicit locator/acceptance arguments. A blocking manual job
may wait for review selection without adding a new human approval ceremony.

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
must still match. Native `--review-ci` selection currently supports pre/post-merge,
so delivery uses an explicitly selected locator and acceptance identity.

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
