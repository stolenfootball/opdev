# Resumable, evidence-driven workflow redesign

Status: design proposal, consolidated 2026-10-05. This records the redesign
discussed with the developer; it does not implement or activate it. Existing
versioned specifications, rules, schemas, commands and project decisions remain
authoritative until their individual changes are reviewed and delivered. Command
names and record fields below describe capabilities, not a supported new API.

This document is the architecture and rationale authority for the proposal.
Implementation sequencing, milestones, task-specific acceptance, open work and
progress belong in the [GitLab redesign roadmap](https://gitlab.com/stolenfootball-tools/opdev/-/work_items/62)
and its linked issues. They are not duplicated here. This write-up does not
authorize implementation, provider changes, release, consumer migration or
installed-plugin updates.

## 1. Intended outcome

A developer can give OpDev a software-development objective, obtain useful
feedback quickly, and resume work across sessions without repeating valid work
or settled decisions. OpDev still establishes meaningful acceptance, integration
and delivery evidence for the exact subject being qualified.

The change is from an agent repeatedly reconstructing and following a procedure
to a small deterministic core that explains what is known, what is missing, and
which actions can advance the requested outcome. Agents supply investigation,
implementation and semantic judgment; the core manages identities, dependencies,
validation and gate aggregation.

The design applies to general software: applications, libraries, services, CLIs,
embedded software and other deliverables. It does not require a web interface,
particular language, test reporter, deployment topology or agent vendor.

Success means less unnecessary execution, evidence maintenance and questioning
without increased false qualification or missed requirements. Research supports
the direction, not a measured OpDev speedup. Context reduction, total token usage,
elapsed time, correctness and user effort are separate outcomes.

## 2. Why the current design needs more than wording changes

The current system already has useful foundations: canonical command vectors,
stage-specific gates, acceptance mappings, source fingerprints, provider audits,
execution previews, evidence preparation and proportional planning guidance.
Keep these capabilities and their tested boundaries.

The remaining structural constraints are:

- [Acceptance evaluation](evidence-ledger.md) requires a mapped suite to run
  inside the current check. [Saved execution receipts](test-reports.md) are
  diagnostic only. A previously successful CI job cannot simply supply that
  execution evidence to a later evaluator.
- Whole-tree binding is deliberately conservative. It has little ability to
  distinguish the lifetime of an execution result, assertion review, developer
  decision and artifact qualification.
- Agents reconstruct progress and approval scope from multiple sources. A new
  context can repeat questions, investigation or qualification unnecessarily.
- Guidance distinguishes feedback, integration, candidates and publication, but
  the agent still carries much of the work of selecting the next useful action.
- Delegation lacks an OpDev-specific, shared task/result boundary. Additional
  agents could therefore multiply the same research, edits and full-suite runs.

The ledger is already excluded from the staged source fingerprint. This is not
a repair for every ledger edit changing that fingerprint. The separate acceptance
digest still binds its payload; semantic review and execution need distinct
identities and invalidation rules.

Some delays are necessary work: real defect repair, distinct native-platform
testing, sustained trials and external queues. This redesign targets avoidable
overhead, not a promise to eliminate those costs.

## 3. Guarantees and boundaries retained

1. Accepted requirements precede implementation. Assertion adequacy, boundary
   cases and regression protection remain substantive review obligations; green
   suites, coverage numbers and agent agreement do not prove them.
2. Required pre-merge and post-merge verification remain distinct. An unchanged
   command line does not make their subjects or purposes equivalent.
3. CI remains the exclusive software-delivery path. Build once, identify immutable
   artifacts, and promote the qualified bytes. Apply production-like checks and
   tested recovery appropriate to the software.
4. Preserve one integration trunk, delivered behavior and red-trunk restoration
   priority. Catalog 2's daily-integration finding remains visible for compliance,
   not a newly introduced one-day merge blocker or exception workflow.
5. Preserve `passed`, `failed`, `unverified`, `not_applicable`, `error` and
   `migration_required`. Only a pass or justified non-applicability satisfies an
   applicable required rule. Workflow activity labels are not new rule outcomes.
6. Requirements, evidence authenticity and semantic completeness are not proven
   by hashes alone. Explicit uncertainty stays visible; extensions cannot waive
   core requirements or turn unknown evidence into a pass.
7. User approval, agent review, execution permission, observed feedback and
   publication authorization are different facts. No agent can manufacture one
   from another. Existing host permissions and approval boundaries remain intact.
8. Keep adoption opt-in for uninitialized projects and seamless use for initialized
   projects. Routine operations and unrelated infrastructure do not activate the
   development lifecycle. Do not restart adoption during ordinary work.
9. Keep root `AGENTS.md` and `CLAUDE.md`, including reliable discovery guidance.
   Preserve project-owned content, authority locations and adequate tooling.
10. Separate integration from experimental activation, and activation from
    shipping. Existing compact-output experiments remain explicitly opt-in; the
    redesign does not silently enable or establish their effectiveness.

MinimumCD requires automated tests before merge and after integration and assigns
the releasability decision to the pipeline. The proposal changes how execution
evidence reaches that decision, not those obligations. A passing operational gate
is still not a blanket compliance claim. [MinimumCD](https://minimumcd.org/)

## 4. Architecture: execute, evaluate, continue

Keep the Rust CLI as the portable deterministic core and use thin native host
adapters. Do not add a daemon, hosted coordination service, mandatory database,
general agent framework or replacement CI scheduler.

| Component | Responsibility | Does not establish |
| --- | --- | --- |
| Planner/preview | Resolve the declared checks, stages, inputs and dependencies | Execution, consent or qualification |
| Executor | Run an authorized canonical command once and record the attempt | Requirement adequacy or another stage's result |
| Evidence validator | Check record shape, subject, provenance, applicability and freshness | Truth of arbitrary semantic claims |
| Evaluator | Aggregate valid evidence and required rules for a specified subject/stage | Permission to merge or publish |
| Continuation projection | Explain completed, missing, stale and eligible work | A new source of policy or autonomous authority |
| Controller and optional specialists | Investigate, implement, review and coordinate within scope | A substitute for CLI validation or actual developer decisions |
| CI/host adapters | Map native capabilities to the shared contracts | A provider-specific exemption from core behavior |

The familiar check entry point should remain convenient: plan, validate available
evidence, execute authorized missing checks, then evaluate. Also provide explicit
evaluation-only and read-only continuation capabilities. Neither may silently
run commands, contact providers or repair files. Exact command names and exit
contracts belong in the versioned interfaces when the capabilities are adopted;
this proposal does not establish them.

Perform cheap structural, capability and subject validation before expensive
qualification. Report all useful discoverable problems together rather than one
new paperwork error after each long run. A separate focused-feedback path may
still run while acceptance is incomplete; its result cannot qualify integration.

OpenSpec's dependency-informed actions and LangGraph's saved progress support
this direction. Adopt their principles, not their runtimes or permissive archival
semantics. A workflow checkpoint by itself is not trustworthy test evidence.
[OpenSpec workflows](https://raw.githubusercontent.com/Fission-AI/OpenSpec/main/docs/workflows.md),
[LangGraph checkpointers](https://docs.langchain.com/oss/python/langgraph/checkpointers)

## 5. Shared records and ownership

Define narrow, versioned schemas shared by both agent hosts. Developers should
not invent a per-project orchestration format. Generate mechanical fields from
existing contracts and observations; do not require hand-maintained copies of
the same facts in several files.

These are logical record types, not a requirement for six new files:

| Record | Minimum meaning |
| --- | --- |
| Work reference | Accepted outcome, scope, condition references, dependencies and original authority |
| Subject | Exact source/input identity, applicable configuration, stage and artifact/environment identities |
| Execution attempt | Check identity, canonical command binding, executor/version, run/job/attempt identity, observed start/end, result and retained output reference |
| Acceptance review | Condition inventory, assertion mappings, reviewer identity, source references, review subject and limits |
| Decision/observation | Actual source, kind, approved or observed subject, scope, limits and any supersession/revocation |
| Delegation | Bounded assignment, owner, source subject, permitted work, effort policy and structured result |

Derived readiness joins these records; it is not another manually asserted pass.
Records identify their schema and producing protocol version. Unknown critical
fields or unsupported semantics cannot be ignored to obtain qualification.

### Storage and authority

- Project policy and commands remain in the project contract; requirements,
  design decisions and work status remain in their existing routed authorities.
- Extend existing evidence structures where suitable. Store execution records
  with private CI artifacts or managed local execution storage, with content
  identity and adequate retention. Do not commit large logs into the repository.
- Place any necessary OpDev-owned durable metadata under `.opdev/`, without
  repurposing project `docs/` or introducing root PLAN/TODO/STATUS files.
- Derived local indexes and continuation caches are disposable and rebuildable.
  Actual approval provenance and required evidence cannot exist only in such a
  cache. A reference to deleted evidence does not remain qualifying.
- Store attributed decision content or a durable accessible reference sufficient
  to establish scope. Do not require the entire chat transcript, copy private
  personal context into public work items, or claim an agent-authored record
  authenticates its alleged human author.
- Use atomic updates and optimistic concurrency checks. A stale controller or
  worker cannot overwrite newer decisions, subjects or evidence. Preserve history
  and supersession rather than editing old attempts into successes.

The shared protocol specifies logical ownership without requiring a particular
on-disk layout here. Concrete storage and schema contracts must preserve these
boundaries with minimal generated storage, not a second project-management system.

### Document placement before writing

The existing [documentation ownership policy](documentation-layout.md) already
separates durable knowledge from live work. The proposed safeguard makes that
decision explicit before an agent creates or materially expands a project
document; a folder name or a document's title is not sufficient classification.

Resolve four facts from the request and the current project contract:

1. Whether the content is durable knowledge, implementation tracking, temporary
   investigation, or a mixture.
2. Which declared authority owns it, including any relevant context route.
3. Whether an adequate existing document or work item can be reused.
4. Whether the requested action authorizes writing to that destination.

| Content | Owning authority | Placement behavior |
| --- | --- | --- |
| Behavior, interfaces, enduring rationale, operational guidance | Declared design/documentation authority | Extend suitable existing material; create a document only when justified |
| Milestones, sequencing, task-specific acceptance, open implementation questions, progress | Declared work authority | Update the existing work item or an authorized new one |
| Temporary investigation or an unapproved proposal | Conversation or suitable disposable context | No mandatory registry or permanent planning file |
| Mixed durable design and implementation plan | Both appropriate authorities | Split the content and cross-link instead of duplicating live status |

The rule follows declared ownership, not a universal GitLab requirement. An
explicit repository-based work tracker is valid. Preserve custom locations,
project-owned files and adequate existing structure; do not infer ownership from
`docs/`, `.opdev/`, or a PLAN/TODO/STATUS filename alone.

If a required tracker is unavailable or the write is unauthorized, retain the
proposal in the conversation and explain the missing action. Do not quietly
substitute a repository backlog. Resolve genuine authority ambiguity with a
focused question, without restarting adoption or repeatedly asking settled
placement decisions. Inspection or planning alone cannot authorize a write.

Use deterministic validation for declared authorities, destination types and
conflicts, plus a focused semantic classification in the agent workflow. Neither
hashes nor keyword matching prove document purpose: a permanent migration guide
can legitimately contain phases and plans. Do not introduce a keyword-based CI
blocker, a mandatory reviewer agent, a classification file for every document,
or an extra approval round for every paragraph.

Both host integrations share this boundary and discover it in fresh contexts.
The check is prospective, not authority to reorganize existing projects. For an
authorized relocation, verify that the destination preserved the content before
removing the source and update the links. The implementation and regression work
belongs in the linked roadmap, not a new checklist in this design.

## 6. Trustworthy execution reuse

### First supported boundary

Begin with completed producer checks in the same explicitly identified, reviewed
CI pipeline/run and applicable stage. Bind results to exact inputs and execution
context. Use a canonical-execution wrapper or equivalent reviewed integration
that preserves argv, working directory, timeout and failure propagation without
requiring JUnit or a language-specific adapter.

Validate all of the following before a result contributes to a gate:

- Repository, revision/input identity and work subject.
- Canonical check identity, command configuration and selected stage; distinct
  declared checks remain distinct even if their argv match.
- Effective CI configuration and trusted workflow/producer/job identity.
- Run and attempt identity, complete required producer inventory and retry state.
- Relevant environment, architecture, toolchain, dependencies and configuration.
- Immutable artifact identity when the evidence concerns built package bytes.
- Applicable freshness and absence of observed mutation or conflicting results.

A file containing these fields is not authenticated merely because it has a
digest. The provider adapter must bind the record to the expected producer and
artifact, and the reviewed producer configuration must establish what it ran.
Keep the evidence channel outside locations test code can casually overwrite.
Document the remaining trust in project-controlled code, CI administrators and
the executor; do not imply resistance to a compromised trusted producer.

Record only safe, relevant environment metadata. Do not dump environment
variables or publish secret-derived hashes. Use protected references for
sensitive configuration identity. Unknown relevant inputs mean no reuse; they
do not justify collecting all secrets or guessing equivalence.

### Evaluation and continuation

1. Load the requested subject, accepted conditions and reviewed policy.
2. Validate available execution and review records independently.
3. Preserve known failures and identify missing, stale or incompatible evidence.
4. Execute only authorized missing work, or report the exact prerequisite.
5. Evaluate the selected gate from the resulting evidence set.
6. Explain which results were executed, consumed or rejected, and why.

An evaluator running inside CI can consume completed required producer jobs while
its own job is still running. It must not require its own successful completion
as an input or claim the whole pipeline has already passed. Post-run remote
qualification still observes completed required jobs, policy and current branch
state; continuing a candidate cannot bypass those requirements.

A later review-only correction can permit reevaluation without producer reruns
only when execution inputs truly remain unchanged and the supported run boundary
still holds. If a command reads the changed review metadata, that metadata is an
execution input. A new commit or new pipeline is outside the initial same-run
reuse scope, even when the change looks administrative.

### Deliberate exclusions and failure behavior

- No cross-commit cache, arbitrary historical-run selection or same-SHA shortcut
  in the first increment. Local receipts remain observations until a separately
  specified local trust boundary is implemented.
- No reuse across required pre-merge/post-merge stages, distinct platforms or
  supported configurations merely because commands or code are similar.
- No success-filtered selection or fallback from newer failed, pending or
  incomplete evidence to an older green result. Preserve all observed attempts;
  visible successful retries do not erase failures or bypass flake policy.
- Missing, expired, ambiguous or unsupported provenance remains unverified;
  observed mismatches/failures and verifier errors retain their appropriate
  existing outcomes. Do not silently downgrade to weaker verification.
- A passed suite still does not establish complete test selection, adequate
  assertions or real user effectiveness. Those obligations remain separate.
- Legacy receipts and report inspections are not reinterpreted as qualifying
  evidence. Introduce a new explicitly versioned trust contract.

Bazel's documented cache-input pitfalls are a useful caution: undeclared
environment and toolchain inputs can make apparently matching results unsafe.
SLSA's separation of build definition and run details informs identity design;
using that idea does not claim SLSA conformance or mandate its tooling.
[Bazel caching](https://bazel.build/remote/caching),
[SLSA provenance](https://slsa.dev/spec/v1.1/provenance)

## 7. Evidence lifetime and selective invalidation

Separate what each record describes from when it was collected:

| Evidence | Invalidation basis |
| --- | --- |
| Assertion adequacy review | Changed requirements, assertions, relevant implementation or review scope |
| Execution result | Changed execution inputs, environment, stage, trust or freshness |
| Artifact qualification | Changed artifact bytes, material configuration/environment or required freshness |
| Developer decision | Exceeded approved scope, changed decision subject, expiry, supersession or revocation |
| Current remote readiness | Changed branch, latest required run, attempt, producer or protection policy |

Whole-tree source binding remains the safe fallback. A later increment may
support narrower identities based on reviewed explicit dependencies or immutable
artifact subjects. Do not introduce universal path-exclusion heuristics or let an
agent infer that an unrelated-looking file can never affect a check.

The overall acceptance inventory must still be reviewed for omissions and newly
introduced requirements. Reusing some unchanged mappings cannot cause new
requirements to disappear from the aggregate review. Changed subjects invalidate
only records proven dependent on them; unknown dependency scope invalidates
conservatively. Historical results remain historical rather than being rewritten.

Report the reason in ordinary language, for example: "The Linux result is still
valid. The Windows result must be repeated because its compiler configuration
changed." Do not promise that conclusion until the dependency relationship is
actually established.

## 8. Resumable progress, decisions and clear completion

Keep execution state, evidence validity, gate outcome and authorization separate.
For example, a check can have completed successfully but no longer be valid for
the current source. A valid integration gate does not authorize publication.

The continuation view explains:

- The current requested outcome and exact subject.
- Valid completed work, with evidence references.
- Missing or stale evidence and the concrete cause.
- The next eligible action, its dependencies and any required developer decision.
- What changed since the previous observation and what must be rechecked.

On resume, reconstruct from durable authorities, verify identities and refresh
time-sensitive facts. Do not rerun adoption, reread every authority or restart
completed investigation without a relevant change. Missing historical context
is uncertainty to resolve, not an excuse to invent progress or consent.

Use typed decisions: policy choice, bounded delegation, implementation approval,
execution permission, observed user feedback and release authorization. Link each
to its real source and scope. Ask a focused follow-up when an answer is ambiguous;
do not repeatedly request a settled decision. Required human observation remains
pending until it was actually supplied, even if the user approved performing it.

There is no single undifferentiated "done" flag. Report implementation, integration,
artifact qualification, feedback and release request separately, using the existing
rule semantics beneath the view. Example, not an actual project result:

> Implementation and required integration checks are complete for revision A.
> The packaged candidate has not been checked on Windows. Next: run that check
> against artifact B. No release was requested.

Diagnostics explain the unmet condition, relevant suite/stage, observed fact and
smallest next step before showing internal rule IDs. Distinguish not attempted,
waiting, denied, failed and unavailable observations. Never hide missing evidence
behind a summary success statement. New views have an explicit schema and do not
silently change the existing experimental compact-report contract.

## 9. Optional controller and specialist agents

### Execution strategy, not a new gate

Support one accountable controller with a small pool of optional specialists.
The controller may do small tasks directly. Delegation should isolate substantial
noisy context, enable genuinely independent work or obtain useful fresh review.
It is not a required step for every change and does not itself establish quality.

Start with reusable responsibilities, not a permanent team:

| Role | Bounded responsibility | Initial effort policy to evaluate |
| --- | --- | --- |
| Controller | Scope, sequencing, user decisions, delegation, reconciliation and integration | Medium; higher for consequential uncertainty |
| Investigator | A specific research, code-navigation or diagnostic question | Low for lookup; higher for complex diagnosis |
| Implementer | One coherent change, including meaningful tests | Medium; higher for difficult or risky work |
| Acceptance reviewer | Original requirements versus actual behavior, assertions and omissions | Medium/high according to risk |
| CI/evidence analyst | Interpret actual runs, missing evidence and safe continuation | Deterministic tools first; low for extraction, higher for judgment |

Effort is selected by uncertainty and consequence, not job title alone. Extracting
a result is different from deciding whether it proves acceptance. Prefer CLI
operations for hashing, validation, command execution and aggregation instead of
spawning an agent to perform mechanical work.

Roles divide reasoning responsibilities, not the product into whole technical
layers. An implementer delivers an end-to-end increment with tests; do not create
a backend team followed by a testing team and postpone useful feedback again.

### Delegation contract

Every assignment carries these facts in a shared record or structured host
message, not a new Markdown file per worker:

- Assignment ID, parent work reference, question/outcome and stopping condition.
- Exact subject/revision and relevant original authorities, constraints and
  settled decisions; do not pass only an implementation-biased summary.
- Allowed actions, owned files/resources and explicit exclusions.
- Expected checks, result shape and evidence locations.
- Requested and observed model/effort when available, budget and escalation rule.

Every return identifies the inspected subject, work actually performed, findings
with evidence references, changed files, checks run/not run, uncertainty and
remaining decisions. Assignment completion is not project completion. Preserve
failed/interrupted attempts; an incomplete response is not a successful review.

Keep bulky logs and exploration in the worker context with inspectable references.
Pass the smallest sufficient context while preserving applicable instructions.
Full-history forks can be useful when background is inseparable; they should not
be the automatic answer to every task. Context saved in the controller is not
the same as total tokens saved across the team.

### Ownership, independence and stopping

- The controller owns user-facing questions and canonical workflow-state writes.
  Workers return proposals; they do not independently expand scope, select policy,
  close work items, merge or publish. An assignment cannot grant absent authority.
- Start with parallel read-only investigation/review. Use one writer per shared
  area; concurrent writers need separable ownership or isolated worktrees bound
  to the intended base. Integration still needs its own checks.
- Review a fixed subject. The acceptance reviewer gets original conditions and
  actual assertions, can report omissions and is not told to approve the author's
  conclusions. A source change can invalidate the review.
- The controller verifies actionable findings and reconciles references without
  blindly trusting summaries or repeating the entire investigation. The CLI still
  performs deterministic validation. Agreement between agents is not proof.
- Keep a shallow controller-worker topology by default, with a small configurable
  concurrency cap and no automatic recursive delegation. Reuse a worker for a
  coherent follow-up; retire it when its bounded task is complete.
- Coordinate focused checks and canonical qualification centrally. Do not run
  the full suite once per worker or create an issue, branch and approval round
  merely because a worker was spawned.
- Distinguish defects from optional improvements. Real acceptance failures need
  repair; discretionary polish must not continually expand the finish line.
  Budget exhaustion yields a checkpoint, narrower proposal or decision request,
  never a pass or a reason to abandon still-required evidence silently.

### Host portability and developer control

Use host-native subagents through thin adapters, not a mandatory external agent
SDK. Share role instructions and delegation semantics. Discover supported host
features, permissions, context inheritance, model choices and effort levels.
Current official documentation describes configurable subagents for both hosts;
their exact capabilities are version-dependent.
[Codex subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents),
[Claude Code subagents](https://code.claude.com/docs/en/sub-agents)

Offer a single-agent strategy and an explicitly enabled bounded-delegation
strategy. Reuse an approved preference; do not ask on every task. Apply existing
user/host model constraints, record actual settings and disclose unavailable
capabilities. Do not silently switch models or broaden permissions. Capability
limits must leave the ordinary single-agent workflow usable.

Anthropic's controller-worker research supports bounded delegation but also
reports substantial token costs and difficulties with tightly coupled work.
Its harness research found intermediate ceremonies could become unnecessary
while targeted evaluation still caught meaningful defects. These support testing
selective delegation, not claiming that more agents or lower effort is better.
[Multi-agent research](https://www.anthropic.com/engineering/multi-agent-research-system),
[Harness design](https://www.anthropic.com/engineering/harness-design-long-running-apps)

## 10. Developer workflow and CI behavior

### The ordinary loop

1. Understand the requested outcome and reuse current project decisions. Research
   only consequential unknowns with a question and an effort/scope bound.
2. Select the next small demonstrable increment or bounded enabling step. Keep
   later increments provisional and preserve explicit user scope.
3. Implement with focused checks. Disposable local exploration retains its
   existing boundary; retained work remains subject to normal integration gates.
4. Review actual acceptance coverage once the intended change is sufficiently
   settled. Prepare bindings mechanically, but never generate semantic approval.
5. Execute or consume required evidence under the supported trust contract.
   A failed gate explains the missing action instead of restarting the lifecycle.
6. Integrate through required CI, verify the integrated subject, and obtain a
   candidate or user feedback when the outcome requires it.
7. Stop at the requested completion boundary. Publication remains a separate
   explicitly authorized action, not the default ending of every milestone.

These are dependent capabilities, not mandatory agent-to-agent handoffs. A routine
change may need one agent and a short work item. More substantial work may benefit
from investigation or independent review without introducing new ceremonies.

### First-class CI providers

Keep GitHub Actions and GitLab CI first-class behind a provider-neutral evidence
interface. Extensions/additional providers must establish equivalent facts and
cannot weaken core requirements. Unknown provider behavior stays explicit.

Use the existing run and its downstream artifact dependencies when they already
support the authorized continuation. Do not dispatch a duplicate API pipeline
merely to reach candidate packaging. Build once and pass the same artifact to
qualification and subsequent authorized delivery. An overall green pipeline is
insufficient if an explicitly required trial was skipped, optional or failed.

Review CI bottlenecks when relevant: duplicated triggers, redundant qualification,
incorrect cache boundaries, missing early native feedback and cancellation of
long required trials. Separate observed duplication from hypotheses and queue
latency from runner cost. Preserve native/configuration coverage and demonstrate
equivalence before suggesting removal of checks.

Optimization recommendations remain advisory. Do not rewrite custom YAML, change
provider settings, provision runners, cancel work or create tracker items without
applicable authorization. A developer may keep an inefficient compliant setup
without an exception record or failed gate. Do not ask again absent material new
evidence or explicit reconsideration.

Use bounded diagnosis for repeated failures: identify the failing layer, reproduce
the smallest useful case and test a specific hypothesis. Use early authorized
native CI diagnostics when local checks cannot exercise the failure. Preserve
failed attempts. Do not equate legitimate long-running work or unchanged polling
with an agent loop, or impose a universal retry-count failure threshold.

## 11. Adoption, migration and compatibility

Existing 0.x consumers require an explicit migration even before 1.0. Introduce
new semantic contracts with versioned schemas and capability negotiation, updating
the [compatibility policy](compatibility.md) with each affected increment.

- Preserve readable historical ledgers and reports with their original meanings.
  Never relabel old diagnostic receipts as new qualified records.
- Preview required manifest, evidence, CI and managed-guidance changes. Preserve
  unrelated fields, existing authorities, tools, branch names and developer choices.
- Surface new trust, retention, freshness and delegation choices proportionately.
  Reuse adequate decisions; no blanket re-adoption or automatic policy selection.
- Detect local/plugin/CI version disagreement before relying on a new capability.
  Refuse unsupported rewrites and explain a compatible path without inventing a pass.
- Permit explicit rollback to the previous execution path, with required checks
  rerun where evidence cannot be interpreted. Preserve history; rollback must not
  obtain a false pass through weaker interpretation.
- Keep the older same-invocation path usable during a reviewed transition. Activate
  new qualification semantics only with compatible consumers and reviewed policy.
- Keep release versions, managed CLI pins and installed consumers unchanged until
  a separately authorized and qualified release/migration.

Do not hard-code model vendors, ecosystem tools or specialist files into every
consumer project. Shared defaults should reduce setup; material project choices
remain explicit. The standard schema is an interoperability boundary, not a demand
for developers to fill out a large new questionnaire.

## 12. Assurance and effectiveness semantics

Verification covers actual command counts, behavior and filesystem/provider side
effects, not just phrases in output. Mechanical schema validation, semantic
assertion review and observed agent behavior establish different facts. Green
parsing or execution cannot prove omitted requirements, authentic human approval
or producer trust. Red/green regressions and bounded mutation cases are useful
assertion evidence where applicable, not universal scores or substitute gates.

The existing [evaluation contract](token-efficiency.md) remains the measurement
authority. Comparisons need frozen identities, inputs, models, effort, host
versions, independent acceptance oracles and disclosed scope. Separate the effects
of the core redesign, delegation and effort routing instead of attributing a
combined change to one mechanism. Historical compact-output results are not a
baseline that can be pooled indiscriminately with a new experiment.

Useful measures remain distinct:

- Time to first useful feedback and total time to the independently accepted outcome.
- Agent-active time, command duration and CI queue delay separately; overlapping
  intervals are not additive by default.
- Check counts, repeated qualification, evidence rewrites and repair rounds.
- Repeated or unnecessary questions, missed decisions and manual setup burden.
- Controller and every worker's observed usage, including failed attempts,
  cache details and compactions where available.
- Acceptance defects, false completion/qualification claims and preserved tests.

Missing usage is unknown, not zero; a small controller transcript is not proof of
lower total consumption. Record requested versus actual model/effort. Preserve
capacity failures, interruptions and authorized later retries separately. Do not
switch models or retry until a favorable result appears. Live trials remain
opt-in and outside required deterministic CI.

Independent evaluation inspects behavior, test preservation, assertion adequacy,
actual gates and side effects; it does not accept the agent's completion claim.
An effectiveness claim needs a prespecified comparison criterion, repeated
balanced observations and disclosure of regressions and false qualifications.
Small-task regressions cannot be hidden by gains on complex tasks. Medians,
variation and tail latency are more informative than a best run; a small sample
cannot prove universal safety or speed. Concrete trial matrices, schedules,
threshold decisions and results belong in the work authority.

Publish only reviewed neutral fixtures and aggregate findings. Keep private
consumer details, raw transcripts, credentials and personal context out of public
repositories and reports. This design does not authorize live canary creation.

## 13. Alternatives, risks and reversal triggers

| Alternative | Decision and rationale |
| --- | --- |
| More instructions only | Insufficient for execution/evaluation coupling and durable state; guidance still matters |
| Weaker tests or a universal skip switch | Rejected: changes assurance rather than removing redundant work |
| General cross-revision cache or build system | Deferred: input completeness and maintenance exceed the first useful boundary |
| Agent for every rule or phase | Rejected as default: duplicates context, execution and approval work |
| Mandatory external orchestration framework | Rejected: unnecessary deployment, host coupling and maintenance |
| Single-agent only | Remains supported; may be best for small or tightly coupled tasks |
| New test-producer/JUnit adapter ecosystem | Not required; canonical execution remains tool-neutral |
| Automatic CI optimizer | Rejected: developers retain policy and project ownership |

The principal risks are unsafe reuse, stale or falsely attributed decisions,
concurrent writers, excessive schema maintenance, reviewer false positives and
coordination overhead that exceeds its benefit. Bounds, versioning, source-bound
review, explicit provenance limits, conservative fallback and complete-session
evaluation address these risks without claiming to eliminate them.

Reverse or narrow a capability if trials show false qualification, repeated
missed requirements, loss of developer control, materially worse routine-task
overhead or unverifiable worker accounting. A context-management benefit may
justify some additional tokens, but report and accept that tradeoff explicitly;
do not label it token savings. A stalled worker should not restart valid work by
other workers, and an inconclusive trial should not become a production claim.

Unresolved transport, input completeness, storage or host guarantees are limits
on qualification, not facts supplied by this proposal. The linked work authority
owns their bounded investigation. Dependent qualification cannot rely on an
unresolved trust assumption.

## 14. Research lessons retained

The architecture above is an OpDev proposal informed by these sources, not a
claim that another framework validates its performance:

- [Aider lint/test loop](https://aider.chat/docs/usage/lint-test.html): obtain
  focused feedback close to edits; this is not permission to skip handoff checks.
- [Superpowers plan execution](https://raw.githubusercontent.com/obra/superpowers/main/skills/executing-plans/SKILL.md)
  and [systematic debugging](https://raw.githubusercontent.com/obra/superpowers/main/skills/systematic-debugging/SKILL.md):
  bound execution and investigate causes rather than repeatedly trying broad
  fixes. Do not adopt fixed retry thresholds or external permission policies.
- [OpenHands stuck detection](https://docs.openhands.dev/sdk/guides/agent-stuck-detector):
  repeated behavior is a useful diagnostic signal, not sufficient proof that
  legitimate polling or long execution should be interrupted.
- [Spec Kit decomposition](https://github.github.com/spec-kit/concepts/spec-of-specs.html):
  decomposition itself carries overhead; prefer the lightest structure that keeps
  outcomes and dependencies clear.
- OpenSpec, LangGraph, Anthropic, Codex, Claude Code, Bazel, SLSA and MinimumCD
  are linked at the design choices they inform above. Revalidate host/provider
  facts when implementing; do not turn today's documentation into permanent
  assumptions about installed capabilities.
