# Early-offer targeted qualification — 2026-09-16

Scope: the [early-offer scenario](early-offer-canary.md), using a synthetic local
checklist design, no remote, and no material from a consumer project. This is a
small behavioral canary, not a reliability estimate or marketplace qualification.

## Candidate and execution

Base source: `c2bac47fdde950ff404e8309425dc673b6bfede9`, with the early-offer
working-tree changes. Snapshot SHA-256 identities:

- Shared skill: `81979d8fea797d2b4e750517640537f40e467f937e2f88b19e7ddec2709bc41c`.
- Claude hook: `a506c1dec43e2535d421913179d48e41602c0d0e78b55a4f31b19b8441132990`.

Each fresh trial had an empty Git repository with only `tmp.md` as product
content. Codex additionally had the candidate's local skill-discovery scaffold;
Claude loaded an isolated plugin snapshot using `--plugin-dir`. No AGENTS.md or
CLAUDE.md instructed the agent to offer adoption. The user prompt did not mention
OpDev. Default host models were retained; no model override was supplied.

Codex CLI `0.154.0-alpha.6.2` used `exec --json --ignore-user-config`, a read-only
sandbox and approval policy `never`. Actual model identity was not exposed in
the captured exec stream, so it is not inferred. Claude Code `2.1.273` reported
`claude-opus-5[1m]`, loaded only the inline candidate OpDev plugin, used empty
setting sources, strict MCP configuration and no permission prompts or bypass.
Its initialization event confirmed the candidate plugin path/version and skills;
the hook payload was not separately surfaced in the captured event stream.

Each process had a 300-second timeout; all completed normally, with no retries
or reported denied tools. Raw event/request/result files remain local. Codex
reported harmless global-ignore access warnings and a rollout-flush warning;
these are retained and no resume qualification is inferred from these runs.
The initial tests do not require writes; read-only execution does not prove
write-capable hosts will always honor the same boundary.

## Observed results

| Trial | Result | Evidence from complete event order |
| --- | --- | --- |
| Codex design implementation | passed | Loaded candidate skill, read local design/state, visibly offered adoption, then continued ordinary research with no answer; no runtime probe or OpDev state write. |
| Claude design implementation | passed | Read design and listed local files, then offered adoption before any upstream research or implementation. No runtime probe or OpDev state write. |
| Codex git status | passed | Ran git status and reported it, without an OpDev offer or runtime lookup. |
| Claude git status | passed | Ran git status and reported it, without an OpDev offer or runtime lookup. |

Codex's design trial demonstrates the non-answer path within a noninteractive
turn: the offer precedes research but does not activate OpDev or block ordinary
research. Its final inability to implement came from the test's read-only
sandbox, not an adoption prerequisite. Claude presented the choice in its final
response and made no research calls. Both fixtures remained unchanged apart
from the pre-existing Codex discovery scaffold; no `.opdev` or runtime data
directory appeared.

Local raw-event SHA-256 identities, in table order:

1. `ddf313b384be7c4f93dd6ca944f438f50b0312b6a79000e3ba663ef1df54bb18`
2. `0846d9956134392399a70f8e057ec4e7c6d7317643bb8707dc5fff2a59f02d9e`
3. `77f4cecdde04aa13b90ad96a685d589306d7e628e5e790452bbcb3a057e5f7b5`
4. `f8b0fb162146a55748a254581dc4857d4f5aa4c25417c1c770d4b55b3adbfc7a`

## Remaining boundaries

Interactive acceptance/decline/unanswered continuation rounds, live no-Git
folders, pull-and-start, unrelated prompts and configured-project controls were
not run in this targeted host evaluation: **unverified**. Their review procedure
is retained in the scenario. Seven deterministic hook tests passed, covering
non-mutation, runtime non-execution, design-only Git/no-Git detection, repeated
prompts, nested projects and worktrees. These tests do not grade model intent.

The shared skill and both host plugin validators passed, as did formatting,
strict clippy and all enabled workspace tests on Windows. Documented platform
and live tests remain excluded locally; required CI remains independently
necessary for integration. No installed plugin, consumer project, CLI version
or experimental default was changed by the trials.

Method reference: [Codex noninteractive execution](https://learn.chatgpt.com/docs/non-interactive-mode).
Installed CLI help supplied the exact supported invocation flags.
