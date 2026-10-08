# Project evidence ledger

## Explicit offline evidence envelopes

`evidence bundle export --stage STAGE --output FILE [--attempt ID]` exports one
exact-current schema-2 change, its original project assertions and optional local
check attempt. It preserves the original attributed acceptance inventory, review
and rule assertions rather than manufacturing a successful review. All referenced
source/assertion bytes must still match the staged index. Export executes no
project commands, performs no upload, changes no ledger and never retires history.

The schema-1 envelope contains the existing workflow subject (source fingerprint,
effective configuration digest and stage), serialized validated configuration,
acceptance digest and a one-change projection of the existing ledger. An optional
attempt retains its local ID, original start/completion metadata and exact report
bytes as a JSON string. Exporter CLI bytes/version, local observation time and
OS/architecture are mechanical observations, not signed provenance or a complete
environment identity. The declared remote in configuration is not authenticated
origin. Existing local IDs remain local, not globally trusted producer identities.

The caller chooses a new file outside product source and Git metadata; its parent
must exist. Linked/reparse/traversal paths and existing destinations are rejected.
The writer syncs a temporary file and persists without replacing another attempt.
Input/output envelopes are bounded to 8 MiB; an oversized export fails rather than
dropping evidence. No new fingerprint exclusion is introduced: only the existing
legacy ledger exclusion applies, and configuration/guidance/docs remain material.
Retain every earlier attempt separately. Export does not copy historical changes
into each envelope or delete them from the original store.

`evidence bundle inspect --input FILE --sha256 EXPECTED --acceptance-sha256
EXPECTED --stage STAGE [--attempt-runtime-sha256 EXPECTED]` is offline inspection
against the selected project's current staged source/configuration. Obtain expected
identities from the authorized original work/evidence authority, not an untrusted
bundle's own summary. It verifies exact bytes, known nested schemas, internal
bindings and tracked references; a matching outer hash cannot hide inconsistent
configuration, acceptance or completed-report bytes. Unknown versions, missing
files and mismatches are errors, with no older-result or network fallback.
Imported parser and nested-validation errors name the failed schema/binding area
without echoing arbitrary field values or private command output. Inspect details
locally when needed; do not paste the original envelope into public diagnostics.

Inspection reports integrity separately from the attributed review outcome and
whether that review's own binding is current. It always reports origin and
qualification as unverified. A completed attempt remains a diagnostic observation;
missing completion remains `unfinished_or_interrupted`, even if partial report
bytes exist. A new review captured at export does not prove it existed when the
older command ran. Report gates are historical claims, never qualification for
this invocation. Neither this interface nor a bundle hash constructs authenticated
execution, proves semantic review, creates consent, authorizes release or proves
that the chosen expectations are still authoritative.

The envelope is a tool-neutral transport format, not a storage service or a
retention policy. Copying it to Git or a package store does not by itself establish
trustworthy durable retrieval. Provider-origin observation, reviewed retention and
access controls, immutable locations and migration have separate requirements.
Do not delete the legacy ledger or treat offline envelopes as its qualifying
replacement. Reports and declared configuration can contain private output and
references: inspect permitted content before explicitly sharing it; export does
not automatically redact or publish. No raw conversation capture is requested.

`schema/evidence-bundle.schema.json` references the bundled project, evidence and
local-state schemas by their IDs. Register those local resources when validating;
no network schema retrieval is necessary. Reusing these record types avoids a
parallel assertion/approval format. Revisit the bounded envelope or its transport
when demonstrated consumer evidence cannot fit without losing required history;
do not silently trim records or weaken qualification to make an export succeed.

## Exact archive retrieval

`evidence bundle retrieve --locator FILE --acceptance-sha256 EXPECTED --stage
STAGE --output FILE [--attempt-runtime-sha256 EXPECTED]` retrieves and inspects
one explicit Git-backed envelope. The locator is strict JSON described by
`schema/evidence-archive.schema.json`: schema 1, `provider` (`github` or `gitlab`),
positive numeric `repository_id`, full lowercase SHA-1 `commit`, exact `path`,
and independent raw-byte `sha256`. Obtain it and the acceptance expectation from
the existing authorized work/evidence authority. The CLI does not infer the latest
record, choose a retention policy or treat caller-selected identities as approval.

