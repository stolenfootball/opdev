# Requirements and verification

OpDev's optional requirements catalog records what your software promises and
how those promises are checked. It is a set of versioned JSON files in Git, not
a database service and not a history of every test run.

This capability requires `requirements.catalog.v1`, explicit requirements policy
1, layout 2, project schema 3 and MR/PR review storage 2. Installing a newer plugin
does **not** select these policies or migrate your project.

## What belongs where

| Information | Home |
| --- | --- |
| Supported product guarantees, criteria, verification plans | `.opdev/requirements/<capability>.json` |
| Existing authoritative specifications | Their declared location; reference an exact fragment rather than copying it |
| Test implementation and fixtures | The project's normal test/source directories |
| Commands, suites, stages and supported configurations | `.opdev/project.yaml` |
| Current change intent, one-off conditions, decisions and observations | Existing MR/PR |
| Milestones, deferred features and progress | Existing work tracker |
| Detailed test output | Existing CI/local reporting with bounded retention |

Use meaningful capability files such as `storage.json` or `authentication.json`.
Do not create a file for every helper function, store transcripts or test logs in
the catalog, or move the project's whole backlog into it. IDs remain stable when
a record moves between files. There is no committed SQLite cache or evidence repo.

## The model

A **requirement** is a supported promise, such as “rejected input does not alter
stored data.” A **criterion** makes it observable: submit malformed input against
nonempty storage, receive the documented error, and verify unchanged bytes.

A **verification** points to an actual assertion or a repeatable manual procedure.
A **plan** links a criterion to all the verifications needed for one named
configuration and stage. A **mapping review** explains why those assertions are
adequate. Multiple requirements can reuse one verification; several checks can
jointly establish one criterion. Every required plan member must pass.

An empty criterion list or empty plan is incomplete, not a pass. A group of green
component tests does not automatically prove the assembled product behavior:
write and verify that consumer-facing criterion too.

The complete format is [requirements.schema.json](../schema/requirements.schema.json).
The [normative contract](../spec/requirements-catalog.md) defines identities,
freshness and qualification boundaries.

For example, a capability document contains the following records (the repeated
post-merge plan is shown explicitly so its separate obligation is visible):

```json
{
  "schema": 1,
  "requirements": [{
    "id": "R-storage-rejection",
    "title": "Safe rejection",
    "statement": {"kind": "inline", "text": "Rejected input must not alter stored data."},
    "origin": "Accepted storage contract",
    "rationale": "Prevent data loss on invalid input.",
    "configurations": ["default"],
    "criteria": [{"id": "C-preserved-bytes", "expected": "Malformed input produces the documented error and leaves nonempty storage byte-identical."}]
  }],
  "verifications": [{
    "id": "V-rejection",
    "target": {
      "path": "tests/storage.rs",
      "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
      "excerpt": "assert_eq!(before, after);"
    },
    "inputs": [],
    "method": {"kind": "automated", "suite": "storage", "assurance": "suite"}
  }],
  "plans": [{
    "id": "P-rejection-pre", "criterion": "C-preserved-bytes",
    "configuration": "default", "stage": "pre_merge",
    "members": [{"verification": "V-rejection", "assertion": "Checks exact storage bytes after the documented rejection.", "discriminating_case": "Start with nonempty data; malformed input must not erase or partially overwrite it."}],
    "review": {"outcome": "unverified", "reviewer": "", "reference": "", "rationale": "", "subject_sha256": ""}
  }, {
    "id": "P-rejection-post", "criterion": "C-preserved-bytes",
    "configuration": "default", "stage": "post_merge",
    "members": [{"verification": "V-rejection", "assertion": "Checks the same guarantee on integrated source.", "discriminating_case": "Integrated code must reject malformed input without changing nonempty storage."}],
    "review": {"outcome": "unverified", "reviewer": "", "reference": "", "rationale": "", "subject_sha256": ""}
  }]
}
```

This is an **unverified authoring example**, not a qualification record. Replace
the placeholder target with `requirements bind` output, declare the real `storage`
suite at both stages, list actual inputs and review actual assertions. An excerpt
must identify one occurrence; include test-name/context lines if it is ambiguous.
If you also select `local` in policy, add its plan or inspect the still-visible gap.

## Select the policy deliberately

In an explicitly reviewed project contract, the additional selection is:

```yaml
layout:
  version: 2
  review_reference: <actual migration decision>
assurance:
  # Keep the existing engineering, review_storage, safeguards and profiles.
  requirements:
    version: 1
    review_reference: <actual catalog decision>
    configurations:
      default: [local, pre_merge, post_merge]
```

This is a fragment, not a replacement contract. Configuration names describe
real supported modes; the mapped suites and their exact arguments must exercise
those modes. Naming a configuration does not configure the application or prove
which mode a test ran. Pre-merge and post-merge verification are mandatory for
each supported configuration; other stages are additive.

New projects use the reviewed full contract with `init --project FILE`. Existing
projects use the coordinated `upgrade --migration FILE` preview/application
workflow. Its optional `requirements` object maps exact capability paths to
explicit documents. Imported mapping reviews start `unverified`; migration
cannot promote old approvals or invent requirements from green tests.

The existing migration workflow retains temporary recovery protection, refuses
changed inputs and preserves later edits. Under MR/PR storage 2 it removes the
obsolete active ledger without requiring a permanent archive. Verify the migrated
project, then remove temporary recovery material through the existing cleanup
procedure. Migration writes alone do not establish completed adoption.

## Add or change a guarantee

1. Start from the accepted behavior, not from whatever the code currently does.
   Give the requirement and each observable criterion stable unique IDs. Keep a
   normative statement inline **or** reference its existing specification—not both.
2. Reuse or add discriminating tests, including relevant boundary and failure
   behavior. Stage the intended test/specification files before binding them.
