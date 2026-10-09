---
name: opdev
description: Follow OpDev for project software development or explicit adoption. Offer adoption early for substantive work in uninitialized projects. External infrastructure operations and routine pull/Git-status/dev-server tasks need no activation. Retained work can reach feedback with focused checks; required gates apply at integration. Loading this skill checks applicability, not consent.
---

# OpDev

Report actions from execution evidence, not from plans or permission assumptions.
Distinguish not attempted, expected to require approval, attempted and denied,
and executed with an observed result. A denial for one invocation proves nothing
about unattempted commands. Before progress or final summaries, reconcile claimed
actions and blockers with the actual tool record; keep unknown results unknown.
Use [reporting evidence](references/results.md#reporting-actions-and-blockers)
when a command is blocked, fails, or has an uncertain result. These are action
descriptions, not new OpDev rule outcomes or an extra project ledger.

Apply the workflow only after the project-state and consent gate below. Loading this skill, installing the plugin, or a host requiring skill inspection does not authorize following OpDev in an uninitialized project. Do not announce "using OpDev" merely because this skill was loaded.

## Establish state

First identify the requested task scope, then the target repository (not an unrelated current workspace). External host, runner, storage, network or account maintenance alone needs ordinary operations guidance, even when the resource serves a configured project's CI. Consulting job names does not activate the development lifecycle. Activate at the point work changes project-owned software, repository configuration, contracts or delivery definitions. Mixed tasks may use both scopes; do not substitute development evidence for operational verification.

Interpret a progress question in the current assignment before choosing a
status-only path. If an actual developer-authorized implementation assignment is
unfinished, answer the status question and follow [scoped continuation](references/workflow.md#scoped-continuation)
within that existing scope. A standalone status/review question grants no new
implementation authority. Unfinished backlog items or an agent-authored note do
not establish a standing assignment; verify the original decision and honor any
later pause, replacement or narrowing. Do not ask to resume work that is already
authorized merely because the latest message asks how it is going.

For project work, look for `.opdev/project.yaml` at its Git root, including when working in a subdirectory or worktree. If Git is not initialized, inspect the target project directory without creating a repository. This applicability inspection does not require an OpDev CLI.

- **Configured project:** read the contract and project instructions and classify the work. For retained development, resolve the runtime and apply OpDev seamlessly. For isolated disposable local exploration, use the boundary below and load only relevant authorities; no production runtime/gate/evidence preparation is needed merely to obtain feedback. A malformed or unreadable contract is an error, not an uninitialized project. Configuration is not proof of complete adoption.
- **Explicit adoption request:** consent to assessment is already supplied. Resolve the runtime, then follow [adoption.md](references/adoption.md). This is not blanket permission for installation or other material changes.
- **Uninitialized, substantive development:** offer adoption once in the first response after minimal local inspection establishes applicability, before substantive research, planning, or edits. Implementing a supplied design in an otherwise empty folder qualifies. Reading that design, existing instructions and repository state is allowed first; do not defer the offer until after upstream browsing, dependency investigation, architecture selection or a completed plan. Wait for affirmative consent before applying the workflow, probing its runtime, or initializing. If declined or unanswered, continue the original task without OpDev; do not repeatedly ask in the same task or make adoption a prerequisite for ordinary work.
- **Uninitialized, routine repository operation:** handle the request directly without an OpDev announcement, suggestion, runtime lookup, installation, or gate. For example, "Pull down the most recent changes to the courses repo and start the dev server" needs ordinary repository safety and project instructions, not adoption. Inspecting status or running an existing command alone is also not a reason to adopt. If later work becomes substantive development, reassess then.
- **Unrelated task:** do not interrupt it with OpDev.

An explicit runtime-setup request uses the packaged setup skill without adopting any repository. An OpDev upgrade request follows [upgrades.md](references/upgrades.md), including read-only inspection of incompatible installations for repair; it does not authorize project adoption. For mixed requests, judge the actual development scope rather than matching words such as "repo" or "server".

Keep the offer short and neutral, for example: "OpDev is installed and this is a new software project. Would you like to use it for planning and development? Otherwise, I'll proceed normally." Use an available, permitted host question tool or plain chat; a missing question tool is not a blocker. Honor an earlier decline in the current conversation. The offer is optional for the developer, not optional for the agent when this branch applies. Do not create an adoption record or consent marker merely to remember an offer.

For the substantive-development branch, minimal pre-offer inspection means the
supplied design, existing instructions, contract presence and repository state.
Checking installed language runtimes or package managers to choose a stack is
implementation investigation: offer first, then continue that investigation
without OpDev if the user has not opted in. This ordering does not apply to
routine operations, explicit adoption, or an already-configured project.

## Resolve runtime after activation

For an explicit mockup, alternative layout or disposable feasibility probe, use
the [planning exploration boundary](references/planning.md#build-verify-learn-revise).
Keep prototypes isolated from retained source/configuration and live data. Use
existing work/conversation context for purpose, temporary location, actual checks
and discard-or-promote intent. Obtain feedback with the checks needed to execute
and safely assess the prototype; no production ledger, full suite, release docs
or exploration registry is required. Ask only if retention/distribution materially
changes the scope. Retained product edits are ordinary development even if called
a preview. Promotion brings retained code/dependencies through accepted conditions,
meaningful tests, current review bindings and integration gates. Distributed
previews use CI and applicable delivery checks. Exploration cannot satisfy a
production gate or authorize publication/merge; ordinary safety still applies.

Resolve the runtime below for retained development and applicable qualification.

For retained work, use the [ready-for-feedback checkpoint](references/planning.md#ready-for-feedback).
Show a meaningful result after focused tests and safety checks; do not require
full integration qualification merely to request feedback. Keep real edits and
tests current without calling them disposable. Finalize source-bound acceptance
and required canonical checks at integration handoff. Preview success does not
authorize merging, releasing or changing CI policy. Runtime resolution is not a
requirement to run every gate on each edit.

Before the first OpDev action in each task, resolve `../../` relative to this
skill directory to find the plugin root and select a CLI:

1. Prefer the plugin-managed CLI: run `sh <plugin-root>/scripts/runtime.sh --path`
   on macOS/Linux, or `powershell -NoProfile -ExecutionPolicy Bypass -File <plugin-root>/scripts/runtime.ps1
   -Mode Path` on Windows. This lookup has no network or installation side effects.
   The Windows override applies only to this child process and the installed,
   reviewed plugin script. Never change persistent execution policy or work around
   organization Group Policy. Policy denial is a lookup error, not runtime absence;
   report it and use an already verified compatible native CLI only with an explicit
   explanation of the selection.
2. Only if lookup exits 3 (no managed CLI), check whether `opdev` on PATH verifies against the
   packaged `opdev-compatibility.json`. A compatible standalone installation is
   sufficient; do not replace it. Any other lookup failure must be reported
   before proceeding; it is not evidence that no managed runtime exists.
3. If neither is available, inform the user and offer the packaged `setup` skill.
   Install only after setup is accepted (an explicit setup request suffices);
   use the host's normal execution/network approval flow. Do not bypass a denied
   approval, install for unrelated tasks, or initialize a repository as part of
   installation. Report setup failures and do not claim OpDev is active.
4. Run the selected executable's
   `plugin verify --contract <plugin-root>/opdev-compatibility.json`, resolving
   that package file to an absolute path. This is not `.opdev/project.yaml`:
   the project policy is not a plugin compatibility contract.
   A zero exit verifies runtime compatibility, not project consent. Exit 1 is an incompatible combination; exit
   2 is a verification error. Neither permits the workflow to proceed. Use that same executable
   for the rest of the task, including calls shown as `opdev` in the references.

Keep runtime setup separate from repository initialization. A damaged managed
runtime must be reported with its exact path; do not silently delete it or fall
back to another binary. The setup skill documents recovery.
If setup is unavailable or fails after activation, report the concrete failure and offer the manual choices in [initialization.md](references/initialization.md). Do not silently replace the configured project's process.

The Claude Code prompt hook may provide the same state as additional context. Treat that as a detection aid, not as a substitute for checking the project contract.

## Work in an initialized project

After activation/runtime resolution, use read-only `doctor` before substantial
implementation or verification in a new or changed execution environment, or
when diagnosing setup failures; do not rerun it for every turn or routine task.
Check `doctor --help` for `--format` and `--plugin-root` first. On capable CLIs,
pass the actual package root and use human/JSON findings to separate local
prerequisites, adoption gaps and qualification. Missing capabilities need an
explicit upgrade offer, not a fabricated pass or automatic installation. Older
doctor's success exit is not readiness. Use `--remote` only for relevant,
authorized provider inspection. Never execute suggested repairs automatically
or treat doctor as adoption consent, completed tests or gate evidence.

1. Read the root instructions and project contract. With explicitly selected layout 1, both hosts point to `.opdev/guidance.md`: read it fully and reload it after context reset. Missing guidance requires a repair offer, not a silently invented replacement. Legacy `CLAUDE.md` imports `AGENTS.md`; do not migrate it merely by loading this skill.
2. Load authorities selected by `context.always` and by every relevant task route. Follow [project-contract.md](references/project-contract.md) when interpreting fields. Do not assume design material belongs in `docs/`. Follow [documentation ownership](references/project-contract.md#documentation-locations-and-ownership) before choosing or changing authority locations.
3. Before creating or substantially expanding a document, classify its content as durable knowledge, work tracking, temporary investigation, or mixed; use the [placement check](references/project-contract.md#before-writing-a-document). Resolve its declared owner, inspect adequate existing material and apply existing write authorization. Split mixed design/roadmap content; unavailable or unauthorized tracker writes stay in the conversation, not a substitute repository backlog. Permanent operator steps and explicitly routed repository trackers remain valid.
4. Establish the outcome, scope, exclusions, acceptance conditions, risks, and evidence before substantive edits. For every planning objective, including "what is the next step?", follow [planning.md](references/planning.md): prefer demonstrable consumer increments, keep distant steps provisional, and honor specific user requests. Before committing to a substantial request, milestone approach or smaller high-risk change, apply its uncertainty checkpoint: reuse sufficient current evidence or investigate consequential unknowns proportionately. Scale design work to risk and reversibility.
5. Make small, reviewable changes. Preserve supported behavior unless the accepted change deliberately migrates it. Connect larger goals through useful phases and milestones without losing accepted later requirements. Continue authorized work through local blockers using the scoped-continuation boundary above; honor actual stop boundaries and do not treat feedback readiness as an automatic end to an implementation assignment.
6. Apply [testing.md](references/testing.md) and [acceptance evidence](references/acceptance.md) for substantive changes: derive expected results from accepted requirements, review actual assertions, and bind the inventory/mappings to the current change. Run canonical command argument vectors directly; do not reinterpret them through a shell.
7. For facts the CLI cannot infer, follow [evidence.md](references/evidence.md) for the selected storage policy. MR/PR storage 2 keeps current review in the existing change discussion and routine reports in bounded CI storage; never recreate its retired ledger or evidence archive. Legacy ledger projects still use schema-backed bootstrap with unresolved decisions. Never reuse an assertion after its source or authority changes without rechecking it.
8. Use focused checks for feedback and `opdev check` for required local qualification at integration handoff, not automatically on every edit or feedback turn. Use `opdev check --ci` in integration CI and `--remote` only when a read-only provider audit is relevant. Use full human/JSON output by default. Follow [results.md](references/results.md) for retained reports; compact views require explicit experimental opt-in from the user or project, never just a capable runtime. Report blocked or unavailable evidence honestly.
9. Reconcile implementation, tests, declared authorities, delivery behavior, and tracked work before completion. For new or substantially expanded documents, include the [focused content review](references/project-contract.md#review-document-content) in normal review; inspect actual claims, not just successful routing or the author's summary.

When CI design/changes or observed repeated waits make verification cost relevant,
use [CI design and improvement](references/ci-design.md) proactively. Recommend
evidence-backed options; do not optimize CI automatically. Developers may keep
their existing compliant setup, including deliberate extra checks.

Read [workflow.md](references/workflow.md) for the full lifecycle and gate behavior when planning or carrying out a substantive change.

Release-specific preparation and execution require explicit developer release
authorization under the [release boundary](references/workflow.md#release-authorization).
Milestone/implementation approval, preview acceptance and green CI do not grant
it. Recommend a release when useful; do not make one a prerequisite for ordinary
milestone completion. Relevant package/recovery regression tests still apply.

Use one agent by default. Before spawning or repurposing a worker, check the
[permission scope](references/delegation.md#permission-scope) against the actual
assignment. Standing permission for isolated evaluation agents does not enable
implementation, acceptance-review or CI specialists. Approval of the underlying
task is not permission to delegate it. Reuse permission within its original
limits; otherwise continue directly or ask before expanding delegation. When
enabled, use the bounded assignment and current host adapter. Do not create a
team, extra questions or per-worker full-suite runs merely because tools exist.

For a requested requirements-to-implementation consistency review (for example,
"does this satisfy the agreed plan?"), use
[consistency-review.md](references/consistency-review.md). It reuses declared
authorities and returns advisory findings without edits, tracker updates or gate
approval. Do not expand ordinary questions or narrow code reviews into a full
project audit.

For initialization, conversion to OpDev, or resolving incomplete adoption, read
[adoption.md](references/adoption.md). Assess every supplied practice, preserve
existing choices, research only project-specific gaps, and verify completion.
An explicit adoption request supplies consent to assess; do not ask again.
It does not authorize choosing policies for the developer. Present material
choices and wait for their response or explicit bounded delegation before
implementation. Follow the adoption reference's approval and completion gates.
Use its [decision review](references/decision-review.md) before requesting plan
approval: resolve material choices in relevant question rounds, verify inherited
consent, and keep implementation authorization separate from policy selection.

For experimental features or releases that must exclude unfinished behavior, read
[experiments.md](references/experiments.md). Keep stable behavior releasable and
record opt-in, ownership, review, tests, and cleanup at the existing work authority.

## Preserve the core

Read the project's actual policy before interpreting compliance. With explicit
project schema 3 and CLI capability `engineering.assessment.v1`, follow the
[engineering policy guidance](references/project-contract.md#engineering-policy).
Its mandatory engineering gates and separately selected MinimumCD assessment
are distinct. Do not infer migration from a new plugin/CLI, disable baseline
requirements by changing profiles, or repeat command execution for each assessment.
The catalog-2 rules below retain their meaning for unmigrated projects.

MinimumCD delivery and testing requirements remain mandatory. Daily integration is a monitored target, not an operational merge deadline in catalog 2: explain delays and replan without inventing a recovery exception. Missed or unverified cadence remains visible and prevents a compliance claim. Keep one trunk and short-lived branches; do not reintroduce a one-day timer under another rule. An older CLI may still block cadence; report its actual result and offer a compatible upgrade rather than bypassing it. Extensions may add or strengthen checks but cannot disable a core rule, change its applicability, replace its result, or suppress required evidence.

Use only these rule outcomes: `passed`, `failed`, `unverified`, `not_applicable`, `error`, and `migration_required`. Only `passed` and justified `not_applicable` satisfy a required rule. Never turn missing evidence, permission failure, tooling failure, or a known migration gap into a pass.
