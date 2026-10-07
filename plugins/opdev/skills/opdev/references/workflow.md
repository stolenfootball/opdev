# OpDev lifecycle

Repeat this lifecycle per meaningful increment, not as project-wide phases.
For all plans and next-step recommendations, use [planning.md](planning.md).
Specific user objectives control scope; technical tasks support demonstrable
outcomes or bounded enabling work. An advice request does not authorize execution.

## Specify

Apply the [planning boundary](planning.md#build-verify-learn-revise) before retained
development. Disposable local exploration reaches feedback with appropriate
execution checks; retained code then enters this lifecycle. Ordinary integration,
CI candidates and published checkpoints have different acceptance needs. A
milestone does not itself request publication.

Identify the problem, intended outcome, affected consumers, scope, exclusions, acceptance conditions, evidence, authorities, dependencies, and material risks. Put active status in the declared work system.

## Design

Apply the [planning checkpoint](planning.md) before committing to consequential
approaches: reuse current evidence or investigate material unknowns within a
useful bound. Provisional planning and justified learning increments remain valid.

Scale design to novelty, reversibility, blast radius, data and security consequences, and operational risk. Durable architecture or contract decisions record the generalized problem, alternatives, rationale, evidence, canonical authority update, and reversal trigger.

## Implement and integrate

Use one trunk (the branch where changes come together). Keep changes small and branches short-lived; branches start there, merge back there, and are removed afterward. Aim to merge tested work daily. Missing that target prompts reassessment of the blocker and next useful slice, not a merge prohibition or an exception-approval workflow. Do not use branch age alone as a one-day failure under another rule. Catalog 2 keeps daily cadence visible for compliance but removes it from development and integration gates; older CLIs still enforce their own catalog, so explain a version mismatch rather than claiming a blocked check passed. Stop feature work while required trunk CI is red; diagnosis and restoration take priority.

Use the [ready-for-feedback checkpoint](planning.md#ready-for-feedback) for retained
work as well as prototypes: focused checks and a meaningful observation can
precede integration readiness. Run required local and pre-merge suites at the
integration handoff and required post-merge checks on integrated trunk. Preserve
supported delivered behavior, or record and test an intentional migration.
Agent-authored work meets exactly the same standards as human-authored work.

For new or substantially expanded documentation, include the
[focused content-placement review](project-contract.md#review-document-content)
in normal review. Compare actual changed claims with requirements and work
tracking; a successful path check does not establish semantic separation.

Integration need not enable unfinished behavior. For experiments, apply
[the experiment lifecycle](experiments.md): explicit isolation and stable defaults,
bounded configuration tests, accountable review, and promotion or cleanup. No
extra record is required for ordinary work.

## Package and deliver

### Release authorization

Recommend a release when useful, but obtain explicit developer authorization
before release-specific preparation or execution: choosing/applying a release
version, writing release notes, creating tags, publishing, deploying, or running
a release-specific rollback/recovery qualification workflow. Completing a
milestone, approving implementation, accepting a preview, passing CI, or an
identified consumer need does not grant that permission. A general instruction
to finish the work does not expand its scope to release.

Reuse an existing explicit release authorization within its stated scope; do not
ask again for each routine step. Clarify only material unresolved scope or a new
side effect. Keep permission in existing conversation/work context, not a new
approval registry. This boundary does not prohibit authorized CI candidates or
normal package/recovery regression tests needed for changed behavior; a release
workflow is not required merely to complete those tests.

When integrated work meets its accepted outcome without a release request,
report the milestone complete and release not requested. Keep outstanding
delivery readiness separate and visible; do not declare it passed or start
release/version/rollback work to make every gate green.

CI is the exclusive supported delivery path. The pipeline gives a definitive verdict, builds a deployable artifact once, identifies it immutably, and promotes the same bytes. Qualify in an environment representative of material destination risks. Version and test behavioral configuration; inject environment-specific values without rebuilding.

Use one consumer-facing delivery path and an automated, tested recovery strategy appropriate to the software: rollback, previous-artifact redeploy, disablement, restoration, safe roll-forward, or a focused forward fix when reversal is unsafe.

## Operate and learn

For operated software, collect user-centered health and diagnostic evidence after delivery. Reconcile defects, incidents, decisions, tests, and the project contract. Evaluate intended effectiveness separately from deterministic correctness.

## Resume from existing evidence

At a context handoff, read the current work authority and only the relevant
decisions/evidence. Do not restart completed research, adoption or settled user
questions without a changed fact. Distinguish completed work from whether its
evidence is still current, and distinguish permission to execute from actual
product feedback or permission to publish.

If a project already uses the optional `workflow.references.v1` protocol,
`workflow inspect` can reconstruct its journal against an explicitly supplied
current subject and retained evidence. It runs no commands or network calls and
does not refresh the subject, authenticate human attribution or qualify a gate.
For acceptance-review records, supply the current complete acceptance digest;
unchanged product source alone does not cover added or revised conditions.
Missing inventory identity leaves that review stale. Do not clear valid execution
history or rerun checks merely to repair a review explanation; qualify execution
only through its supported same-run policy or fresh canonical path.
Read original scope before relying on a record. Missing or conflicting sources
are unresolved; locate them before asking the developer to repeat a decision.
Use explicit append with the inspected previous digest only for authorized
record updates; preserve revoked/superseded history. Existing authority references
are preferable to another journal when adequate. Do not create a new tracker,
mandatory cache, private transcript copy or migration just to resume ordinary work.

## Gates

The four aggregates are development, integration, delivery, and compliance. A gate is blocked when any applicable required rule or blocking project check is not `passed` or justified `not_applicable`. A report may be useful even while blocked; never summarize it as successful.

On a CLI advertising `check.post-merge-integration.v1` in `doctor`,
`check --ci --post-merge` selects integrated-trunk suites/extensions and follows
the integration gate. Required post-merge check failures block integration and
delivery; missing delivery-only evidence does not block integration. All gates
remain in the report. `check --ci --delivery` still requires delivery readiness;
neither command grants release permission. Older CLIs select delivery for the
post-merge exit: explain that mismatch and offer a compatible upgrade, rather
than ignoring the exit, claiming a pass, or undertaking an unrequested release.
