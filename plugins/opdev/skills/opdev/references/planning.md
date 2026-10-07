# Plan outcomes, not completed layers

Use this guidance whenever OpDev is active and you form, refine, review, or
recommend a plan: initial discovery, roadmaps, task breakdowns, prioritization,
adoption sequencing, replanning, and questions such as "what is the next step?"
The activation/consent gate still applies. A planning question authorizes an
answer, not implementation, tracker writes, installation, or publication.

Before saving a plan, apply the [document placement check](project-contract.md#before-writing-a-document).
Implementation sequencing and task acceptance belong to the declared work
authority; durable design belongs to its own authority. Split mixed content and
reuse existing records. If saving the work portion is unavailable or unauthorized,
keep it in the answer rather than embedding a substitute backlog in the design.

## Start from the requested objective

Honor explicit user scope, technology choices, ordering, deliverables, and output
format. If asked for a database migration, plan that migration; do not substitute
a new UI feature or demand product discovery. Explain material risks and propose
alternatives when useful, but do not silently replace the request. Core safety,
testing and delivery requirements still apply.

For an open-ended objective, identify the consumer, desired outcome and largest
uncertainty. Reuse current project/work facts; ask only questions that materially
affect the next decision. Consumers may be people, API clients, library callers,
operators, downstream pipelines or hardware. A visible UI is not required.

## Check uncertainty before committing to an approach

For substantial requests and milestone boundaries, assess what is known and
which unsupported assumptions could materially change the approach. Do the same
for a smaller high-risk change: unfamiliar dependencies, public contracts,
migrations, recovery, performance and consumer needs are common signals. A
provisional plan can expose these questions. Do not impose a research phase
because a task is large, or postpone urgent restoration or routine operations.

Inspect existing requirements, accepted decisions, implementation, tests and
prior findings first. Reuse them when still current and applicable; briefly
explain their sufficiency when no further investigation is needed. At later
milestones revisit changed assumptions, not every settled decision. An explicit
research request still requires investigation, within the user's constraints;
do not ask again for permission to perform that already-authorized read-only
research. If the needed method is unavailable, report that limitation rather than
mislabeling it as missing consent.

For material unknowns, identify the decision at stake, evidence that would resolve
it, a scope or effort bound, the findings and what remains uncertain. These are
reasoning prompts, not a form or required new file. Use current primary sources
for external compatibility/behavior claims, inspected code and tests or bounded
experiments for local behavior, and developer input for unresolved needs or
preferences. Check that sources support the claim in the relevant version and
context. Distinguish observed facts from inference; documentation alone cannot
establish local performance or actual user demand. Limit conclusions to the
tested/observed scope: a representative probe is not proof of a general guarantee.
Avoid source-count quotas.

Conclude with a justified approach, a bounded learning step, narrower scope or a
focused question. Budget expiry, unavailable evidence or an inconclusive result
does not settle the question. Do not commit dependent work to a consequential
unsupported assumption; independent authorized work may continue. If an assumption
is safely reversible, state it with its validation and revisit condition, without
waiving a core requirement or substituting for required developer consent.

Keep useful findings, references and limitations in the existing work authority;
durable decisions belong in the routed design authority. Advice-only work may
remain in the answer. Carry accepted findings into applicable acceptance conditions
and tests using the existing evidence process. No new ledger, schema, blanket
approval round or research-complete flag is required. Research is not implementation,
approval or release evidence. Defer unrelated later-slice research until useful.

Research does not grant permission to install tools, contact users, disclose
private code/data, mutate external systems or experiment on production. Respect
access/browsing restrictions and report unresolved consequences. Do not repeat
discoverable questions or silently override the developer's supplied plan/policy.

## Choose the next meaningful increment

Prefer a thin, demonstrable end-to-end capability across the technical boundaries
needed for that outcome. Detail the next increment; describe later ones as
provisional options, not a fixed sequence immune to feedback. Group database,
API, interface and test tasks beneath the outcome rather than making "finish
database, finish backend, finish frontend, then test" the default roadmap.
Small commits alone do not establish usable value or validated learning.

For "what next?", inspect current progress, blockers and evidence, then recommend
one next outcome or bounded enabling step, why it comes next, and how completion
will be demonstrated. A short answer can carry this method without a formal plan
or questionnaire. Do not execute it unless asked. Required trunk restoration and
urgent recovery take priority over new feature work; do not invent current CI
status when it cannot be checked.

Capture these facts proportionally in the existing work item or answer, not a
new mandatory project file or form:

- Consumer and concrete outcome (or uncertainty/decision to resolve).
- Smallest coherent scope, exclusions and material dependencies/risks.
- Demonstration and acceptance evidence, including relevant automated tests.
- Feedback source and the observation that will guide the next decision.
- Delivery/exposure and recovery appropriate to this increment, when applicable.

For example: "A library caller can parse one supported input format and receive
a useful error for malformed input" is a slice. Implement only the supporting
parser, public interface, documentation and tests needed to demonstrate it.
Do not first complete every internal abstraction for every future format.

## Enabling work and learning are legitimate

Use bounded technical work when it enables the requested outcome or reduces a
material risk: a feasibility spike, migration rehearsal, security prerequisite,
build pipeline, integration probe or recovery fix. Name the outcome it supports,
the question it answers, a time/effort/scope bound, and its exit evidence or
decision. Avoid an indefinite foundation phase; do not fabricate user value for
every commit. Explicit component requests can themselves be the objective.

A walking skeleton exercises the necessary path through the system with minimal
functionality. It proves connectivity/architecture, not necessarily usefulness.
A learning prototype can use controlled data or manual operations, with those
limits disclosed. Distinguish testable, usable and release-ready; never present
a mock or prototype as evidence of an untested production capability. A feedback
checkpoint may need to wait for real evidence: report that dependency, do not
invent user validation or force a production release.

## Build, verify, learn, revise

Distinguish disposable local exploration, retained implementation, CI candidates
and publication when choosing the next outcome. For an isolated disposable probe,
capture purpose, temporary location, actual checks and discard/promotion intent in
the existing conversation/work context. Use only checks needed to execute and
safely evaluate it; obtain feedback before production polish, full suites or
acceptance-ledger assembly. No exploration schema or release-bound experiment
record is required. Ordinary authorization, secrets, data and process safety
apply. Temporary work cannot become another production path or qualify gates.

For an environment-specific unknown, a bounded CI/remote diagnostic can obtain
feedback before another merge/package cycle using existing authorized tooling.
State the question, environment, effort bound, side effects and next decision in
current work context. Review source trust, runner privileges, inherited secrets
and log/artifact exposure; prefer observation. For isolated temporary resources,
record ownership, verify it before cleanup and check removal. Preserve failures
and report observations as diagnostics, not integration/delivery qualification.
This adds no automatic remote execution, runner mutation or blanket approval step.

For retained work, identify expected behavior before implementation and use
focused checks during iteration. After feedback settles the intended source,
prepare exact-source acceptance bindings and run required canonical checks at
integration handoff. Material edits still require fresh review/execution. Reuse
current context; reread authorities when relevant facts or assumptions change.

Milestones normally close on a demonstrated consumer outcome and appropriate
integration/candidate evidence. Do not automatically add version bumps, release
notes, tags, registry verification or publication to every issue. Use CI-built
candidates when consumers need package bytes or affected packaging/native assets
need qualification. An identified consumer need is a reason to recommend a
supported checkpoint, not permission to release it. Follow the explicit
[release authorization boundary](workflow.md#release-authorization) before
release-specific preparation or execution. Package/recovery tests follow affected
behavior and actual delivery; reuse observations only for the same unchanged
artifact/configuration and required freshness. Explicit releases retain the
declared delivery path and immutable-artifact requirements.

Plan tests and safety within each increment, not a final testing/hardening phase.
Use the narrow tests and integration/consumer-path checks needed to establish
its behavior. User feedback establishes usefulness, not test correctness.
Preserve existing behavior, integrate frequently, and retain all core gates.
Integration need not activate incomplete behavior; apply the experiment guidance
when warranted. Smaller scope is not a waiver of privacy, security, compatibility,
CI, qualification or recovery requirements.

Reassess when supporting work becomes the bottleneck: repeated harness repairs, a growing matrix,
duplicate full checks or fragmented MRs can delay the consumer observation the
work was meant to enable. Name the distinct risk each remaining check resolves
and the feedback currently missing. Consolidate tightly coupled repairs into the
smallest coherent increment, or use a bounded diagnostic before adding machinery.
Preserve genuinely distinct platform/configuration coverage; matching command
text does not prove equivalent environments. If a required policy should change,
propose that change with evidence and tradeoffs instead of silently skipping it.
No fixed failure count, new ledger, questionnaire or fresh permission round is
needed within existing authorization.

### Review the CI feedback path when relevant

During CI design/change, or when observed repeated checks, queueing or setup delay
the requested outcome, inspect the relevant path from change to useful feedback.
Do not wait for the user to explicitly request optimization, but do not turn every
task into a pipeline audit. Use available configuration and recent observations;
name missing timings or external includes rather than inventing their behavior.

Look for duplicate branch/MR triggers, repeated full qualification merely to enter
a candidate pipeline, caches with no compatible producer, and cancellation rules
that discard deliberately started long trials. Compare checks by the distinct
risk and execution context they cover, not matching command names. Preserve MR
and integrated-trunk verification and required native/configuration coverage.
Candidate continuation must retain the exact source, effective configuration,
environment, artifact identity, trust boundary and required freshness. A matching
commit alone is insufficient; a cache hit is not qualification or artifact identity.
An optional trial's failed/missing result still blocks its acceptance claim even
when the provider reports a green pipeline. Do not make all jobs non-interruptible.
Keep outcome labels distinct: an executed test with a failing assertion is
`failed`; a required result that is absent or not run is `unverified`; a broken
test launcher/tool invocation is `error`, not a product failure. All three block
the applicable qualification, but require different next actions.

Offer a small, ranked recommendation with observed cost, uncertainty, tradeoff
and a way to validate it. Separate summed runner time, queue delay and end-to-end
feedback latency; do not label all repeated work waste or promise a speedup from
configuration inspection. Generic principles apply across providers; consult
current provider documentation for proposed syntax/semantics when needed.

Carry an already observed opportunity into the
[development handoff](results.md#handoff-after-development), after completing the
requested slice. Merely calling CI "out of scope" is not a recommendation.

Recommendations are advisory, not a new gate. An implementation request for a
product feature does not authorize unrelated CI optimization. Do not edit CI,
cancel jobs, change caches/runner policy, open optimization issues or launch paid
trials merely because an opportunity was found. Obtain authorization for the
proposed change. If the developer declines or prefers the existing compliant
setup, acknowledge that choice, continue the requested work and do not repeatedly
raise the same suggestion without new material evidence or a request to revisit.
No mandatory opt-out file, rationale form or new schema is required. A real core
requirement failure remains a separate finding, never disguised as an optional
efficiency preference or waived by declining an optimization.

After acceptance checks and meaningful feedback, revisit the next recommendation:
continue, revise, stop, or take a bounded enabling step. Do not automatically
execute an entire speculative roadmap. Keep durable contracts/design rationale
in their existing authorities, current sequencing in the work tracker, and
project commands in the project contract. Assess all adoption practices, but
sequence their implementation around useful increments; unresolved adoption
remains incomplete rather than blocking all learning or becoming a hidden pass.
