# Project evidence ledger

## Schema-2 acceptance evidence

Development CLIs read ledger schemas 1 and 2 and emit bootstrap schema 2. Version
1 remains readable; it is not silently rewritten. TEST-002/003 now require typed
current-change evidence rather than generic rule assertions or a declared testing
policy. Old ledgers therefore leave those checks unverified until explicitly
reviewed migration. Other rules retain their existing interpretation. Bootstrap
does not offer boolean decisions for these two rules; its separate acceptance
template begins unresolved and cannot qualify a gate.

Each schema-2 change may contain `acceptance`. It inventories material conditions
and selected risk objectives and maps each exactly once to verification evidence.
Multiple conditions may reference the same test, but each needs its own explanation.
The shape is defined by `$defs/acceptance` in the ledger and bootstrap schemas;
unknown fields, duplicate IDs/mappings, unknown condition references, invalid
outcomes and incompatible methods are rejected. Partial inventories/mappings may
be retained with unverified results; they are not silently considered complete.

Required fields:

| Object | Fields and meaning |
| --- | --- |
| `acceptance` | `scope`, `rationale`, `conditions`, `verifications`, `review` |
| condition | `id`, accepted `statement`, original `authority`, tracked `source` |
| verification | `condition` ID, `method`, tracked `target`, actual `assertion`, `discriminating_case`, `outcome` |
| automated method | A declared `suite`; no automation-limitation field |
| review method | Specific `automation_limitation`; no suite; appropriate reviewed evidence |
| tracked reference | Repository-relative `path`, SHA-256 of staged Git blob bytes in `sha256`, exact nonempty `excerpt` |
| review | `outcome`, actual `reviewer`, `reference`, `rationale`, `subject_sha256` |

Review/mapping outcomes are `passed`, `failed`, `unverified`. These are reviewed
claims, not saved execution results. `behavioral` and `non_behavioral` scopes need
a nonempty condition inventory. `no_material_conditions` requires an empty
inventory/mapping set plus an exact-change reviewed justification, not an inferred
extension-based exclusion. Nonbehavioral evidence may satisfy TEST-002 while
TEST-003 is not applicable. A reviewed automation limitation can use appropriate
non-executable evidence; it never waives another core requirement.

The CLI verifies sources from Git's index, accepting only regular stage-zero
blobs, each at most 8 MiB. It does not follow symlinks, retrieve URLs, accept path
traversal or permit references to the ledger itself. Excerpts must match UTF-8
blob content exactly and the whole blob digest must match. Hash Git blob bytes,
not a Windows checkout with transformed line endings. External requirements need
an explicitly attributed versioned capture at an appropriate existing authority;
the original location remains recorded. Authority authenticity/currency remains
a review obligation, not a network verifier.

### Review binding and execution

1. Identify accepted conditions and expected results before implementation.
2. Prepare/reuse actual assertions, meaningful boundary/counterexamples and their
   source references. Record unknowns or contradictions, not optimistic passes.
3. Stage material files and obtain `evidence fingerprint`. Prepare the schema-2
   ledger with the current work reference and `review.outcome: unverified`.
4. `evidence acceptance-digest` prints the review subject without executing tests,
   changing files or granting approval. After review, record the actual reviewer,
   reference, rationale, outcome and that digest. Do not infer developer consent
   from an agent reviewer or hash-generation command.
5. Run the applicable normal check. Automated methods require the named canonical
   suite's actual outcome in this check, not a saved receipt, extension result
   with the same ID or JUnit file. Missing/skipped-stage execution is unverified;
   a suite failure is failed and an execution failure is error.

The review digest is SHA-256 of compact `serde_json::Value` serialization of
`{protocol: 1, fingerprint, work, scope, rationale, conditions, verifications}`.
Object keys use serde_json's sorted map representation; array order is retained.
It excludes `review` to avoid circularity. Changing any subject field invalidates
the review. Identity does not authenticate the person or establish that their
review was competent. The CLI rechecks staged source and ledger before/after
executing commands. Edits made and reverted during execution and ignored inputs
are outside that observation, as with existing execution receipts.

### Meaningful and proportionate assertions

