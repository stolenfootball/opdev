# Engineering policy 2

This document defines the accepted policy-2 design. Policy definition inspection
is separate from project activation and enforcement. The read-only `policy`
commands described in the [user guide](../docs/engineering-policy.md) do not
enable a schema-4 project or establish compliance. Current implementation and
remaining work are tracked in [GitLab](https://gitlab.com/stolenfootball-tools/opdev/-/issues/106),
not inferred from this specification.

## Guarantees and tailoring

Projects choose **how** to meet engineering obligations and which additional
standards to require, not whether missing verification should count as passing.
The four layers are mandatory baseline, capability safeguards, selected external
or organization policies, and implementation preferences within those guarantees.

The baseline covers authority/scope, controlled inputs and reproducible setup,
integration discipline, adequate tests, verification integrity, coding quality,
security hygiene, supported compatibility/transitions, applicable safe delivery
and recovery, and developer authority. Existing tools and equivalent adequate
controls remain valid. No vendor, universal coverage percentage, framework,
directory outside the selected OpDev namespace, or trunk spelling is prescribed.

Applicability depends on actual capabilities and change impact. Missing settings
or an absent tool do not demonstrate absence of risk. Unknown applicability is
unverified. Conditional does not mean optional. Capability facts, change review,
current execution and actual authorization remain distinct.

## Integration and recovery

One ordinary integration stream and small coherent increments are the default.
Bounded maintenance lines require supported scope/lifetime, protection, their own
pre/post-integration verification and retention of relevant fixes in trunk.
Cadence problems prompt diagnosis and replanning, never a retrospective timer gate.

Failed required trunk checks receive restoration priority. Affected delivery and
unsafe or dependent integration are blocked. Work demonstrably independent of the
failure may continue only if it does not hinder restoration. Different filenames
do not prove independence; shared dependencies, interfaces, build configuration and
unknown impact require conservative treatment. A known assertion failure, a broken
verifier and missing execution remain different outcomes, none of which qualifies
the affected boundary.

Safe recovery is demonstrated behavior, not a strategy label. Rollback, tested
forward transition, restoration or isolation/disablement may be adequate only when
they restore the required safe behavior for the actual failure scope. Review
retained state, new writes and external effects, triggers, recognition of recovery
success, and relevant time/data-loss constraints. A promise to write a patch later
does not qualify. MinimumCD's literal rollback requirement remains distinct.

Local development, an explicitly authorized preview and supported delivery have
different boundaries. Supported delivery uses the reviewed controlled CI path and
qualified immutable artifact identity. Different platform builds are different
artifacts; promotion preserves the tested bytes. Preview success cannot stand in
for required integration or delivery verification. No readiness verdict authorizes
release preparation, publication or deployment.

## Rule identity and complete accounting

Historical catalog-2/3 statements and results retain their meanings. Policy 2 uses
catalog 4; changed semantics receive different rule IDs. The pure resolver accounts
for all 42 catalog-3 rules. Thirty-five keep their statements and engineering
membership; four receive replacements; three stay external-only. There is no
removed rule silently treated as passed.

| Catalog-3 rule | Policy-2 engineering interpretation | Reason and verification obligation |
| --- | --- | --- |
| MCD-FLOW-001 | OPDEV-FLOW-001 | Contain affected work while prioritizing restoration; review current trunk observations and actual dependency/repair impact, not a declaration of independence. |
| MCD-DELIVERY-001 | OPDEV-DELIVERY-001 | Distinguish feedback, authorized preview and supported delivery; inspect actual paths, authorization and subject-specific checks. |
| MCD-PIPELINE-001 | OPDEV-PIPELINE-001 | Require complete qualification at the supported delivery boundary without imposing delivery qualification on every local iteration. |
| MCD-RECOVERY-001 | OPDEV-RECOVERY-001 | Demonstrate safe recovery including state and external effects, not merely a declared forward-fix strategy. |
| MCD-TRUNK-002 | External only; OPDEV-BRANCH-001 remains engineering | Preserve strict historical branch semantics separately from bounded maintenance. |
| MCD-TRUNK-003 | External only; advisory replanning for engineering | Keep actual cadence findings without a timer gate. |
| MCD-RECOVERY-002 | External only | Safe forward recovery does not establish literal rollback. |
| All remaining catalog-3 IDs | Retained | Existing statement, applicability, boundaries and verification responsibility remain unchanged. |

For every retained control, its catalog statement/sources and the
[evidence responsibility table](result-semantics.md#rule-to-evidence-responsibilities)
remain the rule-specific risk rationale and verification contract. New rule
statements and sources are in `rules/engineering-2.yaml`. Machine checks validate
identities, references, state and actual command/provider observations; review
establishes semantic adequacy, impact and alternatives. Definitions alone never
qualify behavior. Changed source, policy, evidence inputs, stage, environment,
artifact or time-sensitive observation invalidate the affected finding under the
existing conservative freshness rules.

Control cost is justified by its affected risk and boundary, not by how often an
agent can run it. Use focused feedback checks and shared observations at a required
boundary; do not run another suite merely because another profile maps to it.
Revisit controls that produce false blocks, repeated questionnaires, duplicate
execution or unsafe passes. Revisions change explicit versioned semantics rather
than adding hidden exceptions. Existing source/stage reuse restrictions remain.

## Current-control verification binding

Policy-2 replacement controls consume the ordinary current change review, not
project-level generic passing assertions. Optional `acceptance.policy_controls`
contains exact `version`, `definition_sha256`, `stage` and a bounded `bindings`
map from the four replacement IDs to existing condition/criterion IDs. The
containing acceptance digest includes this object when present; omission preserves
historical digests. Unknown controls, mismatched versions/digests, duplicate or
empty links are invalid, not silently discarded. Legacy catalogs reject this
new selection rather than reinterpret it.

Qualification requires the current acceptance review and its actual stage
verification. Durable criterion links require that stage's verification plan and
current catalog evaluation; change-only links require automated verification.
Review-only mapping judgments alone are not observations. Recovery requires at
least one automated exercised path among its all-required linked conditions.
Current failed/error/migration findings remain visible; missing or stale review
cannot convert them into passing. Reports identify the exact resolved definition
and evaluated stage, without making saved reports reusable execution evidence.

Semantic adequacy still requires review of the entire control statement. The CLI
does not infer dependency independence, recovery safety, inventory completeness or
developer permission from a link, strategy name, green command or stage label.
Requested provider qualification for the current boundary cannot be waived by
independence review. Known absence of distribution and supported operations must
be reviewed before excluding a replacement delivery control; missing facts remain
unverified. These rules add no per-control approval round or execution duplication.

## Standards and organization policies

Exact-version selection has three modes:

- `guidance`: useful mapping, no conformance verdict.
- `assess`: independent assessment, nonblocking for ordinary engineering gates.
- `require`: applicable requirements also block their selected boundaries.

A standard cannot postpone or weaken a baseline duty. Unsupported versions,
conflicts, unknown requirements and malformed definitions fail closed. Empty or
partial mappings cannot establish full conformance. Repository checks cannot
prove organization-wide governance, an external assessment or certification.
MinimumCD is the first complete built-in external mapping; existing SSDF, OSPS
and SLSA-derived mappings retain their explicitly limited claims and exact pins.

Organization policies are additive declarative definitions, not executable policy
programs. Stable IDs, exact versions, source references, constrained applicability,
typed bounded parameters, boundaries and existing suites/extensions/attributed
reviews connect requirements to verification. Selection pins actual content, not
just a filename. No arbitrary script during loading, remote auto-fetch, general
waiver language or baseline override is supported. A versioned strict layout may
allow only current definitions at `.opdev/policies/<policy-id>.json`; built-in-only
projects need no additional file. Tests and execution history do not belong there.

## Resolution, evaluation and migration

A pure resolver computes exact definitions, obligation membership, applicability
conditions, boundaries, references, conflicts and a deterministic definition
identity. It executes no project command or network call and grants no consent.
The evaluator separately applies current observations and the unchanged six
outcomes. The definition digest identifies embedded semantics, not a project
approval, artifact, execution subject or fresh verification result.

Project schema 4, engineering 2, catalog 4 and check-report schema 3 are explicit
contracts, not silent reinterpretations of older data. A policy preview must
expose actual additions/removals, applicability, enforcement boundaries and
freshness consequences. Review policy changes under the preexisting change-control
authority: a candidate cannot authorize its own weakening. An explicitly removed
optional standard need not pass before removal, but its removal requires actual
authority and cannot erase historical failed results.

Selections and parameters belong in `project.yaml`; durable product guarantees
and test mappings remain in the requirements catalog; adoption decisions remain
in the adoption record; current review uses the MR/PR; routine execution reports
have bounded CI retention. This adds no ledger, database or policy service.

Empty, existing non-OpDev and older-OpDev projects all converge on a clean reviewed
target. Preserve adequate tools and unrelated content; remove reviewed retired
artifacts and reopen changed obligations. Interrupted migration must preserve later
edits and require current inputs. Existing decisions are reused during ordinary
work, not converted into a per-rule questionnaire. Pending required gaps remain
incomplete adoption.

## Rationale, alternatives and limits

[NIST SSDF](https://csrc.nist.gov/projects/ssdf) motivates outcome/risk-based
implementation choices. [OSCAL](https://pages.nist.gov/OSCAL/learn/concepts/layer/control/)
provides precedent for separating catalogs, profiles and assessment; OpDev does
not adopt OSCAL's complete tailoring language.
[DORA](https://dora.dev/capabilities/continuous-delivery/) supports rapid feedback
and separates delivery capability from automatic deployment. The
[SRE error-budget policy](https://sre.google/workbook/error-budget-policy/) offers
precedent for scoped restoration decisions, not proof of a specific CI rule.
[MinimumCD](https://minimumcd.org/) retains its independently assessed literal
requirements rather than lending its name to a looser baseline.

[Hilton et al., FSE 2017](https://www.cs.cmu.edu/~mhilton/docs/HiltonFSE17.pdf)
describe assurance/speed and flexibility/simplicity tensions.
[Inozemtseva and Holmes, ICSE 2014](https://www.cs.ubc.ca/~rtholmes/papers/icse_2014_inozemtseva.pdf)
support not treating coverage percentage as assertion adequacy.
[SPACE](https://www.microsoft.com/en-us/research/publication/the-space-of-developer-productivity-theres-more-to-it-than-you-think/)
supports evaluating multiple dimensions, not maximizing a single throughput metric.
These precedents motivate the design; they do not prove OpDev's safety, universal
applicability, latency or token savings. Measure unsafe acceptance, false blocks,
unnecessary questions, time/CI/token costs and developer understanding separately.

Alternatives rejected: universally enforcing all MinimumCD clauses would retain
the observed tailoring conflicts; unrestricted user standards could remove minimum
guarantees; an executable policy engine would introduce unnecessary trust and
maintenance costs. Revisit the constrained model if real needs cannot be expressed
without repeated duplicate declarations. Do not fix model limits through hidden
waivers, false passes or automatic installation.
