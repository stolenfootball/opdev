# Adoption decision review

Apply this review to adoption and reassessment, not as a questionnaire on every
development task. Separate observed facts, developer choices, and permission to
implement. A plan hash binds content; it does not prove any of these were reviewed.

## Establish what is already settled

Inspect the actual decision source before calling a previous choice accepted:
a user response, reviewed decision, or explicit delegation with its scope.
An owner field, issue state, agent-written summary, migrated record, or approval
reference whose contents cannot be checked is not sufficient by itself.
When the user disputes an earlier adoption, revisit the disputed choices rather
than carrying forward those assertions as consent. Preserve the original record
while assessing; do not erase history or restart trustworthy, unaffected decisions.
State which approvals are verified and which remain unconfirmed. Ask for missing
decisions, not for permission to read facts that can be safely inspected.

## Ask relevant questions in rounds

Identify material choices whose answers change policy, scope, cost, supported
behavior, dependencies, maintenance obligations or migration. For each, explain
the observed situation, meaningful valid alternatives, your recommendation and
its tradeoff. Ask the developer to choose; do not hide the alternative inside a
preselected plan. Resolve dependencies first and use one question or a small
related group per round. There is no fixed total question limit: continue while
necessary choices remain. Do not ask about every catalog entry, discoverable
facts, adequate choices with verified approval, or mechanical fixes individually.

For an unresolved non-main trunk name, explicitly offer keeping the current name
and renaming to `main`. Explain actual migration cost without treating cost as a
reason to omit the choice. Separately address integration/release-role conflicts;
neither naming choice waives the single-trunk requirement. An existing `main`
branch needs inspection and scoped permission before any destructive action.

Use the host's structured question interface when available and permitted for
the current mode and question: for example Codex `request_user_input` or
`request_user_input_async`, and Claude Code `AskUserQuestion`. Do not assume a
specific tool exists, switch modes automatically, or use an optional-only tool
for required authorization. When the host restricts the tool, ask plainly in
chat and end the turn for a required decision. Keep execution-permission requests
in the host's approval flow. For asynchronous questions, only independent,
already-authorized work may continue before the response arrives.

Recommendations, preselected options, timeouts, skipped/empty answers and silence
are not consent. Keep dependent work unresolved. A partial answer settles only
the answered choices. Explicit bounded delegation can settle choices within its
limits; record the grant and actual selections, and ask about anything outside it.

## Separate capability evidence from preferences

Do not ask a user to vote a capability into existence. For each proposed
`implemented` or `not_applicable` classification, identify the assessed scope,
applicable needs, inspected evidence and what remains unverified. Map assertions
to those needs. Plain-text/keyboard CLI interaction, stderr, exit codes or typed
events are useful evidence candidates, not automatic proof that accessibility
or observability is satisfied. Inspect representative interactions and relevant
tests/reviews; explain their limits. Avoid both blanket N/A and blanket
implemented. Insufficient evidence stays unresolved, even with user approval.
Ask about intended users or operating requirements only when inspection cannot
resolve them and the answer affects the assessment. Keep this proportional to
the software; do not impose a web standard or telemetry stack on every project.

## Approve implementation after choices

Summarize selected choices with their actual response references, verified
preserved decisions, grouped mechanical repairs, and remaining evidence/work.
Keep this in the existing work authority and adoption record references; do not
invent new schema fields or a second decision ledger. Mechanical repairs include
syntax corrections or guidance refreshes only when they introduce no policy
choice; evidence reclassification is not merely a syntax repair.

Only then request approval of the resulting implementation scope and exact plan.
Do not bundle unanswered policy choices into that request. An answer already
authorizing the exact resolved implementation needs no redundant approval, but
choosing a policy alone is not permission to publish, rename/delete branches or
change protections. Changed choices require renewed review of the affected scope.
Report unresolved choices, approved policy, implementation and verification
separately. Do not close work merely because its policy was selected.
