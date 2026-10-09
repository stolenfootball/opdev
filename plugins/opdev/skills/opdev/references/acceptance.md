# Acceptance evidence for substantive changes

Use during normal OpDev development, not only requested audits. Review-only tasks
still authorize only reading and an answer, not tests, ledger writes or faults.

Before implementation, identify material acceptance conditions and selected risk
objectives from actual accepted authorities. Derive discriminating examples and
expected results from requirements, not current code. Ask about material ambiguity;
never change requirements or infer consent to make tests and code agree. Reuse
existing locations and decisions; no new specification hierarchy is required.

Read each relevant assertion. Explain its observable property and a case that
distinguishes correct from plausible incorrect behavior. Caller-order preservation
needs input whose order differs from sorting; a maximum-count test needs enough
eligible items to expose an extra result. Names, coverage percentages and green
suites alone do not establish this relationship. Check affected documentation too.

Review representative happy, boundary and failure cases at the affected consumer
interface; do not demand every test layer. Inspect snapshots and runner selection
when they affect the claim. Preserve or improve actual guarantees: stronger
assertions may legitimately change an old test body. A stale binding needs fresh
review, not reverting the stronger test or classifying every edit as weakening.
Do not claim semantic equivalence from syntax alone. Quarantines retain the lost
assurance as well as owner, issue and expiry; known required failures stay visible.

Reuse adequate existing tests. For important changed behavior or escaped defects,
seek a meaningful red/green transition, regression against the known defect, or
bounded isolated mutation using project tooling. Failure must be for the intended
assertion, not compilation or setup. Record observations and limits at the existing
work/testing authority. Never fabricate runs or mutate live systems. No mutation
framework or universal score is required; the CLI does not authenticate this evidence.

For a promised failure outcome, trace the consumer path through called helpers,
not only explicit rejection branches. An invalid value with a familiar representation
may reach the intended guard while another representation fails inside a dependency
first. When that risk applies, use a small discriminating example and verify the
promised error type/status and relevant side effects, not merely that something
failed. Do not assume a helper's input domain or exception matches the public
contract. Reuse adequate tests; this does not require an exhaustive input matrix,
a new test tool or another review round.

When an independent requirement-derived check or review contradicts green tests,
keep that finding open: reproduce the consumer-visible difference, add a regression,
repair within the authorized scope, and recheck the affected path before acceptance.
Preserve the failed attempt and correct earlier completion claims; a later repair
does not make the first result a pass. Review the finding against the original
contract rather than copying a proposed fix or changing expectations to fit code.
A clean comparison needs no invented defect or edit. Independence means evidence
not inferred from the implementation's own success claim; it does not require a
second agent, new tool, universal extra suite or routine approval round.

## Record and review

