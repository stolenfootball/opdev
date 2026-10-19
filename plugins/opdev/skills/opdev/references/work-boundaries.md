# Select work by purpose and readiness

Use this shared decision rule before planning execution, choosing checks or
reporting completion. These are work purposes, not assurance presets, CLI stages
or a new persisted state machine. The selected project's obligations remain intact.

| Current purpose | Work and useful result | Verification now |
| --- | --- | --- |
| Discover and design | Clarify material needs, research, propose behavior; use authorized bounded prototypes to resolve uncertainty. | Checks that make the proposal trustworthy or prototype safe to inspect. |
| Implement for feedback | Build an accepted slice and revise it within the agreed design; present a meaningful observation. | Behavioral/regression checks and safety checks relevant to the changed scope. |
| Integrate | Merge an authorized, sufficiently settled, useful increment. | All selected integration checks, and required integrated-source verification. |

## Advance deliberately

Choose the current purpose from the actual request, accepted decisions and
readiness, not file retention, a commit, completed trial, milestone label or end
of an agent turn. Broad implementation or eventual merge authority is not proof
that an unseen design is accepted or that a candidate is settled. Continue
autonomously within the agreed boundary, not automatically through all boundaries.

For new capabilities, substantial changes and redesigns, ask material unanswered
questions, present concrete behavior with an example, scope, limitations and
tradeoffs, then obtain design acceptance before dependent implementation. A task
list is insufficient when product choices remain unresolved. Requirements answers,
silence, passing tests and agentic technical review are not design acceptance.
Interpret "do it" against what was actually proposed. Reuse already accepted or
fully specified designs, direct repairs restoring agreed behavior and explicitly
delegated design choices; do not add repetitive approval questions.

Return to design when new facts materially change behavior, scope, workflow or
consequential tradeoffs. Continue only independent authorized work while waiting.
An authorized learning prototype is not permission to promote it into the product.
Technical self-review assesses implementation against the accepted design; it
cannot accept the design on the developer's behalf.

Integrate when a coherent result is settled and independently useful, a real
authorized dependency needs it on trunk, or branch drift makes integration useful.
Finish tightly coupled comparisons likely to revise the same result first; don't
delay an independent stable fix for unrelated experiments or accumulate indefinite
branches. No extra merge approval is needed within existing authority, but permission
and readiness must both hold. Release remains separately authorized.

## Before expensive verification

Before a full suite, native matrix, broad model evaluation or costly setup, briefly
identify the decision or risk it resolves, whether the relevant design is settled,
why it is needed at this boundary, and whether valid equivalent execution already
exists. Reuse an adequate explanation until inputs or purpose change. This is a
reasoned choice in existing work context, not a form or user approval request.
"OpDev requires checks" and "files changed" are not sufficient explanations.

Run broad checks early when they answer a concrete uncertainty or protect against
material risk: for example, an authorized migration compatibility probe before
choosing a design. State its bounded question and limits; early execution is not
design approval or integration qualification. Otherwise prefer the smallest check
set that makes the current observation meaningful, not a universal smoke test.

Inspect actual commands and consumers. `opdev check` may execute the whole declared
suite even without `--ci`; inspect `check --plan` when scope is unclear. Use suitable
existing focused commands during preparation and report their limited scope.
Default commands and `stages: [local, pre_merge, post_merge]` describe execution
selection, not an instruction to invoke all of them on every edit. "Required local
integration suite" means required when preparing integration, not required merely
because work happens on a local machine. During a requested feedback preview, do
not invoke that broad default solely because it is listed; run the relevant focused
checks and say integration remains unverified. An explicit project instruction
requiring a check before every edit/preview still applies; explain the conflict and
propose a reviewed change instead of inventing an exemption.
Don't run a full suite merely to produce a second report. Same argv alone does
not justify reuse across revisions, environments or required stages.

Prose-only changes normally need content, consistency, references and relevant
format/render checks. Executable instructions, tested examples, generated inputs,
shared dependencies and changed contracts may need broader verification. Mixed or
unknown impact requires investigation and conservative checks, not a blanket
Markdown exemption. Complete content/implementation review before expensive final
qualification to avoid preventable reruns for unfinished text or known omissions.

At integration, all selected requirements still apply. If no proportionate path
exists, report the mismatch and propose reviewed command/CI changes using
[impact-based verification](ci-design.md#impact-based-verification). Do not silently
change policy, skip required jobs, fabricate success or reuse stale results.

## Deliver without expanding completion

Present the requested design, documentation or feedback result as soon as it has
the relevant review and checks. Don't withhold it until an unrequested merge or
unrelated native qualification finishes. State integration status separately.
Already authorized integration still needs completion; present the useful answer
while continuing it, rather than treating the answer as a reason to abandon work.

A feedback checkpoint stops dependent work when the user requested that stop or a
decision is needed. Otherwise continue meaningful work within accepted scope.
While CI runs, preserve its exact candidate and do safely separated authorized
work when useful; prioritize restoration of required failures. Do not manufacture
busywork or repeat unchanged polling narration. Permission to continue grants no
new delegation, policy change, resource use or release authority.

Keep purpose, actual decisions, observations and missing evidence in existing
conversation/work context. No boundary registry, required file, new CLI flag or
approval ledger is introduced. Report only the boundary actually verified.
