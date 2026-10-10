# Documentation locations and ownership

The project contract, rather than a folder name, selects authoritative material.
The following defaults describe legacy layouts; the explicit strict target and
its read-only inspection are defined below. Inspection alone changes no policy.
OpDev configuration and evidence live in `.opdev/`. For projects without an
established structure, internal working documentation SHOULD also live there:

| Optional location | Purpose |
| --- | --- |
| `.opdev/design.md` | Current design and architecture |
| `.opdev/development.md` | Human development guidance |
| `.opdev/delivery.md` | Release and recovery guidance |
| `.opdev/specs/` | Individual capability specifications |
| `.opdev/decisions/` | Significant durable decision records |

These are placement defaults, not an initialization scaffold. Agents MUST NOT
create empty directories, placeholder documents, or a document per practice just
to fill out this layout. Create a document only when its purpose and durability
justify it; a small change can live entirely in its work item. Start with one
design document and split it when navigation warrants it.

Product documentation, contribution and security guidance, changelogs, and
packaging inputs retain appropriate project/ecosystem locations outside this
internal default. Keep the product README at the root and link to internal
development guidance when it exists. Do not move public material into `.opdev/`
merely because an agent authored it. No additional Markdown index is required.

Commands remain authoritative in `project.yaml`; current status and sequencing
remain in the declared work authority. Agents MUST NOT create competing PLAN,
TODO, or STATUS documents or duplicate command definitions for convenience.
If work is tracked in repository files, explicitly route that existing authority
rather than imposing a second tracker. Durable specifications describe behavior
and rationale, not a second copy of the live backlog.

## Strict namespace contract (layout 1)

Explicit layout 2 retains layout 1 and adds only
`.opdev/requirements/<capability>.json`, validated against the
[requirements catalog](requirements-catalog.md) format. Lowercase ASCII
letters/digits, hyphens and underscores name capability files; no nested scratch
namespace or database files are allowed. Layout selection alone does not enable
catalog qualification: select requirements policy explicitly too.

The strict target replaces the internal placement defaults above only through an
explicit version-aware project migration. Existing schema-1/2/3 contracts and
their authorities are not automatically rewritten or opted into enforcement.
The read-only inspection below checks a proposed destination structure; it is
not evidence that migration, shared entry guidance or external evidence storage
is implemented. A reviewed storage migration owns ledger removal: policy 2 removes
it without an archive; legacy archive policy 1 retains its retrieval requirements.

| Required location | Owner and meaning |
| --- | --- |
| `.opdev/project.yaml` | Effective versioned policy, commands and authority routing |
| `.opdev/adoption.yaml` | Reviewed decisions and their provenance, not current execution proof |
| `.opdev/guidance.md` | One managed versioned entry guide shared by both agent hosts |

Optional durable Markdown has fixed roles at `.opdev/docs/design.md`,
`development.md`, `testing.md` and `delivery.md`; focused specifications go in
`.opdev/docs/specs/<capability>.md`, decisions in
`.opdev/docs/decisions/<id>-<decision>.md`, and referenced supporting assets in
`.opdev/docs/assets/`. Use portable ASCII letter/digit/hyphen/underscore/dot names
without traversal or ambiguous trailing characters. Paths are also constrained
by cross-platform names: Windows reserved device names (including
names with extensions) and case-folding path collisions are findings on every
platform. This follows [Windows naming rules](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file);
inspection does not rename either conflicting path. Supported asset formats are
PNG, JPEG (`jpg`/`jpeg`), GIF, WebP, SVG and PDF. A permitted asset path is not proof
its bytes are safe or its reference meaningful. No scripts, archives, package
caches, runtime installs, extra config formats or miscellaneous escape folders
belong in this namespace. Adequate custom authorities outside `.opdev` remain
project-owned and do not need a duplicate internal document.

Use only justified documents, not empty scaffolds. A specification describes
observable behavior, boundaries, errors and verification; a decision describes
context, alternatives, decision and consequences; operational guidance describes
enduring prerequisites, actions, checks and recovery. Adapt headings to the
actual content. These concise roles are templates, not a semantic heading gate.
Work status, milestone order, task-specific acceptance, investigation dumps,
transcripts and growing execution histories belong to their tracker or retained
local/evidence owner, not another document in `.opdev`. Important research
conclusions and recovery procedures remain durable knowledge.

The shared guide is the authority for OpDev instructions; short marked root
AGENTS/CLAUDE sections route to it without duplicating its full text or containing
machine-specific cache paths. Preserve unrelated instructions and imports.
Readers must reload the guide and relevant contract authorities after context
reset, retain routine/external-operation exclusions, and report missing guidance
or runtime with a repair/install offer rather than silently installing or
improvising a replacement. Moving text alone establishes no token-saving claim.
These are migration requirements, not permission for an inspector to edit files.

### Read-only proposed-layout inspection