Use the [ready-for-feedback checkpoint](planning.md#ready-for-feedback) without
finalized evidence bindings when the next decision is about direction, not merge
readiness. This is not an exception or a passed acceptance gate.
While retained source and assertions change, keep expected conditions and focused
checks current without repeatedly finalizing staged hashes/review digests. After
feedback settles the intended increment, stage and review exact source, then run
required checks. Further material edits invalidate bindings as before. Isolated
disposable exploration uses the planning boundary; its results are not production
qualification.

Capability-check `opdev evidence acceptance-digest --help`. Older CLIs can support
semantic review but cannot enforce this contract. Report that gap and offer an
appropriate runtime upgrade; do not call policy-only passes acceptance qualification.

For selected MR/PR storage version 2, use the current record described in
[evidence.md](evidence.md#mrpr-native-review-selected-storage-version-2); do not
recreate a source ledger. Conditions can reference provider-observed work excerpts
without committed copies. The mappings/review duties and fresh execution below
remain unchanged. For legacy ledger projects, use schema 2 of `.opdev/evidence.yaml`, under the exact current change's
`acceptance`. New bootstrap includes an unresolved template. Existing ledgers need
an explicit reviewed schema-2 edit preserving history and unrelated assertions,
not create-new bootstrap or automatic migration. On a CLI supporting
`evidence prepare --help`, supply the explicit acceptance inventory/mappings with
path/excerpt references using `--input FILE --work AUTHORITY`. Hashes and outcomes
may be omitted: it calculates bindings and resets all review decisions to
unverified. Keep the emitted draft outside the worktree or in existing ignored
scratch space. `--draft FILE` previews the candidate and subject digest; review
the actual mappings, record reviewer/reference/rationale/outcome and the exact
digest, then use `--draft FILE --write`. Changed source, mappings or ledger need
fresh preparation/review. It preserves semantic history and unrelated assertions,
not YAML comments/formatting; it runs no suites and approves nothing. Manual edits
remain supported. No extra project policy file is required.

- `scope`: `behavioral`, `non_behavioral`, or justified `no_material_conditions`;
  explain applicability/exclusions in `rationale`, not by filename heuristics.
- `conditions`: all material conditions/risk objectives with local ID, statement,
  original authority and exact tracked source reference.
- `verifications`: one applicable mapping per condition ID with actual assertion,
  `discriminating_case`, target source, method and reviewed outcome. The same
  assertion can serve multiple conditions with separate explanations. `automated`
  names a declared `suite`; `review` needs appropriate evidence and a specific
  `automation_limitation`. This cannot waive core rules or bypass an available test.
- References contain a staged regular file `path`, SHA-256 of its Git blob bytes
  and exact `excerpt`. For external authorities, use an explicitly attributed,
  versioned capture at an existing suitable authority and retain the original
  location. Unauthorized capture or unknown currency leaves evidence unverified;
  the CLI does not retrieve or authenticate external requirements.
- Mapping outcomes: `passed`, `failed`, `unverified`. A concrete contradiction
  is failed despite a green suite; uncertainty is not an invented exclusion.

On a CLI advertising `acceptance.stage-mappings.v1`, explicit engineering policy
allows optional nonempty `stages` on each mapping. For example, map the same
condition to a pre-merge unit assertion and a separate post-merge consumer test.
Use stage names from the project contract. The mappings must not overlap, and
every inventoried condition needs exactly one mapping at the requested stage.
Omission retains legacy all-stage behavior and its review digest; legacy project
policy rejects scoped mappings rather than weakening its existing checks. Review
what each boundary proves before an authorized policy change. All tracked targets
and reviewed outcomes remain checked: a known contradiction at another stage
cannot be hidden. Only the selected stage's automated mappings require current
execution here; other-stage results cannot substitute for it. No predictive test
selector, cross-revision cache or extra evidence file is introduced.

Stage material files and obtain `opdev evidence fingerprint`. Prepare mappings
with `review.outcome: unverified`. `opdev evidence acceptance-digest` computes the
subject binding fingerprint, work, scope, inventory and mappings; it approves
nothing. Review completeness, authority, applicability and assertion meaning before
recording actual reviewer identity, reference, rationale, outcome and
`subject_sha256`. Agents identify themselves as agents and cannot manufacture
developer responses. Material policy choices need a developer response or genuine
bounded delegation. Two agents are not mandatory; agreement is not proof.

Recheck changed sources, scope, mappings or work before renewing the binding.
The ledger is excluded from the staged fingerprint, so its separate payload digest
is essential. Never copy an old review into changed assertions.

With capability `state.local.v1`, `evidence prepare --input FILE --work REF
--retain-draft` can store the same new draft in CLI-owned private state and emit
its path. This avoids choosing repository scratch locations. Mapping and review
decisions still start unverified; apply only through the existing current-review
path. Do not repeat preparation just to move a valid draft, infer consent from
storage, or copy raw personal context into it.

For an explicitly requested evidence transfer, capability `evidence.bundle.v1`
supports `evidence bundle export`, offline `inspect` and explicit provider
`retrieve` with an independently selected immutable locator. Select exact source/stage,
review identity and any retained attempt; retain failed and unfinished attempts
separately. Keep the new file outside product/Git storage and review private content
before sharing. Inspection compares explicit expectations and stays unqualified;
it does not authenticate decisions, upload, supply current execution or authorize
deleting the ledger. Do not add export to every feedback loop or claim a durable
archive from a local copy. Provider-observed storage authenticates retrieval, not
the report's claims or a developer decision. Provider retention/access and a reviewed migration
remain necessary before changing the evidence owner.

## Verify and report

Use normal authorized canonical checks. Automated mappings need their suite in
the current `opdev check` stage, with unchanged staged source and ledger before
and after. Do not rerun suites merely for receipts. Saved reports, `test-execution`
and JUnit inspection remain diagnostics, not alternate qualification inputs.
For automated mappings, missing/skipped suites or ordinary `--no-exec` leave
execution unverified. The explicit same-run reuse path described in
[testing](testing.md) can supply validated current producer execution instead;
it does not reuse an old acceptance review or convert diagnostic receipts.
Review-only mappings do not need a suite. Stale sources or pending
review remain unverified; suite failure is failed; verifier/execution failure is error.

The gate checks completeness relative to the reviewed inventory, source references,
binding and current execution. It cannot discover omitted prose requirements,
prove assertion semantics, establish individual test selection from exit zero, or
authenticate review claims. Review these facts explicitly. Explain clean comparisons
as well as defects; never call a mechanically valid record proof of complete
correctness or hide unresolved conditions in a success summary.
