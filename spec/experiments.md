# Experiments without blocking unrelated releases

OpDev separates integration, artifact delivery, and feature activation. An
experiment MAY integrate on the single trunk while unrelated, qualified behavior
is released. Experimental effectiveness may remain unverified; correctness and
deployability for the delivered scope still require evidence. This policy applies
to all software kinds, not just hosted services or languages with build flags.

## Scope and isolation

Before implementing an experiment, distinguish **not enabled** from **not shipped**
with the user when that choice materially affects the result. Select the smallest
mechanism that preserves supported behavior:

- Runtime opt-in when dormant code can safely ship in the same artifact.
- Build-time exclusion when code or dependencies must be absent from stable assets.
- Branch by abstraction for incremental implementation replacements.
- Disconnected components when live wiring can be deferred safely.
- A bounded, isolated prototype for exploratory work not ready for product integration.

A preview channel describes distribution and support, not isolation. It MUST use
the declared CI delivery path and does not establish that stable artifacts exclude
experimental behavior. A prototype MUST NOT become an alternative production path
or a permanent second integration trunk. Useful product changes return through the
normal integration process; a prototype is not automatically release-qualified.

Isolation MUST account for shared code, startup, dependencies, schema/data changes,
security and resource effects, not just a hidden UI or disabled switch. A feature
flag is not an authorization boundary. Recovery MUST address persistent effects;
turning a flag off does not undo a migration or corrupted data.

## Lightweight record and lifecycle

No experiment registry, flag service, manifest field, or empty directory is
required for projects without experiments. For each active experiment, use the
existing work authority as the canonical home for:

1. Stable ID, work reference, accountable owner, and an inclusive UTC review date.
2. Purpose or hypothesis and observable promotion and abandonment criteria.
3. Isolation mechanism, explicit opt-in, stable default, and recovery approach.
4. Supported stable and experimental configurations, mapped to declared test suites.
5. A cleanup plan for the flag, obsolete implementation, tests, configuration,
   documentation, and any preview artifacts/support commitments.

The [record schema](../schema/experiment.schema.json) and
[editable template](../plugins/opdev/skills/opdev/references/experiment.yaml)
standardize these fields. Store the record in the work item, or make the work item
point to its canonical repository file. A temporary tracker export is a snapshot,
not another authority. Do not maintain competing lifecycle status in static specs.
If a repository work authority has no established location, propose a file under
`.opdev/experiments/`; do not create it merely during initialization. Existing
authority locations always take precedence.

Lifecycle: propose and bound the experiment; implement isolated increments;
evaluate correctness, deployability, and effectiveness separately; then promote,
abandon, or explicitly extend with reviewed evidence and a new review date.
An overdue review requires a decision, not automatic enablement, removal, or an
unreviewed date extension. Record unresolved review gaps when judging affected
work or delivery. Do not use unrelated documentation work to silently approve them.

Promotion is not complete until temporary machinery is removed or explicitly
reclassified as a supported permanent option with ownership and tests. Operational
kill switches and permanent product options are not failed experiments: they have
a different support lifecycle. No universal time-to-live or flag-count quota is
imposed. Review dates and supported configurations are proportional to risk.

## Testing and delivery

Required pre-integration and integrated-trunk suites MUST cover the actual stable
configuration and every supported experimental configuration. One suite MAY
exercise several configurations; reviewers MUST verify it actually does so.
All-features testing alone does not establish feature-disabled behavior. Test
material interactions, activation, disablement, compatibility, and recovery
according to risk. Document unsupported combinations rather than claiming an
exhaustive configuration matrix. Keep the number of supported variants bounded.

Build options are part of artifact identity. Each distributed variant MUST be
qualified independently and promoted unchanged; rebuilding with different flags
cannot reuse another variant's artifact evidence. Runtime behavior configuration
MUST remain versioned, tested, and delivered through the declared automated path.
Evidence identifies the artifact and tested configuration. An experiment does not
weaken a required suite or permit a red trunk.

## Validation and gate evidence

Development CLIs with `opdev experiment validate` accept a YAML or JSON record:

```sh
opdev experiment validate path/to/experiment.yaml --root .
```

The command validates schema version 1, nonblank metadata, calendar/review date,
unique configuration IDs, both exposure categories, declared suite references,
and pre/post-integration suite coverage for each configuration. Review dates are
inclusive and evaluated against the current UTC day. Exit 0 means record
validation only; exit 2 means invalid input or a validation/tooling error.
It does not execute commands, inspect source isolation, fetch tracker references,
write evidence, discover experiments, or mark an OpDev gate satisfied. Validate
active records, not historical snapshots, when adding this command as a canonical
project suite. It never automatically scans every archived record in CI.

Agents MUST review the applicable experiment record and actual evidence under the
existing rules; there is no new waiver or per-change no-experiment declaration:

| Existing rule | Experimental-change evidence |
| --- | --- |
| OPDEV-WORK-001, OPDEV-DESIGN-001 | Bounded work, owner/review, decision criteria, isolation and reversal rationale. |
| OPDEV-TEST-002, OPDEV-TEST-003, MCD-TEST-001/002 | Configuration-specific acceptance and executed tests. |
| MCD-COMPAT-001, MCD-CONFIG-001/002 | Stable behavior and governed activation/configuration. |
| MCD-ARTIFACT-001/002, MCD-ENV-001, MCD-RECOVERY-001 | Exact variant qualification and tested recovery. |

Missing required behavioral evidence remains `unverified`, even with a valid
record. Change assertions belong in the existing fingerprint-bound evidence flow,
not permanent project-wide claims that all future experiments are safe.

Older published CLIs do not implement record validation. Check
`opdev experiment --help` before using it; unavailable capability requires explicit
reporting. Agents can still apply this lifecycle through existing work, testing,
and evidence rules, but MUST NOT claim automated record validation occurred. No
project-manifest schema migration, new rule ID, profile change, or runtime-pin
upgrade is introduced here. Schema 1 records fail closed on unknown fields or
versions. Existing projects do not acquire fabricated experiment evidence.

## Decision, alternatives, and reversal trigger

The problem is independently releasable work sharing trunk with uncertain features.
MinimumCD's [single-path guidance](https://beyond.minimumcd.org/docs/migrate-to-cd/pipeline/single-path-to-production/)
recommends flags and abstraction. [Feature-toggle guidance](https://martinfowler.com/articles/feature-toggles.html)
distinguishes temporary release/experiment controls from long-lived controls.
[GitLab's lifecycle practice](https://docs.gitlab.com/development/feature_flags/controls/)
demonstrates explicit aging review. [Cargo's feature semantics](https://doc.rust-lang.org/cargo/reference/features.html)
illustrate why one language's mechanism cannot be the universal policy.

Choose a standard record and existing evidence gates rather than a second trunk,
mandatory global registry, custom flag platform, or prescribed build-flag framework.
This minimizes setup and tracker duplication, at the cost of requiring reviewed
discovery and semantic evidence: metadata validation alone cannot enforce real
isolation. Revisit automated inventory/provider adapters if missed experiments or
stale flags recur in actual usage. Add stronger checks without weakening core
gates or turning every ordinary change into an experiment.
