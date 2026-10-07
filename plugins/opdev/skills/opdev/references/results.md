# Inspect check results

## Handoff after development

At a [ready-for-feedback checkpoint](planning.md#ready-for-feedback), show the
observation, actual focused checks, untested scope and the decision needed next.
Say "Ready for feedback; integration not yet verified" when that is the observed
state, not "complete" or "all checks passed". If feedback is not yet meaningful
or safe, explain the relevant failure instead. An accepted direction is not a
merge/release approval and does not make stale evidence current. Do not delay
useful feedback to assemble an integration-completion report.

Report the requested outcome and its actual verification first. If project facts
read during the task exposed a concrete CI feedback bottleneck, include a brief
optional recommendation in the final handoff: what to change, the observed cost
and remaining uncertainty, and how to validate it while retaining required checks.
Ask whether the developer wants that change; do not implement it or open an issue.
This applies even when the requested feature is complete and CI optimization was
not requested. Apply the [planning review boundaries](planning.md#review-the-ci-feedback-path-when-relevant):
no extra audit merely to find a suggestion, no pitch for adequate CI or a retained
policy, and no repeated suggestion after a decline without new material evidence.
Advice is not an integration gate or a reason to delay the requested work.

## Result meanings

Explain blockers for a developer who has never used OpDev. Lead with the action
that cannot proceed (for example merging, publishing, or claiming compliance),
what was observed or is missing, and the smallest useful next step. Keep rule IDs
as references, not explanations. Define jargon inline: evidence means recorded
test results or reviewed facts; a stale fingerprint means the review describes
different file contents. Say which file, test, command or decision matters when
known. Do not prescribe running all tests for a missing approval or claim a
software bug when a test tool could not run. Daily cadence is an advisory work
target in catalog 2, not a merge blocker; explain separately if compliance is
still unverified. Never fabricate a missing detail just to make an error actionable.

For example, a completed test with a failed assertion is `failed`; an absent or
unstarted required trial is `unverified`; a launcher that cannot start the test
is `error`. None satisfies the requirement. A green aggregate pipeline does not
turn any of these into a passed acceptance condition. Report the observed test
result separately from whether the overall acceptance requirement is satisfied.

## Reporting actions and blockers

For each action mentioned in a progress update or final summary, use the state
supported by its actual invocation and result:

| Evidence | Accurate report |
| --- | --- |
| No invocation in the available record | Not attempted; explain the actual reason if known. |
| Policy suggests a prompt/denial, but no invocation | Expected to require approval, not an observed denial. |
| Matching tool invocation returned a permission refusal | Attempted and denied; the command did not execute. |
| Matching invocation executed and returned output/exit status | Executed; report the observed result and only the conclusions it supports. |
| Invocation started but result is missing, interrupted or unavailable | Result unknown/in progress; do not infer success, failure or denial. |

Reconcile the command, arguments, working directory and attempt when they matter.
Keep failed/denied attempts visible alongside successful retries; a denied nested
shell invocation is not a denial of a different direct invocation. One command's
denial, a missing allow-list entry, an absent prerequisite or a previous session's
result must not be reported as a tool result for another command.

Before summarizing, match each claimed action/blocker to the available execution
record. Correct unsupported claims explicitly if already reported. Do not run a
mutation merely to find out whether it is permitted, retry a denied operation to
manufacture evidence, bypass permissions, or ask the user to approve a command
that is not needed. Use the existing host transcript/results; no extra project
file, schema or routine user confirmation is required. If context was lost, say
the result cannot be verified from the available record.

For example: runtime lookup was denied, while `init` and `adoption approve` were
never called. Report that specific lookup denial and that initialization/approval
were not attempted. Do not say all three commands were denied. An executed help
command proves the help ran, not that initialization or adoption passed.

These descriptions do not change rule outcomes: missing evidence remains
`unverified`, and an observed tooling failure is `error`, not a product `failed`.

## Inspect retained reports

Use ordinary `opdev check` human output or `--format json` by default. Full report
persistence (`--report PATH`) is stable on CLIs that support it. Do not select
compact output merely because the runtime supports it.

Only when the user or an explicit project decision opts into the compact-context
experiment, inspect `opdev --help` for `--experimental-compact`. If absent, keep
the stable workflow; do not use older ungated compact commands. When present:

```sh
opdev --experimental-compact check --report <new-file-outside-worktree.json> --format summary
```

Add `--ci` or `--remote` only when that evaluation is appropriate. This runs the
checks once, saves full JSON without replacing a file, and prints compact JSON.
The exit status still follows the requested check gate; inspect all returned
gates before making broader claims. Choose a fresh report path for each run.

For an existing full report, use `opdev --experimental-compact report summarize <file.json>` instead of
rerunning checks. Exit 1 means a recorded gate is blocked; exit 2 is a parsing,
validation, or filesystem error. This reads historical results, not current
repository or CI state. Schema-1 reports do not record the evaluation stage;
`stage: null` must not be guessed from the command used to summarize them.

Findings include every non-satisfying rule, and all checks retain their outcomes
and blocking flags. Excerpts have explicit truncation flags. Retrieve omitted
facts or diagnostics from the hashed full artifact using `source_pointer` (a
JSON Pointer), especially when an excerpt is insufficient to diagnose a failure.

Omitting the flag restores the stable workflow; it is not persisted. Token savings
and quality equivalence are not established for general development sessions.

Older compatible CLIs lack these commands. Use their existing human check output
or save full JSON and inspect the needed fields. Do not install or replace a
runtime just to obtain a smaller view. Once a capability is supported, report
its execution errors rather than treating them as an old-version fallback.
