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

## Documentation locations and ownership

Read the existing project contract and inspect established documentation before
choosing paths. `.opdev/` holds OpDev configuration and evidence. For a project
without established locations, suggest `docs/` for human guidance, `spec/` for
behavioral/design contracts, and `release/` for packaging inputs, release
procedures, recovery, and changelog. These are advisory defaults, not reserved
folders or a required scaffold. `DELIVERY.md` is optional.

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
creates only the contract and managed root agent instructions, not documentation
folders. Moving documentation is a separate, explicitly scoped project change;
update links, evidence references, and provider discovery settings together.
