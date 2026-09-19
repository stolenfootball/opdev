# Acceptance-enforcement implementation review: 2026-09-19

## Delivered locally

The revised #46 increment adds typed schema-2 acceptance evidence to the existing
ledger and Rust evaluator. TEST-002/003 no longer qualify through a generic pass
assertion or testing-policy declaration. Mapping completeness, staged source
hashes/excerpts, review-payload binding and current canonical-suite outcomes are
checked together. Old ledgers remain readable but need explicit reviewed migration
for these checks. No external reporter, mutation framework, second agent or new
consumer policy file is required. The earlier staged-model prototype is unchanged
as historical comparison evidence, not shipped as the enforcement mechanism.

New bootstrap is unresolved and provides a separate acceptance section rather
than boolean approval questions for TEST-002/003. `evidence acceptance-digest`
computes a review subject only. Current-evidence views preserve typed evidence and
advertise schema 2 for schema-2 ledgers. Agent and generated managed guidance route
routine substantive development through requirements-first assertion review.

## Observed automated evidence

The final `cargo test --workspace --all-features` completed successfully, including
ten new process tests in `crates/opdev-cli/tests/acceptance.rs`, the engine's
verifier-error classification regression, and copied-plugin reference routing.
Canonical formatting, strict Clippy, plugin validation, skill validation and diff
whitespace checks passed. Four Docker/remote environment-gated tests remained
ignored; the timeout helper is separately ignored and invoked by its parent test.
These are local Windows results, not a cross-platform or post-integration CI claim.

Process coverage includes:

- A valid current mapping plus current-check suite execution qualifies the two
  acceptance rules; the same automated mapping with `--no-exec` does not.
- Missing/pending mappings, empty material inventories, unknown suites and stale
  sources remain unverified despite green suite execution.
- A reviewed mapping contradiction or failed review stays failed despite a green
  suite. Suite failure is failed, missing executable is error, skipped stage is
  unverified, and source mutation during a passing command invalidates binding.
- Changed requirements, mapping text, work reference, review digest or staged
  content invalidate the prior review. Generic project/change assertions in an
  old ledger cannot override this requirement.
- Duplicate/unknown condition IDs, unknown fields, invalid outcomes, unsupported
  version/field combinations, ledger self-reference and invalid paths are rejected.
- Appropriate review-only non-code evidence can satisfy TEST-002 without implying
  execution; behavioral testing is then explicitly not applicable.
- Bootstrap remains unresolved/read-only and compact views retain acceptance data.

The observed TaskTray-style red/green regression executes real Python fixture
code through the declared suite, while production enforcement remains Rust:

| Fixture state | Actual suite | Acceptance verdict |
| --- | --- | --- |
| Incorrect sorted/extra-item code and matching incorrect assertion; review records disagreement with R1 | passed | failed |
| Requirement-derived assertion expecting the correct two items; code still wrong | failed | failed |
| Same meaningful assertion; implementation corrected | passed | passed |

The first row is **not automatic semantic discovery** by the CLI. The fixture
supplies the reviewed contradiction, and verifies that a green suite cannot hide
it. Another deliberate regression shows that a false semantic interpretation with
valid references and an asserted review can pass mechanical validation. This is
an explicit limit: reviewer competence, authority authenticity, inventory
completeness and individual test selection are not established by these checks.

Intermediate failures were retained in the session: the old schema disallowed
acceptance-only changes with empty generic assertions; old bootstrap/adoption
fixtures expected blanket passes; generated/repository managed guidance initially
diverged. The schema, fixtures and guidance were corrected rather than weakening
the new gate, and the complete suite was rerun. Strict-lint failures were also
corrected before its final passing run.

## Independent forward assessment

A fresh-context read-only subagent received the new acceptance reference and the
two synthetic packets, without the grader rubric or prior result documents. No
provider calls, writes, test execution, fabricated hashes or approval actions were
authorized or reported. This was a skill-authoring check, not an installed
Codex/Claude end-to-end trial.

- **TaskTray:** identified R2 sorting and R3 count/assertion contradictions;
  distinguished R1's supported assertion from absent R4 nonmutation protection;
  rejected the CSV availability claim without demanding deferred implementation;
  kept TT-R8 execution unverified because the receipt describes TT-R7. Proposed
  correction and real revision-bound evidence before qualification.
- **PaperMap:** supported E1-E3 from the current passages, respected actual
  supersession, frozen 1.9 history and future-only 2.1 scope. Chose nonbehavioral
  review mappings, not no-material-conditions, and did not claim runtime or
  publication qualification.

The same static review found three issues in the initial implementation/guidance:
success wording implied automated execution for review-only evidence; Git reference
errors were collapsed into missing evidence; guidance overgeneralized `--no-exec`.
These were corrected and error/conditional-execution behavior covered by automated
checks. There was no second independent pass; do not present the first review as
verification of subsequently edited bytes.

Final local reference identities (SHA-256):

- `references/acceptance.md`: `461790a24eafecaefd4646e14acaf57ab2b9dbfcc44f26cf105017f716c88863`
- `opdev-project/src/acceptance.rs`: `90e12b5510e2a4038947879ca14871ddd15e7a44094d4d8840c01c19dc7ce4b9`
- `opdev-engine/src/acceptance.rs`: `77f47d6724b35f4aa7dea130d5eaf27999057b0b77d54bd24e43d23103a3df82`

## Remaining integration evidence

The repository's own unstaged working-state `check --no-exec` reported acceptance
as unverified and gates blocked. That is not a completed acceptance assessment;
neither the old ledger nor passed local tests were relabeled as qualification.
The installed plugin/runtime was not upgraded, and no consumer ledger was migrated.

At the initial handoff, work remained local/uncommitted. Maintainer wording review, final staged
change evidence, MR/CI and post-integration verification remain before integration.
No release, managed-plugin installation or issue closure is claimed. Fresh real-
host usage of the new schema-2 workflow remains unverified. The first slice provides
mechanical enforcement around reviewed evidence, not universal semantic correctness
or authenticated test-case-level execution provenance.

Representative wording for maintainer review:

> Acceptance review is pending, incomplete or bound to a different inventory/mapping payload

> Exact-change sources and all inventoried mappings checked; any mapped automated suites passed in this check. Review-only mappings do not imply execution. Semantic adequacy and inventory completeness remain identified reviewer claims, not machine proof

> Print the current acceptance review's subject digest without approving it

## Integration follow-through

The maintainer subsequently reviewed the three messages reproduced above and
responded "That's fine", then directed "Follow through, finish the issue".
The actual conversation used a typographic apostrophe in the first response.
[Issue note 3866668136](https://gitlab.com/stolenfootball-tools/opdev/-/issues/46#note_3866668136)
records that scoped wording approval and authorization. It is not a semantic
correctness, universal accessibility, release or consumer-upgrade approval.

This repository's existing ledger is explicitly migrated to schema 2 for the
increment, retaining historical records and adding reviewed current acceptance
mappings. The source-built CLI supplies the capability; the installed plugin and
managed runtime remain unchanged. Exact-head MR/CI and post-integration results
belong in issue 46 so later progress does not invalidate this source-bound review.
