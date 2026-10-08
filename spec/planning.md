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

## Connected phases and scope

A larger goal MAY use phases, milestones and implementation slices. Collapse
levels that add no useful decision; a familiar repair does not need a roadmap.
A phase describes increased usable capability, not completion of a technical
layer. An MVP is the smallest accepted useful outcome, not permission to omit
applicable security, data integrity, accessibility or recovery safeguards.

PH-01: Preserve the supplied goal, mandatory requirements and fixed constraints.
Distinguish current scope, already satisfied requirements, accepted later scope,
unresolved requirements and optional ideas in the existing work authority. Retain
the destination, reason and revisit condition for genuine deferral. Later implementation detail may
be provisional without making an accepted requirement optional. Moving a required
outcome out of scope requires an actual developer decision, not an agent label.
PH-02: Give each useful phase a consumer outcome, inclusions/exclusions and an
observable completion demonstration. Explain how its milestones compose into
that outcome and how phases contribute to the goal. Detail near-term slices;
avoid speculative task inventories for distant work.
PH-03: Inspect material dependencies and shared contracts before dependent work:
interfaces, data, compatibility and cross-cutting safeguards. Plan an assembled
consumer-path check as well as meaningful component checks. Individually passing
parts or closed tickets MUST NOT establish that the intended whole works.
PH-04: At a milestone boundary, reuse current evidence and reassess changed
assumptions, dependencies, risks and the next useful outcome. Apply the research
checkpoint only to material uncertainty; no universal browsing, high-effort model,
new approval round or separate planning document is required.
PH-05: Report slice, milestone, phase and overall-goal completion separately when
those distinctions matter. MVP completion preserves accepted later work; it is
neither whole-roadmap completion nor release authorization. Verify the assembled
accepted outcome before declaring its containing scope complete.

## Authorized continuation

SC-01: Continue meaningful implementation within the developer's authorized scope
by default. Honor explicit guided checkpoints and stop boundaries. "Implement the
MVP" does not authorize later phases; "finish the approved roadmap" covers its
committed scope, not optional ideas; "plan this" authorizes no implementation.
Persisting toward an outcome does not expand release, delegation, installation,
spending, external-action or policy authority. Status questions do not cancel
ongoing work; replacing, narrowing, pausing and revoking instructions do change it.
SC-02: Before yielding because one item is blocked, identify what the blocker
actually prevents. Resolve it within existing authority when possible, or ask the
necessary focused question and continue meaningful authorized independent work.
That includes tests, review or another ready slice when they do not presume the
missing answer or create unsafe conflicting edits. Merely promising to continue
is not continuation. Exhausting useful safe independent work, a global dependency,
an actual resource limit or an explicit stop instruction is a valid stopping point.
SC-03: Keep unfinished work bounded and safely resumable. Normally finish one
active slice before starting another; a parked dependency should retain its exact
state, decision needed and resumption condition at the existing work authority.
This is not a numeric WIP gate or permission to override an instruction to continue.
Do not stack speculative branches, bypass a required check, perform busywork or
poll indefinitely to appear persistent. Resume parked work after the actual answer
and recheck changed source/dependencies before reusing evidence.
SC-04: Feedback readiness permits an early observation, not an automatic end to
an authorized implementation assignment. If feedback is required to choose the
dependent approach, honor it and continue only independent work. Otherwise give
the progress update and continue toward the authorized completion boundary.
Guided feedback requests remain valid; neither mode waives integration checks.
SC-05: Fresh context MUST read original current decisions and work status before
acting. Derived summaries and local pointers neither grant consent nor override
later changes. Retain scope, completed/unverified outcomes, remaining dependencies,
pending decisions and the next authorized action in existing work references,
without a new scheduler, approval registry, local backlog or transcript archive.

## Decision, alternatives and reversal

Apply the accepted [workflow usability boundaries](workflow-usability.md): isolated
disposable local exploration reaches feedback with proportional execution checks;
retained implementation finalizes source-bound evidence at integration handoff;
CI candidates and publication follow their actual consumer need. An issue or
milestone does not itself request tags, registry publication or release notes.
Required integration checks and affected package/recovery qualification remain
applicable. Reuse observations only for the same unchanged artifact/configuration
and required freshness.

The risk is that detailed AI-generated component lists delay useful feedback
while appearing complete. Adopt consumer-outcome decomposition with bounded
enabling work; reject both mandatory layer-first roadmaps and a rigid demand for
a user-visible feature in every commit. User intent remains controlling.

The lifecycle in the specification repeats per increment; it is not a waterfall
of project-wide technical phases. Outcome phases may group several such increments.
Planning quality requires semantic review under existing
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

Connected phases balance an overall goal with rolling-wave detail. Reject both
rigid layer-first plans and disconnected small tasks with no composition evidence.
Scoped continuation avoids repeated permission requests while preserving actual
developer control. Revisit if complete-session trials show scope overreach, lost
requirements, unsafe context switching, premature stopping or disproportionate
planning cost. Distribution tests establish routing, not decision quality.

## Supporting-work bottlenecks

SB-01: When repeated tool repairs, expanding matrices, duplicate verification or
fragmented work delays useful feedback, planning SHOULD reassess the supported
outcome, distinct risks and smallest coherent increment. Prefer bounded diagnosis
or consolidate coupled changes when appropriate; do not respond by automatically
adding more infrastructure. There is no fixed retry threshold or required form.
SB-02: Distinct native platforms and supported configurations MUST retain their
required evidence. Policy simplification is a proposed reviewed change, not an
implicit waiver. Existing scope/authorization remains sufficient for routine
replanning; material scope changes still require developer direction.

## Advisory CI feedback-path review

CF-01: CI design/change or observed verification bottlenecks SHOULD trigger a
proportional review of the relevant end-to-end feedback path without requiring
an explicit optimization request. Routine unrelated work MUST NOT trigger a full
audit. Inspect evidence before identifying redundancy; preserve distinct risks.
CF-02: Consider trigger duplication, redundant qualification before candidate
continuation, cache producers/consumers and deliberate long-trial cancellation.
Preserve required pre/post-integration verification and native coverage. Reuse
requires source, effective configuration, environment, immutable artifact, trust
and freshness equivalence, not commit/argv equality. Optional trial failures or
missing results cannot establish acceptance through a green aggregate pipeline.
CF-03: Recommendations MUST separate observations from hypotheses and runner cost
from queue/end-to-end latency. Include a bounded validation and tradeoff; no
unmeasured speed claims, blanket cache sharing or cancellation policy.
CF-04: Efficiency is advisory. Do not automatically implement, mutate provider
state or create work items. A developer may retain a compliant inefficient setup
without a new record or failing gate; honor declines until materially new evidence
or explicit reconsideration. Core failures remain separately reported and cannot
be waived. This introduces no provider optimizer, schema or mandatory checklist.

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