Supported archive paths are regular files with at most 16 ASCII portable-name
components and 1024 bytes. SHA-256 Git repositories, self-hosted provider origins,
Git LFS resolution, links and submodules are not silently adapted. The archive
may be a separately authorized evidence repository; export/retrieval does not
create one or add historical files to product `.opdev`. If an archive shares a
product repository, its retained branch/commit path and access need explicit
review; no product fingerprint exclusion follows from calling it an archive.

Retrieval requires existing provider credentials, uses fixed HTTPS origins and
GET only, disables redirects and never falls back to public/cache/older bytes.
Credential precedence remains the [remote adapter contract](remote-audits.md);
GitHub does not automatically read `gh` credentials. A caller may explicitly
inject an existing authorized credential into this one process, never into files.
It checks numeric repository identity and full commit, traverses Git trees to
bind the requested path to a regular blob, retrieves exact raw bytes and compares
their independent SHA-256. GitHub's slug-based object path is checked against the
numeric repository again after retrieval. GitLab uses numeric project paths.
GitHub tree truncation is rejected; GitLab enumeration is limited to ten pages of
100 per directory. Metadata is limited to 1 MiB/response, bytes to 8 MiB. Requests
share a one-minute network budget with at most 20 seconds per request. A missing,
denied, changed, oversized or unsupported result produces an error, not another
attempt's result. Diagnostics do not copy server bodies or credentials.

Only after the ordinary bundle inspection succeeds and the current source is
rechecked are bytes written to a new local destination outside source/Git metadata.
The JSON result says `origin: provider_observed` and records the exact locator and
blob; it still says `qualification: unverified`. Later offline inspection cannot
recreate provider observation by reading a saved result. Authenticated storage
does not authenticate a report's author, a developer's decision or a command's
execution. Existing authenticated same-run execution remains the only supported
execution-reuse boundary; an archived diagnostic report never enters it.

Before declaring a retained archive suitable, review at the existing authority its
owner, reader/writer access, required lifetime, protected reachability/reference,
deletion controls, capacity, backup/recovery and cleanup conditions. Preserve
failed, superseded and interrupted attempts individually. An immutable commit
identifies bytes, not continued availability: repository deletion, permission
revocation, unreachable-object cleanup and deliberate history removal can still
lose evidence. Retrieval observes current access, not future retention or policy
approval. Do not retire a legacy ledger until required history has a reviewed
destination and verified independent retrieval/recovery; a sole local download
or an expiring job artifact is insufficient.

The Git-backed option reuses provider access and object identities without a new
hosted service or per-milestone release. Large binary artifacts belong in their
reviewed immutable artifact store, not automatically in Git envelopes. Revisit this
transport when demonstrated evidence volume makes it impractical, preserving
identity/trust boundaries rather than weakening them. Provider package/job stores
can be future adapters but their expiration/overwrite behavior needs its own policy.

