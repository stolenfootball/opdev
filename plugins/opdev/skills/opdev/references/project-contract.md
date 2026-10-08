# Project contract interpretation

`.opdev/project.yaml` is a versioned, strict schema. Unknown fields and unsafe paths fail validation rather than being ignored.

- `project` declares a general software kind, one integration trunk, the CI provider, and the read-only remote.
- `authorities` maps important fact categories to one repository path, URL, or tracker. These are locations, not mandatory folder names.
- `commands` contains exact argument vectors, optional project-relative working directories, and timeouts. OpDev executes them directly except for its constrained Windows handling of allowlisted Node package-manager batch shims.
- `quality.risks` selects the characteristics that drive acceptance and testing depth.
- `testing` declares change and regression policy, flake visibility, coverage strategy, and suites by lifecycle stage.
- `delivery` describes the consumer-facing action, immutable artifact identity, representative environments, and recovery strategy. `migration_required` is a tracked gap, never compliance. `configured` is a reviewed assertion that the declared CI provider is the single supported delivery path; it requires a recovery strategy and at least one production-like qualification environment, while concrete pipeline and artifact claims still need independent evidence.
- `operations` routes health and observability evidence for operated software.
- `assurance.profiles` pins optional derived guidance by name and version.
- `extensions.checks` adds project-owned protocol commands. Blocking extensions affect their declared gate but remain separate from core rule results.
- `context` routes each task to only the authorities it needs. `always` is the small baseline.

When project facts change, update their declared authority and then reconcile pointers in the contract. Do not create a second authoritative copy for agent convenience.

Experiments reuse `authorities.work`, relevant context routes, and existing
`commands`/`testing.suites`. There is no required `experiments` manifest field.
For experimental work, follow [experiments.md](experiments.md); an optional
`experimental_change` context route can select the existing work, testing,
design, and delivery authorities without introducing another registry.

## Engineering policy

Project schema 3 explicitly selects `assurance.engineering.version: "1"` and an
actual developer decision reference. Require CLI capability `engineering.assessment.v1`
locally and in CI before migration; older contracts keep their original policy.
Baseline outcomes cannot be ignored; conditional safeguards require evidence of
actual applicability, not a project label or absent tool. Adequate implementation
choices remain flexible. MinimumCD mapping 1 is independently selected through
`assurance.engineering.minimumcd: "1"`; omission means no assessment, not compliance.
Do not interpret an engineering gate pass as a MinimumCD pass or release permission.
Use the same observations for both assessments, not another full test cycle.

For an authorized policy change, `upgrade --engineering-policy 1
--policy-review-reference <actual-reference>` previews a candidate without writes.
Its default keeps assessment intent; `--minimumcd-assessment none` explicitly
proposes no external assessment without waiving engineering. Review the actual diff,
local/CI capability and developer decision before editing the existing contract.
No routine policy questionnaire, new client file or automatic installation is needed.
The CLI does not authenticate a decision reference or grant consent.

Maintenance declarations identify exact supported branches and existing policy
authorities; they do not verify those branches. Review bounded supported fixes,
support lifetime, protection, pre/post-integration CI and retention of relevant
fixes in trunk. Ongoing features still use one integration trunk. Require actual
branch-specific evidence; do not reuse trunk qualification as maintenance CI.
Keep the stricter MinimumCD branch/cadence/rollback findings separate. Broad
recovery or a successful forward fix cannot alone establish rollback on demand.

## Documentation locations and ownership

Read the existing project contract and inspect established documentation before
choosing paths. `.opdev/` holds OpDev configuration and evidence. Without
established locations, use these optional defaults for internal working material:

- `.opdev/design.md`: current design and architecture.
- `.opdev/development.md`: human development guidance.
- `.opdev/delivery.md`: release and recovery guidance.
- `.opdev/specs/`: individual capability specifications.
- `.opdev/decisions/`: significant durable decisions.

These are placement conventions, not a scaffold. Create only documents justified
by the work; no empty folders, placeholders, or file per practice. Start small
and split documents when navigation warrants it. Small changes can remain in
their work item. Keep commands in `project.yaml` and status/sequencing in the
declared work authority, not competing PLAN, TODO, STATUS or command copies.
Repository-file work tracking remains valid when explicitly routed.

Keep the product README and both agent entry points at the root; preserve the
full managed AGENTS guidance. Link from the README to development guidance when
it exists, so humans can find it despite the hidden directory. Public product
docs, contribution/security guidance, changelogs and packaging inputs retain
appropriate project/ecosystem locations. Do not hide them in `.opdev/` merely
because an agent wrote them. No extra Markdown index or DELIVERY filename is
required.

Existing non-OpDev folders belong to the project. Reuse an appropriate authority
wherever it lives, including inside `.opdev/` if explicitly configured. Do not
repurpose a folder, overwrite a file, move existing material, or create competing
authorities merely to follow a default. Inspect conflicting paths and choose an
unused location consistent with the project; ask only when unresolved ownership
or meaning materially affects that choice. Existing task authorization applies.
Record selected locations in `authorities` and relevant `context` routes.
Folder names alone do not establish their contents as authoritative.

