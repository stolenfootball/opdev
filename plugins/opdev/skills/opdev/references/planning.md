# Plan outcomes, not completed layers

Use this guidance whenever OpDev is active and you form, refine, review, or
recommend a plan: initial discovery, roadmaps, task breakdowns, prioritization,
adoption sequencing, replanning, and questions such as "what is the next step?"
The activation/consent gate still applies. A planning question authorizes an
answer, not implementation, tracker writes, installation, or publication.

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

Plan tests and safety within each increment, not a final testing/hardening phase.
Use the narrow tests and integration/consumer-path checks needed to establish
its behavior. User feedback establishes usefulness, not test correctness.
Preserve existing behavior, integrate frequently, and retain all core gates.
Integration need not activate incomplete behavior; apply the experiment guidance
when warranted. Smaller scope is not a waiver of privacy, security, compatibility,
CI, qualification or recovery requirements.

After acceptance checks and meaningful feedback, revisit the next recommendation:
continue, revise, stop, or take a bounded enabling step. Do not automatically
execute an entire speculative roadmap. Keep durable contracts/design rationale
in their existing authorities, current sequencing in the work tracker, and
project commands in the project contract. Assess all adoption practices, but
sequence their implementation around useful increments; unresolved adoption
remains incomplete rather than blocking all learning or becoming a hidden pass.