Sources: [GitHub tree modes](https://docs.github.com/en/rest/git/trees),
[GitHub blob reads](https://docs.github.com/en/rest/git/blobs), and
[GitLab repository trees and blobs](https://docs.gitlab.com/api/repositories/).
Unlike tree/blob retrieval, the
[GitHub Contents API](https://docs.github.com/en/rest/repos/contents) can dereference
symlinks and therefore is not by itself proof of a regular archived file.

## Schema-2 acceptance evidence

### Separate authenticated semantic review

Development capability `evidence.authenticated-review.v1` supports schema-3
`assurance.review_storage`: version 1, hosted provider, numeric archive repository
ID, actual policy decision reference and an existing retention authority. This is
explicit migration, not a new default for legacy projects. The authority owns
reviewed retention/access/recovery requirements; the declaration is not proof
those controls operate. Existing historical ledgers must remain until the storage
migration has independently verified their retained recovery path.

`evidence bundle export-review --stage STAGE --output NEW_FILE [--ledger FILE]`
mechanically generates a schema-1 `semantic_review` record. It carries one exact
current schema-2 acceptance record and project assertions, bound to the full
material source fingerprint, effective configuration, stage and acceptance digest.
It carries no execution report, producer success or consent assertion. Failed and
unverified review judgments stay unchanged. Export is local and create-new outside
product/Git storage; it does not upload, select retention or delete history.

With that policy selected, `check --review-locator FILE
--review-acceptance-sha256 EXPECTED` retrieves the independently selected exact
record through the authenticated Git archive reader. A locator for a different
provider/repository is rejected before network access. The engine consumes a sealed
provider observation, not a deserialized diagnostic report or an arbitrary local
ledger path. Exact source/configuration/stage, original inventory/digest and staged
requirement/assertion references must match. Unresolved or failed judgments remain
unsatisfied. It still runs the canonical stage checks or separately validates the
existing same-run execution boundary; review storage grants neither execution nor
release authority. No local cache or older green record is a fallback.

Selecting external policy while retaining an active legacy ledger is an unresolved
migration, not permission to choose whichever evidence passes. Missing authenticated
review fails before commands run. A selected immutable archive observation proves
where bytes were retrieved, not author identity, human consent or semantic truth.
Existing semantic review duties remain. The new record creates no source exclusion:
project configuration, guide and relevant documentation still affect qualification.
Expected archive digest and acceptance identity come from the authorized work/review
authority, never solely from the file being evaluated.

### Mutable work content as a minimal observation

`evidence bundle observe-work --selector FILE --excerpt FILE --output NEW_FILE`
captures one selected issue/merge-request description or note through authenticated
provider GETs. The strict selector specifies provider, numeric repository ID,
`issue` or `merge_request` (also GitHub PRs), item number and optional exact note ID.
The UTF-8 excerpt file contains only the exact text needed, at most 16 KiB. The
CLI checks item kind/identity, parent membership, author ID and original provider
timestamps; it rechecks the repository identity and rejects system notes. It emits
only output/hash metadata to stdout. It does not dump a full body or conversation.

The schema-1 record retains the excerpt, SHA-256 of the complete original body,
selector, numeric author ID, creation/update timestamps and local observation time.
The body hash identifies the observed version but cannot reconstruct omitted
context. Review whether the selected excerpt retains enough context; authorship
alone proves neither permission nor approval. Saved timestamps are not signatures.
`--expected-body-sha256` explicitly rechecks a previously selected body identity:
changed/missing/denied content fails instead of substituting old text. This is an
observation at one time, not an atomic snapshot or promise of future authority.
Recheck decisions when their scope, expiry, revocation or supersession matters.

`export-review --work-observation FILE` can bind selected observations into the
semantic record (at most 64). Its independent review identity then includes the
complete retained observations as well as the acceptance inventory; editing an
excerpt or provenance without rebinding is rejected. With no observations the
identity remains the original acceptance digest. Existing per-assertion review
semantics remain separate from observation transport. A deserialized observation
does not recreate an authenticated provider read, and the archive authenticates
storage origin rather than its historical claims. No author label becomes new
developer consent.
At qualification, bound work observations are re-observed before execution and
afterward: changed body/author/provider timestamps or unavailable authorities fail
without an old-text fallback. Each provider group has a one-minute read budget.
This guards the selected content, not unselected decisions elsewhere; review must
identify the applicable authority and scope. Ordinary reviews with no mutable work
observations add no such requests. Sources: [GitHub issue comments](https://docs.github.com/en/rest/issues/comments)
and [GitLab notes](https://docs.gitlab.com/api/notes/).

For durable recovery, Git mirror backups retain repository history and can be
restored to a Git remote; provider migration archives are not interchangeable with
restorable backups. GitHub explicitly says its migration archives have no supported
restore path. Protected Git references reduce accidental history loss but do not
prevent administrative deletion or replace independent recovery. Record observed
protection entitlement limits instead of treating unsupported private-repository
controls as passed. Sources: [GitHub repository backup](https://docs.github.com/en/repositories/archiving-a-github-repository/backing-up-a-repository)
and [GitLab protected branches](https://docs.gitlab.com/user/project/repository/branches/protected/).

Development CLIs read ledger schemas 1 and 2 and emit bootstrap schema 2. Version
1 remains readable; it is not silently rewritten. TEST-002/003 now require typed
current-change evidence rather than generic rule assertions or a declared testing
policy. Old ledgers therefore leave those checks unverified until explicitly
reviewed migration. Other rules retain their existing interpretation. Bootstrap
does not offer boolean decisions for these two rules; its separate acceptance
template begins unresolved and cannot qualify a gate.

Each schema-2 change may contain `acceptance`. It inventories material conditions
and selected risk objectives and maps each exactly once to verification evidence
for the selected check stage. Legacy mappings apply to every stage; the explicit
engineering policy can use the disjoint stage mappings below.
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
| optional verification stages | Nonempty unique `stages` from `local`, `pre_merge`, `post_merge`, `package`, `delivery`, `recovery`, `scheduled`, `evaluation`; omission means all stages |
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

### Distinct stage guarantees

A CLI advertising `acceptance.stage-mappings.v1` supports optional verification
`stages` only under the explicitly selected engineering policy (catalog 3).
Legacy policy rejects stage-scoped mappings before command execution; omission
preserves both the original all-stage behavior and serialized review digest.
Changing project policy requires its reviewed migration, not an automatic rewrite.

For each inventoried condition, exactly one mapping MUST apply to the requested
stage. Disjoint mappings can name different tests and suites: for example,
pre-merge unit assertions and a post-merge consumer test of the integrated source.
Overlap, duplicate/empty/unknown stages and unknown condition references are
invalid. Missing mappings, undeclared suites, skipped execution or stale review
leave the applicable condition unverified. A mapped suite must actually pass at
that stage; a review claim is not a saved execution result. All recorded source
references and reviewed outcomes remain checked, including other stages: a known
contradiction cannot be hidden by selecting a different stage.

Stage selection changes neither the accepted guarantee nor the tracked-input
fingerprint. Review must establish why the chosen assertions adequately verify
the condition at that boundary, including supported environments. A generic smoke
test is not an automatic substitute for integrated behavior. Unknown impact,
shared dependencies, scripts, lockfiles and executable documentation or agent
guidance require broader investigation/checks. There is no predictive selector,
filename-based exemption or new fingerprint exclusion.

Only declared checks for the requested stage execute. This is not cross-stage or
cross-revision reuse: pre-merge success cannot supply missing post-merge execution.
Use one execution owner for each equivalent check and the separately reviewed
[same-run reuse protocol](execution-reuse.md) where necessary. Do not first run
the full suite solely to rerun it through the evaluator. Distinct intended checks
remain distinct even when their command strings match. Check plans preview
selection without qualifying it; feedback and release boundaries are unchanged.

This design follows [DORA's CI guidance](https://dora.dev/capabilities/continuous-integration/)
on rapid feedback and testing before and after integration, and the distinct
confidence-building stages of a [deployment pipeline](https://martinfowler.com/bliki/DeploymentPipeline.html).
[Industrial test-selection research](https://arxiv.org/abs/2501.11550) describes
differing pre/post-submit objectives; it does not establish that a generic OpDev
predictor would safely skip tests. Prefer explicit reviewed stage contracts over
a universal identical full suite or a new language-specific dependency engine.
Revisit if whole-session trials show missed guarantees or more policy work than
saved execution. Observe first feedback, qualification latency, queue/setup and
summed compute; a roughly ten-minute fast-cycle target is guidance, not a gate.

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
