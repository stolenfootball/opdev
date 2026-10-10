# Requirements and verification catalog

Requirements policy 1 is an explicitly selected addition to schema-3 engineering
policy, MR/PR review storage 2 and strict layout 2. Legacy selections retain their
behavior; capability discovery or plugin installation MUST NOT migrate them.
The user guide is [requirements and verification](../docs/requirements-and-verification.md).

## Ownership and rationale

Canonical capability JSON files in `.opdev/requirements/` own the current supported
guarantees and verification design. Existing specifications MAY remain the sole
normative owner through an exact source fragment. Tests, commands, work and
execution reports retain their existing owners. A catalog MUST NOT become a task
backlog, conversation archive, per-invocation ledger or alternate approval store.

The design separates requirements, verification design, observations and
authorization. A review digest binds a judgment's inputs; it does not establish
competent review or consent. Current execution cannot be replaced by a stored
`passed` field. Mandatory engineering rules remain independently applicable.

Versioned text and in-memory reads were selected over a committed database or
service: ordinary diffs and merges, no synchronization authority, disposable
derived state. Local synthetic comparisons found workload-dependent SQLite
benefits but no consistent advantage warranting that lifecycle. They did not
measure Rust performance, language-model token usage or production productivity.
Reconsider indexing only after representative measured query cost warrants it;
an index would remain disposable and could not qualify evidence independently.

Precedents inform, but do not certify, the design:

- [Doorstop](https://doorstop.readthedocs.io/en/latest/reference/item.html) uses
  text requirements and review/change fingerprints. OpDev binds additional
  applicability, stage and policy inputs explicitly.
- [StrictDoc](https://strictdoc.readthedocs.io/en/stable/stable/docs/strictdoc_01_user_guide-TRACE.html)
  demonstrates text-owned requirements and trace relationships.
- [Sphinx-Needs schemas](https://sphinx-needs.readthedocs.io/en/stable/schema/index.html)
  provide precedent for structural and relationship validation.
- [The six-company requirements/V&V study](https://arxiv.org/pdf/2307.12489)
  reinforces that trace links alone do not replace shared understanding and review.
- [NASA requirements management](https://www.nasa.gov/reference/6-2-requirements-management/)
  supports baseline/change control and verification linkage; its full process is
  not imposed on every OpDev project.
- [SLSA verification](https://slsa.dev/spec/v1.2/verifying-artifacts) distinguishes
  matching subjects from trusted observations. No SLSA conformance is claimed.

## Format and identities

`schema/requirements.schema.json` defines document version 1. Parsing MUST reject
duplicate/unknown fields and unsupported versions before interpretation. Document
and aggregate byte/file limits MUST fail closed rather than return partial success.
Files are regular staged Git blobs, not symlinks, submodules or checkout-normalized
bytes. Capability filenames use lowercase portable ASCII letters/digits, hyphens
or underscores with `.json`; no nested arbitrary namespaces or reserved devices.

Referenced excerpts MUST identify one occurrence in their full source file.
Inspection shares only bounded in-memory reads within the current invocation;
no persisted source cache can qualify a later check. The catalog is bounded to
1024 documents, 8 MiB each and 32 MiB total; referenced source inspection is
bounded to 4096 files and 64 MiB total. Bounds do not permit partial success.

All requirement, criterion, verification and plan IDs MUST be unique across the
catalog. Stable identities are independent of file placement. A requirement has
exactly one normative inline statement or exact tracked source, explicit supported
configuration applicability and observable criteria. Empty criteria remain
incomplete. Definitions may be reused; every criterion/configuration/required-stage
combination needs exactly one all-members plan. Empty/ambiguous/missing plans,
dangling links and unknown configurations cannot qualify.

Each member explains its actual assertion and a discriminating example. Mapping
reviews carry current outcome, attribution, rationale, reference and subject.
The subject uses [RFC 8785](https://www.rfc-editor.org/info/rfc8785/) canonical JSON
through the maintained `serde_json_canonicalizer` implementation, not a custom
serializer. Object key order is insignificant; array order remains significant.
Catalog top-level record lists are sorted by stable ID for logical identity.
Moving a record alone does not invalidate it; edits within a referenced whole
source file conservatively do.

## Qualification and review lifetimes

Mapping subjects include requirement identity, normative owner, origin/rationale,
applicability, criterion, plan/stage/configuration, relationships, full assertion
and declared input references, suite/command and testing/quality/assurance policy.
They exclude the review itself and unrelated implementation bytes. An input list
does not prove dependency completeness; unknown/shared changes broaden semantic
review. Mechanical binding MUST NOT write a passed review.

Current change review selects candidate and baseline catalog identities in the
existing MR/PR. Provider-observed target snapshot metadata binds the baseline;
missing or changed metadata MUST NOT select an older/caller-supplied substitute.
Review removals and applicability reductions against that accepted baseline and
actual authorization. Enduring conditions need not be copied into each change;
one-off conditions and capability-impact reviews remain required when applicable.

Execution retains existing exact-source, stage, command, environment, provider
identity and same-run-reuse boundaries. All selected plan members require current
observations. Mapping freshness is not execution freshness. Findings accumulate:
stale or missing review MUST NOT conceal a known execution failure. Catalog
checks add blocking policy results; they cannot override core results.

Suite assurance establishes canonical command execution, not named-case selection.
Unsupported selected case-level assurance remains unverified. There is no required
JUnit adapter. Manual observation is a separate attributed current-change record
with exact plan subject, method context, actual result, observer, reference and
bounded age. It cannot qualify another source/stage. Future, stale, absent and
ambiguous observations fail closed; known contradictions remain visible.

Inspection/query/diff never executes project commands, contacts providers, writes
approvals or qualifies gates. Queries disclose their completeness boundary. No
persistent cache/database is used, so modifying derived data supplies no authority.

## Migration and retention

Migration uses the existing explicit full-contract preview/application/recovery
workflow. Capability documents are exact reviewed inputs; imported mapping reviews
start unverified. A changed request, source or target invalidates the proposal;
later edits are preserved. No upgrade silently selects requirements policy.
Existing declarations of complete adoption do not demonstrate a new inventory.

Current catalog, ordinary Git history, concise work decisions/failures and bounded
CI reports have separate lifetimes. Storage 2 removes obsolete active ledgers
with temporary rollback protection only, without a mandatory archive. Retired
requirements need no infinite tombstone list. Releases retain their distinct
evidence owner and explicit authorization boundary.