`opdev layout inspect --layout-version 1 --scope working-tree|index` is a bounded
local inventory, usable from subdirectories and linked worktrees. Working-tree
scope includes ignored/untracked files; index scope reads the inspected worktree's
staged regular Git blobs and ignores unstaged substitutes. Both reject index links
(including Windows link placeholders), unresolved index entries and unsupported
paths. Filesystem links/reparse points are not traversed. Unknown directories
are reported at their boundary without reading their contents; configuration
content is parsed only at the known contract/adoption paths. Unknown fields,
duplicate keys and unsupported config versions are findings, not repairs.

Exit 0 means no structural finding in that scope, not valid policy, completed
adoption or useful documentation. Exit 1 means findings; exit 2 means malformed
arguments or inspection failure. JSON follows `layout-inspection.schema.json`
and always retains `qualification: unverified`. Human output escapes path control
characters. The inspector runs read-only Git queries, never project commands,
network requests, installs, cleanup or a full behavior suite. It creates no files.
Its bounded walk/config reads are safety limits, not universal project file quotas;
exceeding them is an inspection error rather than a partial success. Observations
are not an atomic snapshot and cannot establish source freshness for a gate.

An old `.opdev/evidence.yaml` produces a storage-migration finding. Follow the
selected policy: MR/PR storage 2 removes it without an archive, with temporary
rollback protection until verification; legacy storage 1 retains its verified
archive requirements. The general inspection notice must make the same distinction. There is no
`.opdev/archive` exception. Content placement, meaningful asset references,
managed-guide currency and actual fresh-agent behavior require their own review;
an empty or misleading Markdown file is not certified by this structural check.
Existing check/init/upgrade behavior is unchanged by invoking inspection.

### Explicit enforcement and shared entry guidance

Schema-3 projects can select `layout: {version: 1, review_reference: ...}` with an
actual reviewed decision reference. Legacy contracts without this selection keep
their existing behavior. A new project can request it with `init --layout-version
1` alongside explicit engineering policy, assessment choice and decision reference.
Initialization creates unresolved adoption, not a claim of completed adoption.
It does not create optional documentation, archive evidence or migrate an existing
layout. Existing projects need a coordinated reviewed migration.

Selected projects use one managed `.opdev/guidance.md` and short marked sections in
both root instruction files. Unrelated text and imports are preserved. Ordinary
guidance upgrade previews include all three files, bind their exact input bytes,
and write the guide before its pointers. Each file replacement is atomic, not the
whole group; interrupted writes remain visible and can be previewed again. Linked
files, Git link placeholders and linked `.opdev` parents are refused. A guidance
update never selects layout policy or removes legacy evidence.

`check` adds a blocking `policy` check named `opdev-layout` for integration,
delivery and compliance. This is a built-in structural observation, not a project
command or reusable CI execution receipt. It reads staged regular files, checks
the selected namespace and exact managed guide/pointers, and does not trust
unstaged repairs. The standalone proposed-layout inspector remains read-only and
unqualified. The check report adds the `policy` kind; consumers must support it
and capability `layout.enforcement.v1`, not reinterpret it as an executed suite.
Other required checks, source-bound acceptance and adoption remain separate.

A structurally valid layout does not prove document purpose, asset references,
developer consent or fresh-agent behavior. Source review must still examine those
claims. A legacy ledger remains a blocking storage-migration finding: no command
silently deletes it or treats export alone as a completed migration. Complete strict
layout adoption requires the coordinated evidence-storage migration, not a manual
ledger deletion to make this check pass.

`layout format` previews deterministic YAML for the two existing maintained
configuration files. Its schema-1 JSON shows exact before/after text and a plan
digest; `--apply PLAN_ID` writes only that still-current proposal. It validates
both records before writing either and compares parsed meanings. It preserves
policy, dispositions and approval provenance; comments, quoting and field order
may change and are visible in the preview. Unknown/future/malformed records are
not repaired. It does not format Markdown, legacy evidence, arbitrary paths or
outside authorities. Formatting alone grants no approval or qualification.
Writes are atomic per file, not a transaction; re-preview after interruption.
Stable canonical inputs produce no changes. This is an explicit formatting tool,
not a requirement to reformat files on every feedback iteration.

The design favors a strict small namespace plus explicit migration over deleting
unknown files or a keyword classifier for arbitrary prose. Revisit its supported
roles/formats when a concrete durable use cannot fit without duplication; changes
must be versioned, not a silent catch-all. Runtime and evidence retention remain
separate from durable documentation ownership.

## Prospective placement check

Before creating or substantially expanding a document, agents MUST classify its
intended purpose: durable knowledge, work tracking, temporary investigation, or
mixed. Resolve the owner from the contract, inspect adequate existing material
and apply existing task write authorization. Durable behavior/rationale and
reusable operational instructions go to their documentation/design authority;
milestones, task-specific acceptance, sequencing, progress and open implementation
questions go to `authorities.work`. Split mixed content and cross-link without
duplicating live tracking. Temporary notes need no mandatory project document.
Future implementation order remains work tracking when summarized as a product
rollout or architectural phases; link to it without repeating the sequence.
Permanent operator steps and enduring behavior/interface constraints, including
proposed capabilities, are durable knowledge.

