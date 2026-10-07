# Optional bounded delegation

Use one agent by default. Use a controller with specialists when the developer
has enabled that strategy and independent investigation or fresh review is worth
the dispatch/context cost. Reuse an approved preference only within its scope.
This does not expand the requested task, host permissions or model choices.

## Permission scope

Before spawning, resuming or changing a worker's assignment, compare its actual
purpose, role, resources and actions with the developer's original permission.
Permission for isolated evaluation agents does not authorize implementation,
acceptance-review or CI specialists. "Do this again in future" preserves the
original scope; it does not broaden it. Approving the underlying task or policy
does not separately authorize delegating it. A read-only task still needs
delegation permission; a scratch directory or an "evaluation" label does not
make retained implementation or source review an isolated experiment.

For example, repeated baseline-versus-candidate trials on disposable copies can
reuse permission for those trials. They do not enable a worker to edit the
retained project, review its acceptance ledger, change CI settings or add release
qualification. Conversely, explicit permission for implementation and review
specialists can cover those roles across issues within its stated limits;
do not ask again for every worker or routine step.

Keep the original permission reference and its limits in existing conversation,
work context or an adequate project authority. A persisted summary must preserve
those limits, not turn a narrow example into blanket multi-agent permission.
Consult the original when the summary is ambiguous or conflicts with it; honor
later narrowing or revocation. If scope cannot be established, do not dispatch:
continue authorized work yourself or ask a focused question before expanding it.
Do not interrupt ordinary single-agent work just to offer delegation. No new
approval registry, consent schema or mandatory per-dispatch question is required.

## Assignments and returns

The controller owns the work authority, questions, decisions, shared writes and
integration. Start with read-only investigator, acceptance-reviewer or CI-analyst
workers; the controller can implement directly. An implementer, when explicitly
assigned, owns one consumer increment and exact files, not a whole technical layer.
No role is mandatory and no agent may recursively delegate by default.

Before dispatch, supply a bounded assignment with: ID and original work/outcome;
original requirements and constraints; exact source and acceptance subjects;
owned paths and permitted actions; focused checks (none by default); model/effort;
time/token budget, stop condition and expected return. Keep these in the host
message or existing private work context, not a new committed document per worker.
On a CLI with `delegation.v1`, `opdev delegation --assignment FILE --subject FILE
--acceptance-sha256 DIGEST [--active FILE]` validates the shared contract. It does
not dispatch, grant permissions or prove the supplied current subject is fresh.

Use one writer per shared area. Compare active ownership before dispatch; aliases,
links and isolated worktrees need actual path/base inspection, not a name match.
Do not run the full canonical suite once per worker. Coordinate qualification
centrally; explicitly bounded focused checks may support a particular question.
An assignment cannot authorize merge, publication, policy selection or human
approval. Do not install agents, broaden permissions or switch models to make
dispatch succeed. When the chosen host/model is unavailable, preserve the failed
attempt and keep valid work; use the authorized single-agent path only if the
user's model/strategy constraints allow it. Otherwise report the specific limit.

For a fresh acceptance reviewer, give the original conditions and actual assertions,
not the author's proposed conclusion. Require concrete missed behavior or a clean
comparison with limits; separate actual defects from optional polish. Bound the
review rather than repeatedly requesting approval until someone agrees.

Every return identifies assignment/source/acceptance subjects, actual model and
effort when observable, completed/partial/failed/interrupted/unavailable status,
changed paths, actions, checks, evidence references, findings, limits, unanswered
decisions, elapsed time and available input/cache/output usage. Unknown is null,
not zero. Preserve unsuccessful attempts. Do not claim matched settings from the
request alone. Use `--result FILE` to check a structured return; stale source,
out-of-scope writes/actions, duplicate checks and mismatched identities are not
accepted as current completion. The validator checks reported facts, not hidden
side effects or the truth of a review. Inspect the actual diff/commands/evidence.

The controller verifies actionable findings without restarting the entire worker
investigation. Include every worker's usage and failed attempts in cost comparisons;
a smaller controller transcript is not lower total token consumption. If overhead
or missed requirements grow, stop delegating rather than making it a new ceremony.

Load only the current host adapter:

- [Codex native workers](delegation-codex.md)
- [Claude Code native workers](delegation-claude.md)
