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

Reuse adequate existing tests. For important changed behavior or escaped defects,
seek a meaningful red/green transition, regression against the known defect, or
bounded isolated mutation using project tooling. Failure must be for the intended
assertion, not compilation or setup. Record observations and limits at the existing
work/testing authority. Never fabricate runs or mutate live systems. No mutation
framework or universal score is required; the CLI does not authenticate this evidence.

## Record and review

Capability-check `opdev evidence acceptance-digest --help`. Older CLIs can support
semantic review but cannot enforce this contract. Report that gap and offer an
appropriate runtime upgrade; do not call policy-only passes acceptance qualification.

Use schema 2 of `.opdev/evidence.yaml`, under the exact current change's
`acceptance`. New bootstrap includes an unresolved template. Existing ledgers need
an explicit reviewed schema-2 edit preserving history and unrelated assertions,
not create-new bootstrap or automatic migration. No extra policy file is required.

- `scope`: `behavioral`, `non_behavioral`, or justified `no_material_conditions`;
  explain applicability/exclusions in `rationale`, not by filename heuristics.
- `conditions`: all material conditions/risk objectives with local ID, statement,
  original authority and exact tracked source reference.
- `verifications`: one mapping per condition ID with actual assertion,
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

## Verify and report

Use normal authorized canonical checks. Automated mappings need their suite in
the current `opdev check` stage, with unchanged staged source and ledger before
and after. Do not rerun suites merely for receipts. Saved reports, `test-execution`
and JUnit inspection remain diagnostics, not alternate qualification inputs.
For automated mappings, missing/skipped suites or `--no-exec` leave execution
unverified. Review-only mappings do not need a suite. Stale sources or pending
review remain unverified; suite failure is failed; verifier/execution failure is error.

The gate checks completeness relative to the reviewed inventory, source references,
binding and current execution. It cannot discover omitted prose requirements,
prove assertion semantics, establish individual test selection from exit zero, or
authenticate review claims. Review these facts explicitly. Explain clean comparisons
as well as defects; never call a mechanically valid record proof of complete
correctness or hide unresolved conditions in a success summary.