Normal review MUST compare actual changed document claims with their source
requirements and related work record or unsaved proposal. If only implementation
priority/status changed, a passage that would need updating belongs in work
tracking, not durable design. Supported interfaces, explicit design exclusions
and reusable operator sequences remain durable when justified by the source.
Review MUST NOT invent exclusions, promote provisional work into commitments or
discard behavior merely to remove sequencing. For mixed content, retain the
behavior, route the work claims to their owner and check the resulting links.
Record concrete corrections or source-backed distinctions in existing review
context, not a new ledger or document. Placement success and author assurances
do not establish this semantic judgment. No second agent or routine additional
developer approval is required.

If work access or permission is unavailable, keep the proposal in conversation
and report that it was not saved. Never invent issue references or quietly make
a repository backlog. Explicit repository-file trackers remain valid. This is
prospective: it does not authorize migration or cleanup of existing documents.
Do not require another approval round for already-authorized writes; ask only
about a material unresolved choice. Ordered operator phases and enduring
requirements remain legitimate documentation, regardless of their vocabulary.

`opdev documentation plan --purpose durable --authority architecture --target
spec/documentation-layout.md` resolves caller-classified ownership without writes,
project-command execution or provider requests. Purposes are `durable`, `work`,
`temporary` and `mixed`. Durable/mixed require a declared durable owner; work
always uses `authorities.work`; mixed returns separate routes rather than accepting
one combined target. Omit `--target` to inspect declared routes. Inspect concrete
destinations separately, reusing unchanged observations rather than rerunning for
every paragraph. CLI capability absence must be disclosed, not silently called a
validated result; no installation is automatic.

Path checks use component boundaries and inspect existing parents. Traversal,
conflicting files, links, inaccessible paths and missing authority roots leave
routing unresolved; no placeholder is created. External authority identifiers
are preserved without contacting providers. Only an exact identifier matches
mechanically; membership of an external child item needs authorized inspection,
not a URL-prefix guess. A declared route does not establish access or permission.

JSON output has `schema: 1`, caller-supplied `purpose`, `routes` (authority,
declared reference, optional target, resolved flag and next step), aggregate
`resolved`, `next_step` and `limits`. Human and JSON exits are 0 for resolved
routing, 1 for unresolved routing and 2 for malformed input/tooling. These are
inspection results, not rule outcomes or qualification. Reports contain declared
locations; review private paths/identifiers before sharing. Semantic classification,
content adequacy and write permission remain review responsibilities. No keyword
CI gate, document registry or mandatory reviewer is introduced.

These defaults MUST NOT override existing authority locations. Existing
project-owned files, directories, and symlinks MUST NOT be overwritten, moved,
or repurposed automatically to match a default. An explicitly configured
location inside legacy `.opdev/`, an external URL, or another folder remains valid
until an explicit migration; the strict namespace does not permit arbitrary
internal overrides. Adequate external authorities remain valid after migration.
Agents MUST inspect ownership and existing content before assigning a purpose.
If an established authority is suitable, reuse it. Otherwise choose an unused
location appropriate to the repository and record it in `authorities` and
`context`. Resolve material ownership ambiguities with the project owner.

Discovery is a read-only proposal. A valid existing contract is returned intact;
an invalid contract fails instead of silently falling back to guesses. Without
a contract, candidate paths must have the expected type. Multiple candidates
for a role produce a warning and leave that authority unselected. A unique
candidate remains an inference requiring review. Absent folders are not created
or asserted to be authorities. Unknown/custom layouts require explicit routing.

New initialization writes `.opdev/project.yaml`, pending `.opdev/adoption.yaml`
decisions, and managed sections of root `AGENTS.md` and `CLAUDE.md`. Legacy projects
start assessment explicitly; see [adoption](adoption.md). Dry-run writes nothing, including for initialized
projects. Neither initialization nor upgrade migrates documentation.
`DELIVERY.md` is not a required filename. Repository-wide agent entry points
remain at the root; detailed project facts belong in their declared authorities.

## Decision and this repository

Grouping internal working documents reduces root clutter and avoids assigning
ownership of `docs/`, `spec/`, or `release/` in new projects. The alternative of
spreading all internal material across those directories adds navigation and
setup decisions; putting all public documentation in `.opdev/` would obscure it.
Dot-directories can be hidden by file browsers, so root README links provide
human discovery while the root agent entry points and contract route agents.
The Markdown remains usable without OpDev. Existing projects incur no migration.
Revisit this default if discoverability or navigation costs outweigh the reduced
clutter; do not replace review with silent directory ownership assumptions.

This repository keeps README, AGENTS, and CLAUDE as its root Markdown files.
Contributor and security guidance live in `docs/`; the changelog joins existing
release material in `release/`. Existing `spec/` and `release/` roles are retained.
Current references move with files; historical fingerprint-bound evidence remains
historical. No released archive is rewritten.

The README links directly to contribution and security guidance so access does
not depend on provider-specific automatic discovery. GitHub supports community
files in `docs/`; GitLab-specific automatic discovery is not assumed. Future
moves must review provider settings and links as well as repository references.
