# Compatibility and versioning

OpDev versions four public contracts independently.

## CLI and plugin release

The CLI and agent plugins use semantic versions. A published plugin declares the
CLI version range it supports in packaged `opdev-compatibility.json`. The shared
plugin skill verifies this relationship before its first OpDev action in each
task. The Claude Code prompt hook only detects project state; runtime verification
belongs to the shared skill after activation. A missing, malformed,
unsupported, or unsatisfied compatibility contract prevents OpDev activation;
Codex plugin installation itself does not provide a portable activation hook.

A plugin-managed runtime may pin an older published CLI within its supported
range. Runtime selection, installation, and recovery are defined in
[`installation.md`](installation.md). The plugin version and its runtime pin
are intentionally distinct.

Plugin-only updates do not require rebuilding or republishing the CLI. Plugin
0.2.5 pins qualified CLI 0.2.2 and requires
CLI >=0.2.0, <0.3.0 for ordinary work. Schema-2 adoption needs CLI 0.2.1;
capability detection must report the gap when an older standalone runtime is selected. Both host
manifests and the packaged compatibility contract must agree on the plugin
version; the current CLI and managed pin must satisfy its CLI range.

Pre-1.0 releases may change command-line and plugin behavior between minor
versions, but migrations and diagnostics are still required for project-owned
state.

## Project-manifest schema

See [coordinated upgrades](upgrades.md) for read-only assessment, reviewed guidance
application, host/runtime boundaries and separate project verification. Bare
`upgrade` now previews; older runtimes require capability detection before use.

`.opdev/project.yaml` contains an integer `schema` version.

- Additive fields that old clients can safely ignore do not change the schema
  version.
- A semantic change, removed field, renamed field, or stricter interpretation
  increments the schema version.
- A CLI MUST refuse to rewrite a newer unsupported schema.
- `opdev upgrade` MUST be explicit, idempotent, and preserve unrelated project
  content.

The first stable CLI will read its current schema and at least one previous
schema when a deterministic migration exists.

Development CLIs now read project schemas 1 and 2. Schema 2 enables an optional
reviewed `project.ci.qualification` policy; schema 1 rejects that field. Discovery
still emits schema 1, and neither initialization nor upgrade chooses a workflow,
check producer or protection policy. Opt-in requires a developer-reviewed diff
to the existing manifest, preserving unrelated fields, and an actual decision
reference. There is no automatic migration or extra policy file. Older CLIs
reject schema 2 rather than silently ignore its meaning. See
[remote audits](remote-audits.md) for fields and API limitations.

`check --remote` now requires exact current-trunk qualification: a schema-1
manifest remains readable, but missing reviewed policy produces unverified remote
qualification, even if generic historical/local evidence passed. Ordinary checks
without `--remote` retain their behavior. CLI capability/version review and
explicit policy adoption must precede changing CI invocations to require this
stronger verification. Returning to schema 1 requires explicitly removing the
new policy and acknowledging that remote qualification is again unverified; it
does not waive required gates.

Adoption decisions now write separate record schema 2 and retain practice catalog 1. Existing
projects without a record are legacy-unassessed, not silently migrated or blocked
by new ordinary-check requirements. New development CLIs distinguish successful
scaffolding from completion; `adoption start` explicitly opts legacy projects in.
Unknown fields, missing/extra practices and unsupported versions fail closed.
Schema 1 is readable but requires an explicit `adoption migrate` preview/write
and genuine approval before completion. Migration preserves old decisions, not
an inferred approval. Older runtimes reject schema 2; capability-check before
upgrading project-owned records. No automatic adoption catalog migration is implemented. Older runtimes cannot
claim this completion check; see [adoption](adoption.md).

Experiment records have a separate integer schema (currently 1). They do not add
fields to the project manifest. Development CLIs expose `experiment validate`;
older published runtimes may still apply the agent lifecycle but cannot claim
automated record validation. The validator rejects unknown record fields and
versions and never rewrites records. See [experiments](experiments.md).

## Rule catalog

The catalog has its own integer `catalog_version`. Rule IDs are permanent.

- Clarifications that do not change required behavior keep the rule ID.
- A changed requirement receives a new rule ID; the previous rule remains
  available for interpreting historical evidence.
- Removing a core requirement requires a major OpDev release and a published
  rationale.
- Generated documentation, diagnostics, profiles, and evidence refer to rule IDs
  rather than copied rule prose.

## Extension-result protocol

Project commands and future rule packs communicate through a semantic protocol
version. Major versions are incompatible. Unknown fields in a compatible major
version are ignored unless the protocol explicitly marks them critical.

## External assurance profiles

External standards and profiles are pinned by name and version. Installing a new
OpDev release MUST NOT silently change an existing project's selected profile
version. Profile upgrades are explicit and report newly applicable or changed
requirements before modifying the project contract.

## Provider APIs

GitHub and GitLab adapters isolate provider API versions from the core domain
model. Provider changes may produce `unverified` or `error`; they MUST NOT cause
a remote policy to be reported as passing based on cached assumptions.

