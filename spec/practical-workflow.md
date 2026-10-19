# Practical development contract

The practical workflow replaces mandatory per-change attestation with explicit
assurance levels. Its project contract is schema 5, policy version 1. Selecting
it is a policy migration, not a consequence of updating a plugin or executable.
Historical project contracts retain their meanings during the transition.

## Obligations and ownership

Lightweight supports low-consequence work with appropriate observable checks.
Standard is the default recommendation for maintained software: requirement-led
implementation and review, relevant regression tests, authoritative integration
checks and integrated-source verification. Assured adds source-bound assurance
and the explicitly selected verification scope. No level authenticates consent,
proves completeness of an inventory or establishes external certification.

The level is selected once, not inferred again on each change. Existing external
obligations cannot be silently dropped. Unknown levels, versions and unsupported
obligations fail before execution. Changes to security, data, public interfaces,
distribution or high-consequence behavior need appropriate safeguards regardless
of the level. Presence of a capability alone does not escalate unrelated work.

The developer owns scope and material policy choices. Agents own substantive
design, test adequacy and review judgments. The CLI validates explicit inputs and
observes executions. A reference or digest does not authenticate a judgment.

## Requirements and execution

Current requirements have stable IDs, observable statements and verification
links. Their links are independent of execution stages. Stage selection belongs
to checks: local feedback, pre-merge, integrated source, delivery or other real
boundaries. One suite can serve multiple requirements without extra execution.
Do not claim that a named test ran merely because its suite exited successfully.
Missing or failing required executions cannot be replaced by stored results.

When a selected catalog exists, new or changed supported behavior updates its
affected enduring promises and actual verification links before implementation
handoff, even when a separate agent owns integration.
Repairs reuse adequate existing statements; unrelated work needs no catalog
expansion. This is change-impact maintenance, not comprehensive traceability or
a new task ledger. Provisional feedback work need not finalize these links early.

The completion review compares the requested outcome with the actual diff,
selected requirement statements and assertions. It identifies the affected
promises and their existing or updated test links, and separates necessary
dependent changes from unrelated repairs. Project authority establishes truth,
not permission to expand a task. These findings belong in the existing review or
handoff; no additional ledger, reviewer, routine approval or duplicate execution
is required. Structural inspection cannot establish semantic completeness.

Standard does not need a selected MR comment, acceptance digest, ledger or
provider credential to run its checks. Existing GitHub/GitLab required jobs own
the native integration boundary. Actual semantic review remains necessary, but
the CLI reports its mechanical scope rather than pretending to certify it.
No cross-revision execution cache or test-selection inference is introduced.

The native check command owns exit semantics: zero requires all selected automated
commands to pass, no selected manual checks and no mapping findings. Failures or
unexecuted requirements return nonzero; execution errors return 2. CI may require
an exact full revision before execution; clean-source and unchanged-source checks
remain mandatory. Consumers preserve that exit code instead of reimplementing
report validation. Reports describe the selected stage, not inferred delivery.

A check selects exactly one canonical command or a repository-relative `manual`
procedure. Manual procedures must be nonempty regular contained files. They are
reported separately as `not_run`, without fabricated exit codes or execution time.
The CLI does not accept saved observations as a manual pass. Required manual work
therefore keeps its boundary unverified; no silent stage move or waiver is implied.
Assured mapping subjects include the procedure content. This avoids misleading
dummy failures without introducing an observation ledger or weakening enforcement.

Local feedback does not need the whole integration suite. Integration and
learning are deliberately separate checkpoints: experiment completion or retained
findings alone do not require integration. Batch tightly coupled learning until a
useful outcome is settled, without delaying independent stable increments or
creating indefinite branches. Pending CI permits safely separated authorized work,
not mutation of its candidate, a presumed pass or bypass of required checks.
Integration and integrated-source checks can differ deliberately. A candidate result is not an
execution on trunk; changed content or execution conditions require appropriate
fresh checks. Platform coverage is not redundant merely because command text
matches. Releases require separate developer authorization.

## Files and adoption

Only project.yaml and short guidance.md are universal .opdev files. Requirements
and selected additional policies have their respective namespaces. Work lives in
the existing tracker, documentation at the declared authority, and temporary
state outside source. No permanent adoption log or execution archive is required.

Adoption inspects, resolves material choices, previews a deterministic change,
applies that reviewed change, verifies and retires superseded active artifacts.
It preserves unrelated edits and fails on stale inputs. Empty repositories may
finish workflow configuration without pretending that a product was tested.
Existing projects reuse adequate tools. Upgrades disclose lost assurance and
preserve actual external obligations. Temporary rollback protection ends after
verification; ordinary Git history remains.

## Cost and limits

One agent is the default; no new reviewer or live trial is implied by a level.
Planning scales from a brief outcome to connected MVP phases, milestones and
slices. Research resolves consequential uncertainty; it is not a routine form.
Concise output is normal, with full diagnostics available. The mechanical
workflow must not depend on repeated agent-generated handoff records.

Performance is evaluated against both the previous workflow and ordinary agent
development. Report feedback/integration latency, execution and queue time,
cached/uncached input, output, interventions and product defects separately.
A 50 percent reduction in process overhead is a target, not an observed result.
Reduce or reverse machinery that adds more cost than useful assurance.

This contract deliberately reduces default provenance, redundant execution and
comprehensive traceability. Stronger scope must be selected, not implied. It is
informed by [NIST's risk-based SSDF](https://doi.org/10.6028/NIST.SP.800-218) and
[DORA's fast CI feedback guidance](https://dora.dev/capabilities/continuous-integration/),
not certified by them. Implementation status and evaluation findings belong in
the work tracker, not this enduring contract.
