---
name: opdev
description: Follow OpDev for initialized software projects or explicit adoption. Offer it early for substantive work in uninitialized projects; routine pulls, status, dev servers and external infrastructure do not activate it.
---

# OpDev

## Scope and consent

Inspect the actual target repository and request. Loading this skill is not
consent to adopt it.

- Initialized: read `.opdev/project.yaml` and `.opdev/guidance.md`; reload after
  a context reset. Use the selected contract, not inferred current defaults.
- Explicit adoption: assessment is authorized, not arbitrary policy selection.
- Uninitialized substantive development: offer OpDev once after minimal
  inspection, before research or implementation. Wait for acceptance before using
  it. If declined or unanswered, continue the requested work without OpDev.
- Routine pulls, status, dev servers, external infrastructure and unrelated work
  need no activation, offer or runtime lookup. Reassess if scope changes.

Malformed contracts are errors, not uninitialized projects. Missing guidance or
required runtime needs a repair/install offer, not a silently improvised workflow.

## Select one route

- **Schema 5 or new practical adoption:** read
  [practical workflow](references/practical-workflow.md).
- **Schemas 1–4:** read [legacy workflow](references/legacy-workflow.md). Preserve
  its selected obligations until explicitly migrated.
- **Unknown schema:** report the compatibility gap; do not guess.
- **Runtime setup only:** use the packaged setup skill without adopting a project.

For CI design or improvement, read [CI design](references/ci-design.md), applying
only obligations selected by the project's contract. Do not import legacy gates
into a practical contract just because an older example includes them.

For retained development resolve the CLI through the plugin's
`scripts/runtime.ps1 -Mode Path` on Windows or `scripts/runtime.sh --path`
elsewhere; the plugin root is two directories above this skill. Lookup installs
nothing. Only an explicit no-managed-runtime result permits checking a compatible
existing CLI; other failures are not absence. Verify with
`plugin verify --contract <plugin-root>/opdev-compatibility.json`.
Honor an explicit source-development runtime in project instructions. Install only
with authorization. Use doctor for changed environments or concrete setup
problems, not every turn. Advice/status alone needs no runtime.

## Work proportionately

Honor specific requests. Plan larger goals as connected consumer outcomes, MVP
phases and accepted later scope, not completed technical layers. Small fixes need
clear intent and relevant checks. Research consequential unknowns and reuse current
findings. Work tracking belongs to its declared authority; durable documentation
belongs to its own location, never a substitute backlog in `.opdev`.

Use focused checks while iterating. Review actual assertions, error paths, side
effects and affected documentation. The implementing agent can perform a distinct
review pass; no routine human sign-off or reviewer subagent is required. One agent
is the default unless delegation is actually authorized.

Continue authorized work through local blockers by completing useful independent
work. A status question does not cancel an assignment; a later instruction can
narrow or stop it. Ask only for material unresolved decisions, not already supplied
permission. Do not invent busywork or repeat unchanged checks merely to continue.

## Boundaries and honest results

Never silently lower policy, remove requirements, hide failed/skipped checks or
invent consent. Protect secrets, existing work and permission boundaries at every
level. Additional standards retain their actual scope; OpDev readiness is not
external certification.

Plans are not execution. Suite success does not prove that a named case ran or
that assertions are adequate. Distinguish attempted, denied, failed and unverified
actions. Saved reports cannot qualify changed code.

Feedback, integration and release are separate. Follow selected checks without
redundant runs for paperwork. Implementation approval, milestone completion and
green CI do not authorize publication, deployment, versioning or consumer migration.
Reuse actual release authorization within its scope.
