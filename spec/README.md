# OpDev normative specification

This directory defines how OpDev interprets and evaluates the machine-readable
requirements in [`rules/core.yaml`](../rules/core.yaml). The rule catalog is the
authoritative home for individual requirements. This specification is the
authoritative home for cross-rule semantics.

OpDev is a development operating system for general software projects. It
combines an evidence-driven development workflow with the delivery constraints
published by [MinimumCD](https://minimumcd.org/).

## Normative language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**,
and **MAY** are interpreted as described by RFC 2119 and RFC 8174 when, and only
when, they appear in capitals.

## Authority order

When two OpDev sources appear to conflict, use this order:

1. The selected versioned rule catalog (`rules/core.yaml`, with catalog-3 additions
   in `rules/engineering.yaml`) and exact policy membership.
2. Result and aggregation semantics in `spec/result-semantics.md`.
3. Compatibility rules in `spec/compatibility.md`.
4. A project's valid `.opdev/project.yaml` contract.
5. Platform adapters, templates, generated documentation, and agent guidance.

Project contracts select applicable profiles and implementations. They cannot
weaken a core rule or redefine a core failure as passing.

See [engineering policy and assurance profiles](assurance-profiles.md) for the
explicit schema-3 baseline, conditional requirements, preferences and separately
versioned MinimumCD assessment. Legacy contracts keep their original selection.

Schema-validated project evidence follows [`evidence-ledger.md`](evidence-ledger.md).
It may satisfy an otherwise unverified rule only when the catalog explicitly
allows evidence verification; it cannot override a concrete failure, error, or
migration requirement.

See [documentation ownership and folder defaults](documentation-layout.md) for
authority placement and conflict handling.

See [compact report and evidence views](compact-views.md) for read-only
projections, retained diagnostics, freshness, and compatibility.

See [test execution evidence](test-reports.md) for tool-neutral canonical
execution receipts and optional read-only JUnit inspection, separate from gates.

See [same-run execution reuse](execution-reuse.md) for opt-in authenticated
producer records, evaluation without rerunning checks, and conservative fallback.

See [resumable workflow references](workflow-records.md) for typed observations,
explicit history-preserving writes and read-only continuation without qualification.

See [bounded delegation](delegation.md) for optional native specialist assignments,
return validation, controller ownership and host limitations.

See [evidence lifetimes](evidence-lifetimes.md) for independent review, execution,
artifact and decision invalidation with conservative whole-subject defaults.

See [execution preview](check-plan.md) to inspect the commands a check selects
without running them or qualifying a gate.

See [CI design and improvement](ci-design.md) for evidence-led recommendations,
provider patterns, developer choice and the limits of optimization evidence.

See [acceptance preparation](evidence-preparation.md) for mechanical draft,
preview and reviewed application to an existing schema-2 ledger.

See [consistency review](consistency-review.md) for advisory comparison of accepted
requirements, implementation, tests and documentation using existing authorities.
Routine substantive development also requires
[change-scoped acceptance evidence](evidence-ledger.md#schema-2-acceptance-evidence)
on capable CLIs; passing suite execution alone does not establish assertion adequacy.

See [token-efficiency evaluation](token-efficiency.md) for the benchmark's
measurement, acceptance, and effectiveness boundaries.

See [experiments](experiments.md) for independently releasable work on trunk,
standard experiment records, configuration testing, and removal decisions.

See [adoption](adoption.md) for the complete practice assessment, project-specific
recommendations, explicit dispositions, and evidence-backed initialization completion.

See [upgrades](upgrades.md) for coordinated plugin/runtime updates, reviewed project
guidance application and preserved project decisions.

See [readiness diagnostics](doctor.md) for read-only prerequisites, scoped exits,
runtime identity, actionable findings and explicit qualification limits.

## Proposed architecture

The [resumable workflow redesign](workflow-redesign.md) consolidates the proposed
execution/evaluation separation, evidence lifetimes, resumable decisions and
optional specialist-agent coordination. It is a design proposal, not implemented
behavior or an amendment to the current versioned contracts. Implementation
sequencing, milestones and progress belong in the
[GitLab redesign roadmap](https://gitlab.com/stolenfootball-tools/opdev/-/work_items/62),
not this specification.

## Lifecycle

Apply [outcome-based planning](planning.md) to formal plans and informal next-step
recommendations. The lifecycle below repeats within increments; it is not a
sequence of project-wide technical phases. Larger goals may group increments
into useful MVP/later phases, with assembled-outcome verification and scoped
continuation through local blockers. Honor explicit user scope and constraints.

OpDev uses this general lifecycle:

```text
Understand -> Specify -> Design -> Implement -> Verify
           -> Integrate -> Package -> Deliver -> Observe -> Learn
```

The lifecycle describes outcomes rather than prescribing a framework, hosting
platform, programming language, repository topology, or work tracker.

## Three independent verdicts

OpDev keeps these judgments separate:

- **Correctness**: the software does what its contracts and acceptance criteria
  say under the tested conditions.
- **Deployability**: the exact artifact can be delivered and recovered through
  the declared automated path.
- **Effectiveness**: measured use or evaluation shows that the software solves
  the intended problem.

A project can be correct and deployable without yet having effectiveness
evidence. It cannot be called deployable when a required delivery check is
missing, stale, or unverified.

## Proportional evidence

See [workflow usability](workflow-usability.md) for disposable exploration,
retained-work handoff, external operations, deliberate publication cadence,
actionable diagnostics and early remote-adoption policy decisions.

OpDev scales evidence to the risk and durability of a change:

- Routine changes use an executable work item, automated tests, and merge
  evidence.
- Contract changes also update the contract's canonical authority.
- Durable architecture changes also record alternatives, rationale, evidence,
  and a reversal trigger.
- Specialized security, accessibility, performance, safety, or effectiveness
  evidence is required when the corresponding risk or profile applies.

Proportionality can reduce ceremony. It cannot remove applicable MinimumCD
requirements or turn unknown evidence into a passing result.

## External standards

OpDev's own [consumer-interface targets](consumer-interface.md) define its
approved CLI accessibility, human-review and operational-diagnostic requirements.
They do not impose the same implementation on consumer projects.

OpDev uses public guidance from MinimumCD, NIST SSDF, SLSA, W3C WCAG, OWASP,
OpenSSF, and the public descriptions of ISO testing and quality models. OpDev
only claims conformance to an external standard when a versioned profile
contains every applicable requirement and the project supplies complete
evidence. Referencing or deriving a practice is not a conformance claim.
