# Optional bounded delegation

Single-agent work remains the default. An explicitly enabled controller strategy
uses the host's native workers with shared `delegation.v1` assignments and returns.
It introduces no daemon, project agent directory, approval authority or gate.
The controller owns user questions, state writes, qualification and integration.
Read-only investigation and fresh acceptance review are the first supported roles;
an implementer requires explicit file ownership. Recursive delegation is not an
allowed assignment action. Existing host and user permissions still apply.

## Validation, not dispatch

`opdev delegation --assignment assignment.json --subject current.json
--acceptance-sha256 DIGEST [--active active.json] [--result result.json]` reads
explicit local JSON only. `current.json` uses the workflow subject schema; the
acceptance digest identifies the complete current inventory, not a worker's
chosen subset. Inputs are limited to 1 MiB each. No Git, network, check execution,
worker launch or state write is hidden in this operation.

The assignment and result schemas define interoperable mechanical fields.
Semantic checks additionally reject overlapping writers, unsupported actions,
stale subjects, new acceptance inventories, unowned reported edits and repeated
or unassigned reported checks. Path comparisons are conservative and
case-insensitive; the controller must separately inspect real filesystem aliases,
symlinks and worktree isolation. JSON cannot constrain an operating system.

Exit 0 means the supplied records passed structural/boundary checks, not that the
worker was truthful or its review adequate. Exit 1 reports incomplete work,
unknown/different observed model or effort, missing evidence, or exceeded/unknown
configured budgets. Exit 2 is invalid input or a boundary violation. Every view
reports `qualification: unverified`; it never grants approval, merge or release.
The supplied current subject is not independently refreshed or authenticated.

The controller compares actual changes, commands, assertions and original
authorities with the return. Missing usage remains unknown. Include every worker
and failed/interrupted attempt when reporting cost; cached input remains part of
input usage. Controller context reduction alone is not token savings. A worker
must return a bounded partial result rather than silently change model or scope.

## Native host adapters

The shared skill loads delegation guidance only when that strategy is enabled.
Codex uses available native worker tools with explicit requested model/effort and
minimal sufficient original context. Actual permission inheritance must be
checked; a role label is not a sandbox. See the official
[Codex subagent documentation](https://learn.chatgpt.com/docs/agent-configuration/subagents).

Claude Code discovers the packaged read-only `opdev-specialist` agent. It has
Read/Glob/Grep tools and inherits its model; unavailable observed settings remain
unknown. The controller supplies the shared reference and bounded assignment.
Plugin agent `permissionMode` is not relied upon because Claude ignores that
field for plugin agents. See the official
[Claude subagent documentation](https://code.claude.com/docs/en/sub-agents).

Capability `delegation.v1` advertises validation, not native host availability.
Older consumers can continue single-agent work without these files. No global
host configuration or consumer-owned instructions are overwritten. Package
validation and live host trials are distinct evidence; one cannot replace the other.