The reviewed inventory MUST cover representative happy, boundary and failure
behavior and the meaningful consumer interface affected by the change. Choose
the appropriate API, library, CLI, UI, data or device boundary; this is not a
requirement to create every test layer. Explain genuinely irrelevant cases in
the existing review instead of adding placeholder tests. Expected results come
from accepted behavior, not a snapshot of whatever the implementation returns.

Review MUST consider whether assertions discriminate plausible mistakes, whether
snapshots were substantively reviewed, and whether filters, disabled cases,
failure-propagation settings or retries undermine the claimed observation.
Command success does not prove every named test ran. Quarantine records MUST
identify the lost assurance as well as owner, work reference and expiry; neither
quarantine nor a hidden retry can turn a known required failure into a pass.

An escaped-defect fix SHOULD demonstrate the meaningful regression failing on
the original defect and passing after repair. Compilation/setup failure is not
that red observation. If old behavior cannot safely or practically be run, retain
the specific limitation and a discriminating alternative; never invent a red run.
There is no required test-writing order, reporter format or universal coverage
or mutation target. Stronger property, mutation or fault-injection work follows
an actual risk or weakness, bounded scope/budget and reviewed blocking policy.

Legitimate monotonic strengthening of an assertion is not test weakening merely
because the original test body changes. Review retained guarantees, changed
expectations and independent counterexamples; refresh source-bound mappings
without requiring byte-identical tests. Removing a requirement or disabling a
case to obtain green results is different. Frozen historical experiment graders
retain their original limits/results; new evaluations must state their own
preservation oracle and inspect semantic changes rather than silently relabeling
old failures. No general-purpose assertion-equivalence parser is claimed.

Prefer simple adequate design, clear boundaries and reusable existing contracts
when reviewing maintainability. File counts and function size are not general
proofs of quality; honor project-specific checks without imposing universal
quotas or speculative abstractions on consumers.

The executable acceptance fixtures demonstrate wrong-but-green expectations,
meaningful red/green repair and stronger assertions accepted only after current
review. These establish the verifier boundary, not autonomous semantic discovery.
Revisit this review-based design when independent agent trials expose missed
contradictions; do not install a new tool solely to create more evidence.

### Evidence boundary and rationale

Blocked checks retain the underlying staged-source error (including unindexed
paths) and identify a mapped suite missing from the selected stage. These facts
appear in both human diagnostics and the existing structured rule diagnostics;
they do not authorize auto-ignoring files or changing suite assignments.

The gate establishes coverage **relative to a reviewed inventory**, exact-source
references, review binding and current canonical execution. It cannot discover
omitted requirements, understand arbitrary assertion semantics, prove individual
test selection from a suite exit code, or authenticate human/agent claims. These
limits are explicit in the passing diagnostic. A deliberately false semantic
claim with mechanically valid evidence remains a regression test of this limit.
Wrong assertions must be identified by actual review, not by matching keywords.

Compared with a mandatory second model call, this provides deterministic rejection
of missing/stale bindings and missing execution while keeping tooling generic.
Stronger projects may require targeted red/green or mutation evidence and review;
no language adapter, reporter format, mutation score or universal second agent is
mandated. Record demonstrated errors and the limits of this evidence. Revisit the
design if independent consumer trials show unacceptable setup, missed semantic
contradictions or misleading qualification. Normal reviewed CI and roll-forward
apply; neither schema migration nor release is automatic.

## Generic rule assertions (schemas 1 and 2)

`.opdev/evidence.yaml` is an optional, schema-validated ingress for facts the
CLI cannot infer safely. It is not a waiver file. It can assert only `passed` or
`not_applicable`, must include concrete evidence, and can address only rules
whose catalog verification methods permit `evidence` or reviewed `agent` facts.

Project assertions describe durable policy or capability and should be used
sparingly. Change assertions are bound to an exact SHA-256 fingerprint of the
staged Git index. The fingerprint includes each tracked path, mode, stage, and
Git blob identity while excluding only `.opdev/evidence.yaml`, allowing the
ledger to be written after the fingerprint is calculated.

For a new ledger, the preferred safe sequence is:

1. stage every material file for the change;
2. run `opdev evidence bootstrap` and save its standard output outside the Git
   working tree;
3. review each generated rule, replacing `review_required` only with a justified
   `passed` or `not_applicable`, and add concrete shared evidence to the relevant
   scope;