3. Obtain an exact Git-blob reference:

   ```sh
   opdev requirements bind tests/storage.rs --excerpt 'assert_eq!(before, after);'
   ```

   This prints `path`, `sha256` and `excerpt`; it writes nothing and does not
   approve the assertion. Use the returned object as a verification `target`.
   List known fixtures, helpers and selector inputs in `inputs` too.
4. Define one plan per required criterion/configuration/stage. Each member names
   its verification, explains the assertion's relationship to the criterion and
   supplies a discriminating case. New reviews start `unverified`.
5. Stage the catalog and inspect:

   ```sh
   opdev requirements inspect
   opdev requirements show R-storage-rejection
   opdev requirements diff --base <accepted-full-commit>
   ```

   `inspect` reports all discovered input gaps and expected review digests.
   `show` returns the stored requirement and complete linked neighborhood, not
   an inferred pass. `diff` identifies added, removed and changed records; it does
   not infer semantic weakening or consent. For migration preview, `inspect
   --project FILE` reads a candidate contract without selecting it.
6. Review actual assertion meaning, known input dependencies, applicability and
   limitations. Record the real reviewer, review reference, rationale, outcome
   and inspected subject digest. Computing a digest does not perform this review.

Each read-only command runs no project tests, makes no provider calls, and does
not qualify a gate. Inspection exit 0 means no input findings, not verified
software. Exit 1 reports findings; invalid/unreadable inputs use the CLI's error
exit. Inspect staged source intentionally; unstaged edits are not silently included.

## During a normal change

Keep enduring promises and test links in the catalog. The current MR/PR acceptance
record selects `requirements.catalog_sha256`, the provider-observed
`baseline_commit`, `baseline_catalog_sha256`, an impact `rationale` and the actual
`decision_reference`. The ordinary change review binds this selection to current
source. Do not copy every enduring criterion into every MR.

After selecting the provider's actual baseline commit, `requirements diff --base
COMMIT` prints `base_catalog_sha256` and `candidate_catalog_sha256`. Use those as
the review's `baseline_catalog_sha256` and `catalog_sha256`; do not reimplement
the canonical digest or treat the resulting values as approval. The CLI rechecks
both identities when qualifying the change.

GitLab supplies the selected diff's target snapshot (`diff_refs.start_sha`);
GitHub supplies the PR base snapshot (`base.sha`). Missing/changed metadata does
not fall back to an arbitrary local commit. Fetch a needed exact Git object
through your normal workflow; catalog readers do not fetch automatically.

Keep one-off acceptance conditions in the MR/PR. Existing capability safeguards
can link to a catalog criterion ID or an ordinary change condition. Neither path
waives the mandatory engineering checks. A refactor can preserve the catalog
while still needing current execution and a review of its impact.

Review deliberate removals, narrowed guarantees or applicability changes against
the baseline and original developer authorization. A textual assertion change may
strengthen a test; it is not automatically weakening. Do not invent consent or
move a supported guarantee into “future work” merely to get green checks.

## What invalidates a result?

Mapping adequacy and execution have different lifetimes:

- Changes to the criterion, configuration, stage, assertion source, declared
  helpers/fixtures, suite/command or relevant policy invalidate its mapping review.
  Whole referenced files are bound conservatively. Unknown/shared dependencies
  require broader review; a declared input list is not a complete dependency graph.
- An unrelated implementation edit can preserve the mapping review, but it
  invalidates the earlier execution. Run the required checks against current source.
- Editing a stored report or cache cannot qualify execution. The normal `check`
  invocation uses current commands or the existing authenticated same-run reuse
  mechanism. It does not launch an extra run just for catalog evidence.

All applicable plans use current results at the selected stage. A failed check
and a stale review remain separate visible findings. Missing, skipped, filtered
or unavailable required verification must not be represented as passing.

## Suite evidence and manual checks

Automated `assurance: suite` means a reviewed mapping plus current canonical
suite execution. Exit zero alone does not prove that a named case executed or
that a runner exposed every retry. Review filtering, skips, quarantine and failure
propagation. A quarantine leaves an explicit assurance gap; it is not a waiver.

`assurance: case` requests a stronger observation than this first implementation
can supply. It remains **unverified**, even with a green suite. No runner-specific
adapter or JUnit dependency is installed and there is no silent downgrade.

Manual methods identify inspection, analysis or demonstration, the automation
limitation, recheck procedure and `max_age_seconds`. Their current results belong
in the enclosing MR/PR catalog change review's `manual_observations`, not in the
catalog: exact plan/member, plan subject, observation time, result, observer,
actual observed behavior, context/limits and reference. Missing, ambiguous,
expired, future-dated or differently bound observations remain unverified.
An attributed observation is not authenticated human consent.

## Adoption, phases and retention

For a new project, inventory the first useful supported slice and its safeguards.
For an existing project, assess actual promises, contracts, tests and risks;
passing old tests does not establish a complete inventory. During an upgrade,
transfer only facts whose meaning remains valid, resolve gaps explicitly and
remove superseded active artifacts through reviewed migration.

Future phases stay in the work tracker until selected for implementation. Add
their supported guarantees alongside the code and tests. Supported experimental
configurations are explicit configurations, not hidden exclusions.

Keep the current catalog and ordinary Git history. Replace outdated current
mapping judgments instead of appending an audit ledger. Keep useful decisions
and failure findings in existing work. Routine CI detail expires under the
selected retention policy; release provenance retains its separate owner.
Expired reports neither revoke a past merge nor qualify today's code.

OpDev checks consistency with a reviewed baseline. It cannot prove that the
baseline contains every requirement or that arbitrary assertions are meaningful.
That limitation is why honest review and representative consumer tests remain
part of the workflow.
