# Reviewable evidence

## MR/PR-native review (selected storage version 2)

With `assurance.review_storage.version: 2`, require `evidence.discussion-review.v1`.
Keep acceptance conditions at their existing work/design authority and their actual
assertion mappings in a bounded review on the existing code repository's MR/PR.
Do not create an evidence repository, accumulating ledger or permanent copy of
the old ledger. Current reviewed source, configuration, condition/test bindings
and actual stage execution remain required. Finalize at integration handoff,
not on each local edit or feedback cycle.

Prepare the one current record outside source with the existing bootstrap/prepare
flow, then `evidence bundle export-review --discussion --stage STAGE --ledger FILE
--output NEW_FILE`. It renders a review section, does not post or approve it.
Review sensitive content before authorized posting through the existing provider
workflow. Select its exact MR/PR/comment, source-head commit, whole-body SHA-256
and independently reviewed acceptance ID; use `check --review-locator FILE
--review-acceptance-sha256 ID`. Keep temporary selection inputs outside source or
in the reviewed CI input path. Changed body/head/source/conditions need current
review; never choose an earlier green note. Check each required stage, without
rerunning equivalent suites for paperwork. Local preparation is not provider
authentication, human consent or execution.

With `evidence.review-handoff.v1`, use `ci prepare-review --change NUMBER --note ID
--acceptance-sha256 ID [--post-merge]` after the reviewed comment is posted. Inspect
its proposed description and locator; it writes nothing and preserves other-stage
selection and unrelated text. Recheck the original description digest immediately
before an authorized provider edit. Do not derive approval or the independently
selected acceptance ID from the comment. In CI, `ci review-check [--post-merge]`
checks input readiness before expensive tool setup; actual gates still use current
review and real execution, never a preflight receipt.

Discussion export now requires `evidence.discussion-wire.v2` on both local and CI
readers. It losslessly shares repeated strings within the bounded note; the full
typed review, assertions and digests are unchanged. Existing v1 notes remain
readable. This is not the experimental compact-context feature, a larger retention
store or permission to omit review details. Missing reader support is a capability
gap; do not silently install a new runtime.

Requirements may use a minimal `observe-work` observation directly as condition
`source`; tests still reference tracked assertions. This avoids committing a
duplicate of issue requirements. Use the original issue or a separate description,
not the generated review's own body. The CLI rechecks current provider content;
saved observations alone cannot qualify it.

Use the reviewed `report_retention_days` (recommend 30) for detailed routine CI
reports, including failures. Review actual provider expiry/keep-latest behavior;
configuration is not proof it operates. Keep a concise outcome and significant
failure/retry findings in existing work, not every log or invocation. Keep active
verification inputs available until the handoff completes. Expired old reports
limit historical detail; they neither invalidate past merges nor satisfy new
checks. Actual releases retain artifact-bound provenance/qualification separately,
without granting release authority. Remove owned temporary drafts/rollback files
when no longer needed; do not replace the ledger with an accumulating local archive.
Legacy paths below apply only to their explicitly selected policies.

## Legacy evidence paths

For TEST-002/003 use typed schema-2 [acceptance evidence](acceptance.md), not
generic rule assertions. Capable bootstrap includes an unresolved acceptance
section. Schema-1 ledgers remain readable but cannot satisfy those strengthened
checks. Migration is explicit and reviewed; preserve history and unrelated rules.

In legacy projects, use `.opdev/evidence.yaml` only when a core rule accepts evidence and the CLI
cannot infer the fact safely. It is not a waiver or override mechanism.

If the actual schema-3 policy selects `assurance.review_storage.version: 1`, require capability
`evidence.authenticated-review.v1`; do not recreate the old ledger. Obtain the exact
archive locator and expected review identity from the existing authorized work
authority. `check --review-locator FILE --review-acceptance-sha256 ID` reads that
provider-owned semantic review and still requires actual stage checks. A saved
execution report, local download, author label or matching hash is not approval or
fresh execution. Do not infer the latest acceptable record or choose an older green
attempt. Missing or conflicting evidence stays unresolved.

For that explicitly migrated policy, prepare candidates outside source using
bootstrap `--output` or prepare `--ledger-input` and new `--ledger-output`; review
the same conditions/assertions before export. `evidence bundle export-review`
generates mechanical bindings without upload or cleanup. Keep retained history
and independently verified recovery at the reviewed storage authority; export
alone never authorizes deleting a legacy ledger. Installing this capability does
not select it or change any consumer's policy.

When mutable issue/MR content is needed as evidence, `evidence bundle observe-work`
can capture an explicitly selected exact excerpt with original body hash, item and
author identities and timestamps. Retain enough scope to interpret it without
copying raw private conversations. `--expected-body-sha256` rechecks a known body;
changed text requires reviewing the current decision, not silent reuse. Bind
needed observations with export-review `--work-observation`. Authorship is not
permission: preserve original scope and check applicable revocations/supersession.
Do not perform this capture for every routine task or treat metadata as semantic
proof. Existing ledger instructions below apply only to unmigrated projects.

By default, use the full ledger and fingerprint workflow below. Only after an
explicit user or project opt-in to the compact-context experiment, check that
`opdev --help` lists `--experimental-compact`. When available, stage material files and use
`opdev --experimental-compact evidence show --current` to inspect durable facts and only the exact
matching change. Add `--rule <ID>` when investigating one rule. Missing evidence
is explicit; these are input assertions, not evaluated passes. Requery after
the staged state changes. The view never rewrites the ledger or prepares new
approvals. Older compatible CLIs retain the fingerprint-and-ledger workflow
below; do not replace a runtime just to enable the query.

For change evidence:

1. Stage every material file except that the ledger may remain unstaged.
2. When no ledger exists, run `opdev evidence bootstrap` and save its output
   outside the Git working tree. Every decision starts as `review_required`.
3. Review the generated project and change candidates. Add concrete shared
   evidence to each used scope, provide a change work authority, and replace a
   decision only with a justified `passed` or `not_applicable`.
4. Preview the expanded ledger with `opdev evidence bootstrap --answers PATH`.
   Create it only after review with the same command plus `--write`.
5. Stage the ledger and run the applicable check.

Bootstrap is create-new only. It validates the questionnaire schema, exact
candidate set, staged fingerprint, evidence, and rule support. It never chooses
a satisfying result and cannot update an existing ledger. For direct ledger
maintenance, use `opdev evidence fingerprint`, then add assertions under the
matching change entry with a concrete work authority.

The fingerprint excludes only the ledger. Unstaged tracked content and other
untracked files cause fingerprinting to fail. Any later material repository
change invalidates the assertions automatically.

Project assertions are for durable capabilities or policies, not current-change
acceptance. Use them sparingly. An agent's statement that its own output is
correct is never sufficient evidence. Evidence cannot override a concrete
failure, error, or migration requirement.
