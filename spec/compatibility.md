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
0.4.2 pins qualified CLI 0.4.0 and requires
CLI >=0.2.0, <0.5.0 for ordinary work. Schema-2 adoption needs CLI 0.2.1;
capability detection must report the gap when an older standalone runtime is selected. Both host
manifests and the packaged compatibility contract must agree on the plugin
version; the current CLI and managed pin must satisfy its CLI range.

CLI 0.3.0 introduces the strengthened acceptance/remote qualification and doctor
exit contracts described below and in `release/CHANGELOG.md`. Plugin 0.4.2's
managed setup selects the independently published and qualified CLI 0.4.0.
CLI 0.3.1 repairs diagnostics and supported GitLab local-include inspection and
adds read-only remote-policy readiness/observation to adoption planning. It does
not select policy, migrate project state, or change the existing schema versions.
Updating the plugin alone does not migrate project records or CI pins; review
the documented migrations before relying on the strengthened gates.

CLI 0.4.0 adds execution preview and acceptance preparation, advisory CI and
document-placement review, typed workflow references, optional bounded delegation,
and independently invalidated evidence subjects. Catalog 2 changes daily-merge
cadence to a compliance finding rather than a development/integration blocker;
the finding remains visible and does not claim MinimumCD compliance. Existing
project schemas 1 and 2 remain readable. New records and policies have their own
versioned contracts; unsupported versions must not be rewritten or inferred.
Fresh canonical execution and single-agent operation remain the defaults.
Plugin 0.4.2's managed setup supplies CLI 0.4.0; older standalone selections still
require capability checks. Updating a pin does not migrate project state or CI.

Pre-1.0 releases may change command-line and plugin behavior between minor
versions, but migrations and diagnostics are still required for project-owned
state.

## Project-manifest schema

CLI 0.4.1's capability `check.post-merge-integration.v1` identifies the corrected
post-merge integration exit and dual integration/delivery check routing. Discover
it through `doctor`; version labels alone do not identify unreleased source builds.
Earlier CLIs (including released 0.4.0) use the delivery exit for post-merge.
Report that limitation and offer a compatible upgrade; do not reinterpret old
reports, ignore failing exits or initiate a release to satisfy that coupling.
This correction changes no schema or managed runtime pin. Plugin 0.4.2 retains
the qualified CLI 0.4.0 pin during publication; selecting a qualified CLI 0.4.1
supplies the corrected exit behavior without migrating project records.

CLI 0.4.0 advertises `execution.same-run.v1` and supports the separately
versioned [same-run execution policy and record](execution-reuse.md). This is
explicit opt-in, not a reinterpretation of diagnostic receipts or historical
reports. Unsupported fields/versions are rejected without rewriting them.
Inspect both producer and evaluator capabilities before changing CI. The legacy
fresh-execution path remains available; no version or managed pin changes follow
automatically from this capability.

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

CLI 0.3.0 reads project schemas 1 and 2. Schema 2 enables an optional
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

Development capability `upgrade.coordinated-migration.v1` explicitly coordinates
the schema-3/layout-1/external-review transition using a strict schema-1 ephemeral
request and root/runtime/input-bound recovery snapshot. It does not change the
meaning of an ordinary guidance upgrade, legacy project, old approval or diagnostic
report. Older clients must refuse these inputs; install/update capability separately
before applying the reviewed project and CI transition. No consumer migration or
release version follows from introducing this interface. Capability-based safeguards
are independently selected by `acceptance.safeguards.v1`; catalog/schema choices
and changed assurance remain visible rather than being attributed to a plugin update.

Development capability `adoption.clean-target.v1` adds the exact clean-1 destination,
read-only pre-initialization planning, full reviewed `init --project` input and
retirement/source-bound cleanup completion checks. Explicit legacy diagnostics
never report current completion; ordinary checks do not migrate existing policy.

Development capability `upgrade.reviewed-retirement.v1` adds explicit bounded text
moves/file retirement/nonrecursive empty-directory removal to coordinated migration,
bound to clean-target decisions and mandatory private recovery. It is not a generic
filesystem mover or permission to delete legacy evidence without retained history.

Development capability `adoption.external-review.v1` uses the selected authenticated
semantic review for adoption verification without a legacy repository ledger.
Its exact locator and independent acceptance identity follow ordinary check;
current adoption assertions, canonical execution and source freshness still apply.
It does not migrate project policy or establish approval by retrieval.

Development capability `evidence.authenticated-review.v1` adds an explicit
schema-3 review-storage boundary and independent schema-1 semantic-review/work
observation records. Legacy projects retain their ledger and previous qualification
path. Older clients reject the new manifest fields rather than silently ignoring
the storage decision. Local/CI capability must be reviewed before migration;
publishing or installing a new plugin alone selects no policy. Diagnostic envelope
and local-state formats retain their non-qualifying meaning. Actual execution
remains fresh or uses the existing separately authenticated same-run boundary.

Development capability `layout.enforcement.v1` adds explicitly selected layout 1
to schema-3 contracts, one shared guide and a built-in `policy` check result.
Legacy policy remains unchanged. Older clients that lack these fields or result
kinds reject them instead of dropping enforcement. Check local and CI capability
before selection. Existing schema-3 users without layout selection retain their
current checks. No release version, installed pin or consumer migration is selected
by this additive development interface; complete adoption still needs the explicit
evidence-storage transition. Formatting previews have independent schema 1.

