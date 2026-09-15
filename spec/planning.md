# Outcome-based planning

## Applicability and authority

When OpDev is active, all planning objectives MUST use outcome-based incremental
reasoning: plans, roadmaps, reviews, prioritization, task decomposition, adoption,
replanning, and informal next-step recommendations. This does not activate OpDev
in an uninitialized project or expand authorization. Advice remains advice;
writing work items or implementing a recommendation requires task authority.

Explicit user objectives, constraints, requested order and deliverables take
precedence over the default decomposition strategy. A request for a component,
migration, research report or specific repair MUST NOT be replaced with an
unrequested product increment. Surface conflicts and material risks explicitly.
No preference overrides an applicable core requirement.

## Planning contract

Default milestones SHOULD be thin, demonstrable consumer outcomes crossing the
necessary technical boundaries, not completion of whole implementation layers.
Technical tasks are the implementation recipe beneath an outcome. Consumers may
be human users, developers, operators, APIs, data pipelines or devices; no UI,
web stack, public release or business feature is universally required.

For the next increment identify, proportionally: consumer/outcome or uncertainty,
scope/exclusions, dependencies/risks, acceptance and demonstration evidence,
feedback and follow-on decision, and applicable delivery/exposure/recovery.
These are planning facts, not new mandatory schema fields or documents. A
next-step answer may express them in a few sentences. Reuse the existing work
authority and canonical design/contract locations.

Inspect current state before recommending the next step. Prioritize restoration
when required trunk CI is red. When facts or feedback are unavailable, name the
uncertainty rather than fabricate progress or validation. Detail the next useful
increment while keeping distant sequencing provisional. Replan after meaningful
evidence; a completed task list is not proof of product effectiveness.

Bounded enabling work is permitted and sometimes the correct next step. State
its supported objective, uncertainty, scope/time/effort bound and exit evidence
or decision. A pipeline prerequisite, technical spike or migration rehearsal is
not required to pretend to be a user feature. Avoid unbounded foundations and
speculative abstractions. Neither every commit nor every integration must expose
a new capability.

Tests and applicable safeguards belong within increments, not at the end of a
layer-first plan. Distinguish architectural connectivity, functional correctness,
usefulness and releasability. A walking skeleton or controlled prototype may
establish only some of these. Experiments retain explicit isolation and limits;
all distributed variants still need qualification. Manual feedback activities do
not authorize bypassing the CI-only software delivery path. Existing adoption
decisions and core evidence semantics remain unchanged.

## Decision, alternatives and reversal

The risk is that detailed AI-generated component lists delay useful feedback
while appearing complete. Adopt consumer-outcome decomposition with bounded
enabling work; reject both mandatory layer-first roadmaps and a rigid demand for
a user-visible feature in every commit. User intent remains controlling.

The lifecycle in the specification repeats per increment; it is not a waterfall
of project-wide phases. Planning quality requires semantic review under existing
OPDEV-WORK-001/OPDEV-DESIGN-001 evidence practices, not keyword checks or a new
automatically passing gate. No schema/catalog version or runtime pin changes.

Revisit this guidance if recorded planning reviews show excessive ceremony,
neglected cross-cutting risks, missed user constraints, or delayed useful evidence.
Evaluate actual recommendations with the scenarios in
[planning review cases](../tests/planning-review.md), not just document shape.

## Basis

- [Kniberg: earliest testable/usable/lovable](https://blog.crisp.se/2016/01/25/henrikkniberg/making-sense-of-mvp): learn from usable increments rather than assume the final solution.
- [Patton: User Story Essentials](https://www.jpattonassociates.com/wp-content/uploads/2015/03/story_essentials_quickref.pdf): distinguish demonstrable results from technical recipes.
- [DORA: small batches](https://dora.dev/capabilities/working-in-small-batches/): preserve feedback through delivery, not just smaller upstream tickets.
- [Fowler: Keystone Interface](https://martinfowler.com/bliki/KeystoneInterface.html): integrate tested changes without premature activation, within an overall thin-slice approach.
- [MinimumCD](https://minimumcd.org/): preserve CI, testing, deployability and recovery constraints.
