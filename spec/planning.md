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

## Research checkpoint

Before committing to an implementation approach for a substantial request or
milestone, the agent MUST assess material uncertainty. Smaller changes also
require this check when novelty, impact or difficult reversal makes an unsupported
assumption consequential. Examples include unfamiliar dependencies, public
contracts, data migration, recovery feasibility, performance or consumer needs.
Size, file count and the word "milestone" alone do not require external research.
Provisional planning is permitted to identify the questions; this is not a
project-wide research phase or a prerequisite to urgent restoration.

The checkpoint has these acceptance conditions:

- RC-01: Assess consequential uncertainty before approach commitment for
  substantial requests, milestones and smaller high-risk changes. Preserve
  explicit user scope, including detailed supplied plans and advice-only requests.
- RC-02: Inspect and reuse accepted requirements, decisions, implementation,
  tests and previous findings when still current and applicable. Briefly explain
  why they suffice if no further investigation is needed. At later milestones,
  revisit changed assumptions, not settled choices. Honor explicit requests for
  research; do not substitute a sufficiency claim for the requested investigation.
- RC-03: Investigate questions whose answers could change the next approach or
  acceptance conditions. Identify the decision/assumption, evidence needed,
  scope or effort bound, observed findings and remaining uncertainty. Select
  evidence proportionately: current primary documentation for external facts,
  inspected code/tests or bounded experiments for behavior, and developer input
  for preferences and unresolved requirements. Sources must support the actual
  claim and relevant version/context; distinguish observation from inference.
  A representative probe establishes only its observed scope, not a general
  guarantee. Do not require renewed consent for already-authorized read-only
  research or describe unavailable tooling as missing consent.
- RC-04: Conclude with a justified approach, a bounded learning step, reduced
  scope or a focused decision question. A time/effort limit, unavailable source
  or inconclusive experiment MUST NOT become a pass. Do not commit dependent
  work to an unsupported consequential assumption; independent authorized work
  may continue. Reversible assumptions may be made explicit with validation and
  a revisit condition, but never waive a core requirement or required consent.
- RC-05: Keep findings and decisions in existing work/design authorities, with
  supporting references and limitations; an advice-only answer need not write
  anything. Feed accepted findings into applicable acceptance conditions and
  verification. No new mandatory document, schema, citation count, research
  command or completion flag is required. Evidence of research is not evidence
  of implementation, user approval, effectiveness or release qualification.
- RC-06: Shared skill guidance and fresh/upgraded project entry points carry the
  checkpoint while preserving project-owned content. Validate distribution and
  bootstrap mechanically, and assess decision quality through separate scenarios.

Research MUST NOT expand authority to install tools, contact people, disclose
private material, mutate external systems or perform production experiments.
Respect explicit access/browsing constraints and report their consequences;
missing evidence remains unresolved. Do not ask a developer to restate
discoverable facts, silently change accepted policy, or delay routine repository
operations with a questionnaire. Research needed for a later slice may remain
provisional until that decision becomes relevant.

Use the existing OPDEV-WORK-001 and applicable OPDEV-DESIGN-001 review mechanisms;
there is no new automatic research gate. Links or a well-formed record cannot
establish that the right questions were answered. Accepted behavioral findings
use the existing change-bound acceptance process, not a parallel research ledger.

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
The checkpoint makes an existing risk-review responsibility explicit rather than
adding a universal browsing phase. Reject size-only triggers, source quotas and
automatic completion flags: they reward activity rather than decision quality.
Reuse existing records to avoid project-specific setup; retain human/agent review
because uncertainty and evidence adequacy are semantic judgments. Revisit this
choice if fresh-context trials miss consequential unknowns or repeatedly
investigate settled work; do not treat more paperwork as stronger assurance.
Evaluate actual recommendations with the scenarios in
[planning review cases](../tests/planning-review.md), not just document shape.

## Basis

- [Kniberg: earliest testable/usable/lovable](https://blog.crisp.se/2016/01/25/henrikkniberg/making-sense-of-mvp): learn from usable increments rather than assume the final solution.
- [Patton: User Story Essentials](https://www.jpattonassociates.com/wp-content/uploads/2015/03/story_essentials_quickref.pdf): distinguish demonstrable results from technical recipes.
- [DORA: small batches](https://dora.dev/capabilities/working-in-small-batches/): preserve feedback through delivery, not just smaller upstream tickets.
- [Fowler: Keystone Interface](https://martinfowler.com/bliki/KeystoneInterface.html): integrate tested changes without premature activation, within an overall thin-slice approach.
- [MinimumCD](https://minimumcd.org/): preserve CI, testing, deployability and recovery constraints.
- [GOV.UK alpha](https://www.gov.uk/service-manual/agile-delivery/how-the-alpha-phase-works): investigate risky assumptions with the smallest useful probe, not the whole product.
- [GOV.UK research planning](https://www.gov.uk/service-manual/user-research/plan-user-research-for-your-service): choose evidence methods for the question and proportionate effort.
- [Shape Up: risks and rabbit holes](https://basecamp.com/shapeup/1.4-chapter-05): examine consequential unknowns before committing substantial work.
- [AWS decision records](https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/adr-process.html): preserve decision context and consequences for later review.