4. preview the expanded ledger with `opdev evidence bootstrap --answers PATH`;
5. create it explicitly with
   `opdev evidence bootstrap --answers PATH --write`; and
6. stage the ledger, run `opdev check` or `opdev check --ci`, then review and
   commit the evidence with the files it describes.

The bootstrap document is a versioned, schema-validated questionnaire, not an
attestation. It is generated from rules that the current pre-merge evaluator
still reports as `unverified` and that permit reviewed evidence. Every decision
starts as `review_required`, which never enters the ledger and never satisfies a
gate. The completed questionnaire must retain exactly the generated candidate
set and staged fingerprint; added, removed, re-scoped, or stale answers are
rejected. `--write` uses create-new semantics and refuses to alter an existing
ledger. Existing ledgers continue to be reviewed and maintained directly.

The questionnaire intentionally separates `project` from `change`. Project
evidence supports durable policy or capability across changes. Change evidence
requires a work authority and is usable only with its exact fingerprint. Each
scope may cite shared evidence once; the CLI expands accepted decisions into the
ordinary per-rule assertions shown below so the committed ledger remains fully
reviewable. A reviewer must confirm that every shared fact actually supports
every accepted decision in that scope.

`opdev evidence fingerprint` remains available for direct ledger maintenance.

Fingerprinting and bootstrap generation fail when tracked changes are unstaged
or material untracked files exist. Keep the questionnaire outside the working
tree so it does not become unindexed input. CI checks out the committed index
and recomputes the same value.
Any future path, content, or executable-bit change produces a different
fingerprint, so stale change assertions are ignored and required rules return
to `unverified`.

Evidence is applied only when the normal evaluator returned `unverified`.
It cannot override an explicit failure, error, migration requirement, manifest
contradiction, CI result, or remote-provider result. Project extensions remain
structurally separate and cannot write core rule results.

A completed compact review can look like this:

```yaml
schema: 1
project:
  evidence:
    - kind: policy
      summary: The security boundary and reporting process were reviewed.
      location: SECURITY.md
  decisions:
    OPDEV-SEC-001: passed
change:
  fingerprint: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
  work: https://example.test/project/issues/42
  evidence:
    - kind: work
      summary: The issue records scope, exclusions, acceptance evidence, and risks.
      location: https://example.test/project/issues/42
  decisions:
    OPDEV-WORK-001: passed
    OPDEV-DESIGN-001: not_applicable
```

The CLI expands that reviewed input into the committed ledger contract:

```yaml
schema: 1
project:
  - rule_id: OPDEV-SEC-001
    outcome: passed
    summary: The reviewed security policy defines the project trust boundaries.
    evidence:
      - kind: policy
        summary: Reporting, command execution, and remote-audit boundaries are documented.
        location: SECURITY.md
changes:
  - fingerprint: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
    work: https://example.test/project/issues/42
    assertions:
      - rule_id: OPDEV-WORK-001
        outcome: passed
        summary: The work item defines scope, acceptance conditions, evidence, and risks.
        evidence:
          - kind: work
            summary: Issue 42 is the accepted executable work authority.
            location: https://example.test/project/issues/42
```

An assertion is still a reviewed project claim. Fingerprinting prevents reuse
against different bytes; it does not prove that a cited review was competent or
that an external URL is authentic. Higher-assurance projects can add stricter
extension checks or signed attestations without weakening this core behavior.

## Design decision

The generalized problem is that strict gates need human or project evidence,
but a permanent unbound assertion can silently qualify later changes. Weakening
gate aggregation would violate OpDev semantics, while requiring an external
attestation service would make initialization provider-specific and burdensome.
Binding assertions directly to the final Git commit is circular because the
ledger itself changes that commit.

Version 1 therefore fingerprints the staged Git index while excluding only the
ledger. This preserves strict aggregation, works in every Git-hosted software
project, is reviewable in the same change, and invalidates itself when material
bytes or modes change. The tradeoff is that assertions remain project claims
rather than cryptographically authenticated third-party attestations.

Revisit this decision when common CI providers can supply portable, signed,
change-level review attestations with equivalent local developer ergonomics. A
future protocol may accept those attestations alongside the ledger, but it must
not reinterpret existing version 1 fingerprints or silently trust unbound data.
