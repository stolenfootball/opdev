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

`opdev init --dry-run` preserves an existing contract and writes no files.
Uninitialized discovery reports ambiguous candidates without choosing between
them; review its proposals before treating them as authorities. Initialization
creates the contract, pending adoption decisions on capable CLIs, and managed root
agent instructions, not documentation folders. Moving documentation is a separate, explicitly scoped project change;
update links, evidence references, and provider discovery settings together.
