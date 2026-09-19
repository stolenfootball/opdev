# Advisory consistency review

Normal substantive development now also uses
[typed acceptance evidence](evidence-ledger.md#schema-2-acceptance-evidence).
Expected results come from accepted requirements before implementation, then
actual assertions and current execution are reconciled before qualification.
That workflow strengthens TEST-002/003; the requested read-only audit below remains
separate and cannot itself approve a ledger or waive a gate. The rejected staged
prototype is not the enforcement mechanism.

An initialized project's agent can answer "does this satisfy the agreed plan?"
using existing authorities and work decisions. The shared skill routes that
request to its packaged [review reference](../plugins/opdev/skills/opdev/references/consistency-review.md).
Both Codex and Claude Code use the same reference; no new CLI command, manifest
field, mandatory document or project migration is introduced.

The review compares accepted scope with implementation, meaningful assertions,
execution evidence and version-appropriate documentation. It also checks exposed
behavior against accepted scope. Findings retain references, consequence and
uncertainty; justified exclusions and future slices are not implementation gaps.
Unresolved authority conflicts remain questions, not agent-selected policy.

Review requests authorize reading and an answer, not test execution, edits,
tracker writes or gate approval. The review does not infer passing qualification
from a green CI badge or silence. No findings means only no inconsistencies found
within the inspected scope. This is semantic agent assistance, not a deterministic
proof of completeness or a replacement for core verification.

## Rationale and validation

[Spec Kit's analysis](https://github.github.com/spec-kit/reference/agentic-sdd.html)
demonstrates read-only cross-artifact review. OpDev adopts that boundary without
requiring its spec/plan/tasks layout or automatically appending remediation tasks.
[Google's review guidance](https://google.github.io/eng-practices/review/reviewer/looking-for.html)
supports examining intended functionality and meaningful assertions, rather than
treating test existence as sufficient evidence.

Keep this as on-demand shared guidance while authority retrieval and semantic
judgment dominate the task. Consider deterministic tooling only after recurring
measurable failures justify a narrowly defined check; do not build an ecosystem
adapter framework or a second requirements database in advance.

The [behavioral scenarios](../benchmarks/consistency/README.md) separate supplied
facts from reviewer criteria. Resource tests establish packaging, not model
behavior. Live host trials must retain actual responses, attempts, permissions
and coverage limits. A single canary does not establish universal reliability.
Changes use ordinary reviewed MR/CI and roll-forward; no automatic activation in
consumer projects, marketplace update or release is part of this increment.

## Staged-review prototype (not integrated)

After single-pass trials exposed unsupported conclusions, the maintainer approved
a bounded [staged comparison](../benchmarks/consistency/staged/README.md): ordinary
review, the current shared reference, and candidate generation followed by one
fresh evidence-verification call. The prototype uses an automatically generated
snapshot/claim contract, exact-source mechanical checks and a non-generative final
renderer. It does not establish semantic truth, complete requirements coverage,
source authority or developer consent mechanically. Its advisory dispositions
are not core rule outcomes and cannot satisfy a gate.

The prototype lives entirely in benchmarks; its standard-library Python helper
and CLI host controller are not shipped or required by consumer projects. The
existing shared skill has not yet been changed to invoke a staged reviewer.
Production adoption would require reviewed prompts, a qualified native helper if
justified, host-native read-only review integration and evidence of worthwhile
accuracy/cost tradeoffs. Do not infer production readiness from this pilot.

The [completed comparison](../benchmarks/consistency/staged/results-2026-09-18.md)
found no additional root issues over ordinary review, greater resource use, and
weak negative-result traceability. The current recommendation is not to promote
the staged prototype to the default workflow. Its mechanical limitations are
recorded with the results; no consumer project requires its schema or controller.

Research motivating the comparison: [independent verification](https://aclanthology.org/2024.findings-acl.212/),
[separate code-review validation](https://github.com/anthropics/claude-code/blob/main/plugins/code-review/commands/code-review.md),
and [balanced agent evaluations](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents).