CLI 0.4.0's catalog 2 retains the daily-integration requirement and its ID for
compliance, but removes it from development and integration gates. Rule results
are not converted to passes. Existing project manifests and evidence ledgers
remain readable; there is no automatic ledger rewrite. Older CLIs still enforce
their embedded catalog, so review the selected local and CI versions before
expecting the relaxed operational behavior. Plugin guidance must not claim an
older CLI's blocked gate passed. Historical catalog-1 reports retain their
original verdicts; the experimental report reader requires the originating CLI
for a different catalog version rather than reinterpreting old results.

Human report wording adds explanations and next steps without changing JSON
field names, rule identifiers, outcome values or exit codes. Selecting a new CLI
does not rewrite historical results or automatically update managed runtime pins.

### Acceptance evidence capability

Development source additionally corrects declaration-only behavioral findings.
`doctor` advertises `evidence.behavioral-qualification.v1` for this verifier.
Configured CI/delivery, testing-risk/regression/retry/coverage policies and health
authorities remain unverified without the required review/observations. Selected
pre/post-merge testing also requires actual execution and current acceptance.
Local CI inspection cannot erase that failure or missing verification. Missing
selected checks remain visible with `--no-exec` at every supported boundary.
See the [rule-to-evidence map](result-semantics.md#rule-to-evidence-responsibilities).
This strengthens verification of unchanged catalog-2 requirements; it neither
removes a rule nor changes existing outcome/report-schema values. Old report
bytes retain their originating meaning and are not fresh qualification inputs.
Projects must review missing facts rather than mass-copy policy assertions to
recover green gates. Shipping this behavior requires normal qualified version
and migration communication; no managed pin or consumer records change implicitly.

CLI 0.3.0 and development CLIs with `evidence acceptance-digest` read ledger/bootstrap schemas
1 and 2 and generate schema-2 bootstrap. Older CLIs reject schema 2; do not rename
the version to obtain an old-client pass. TEST-002/003 require typed current-change
acceptance evidence on capable CLIs even when the ledger is schema 1. Missing
migration remains unverified, not a policy-only pass. This strengthens the
verifier for existing requirements; their rule IDs are unchanged. No other core
rule is relaxed, and there is no automatic project-state rewrite.

Review migration and the CLI/CI runtime capability explicitly. Keep unrelated
assertions/history intact, add the current inventory/mappings, then review and
execute applicable suites. Reverting to a legacy ledger does not waive the
strengthened checks on a capable CLI. Saved execution/report schemas are unchanged
and remain diagnostic rather than new qualification inputs. Release/runtime pins
must be qualified separately before distributing this development capability.

The catalog has its own integer `catalog_version`. Rule IDs are permanent.

- Clarifications that do not change required behavior keep the rule ID.
- A changed requirement receives a new rule ID; the previous rule remains
  available for interpreting historical evidence.
- From 1.0 onward, removing a core requirement requires a major OpDev release
  and a published rationale. During pre-1.0 development, an explicitly reviewed
  breaking policy change may ship in a minor release only with a new exact policy,
  appropriate schema/catalog versions, published rationale and explicit project
  migration. Renaming a rule or moving it between profiles does not evade this
  requirement. Never silently reinterpret existing contracts or historical reports.
- Generated documentation, diagnostics, profiles, and evidence refer to rule IDs
  rather than copied rule prose.

## Extension-result protocol

Project commands and future rule packs communicate through a semantic protocol
version. Major versions are incompatible. Unknown fields in a compatible major
version are ignored unless the protocol explicitly marks them critical.

## External assurance profiles

Development capability `engineering.assessment.v1` supports explicit project
schema 3, engineering policy 1, catalog 3 and report schema 2. Schemas 1/2 continue
to use catalog 2 and report schema 1; existing verifier corrections still apply.
Ordinary init retains its prior schema pending an actual reviewed policy choice.
The capability does not change this repository's selected policy, managed CLI
pins, installed plugins or release versions. The pre-1.0 amendment above is a
deliberate compatibility change for this policy split, not release authorization.
Consumers and CI must select capable runtimes before explicit migration. Older
clients reject unsupported schema-3 projects/report-schema-2 data rather than
interpreting it as legacy green evidence. MinimumCD mapping versions identify both
the mapping revision and exact upstream source; no floating website assessment.

External standards and profiles are pinned by name and version. Installing a new
OpDev release MUST NOT silently change an existing project's selected profile
version. Profile upgrades are explicit and report newly applicable or changed
requirements before modifying the project contract.

## Workflow references

Workflow journal/subject schema 1 and the `workflow.references.v1` capability are
additive. Their inspection exit is not a core gate verdict; JSON always says
qualification is unverified. Unknown event fields or journal/subject versions
are rejected without rewrites. Existing work authorities, evidence ledgers,
reports, adoption decisions and runtime pins retain their original semantics.
No installed consumer is automatically migrated to a reference journal.

Acceptance-review references compare the complete current acceptance digest
independently of source identity. A reference missing that digest remains readable
but its review is stale, not implicitly current. Other record kinds retain their
separate meanings. Optional JSON fields may be omitted or null as the schema
declares; unknown fields remain errors.

## Optional delegation

Capability `delegation.v1` identifies strict assignment/result schema 1 validation.
It neither promises host dispatch support nor qualifies worker claims. Unsupported
versions fail without rewriting records. Older clients and hosts can continue
single-agent work. Native adapters must disclose missing observed settings and
permission limitations; no global configuration or consumer agent files are
installed automatically.

## Provider APIs

GitHub and GitLab adapters isolate provider API versions from the core domain
model. Provider changes may produce `unverified` or `error`; they MUST NOT cause
a remote policy to be reported as passing based on cached assumptions.

