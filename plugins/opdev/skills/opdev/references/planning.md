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

## Connect phases to the goal

For a larger goal, use goal -> optional phases -> milestones -> slices/tasks only
as far as those levels help. A small repair may need only an outcome and tests.
Make phases increases in useful capability, not "all infrastructure, then all
features, then testing". Preserve the developer's supplied plan and constraints;
propose a different decomposition with reasons rather than silently replacing it.

Separate current scope, already satisfied requirements, accepted later requirements,
unresolved needs and optional ideas at the existing work authority. For genuine
deferral, retain its destination, reason and revisit condition. Keep required outcomes visible when detailed
later work is provisional. Do not silently defer a mandatory feature or promote
an optional idea into an obligation. An MVP needs a usable accepted outcome and
its applicable safeguards; smaller scope does not postpone security or integrity
needed for that outcome.

Give each useful phase an outcome, scope/exclusions and completion demonstration.
Show how the milestones fit together and contribute to the overall goal. Identify
shared interfaces, data, compatibility and material dependencies early enough to
avoid incompatible slices. Include an assembled consumer-path check: individually
passing components and closed issues do not prove the whole works. Detail the next
useful slice; defer speculative design and research for later decisions.

At milestone start, briefly check the current goal, scope, dependency readiness,
changed assumptions, risks and evidence for the next outcome. Reuse settled facts;
investigate only consequential uncertainty using the checkpoint above. This is
not a mandatory questionnaire, browser session, higher-effort model switch or
approval round. Familiar work and urgent restoration should remain direct.

For example, a backup library's MVP might round-trip one item safely. Later
accepted scope adds batch restore; scheduling is optional. The MVP includes
corruption/error handling needed for its one-item path, not a fake success that
defers integrity. Check the storage format and reader together. Finishing this
MVP does not finish batch restore, authorize scheduling or request publication.

## Choose the next meaningful increment

Prefer a thin, demonstrable end-to-end capability across the technical boundaries
needed for that outcome. Detail the next increment; keep later implementation
sequencing provisional without demoting accepted requirements to options. Group database,
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

### Ready for feedback

Ready for feedback is a valid checkpoint for retained work, not just disposable
mockups. Stop there when the developer requested feedback before continuing or
the next dependent action needs their decision. Otherwise show the observation
and continue the authorized assignment; follow [scoped continuation](workflow.md#scoped-continuation)
when only part of the work is blocked. Real source edits may remain on a development branch or worktree while
the developer evaluates the direction. Retention does not make every edit, commit,
agent turn or accepted tweak an integration handoff. Do not call retained work
disposable to obtain a lighter workflow.

Identify expected behavior and the question the next observation should answer.
Run the focused tests, execution and safety checks needed to make that observation
meaningful, then show the result promptly. Keep assertions current as behavior
changes. A relevant failure remains visible: repair it when it makes the feedback
unsafe or misleading; otherwise disclose the limitation without claiming success.
Unknown impact, data/security risks or changed shared contracts may require broader
checks before feedback. Proportional does not always mean fewer tests.

Do not require a full suite, finalized acceptance ledger/fingerprint, broad model
or platform evaluation matrix, plugin installation or release qualification merely
to ask whether a direction is useful. Use a targeted behavioral trial for the
changed hypothesis. Test actual installation/host behavior when that is the
uncertainty; a source-level trial does not establish installed-host behavior.
Repeat the focused loop after feedback, rather than restarting all completion
steps. Ask for feedback when it resolves a real uncertainty, not after every edit
or as a new mandatory approval ceremony for already-authorized routine work.

Reuse project commands and the declared testing authority. A project may record
an optional feedback selection there as existing command IDs, the question/risk
they cover and when broader checks are needed. No new manifest field, lifecycle
stage, CLI flag or required file is introduced. Without a selection, choose and
explain appropriate focused checks from the existing strategy; missing commands
or an empty selection are not a pass. Execute declared argv unchanged; disclose
any narrower diagnostic invocation and do not substitute it for its canonical
suite. This guidance does not change what `opdev check` executes.

For example, a UI hierarchy question needs a rendered treatment and relevant
interaction checks; a library parser change needs caller-visible boundary cases;
an API response change needs representative requests and regressions. A plugin
instruction adjustment may need one realistic behavior trial before wider
evaluation. None requires completing unrelated layers first. Keep purpose,
observations, limits and the next decision in existing work/conversation context,
not a feedback ledger. Feedback readiness is not milestone completion or a gate.

### Development previews

When feedback needs package bytes or a remote environment, use an authorized
development preview through the declared CI path. Bind observations to exact
source/configuration, artifact identity and environment; perform checks required
for safe preview use and applicable packaging, startup and recovery risks.
Installation or deployment must be within actual authorization and isolated from
live consumers/data as appropriate. A need for feedback does not authorize a
consumer migration, plugin-cache replacement, publication or new remote resource.
Use a representative local source path when packaging is not the question.

An unqualified development artifact may support a bounded trial, but preview
success is not integration or release qualification. Preserve existing required
CI and project policies. If they prevent a useful preview, propose a reviewed
policy change; do not bypass them with skip-CI directives, allowed failures,
draft status or a green subset of jobs. A preview-only pipeline must not satisfy
merge requirements; establish that boundary before changing provider scheduling.
Separate producer and qualification checks only under reviewed provider policy.

### Ready to merge and ready to release

Once a small useful increment is sufficiently settled, finalize exact-source
acceptance bindings/review and run required local/pre-merge qualification, then
required checks on integrated trunk. Material changes invalidate affected evidence;
feedback observations or older green runs cannot qualify new source or artifacts.
Retain applicable distinct environments and supported configurations. Reuse
execution only through the supported reviewed mechanism, not informal receipts.

Do not wait for the whole product's final design or keep indefinite feedback
branches. Integrate coherent useful slices frequently; replan if feedback stalls,
and use the experiment guidance for safely isolated unfinished behavior on trunk.
Ready to release is a separate checkpoint requiring explicit authorization and
applicable delivery qualification. None of these checkpoints adds an automatic
approval round. Reuse current context; reread authorities when facts change.

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

For new pipelines and improvements to existing ones, load
[CI design and improvement](ci-design.md). It supplies evidence-led design
choices and provider failure checks without a mandatory audit on every task.

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

After acceptance checks and meaningful feedback, reassess the next useful action:
continue within authorized scope, revise, stop at its boundary, or take a bounded
enabling step. Do not execute optional or speculative roadmap items without
authority, or ask again for each already-authorized milestone. Keep durable contracts/design rationale
in their existing authorities, current sequencing in the work tracker, and
project commands in the project contract. Assess all adoption practices, but
sequence their implementation around useful increments; unresolved adoption
remains incomplete rather than blocking all learning or becoming a hidden pass.