### Before writing a document

For an explicitly requested strict-layout assessment, a CLI advertising
`layout.inspect.v1` can run `layout inspect --layout-version 1` without writes.
The default inventories working-tree files, including ignored/untracked entries;
`--scope index` instead reads staged Git blobs. This previews the proposed three
required files (project/adoption/shared guidance) and optional `.opdev/docs/`
roles; it does not migrate existing projects or enable new check enforcement.
Inspect findings and actual content purpose, preserve adequate external authorities,
and never delete the old evidence ledger merely to obtain a clean layout report.
Retention and retrieval must be proved before evidence migration. Exit zero is
structural inspection only, not useful-document certification or agent behavior.
Do not run this assessment on every ordinary task or silently install a new CLI.

For a new document or a substantial expansion, classify the intended content
before choosing a destination. This is a brief reasoning check, not a new file,
approval ceremony or required reviewer:

- Durable behavior, design rationale, requirements and reusable operator guidance:
  use the relevant declared documentation/design authority.
- Milestones, task-specific acceptance, implementation sequence, open work
  questions and progress: use `authorities.work`, including an explicitly routed
  repository file. A folder named `spec` does not make a roadmap a specification.
- Temporary investigation: use proportionate conversation/disposable context;
  retain only useful durable conclusions or work decisions at their owners.
- Mixed: split the durable and work portions, reuse existing material at each
  owner and cross-link. Do not copy live status into the durable document.

For mixed material, route individual claims, not just document titles. Future
implementation order remains work tracking even when framed as rollout or
architectural phases. Use the [content review](#review-document-content) on the
actual changes before handoff, not merely the intended split.

Inspect existing content before adding another document or work item. Check task
authorization separately from placement: an implementation request may already
authorize the necessary writes, while advice alone does not. Ask only about a
material unresolved choice. If tracker access or write permission is unavailable,
keep that proposal in the conversation and explain what was not saved. Never
invent tracker identifiers or create a local fallback backlog. Do not move or
clean up existing documents without that change being in scope.

On a CLI with `documentation plan`, mechanically check the chosen routing before
the first new/substantially expanded document write; reuse that observation while
the authority, destination and intended purpose remain unchanged. For example:

```text
opdev documentation plan --purpose mixed --authority architecture
opdev documentation plan --purpose durable --authority architecture --target knowledge/system.md
opdev documentation plan --purpose work
```

Use the actual declared authority name/path, not these example locations. Mixed
content returns two routes; check concrete destinations separately when needed.
The command is read-only and does not contact external providers, classify prose,
grant permission or satisfy a gate. External item membership needs provider
inspection, not URL-prefix inference. Exit 1 means unresolved routing, not a
failed product test; exit 2 means invalid input/tooling. If this capability is
unavailable, say so and offer a compatible upgrade; retain the semantic check
without claiming deterministic validation or updating installations automatically.

Ordered operator instructions, migration phases, examples and enduring acceptance
requirements can be permanent documentation. Classify their purpose and lifetime,
not words such as "plan", "phase" or "next". Do not add a keyword CI blocker,
per-document registry, or extra agent merely to perform this check.

### Review document content

During normal review of new or substantially expanded documentation, compare the
actual changed passages with their source requirements and the related work
record (or unsaved work proposal). Do not take the author's summary, destination
name or successful routing command as evidence that the content is correctly
placed. Limit this review to the affected material; it is not a repository audit.

For a potentially mixed passage, ask: if implementation priority or task status
changed without changing the accepted product design, would this passage need
updating? If so, it describes work and belongs at the work authority. This also
applies to a roadmap disguised as "initial scope" or an architectural rollout.
By contrast, supported interfaces, explicit design exclusions, enduring behavioral
requirements and the order an operator must follow remain durable. Check the
source for that distinction; do not invent a scope restriction just to remove
sequencing, promote a provisional feature into a commitment, or delete useful
design constraints. Ambiguous requirements stay unresolved rather than becoming
an assumed policy change.

When a passage mixes both purposes, preserve its behavioral claim, put only the
implementation order/status/task acceptance at the work owner, and link instead
of repeating it. Preserve existing work items. If work writes are unavailable or
unauthorized, keep the pending portion in the response. Repair only within the
task's write authority, then reread the affected passages and their links.

In the existing review or handoff, identify a concrete corrected passage and its
destination, or explain a potentially ambiguous passage that was retained and
the source-backed reason. A blanket "placement checked" is insufficient for
mixed material. If no ambiguity exists, ordinary concise review evidence suffices.
Do not create a per-paragraph ledger, separate report, mandatory second agent or
new developer approval round. This is a semantic review responsibility, not an
automated guarantee or another CLI gate.

`opdev init --dry-run` preserves an existing contract and writes no files.
Uninitialized discovery reports ambiguous candidates without choosing between
them; review its proposals before treating them as authorities. Initialization
creates the contract, pending adoption decisions on capable CLIs, and managed root
agent instructions, not documentation folders. Moving documentation is a separate, explicitly scoped project change;
update links, evidence references, and provider discovery settings together.
