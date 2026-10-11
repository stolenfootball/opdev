# Understanding engineering policy

OpDev separates engineering readiness from compliance with an additional standard.
A project chooses suitable tools and practices, but missing tests or safeguards
cannot become passing simply because the project prefers not to use them.

## Inspect before changing policy

On a CLI advertising `policy.preview.v1` in `doctor`, inspect definitions without
changing your selected project policy:

```text
opdev policy explain --engineering 2
opdev policy explain --engineering 2 --rule OPDEV-RECOVERY-001
opdev policy preview --engineering 2 --root . --format json
```

`explain` needs no project. `preview` reads the current initialized project and
shows its selection alongside the proposed definition. Both identify retained,
replaced and external-only rules; neither edits files, executes project checks,
contacts a provider, selects standards or records approval. Exit zero means the
definition was inspected successfully, not that your project passes it.

**Definition preview is not enforcement.** These commands alone do not enable a
Policy 2 project contract, migrate adoption records or change existing check
behavior. Keep using the currently selected project policy until a supported,
explicitly reviewed migration is applied. The preview's definition digest covers
its embedded semantic definition; it is not an approval or reusable test result.

Use the JSON rule's statement, applicability, boundaries and verification routes
to understand the obligation. A conditional rule needs evidence of whether it
applies. Missing configuration does not prove that it does not apply. Unsupported
versions and unknown rule IDs are errors, never a fallback to fewer requirements.

## Checking an explicitly selected Policy 2 contract

A CLI advertising `engineering.assessment.v2` reads project schema 4 with
`assurance.engineering.version: "2"`. Schema 3 still means Policy 1; schemas 1/2
retain their historical meanings. A valid schema-4 contract includes the reviewed
capability safeguard policy. Missing or unknown applicability is not an exemption.
Changing these fields is a policy migration requiring existing developer authority,
not an automatic update or a side effect of `policy preview`.

`opdev check` evaluates that selection using the normal suites and current review.
Its schema-3 report identifies catalog 4, the exact policy-definition digest and
the evaluated stage. A passing command or saved report cannot qualify another
source, configuration or stage. Existing Policy 1 reports retain schema 2.
An evaluation-only check does not run tests and cannot turn their absence green.

The four changed controls use `acceptance.policy_controls` inside the existing
MR/PR review. This links control IDs to existing acceptance conditions or durable
requirement criteria; it does not create another document, database or history.
The object records the exact policy version, definition digest, stage and
`bindings` map. Each applicable control also needs a current change-level semantic
assessment. A project-wide passing assertion or strategy label is insufficient.
The containing source-bound review includes these links in its identity.

Use existing automated conditions or durable verification plans with current-stage
execution/current manual observations. An older review-only acceptance mapping is
a judgment about adequacy, not a fresh observation. Recovery additionally needs an
automated exercised path; its test and review must cover retained state, new writes,
external effects, failure triggers, success recognition and relevant time/data-loss
constraints. A test passing does not mechanically prove that this inventory is complete.

Required check failures remain visible and block their boundary. An independence
review cannot waive a required failing check for the current source. It addresses
work outside a known trunk failure's actual dependency and restoration scope;
unknown impact does not establish independence. Requested remote qualification
still needs current provider results, not a local assertion.

No distribution/service capability can be inferred from a missing delivery tool.
Only reviewed absence of both distribution and supported operations, confirmed in
the current acceptance review, permits the new delivery controls to be marked
`not_applicable`. That does not waive other applicable baseline obligations.

## Policy decisions and their effects

| Situation | Intended treatment |
| --- | --- |
| Required trunk CI fails | Prioritize repair; block affected integration/delivery. Independent work needs actual dependency and restoration-impact review. |
| A release cannot safely roll back its data | Demonstrate safe tested recovery covering data and external effects. A promise to patch later is insufficient. |
| A developer is iterating on a design | Use meaningful focused feedback checks. Required integration checks still apply before merge and on integrated source. |
| A project needs maintenance branches | Bound supported fixes/lifetime, protect and test that branch, and retain relevant fixes in trunk. |
| An organization requires a standard | Pin the exact supported policy and its enforcement boundaries in addition to the OpDev baseline. |
| Checks pass | Report only the verified boundary. Passing checks never authorize release execution. |

MinimumCD remains distinct. A project may meet OpDev's safe-recovery requirement
and still lack MinimumCD's required rollback evidence. Neither engineering
readiness nor a partial standards mapping establishes full conformance.
Historical rules shown outside the engineering baseline are not automatically
external obligations: the selected standard's exact mapping determines those.
For example, MinimumCD mapping 1 uses `MCD-RECOVERY-002` for literal rollback,
not the older, broader `MCD-RECOVERY-001` recovery statement.

## Choosing additional standards

A CLI advertising `policy.standards.v1` supports `assurance.standards` in a
schema-4 project. The existing `assurance.profiles` list remains informative;
upgrading the CLI does not convert that list into enforced requirements.

| Mode | What the check does |
| --- | --- |
| `guidance` | Shows the pinned mapping and its limits. No conformance verdict. Omit `stages`. |
| `assess` | Reports a separate assessment at the selected stages without adding a gate blocker. |
| `require` | Adds a required check at each selected stage. A missing, failing or incomplete assessment blocks that boundary. |

For example, these are selections to place under the existing `assurance` object
after the developer reviews them, not a complete project contract:

```yaml
standards:
  - name: nist-ssdf-derived
    version: "1.1"
    mode: guidance
  - name: minimumcd
    version: "1"
    mode: require
    stages: [delivery]
```

This example makes MinimumCD required for delivery qualification, not for every
local feedback check. It does not relax OpDev's integration or delivery baseline.
Outside a selected stage the standard says **not assessed**, not passed. Select
the boundaries required by the actual project or organizational policy; an agent
must not remove a requirement merely to get a green result.

Pin exact supported names, versions and optional levels. Unknown versions,
duplicate selections and selecting MinimumCD both here and through the legacy
`assurance.engineering.minimumcd` shortcut are errors before project commands run.
`check --require-minimumcd` can additionally require a selected MinimumCD
assessment for the current invocation's exit code; guidance or an assessment
selected only at a different stage does not satisfy that flag.

All assessments reuse this check's existing rule findings and command results.
Selecting three standards does not run a suite three times. Their definition
digests bind the selected mode, boundaries, level and embedded mapping. These are
identities, not evidence, permissions or reusable execution results. The report
retains the actual upstream source, mapping limitations and missing clauses.

MinimumCD mapping 1 covers its complete pinned manifesto. The derived SSDF/OSPS
and SLSA/CycloneDX evidence-format mappings do **not** establish full conformance,
even if all contributing checks pass. Selecting one of those as `require` leaves
its complete-conformance check unresolved; use guidance or assessment for those
limited mappings rather than calling a subset a certification. OpDev does not
claim third-party certification for any standard.

Organization definitions are a separate policy mechanism, not arbitrary external
scripts, an automatic download or a per-rule exception list.

For exact semantics, rationale, compatibility and evidence responsibilities, see
the [engineering policy specification](../spec/engineering-policy.md). Existing
Policy 1 behavior is documented in [assurance profiles](../spec/assurance-profiles.md).
Implementation progress lives in [GitLab](https://gitlab.com/stolenfootball-tools/opdev/-/issues/106),
not in a repository checklist.
