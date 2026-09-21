---
name: opdev
description: Follow OpDev for configured projects or explicit adoption. For substantive software planning or implementation in uninitialized projects, including design-only folders, offer adoption early before research or planning; do not activate without consent. Routine pull, status, dev-server and unrelated requests need no offer. Loading this skill checks applicability, not consent.
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

First identify the target repository (not an unrelated current workspace) and look for `.opdev/project.yaml` at its Git root, including when working in a subdirectory or worktree. If Git is not initialized, inspect the target project directory without creating a repository. This applicability inspection does not require an OpDev CLI.

- **Configured project:** read the contract and project instructions, then resolve the runtime and apply OpDev seamlessly. A malformed or unreadable contract is an error, not an uninitialized project. Configuration is not proof of complete adoption.
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

Before the first OpDev action in each task, resolve `../../` relative to this
skill directory to find the plugin root and select a CLI:

1. Prefer the plugin-managed CLI: run `sh <plugin-root>/scripts/runtime.sh --path`
   on macOS/Linux, or `powershell -NoProfile -File <plugin-root>/scripts/runtime.ps1
   -Mode Path` on Windows. This lookup has no network or installation side effects.
2. Only if lookup exits 3 (no managed CLI), check whether `opdev` on PATH verifies against the
   packaged `opdev-compatibility.json`. A compatible standalone installation is
   sufficient; do not replace it. Any other lookup failure must be reported
   before proceeding; it is not evidence that no managed runtime exists.
3. If neither is available, inform the user and offer the packaged `setup` skill.
   Install only after setup is accepted (an explicit setup request suffices);
   use the host's normal execution/network approval flow. Do not bypass a denied
   approval, install for unrelated tasks, or initialize a repository as part of
   installation. Report setup failures and do not claim OpDev is active.
4. Run the selected executable's `plugin verify --contract <absolute-path>`.
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

1. Read `AGENTS.md` and the project contract. `CLAUDE.md` imports the same project guidance for Claude Code.
2. Load authorities selected by `context.always` and by every relevant task route. Follow [project-contract.md](references/project-contract.md) when interpreting fields. Do not assume design material belongs in `docs/`. Follow [documentation ownership](references/project-contract.md#documentation-locations-and-ownership) before choosing or changing authority locations.
3. Use the declared work authority for active status, sequencing, and decisions. Keep static specifications free of roadmap drift.
4. Establish the outcome, scope, exclusions, acceptance conditions, risks, and evidence before substantive edits. For every planning objective, including "what is the next step?", follow [planning.md](references/planning.md): prefer demonstrable consumer increments, keep distant steps provisional, and honor specific user requests. Before committing to a substantial request, milestone approach or smaller high-risk change, apply its uncertainty checkpoint: reuse sufficient current evidence or investigate consequential unknowns proportionately. Scale design work to risk and reversibility.
5. Make small, reviewable changes. Preserve supported behavior unless the accepted change deliberately migrates it.
6. Apply [testing.md](references/testing.md) and [acceptance evidence](references/acceptance.md) for substantive changes: derive expected results from accepted requirements, review actual assertions, and bind the inventory/mappings to the current change. Run canonical command argument vectors directly; do not reinterpret them through a shell.
7. For facts the CLI cannot infer, follow [evidence.md](references/evidence.md). When a new ledger is needed, prefer the schema-backed `opdev evidence bootstrap` review flow; it starts every decision unresolved and keeps durable project facts separate from fingerprint-bound change facts. Never reuse an assertion after the repository state changes without rechecking it.
8. Use `opdev check` for local evidence. Use `opdev check --ci` in integration CI and `--remote` only when a read-only provider audit is relevant. Use full human/JSON output by default. Follow [results.md](references/results.md) for retained reports; compact views require explicit experimental opt-in from the user or project, never just a capable runtime. Report blocked or unavailable evidence honestly.
9. Reconcile implementation, tests, declared authorities, delivery behavior, and tracked work before completion.

Read [workflow.md](references/workflow.md) for the full lifecycle and gate behavior when planning or carrying out a substantive change.

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

MinimumCD requirements are mandatory for every initialized project. Extensions may add or strengthen checks but cannot disable a core rule, change its applicability, replace its result, or suppress required evidence.

Use only these rule outcomes: `passed`, `failed`, `unverified`, `not_applicable`, `error`, and `migration_required`. Only `passed` and justified `not_applicable` satisfy a required rule. Never turn missing evidence, permission failure, tooling failure, or a known migration gap into a pass.
