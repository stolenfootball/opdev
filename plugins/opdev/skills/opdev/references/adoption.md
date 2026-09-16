# Adopt OpDev in an existing or new project

An explicit request such as "convert this repo to OpDev" already consents to
assessment. Do not ask again whether to adopt it. Check `opdev adoption --help`:
older compatible runtimes may lack this development capability. Report the gap
and offer a compatible CLI; do not call old initialization complete adoption.

Before presenting choices or requesting approval, read
[decision review](decision-review.md). A single approval question for an
agent-selected plan does not substitute for resolving material developer choices.

## Validate recommendations before asking for approval

Preserving existing choices means preserving **adequate** choices, not blessing
an existing violation. Developer consent selects an implementation; it cannot
waive core requirements. Apply these constraints to the proposal itself, before
writing records or presenting a decision sheet, and again on follow-up turns:

| Decision | Valid choices | What must remain a migration gap |
| --- | --- | --- |
| Integration and release roles | One integration trunk that is also the release source. For a non-main trunk, offer both keeping its name and renaming it to `main`. | Integrating on `develop` then merging into `main` to release is **not a valid completed-adoption option**. The user may defer changing it, but it stays `migration_required`; keeping a name does not permit keeping conflicting roles. |
| Delivery | CI qualifies and publishes/promotes the same immutable artifact through the declared consumer-facing path. A human may approve a CI job; CI still performs delivery. | A tested archive followed by a maintainer's manual upload/distribution is not CI-exclusive delivery. Tags, versioned filenames and green packaging jobs alone do not resolve this. |
| Recovery | An automated, tested way to recover delivered behavior, scaled to the product: e.g. restore a previous CLI/package artifact or qualify a forward fix. | A useful error message/nonzero exit is diagnostics, **not recovery of a bad release**. Being local, small, a library, or not hosted does not waive recovery. |
| Observability and effectiveness | Assess real operations and intended outcomes. For a local CLI, stderr/exit behavior may be proportionate operational evidence; intended user tasks supply effectiveness objectives. | No daemon/telemetry stack does not imply no operational behavior. Missing policy, risk metadata, or a synthetic evaluation fixture does not prove inapplicability. |

Ask how to meet applicable requirements, not whether to waive them. An optional
tool can be declined; an applicable capability still needs evidence. Distinguish
"implemented with a lightweight mechanism" from "not applicable". If uncertain,
leave the assessment unresolved rather than recommending N/A for convenience.

Correctness tests are not automatically effectiveness evidence. For example, a
unit test of a label function does not establish that a user can complete the CLI
task, read its output or recover from an error. Propose a representative task or
usability evaluation, with success criteria and limitations; reuse automated
tests only for the claims their actual assertions support. Do not claim existing
tests cover CLI stderr/exit behavior without inspecting those assertions.

Before sending the proposal, check its branch examples, proposed exclusions and
delivery/recovery claims against this table. Do not label two different branch
roles compliant and expect a later CLI failure to correct the recommendation.

### Keep schema versions separate

On this capable CLI, new `init` creates `.opdev/project.yaml` with **schema 1**
and `.opdev/adoption.yaml` with **schema 2**. These are different contracts.
Inspect the adoption record's own schema before proposing migration. Only an
existing schema-1 **adoption record** needs `adoption migrate`; do not prescribe
`init` followed by migration just because the project manifest is schema 1.

`adoption plan` and `adoption status` are read-only; `plan` needs an initialized
project, not write permission. `init` and `adoption approve` write records;
`adoption check` runs project commands. Distinguish a missing prerequisite from
host permission denial, and do not describe every command as a write.

## Assess and recommend

Use `opdev init --dry-run` for a new project. For an existing contract without an
adoption record, explicitly preview `opdev adoption start --dry-run`. Read
`opdev adoption catalog`: every supplied practice needs a disposition, including
formatting/linting, coding conventions, tests, security, CI, delivery and conditional
capabilities. A missing config file never means the practice was deliberately ignored.

Inspect all relevant components, existing scripts, CI, configuration and conventions.
Discovery is only a proposal and may miss mixed/custom stacks or configuration
inheritance. Preserve adequate choices. Research genuine gaps using authoritative
documentation, current compatibility and project constraints. Recommend one option
with a concise rationale; show alternatives only for meaningful tradeoffs. Reuse
decisions with verified approval on future tasks. Do not research or replace working tools merely
because another tool is fashionable. If a necessary choice cannot be justified,
leave it pending and ask a focused question.

Assess decisions as preserve, add/change, ignore and unresolved. Sequence the
approved work using [outcome-based planning](planning.md), including when asked
what to do next. The checklist is not a requirement to build every foundation
before a useful increment; pending practices still prevent adoption completion. Ask
grouped questions for material choices. Adoption is not blanket permission for
arbitrary installations, broad reformatting, destructive migrations, protection
changes or publication. Keep style-only migrations separate from behavior changes.

## Implement and record

### Stop for the developer's decisions

"Adopt OpDev", "run the adoption workflow", and "let's make the decisions"
authorize assessment and discussion, not selection on the developer's behalf.
Present a compact preserve/change/ignore/unresolved proposal with consequences,
then stop for an actual response before implementing material choices. Record
the response reference and decision maker, not a guessed "owner selected" claim.
Explicit delegation may authorize choices only within the user's stated scope;
record its limits and stop again for choices outside them. Host execution approval
is not approval of adoption policy. Never manufacture approval by calling a CLI.

On a capable CLI, use `adoption plan` to bind the proposal to a plan ID. The
`adoption --help` output must list `plan`, `approve`, `migrate` and
`prepare-evidence` before using schema 2; the original adoption commands alone
do not establish support. Report a missing capability instead of skipping review.
After the
actual response, use `adoption approve --plan <ID> --reviewer <name> --reference
<response-or-review-reference>`; include `--delegation <scope-and-limits>` only for
an explicit grant. Changed choices or contract invalidate the approval. A hash
and a recorded reference do not authenticate consent: review the real response.
For an existing adoption record with schema 1, preview `adoption migrate`, then apply with `--write` only when
requested/approved. It preserves previous claims but invents no approval.

Review branch **roles**, existing instructions, CI triggers and protections.
Offer a non-main project's developer both retaining its name as the single trunk
and renaming it to `main`, explaining migration costs. Never rename automatically.
Keeping `develop` as the sole integration/release source is valid; integrating on
`develop` and promoting through `main` is a workflow migration, not a naming choice.
Record the reviewed roles and name choice in `workflow`; reconcile contradictory
instructions before declaring the migration implemented. Do not delete existing
branches, change protections or rewrite history without scoped authorization.

For example, after the developer chooses to keep `develop` and the reviewed
workflow actually releases from it, the record can contain:

```yaml
workflow:
  integration_branches: [develop]
  release_source: develop
  main_option: keep_name
  references: [CONTRIBUTING.md, .gitlab-ci.yml]
```

Use the project's actual references. `already_main` and `rename_to_main` require
a declared main trunk. This records reviewed facts, not permission to perform a rename.

Assess actual capabilities for applicability: a CLI/plugin may expose user-facing
interfaces and need accessible output, and a local tool may need operational
diagnostics. Missing coverage configuration is not a decision to decline coverage;
missing effectiveness risk metadata is not proof there are no effectiveness
objectives. Optional exclusions need developer agreement; conditional exclusions
need reviewed facts. An evaluator's narrow N/A is not permission to close a gap.

Inspect CI inheritance, job setup, runners, caches, artifact identity and the
actual publishing dependency path before proposing changes. Explain OpDev's
glibc requirement; preserve product images and prefer an isolated compatible job.
Do not share compiled caches across incompatible OS/libc, architecture or toolchains.
Require cold/warm-cache qualification and a release-path check; an MR-only job
cannot enforce delivery. Use `check --ci --delivery` only after checking runtime
support and make publication depend on its result in the same delivery path.
Templates are integration baselines, not a completed release pipeline.

Use one adoption work item by default. Create separate gap issues only if the
developer requests them or the approved plan needs independently owned work.
Never close issues solely because files exist, a proposal was written, a practice
was labelled N/A, or integration CI is green. Reconcile acceptance evidence first.

Run `opdev init` to create new scaffolding, or `opdev adoption start` to explicitly
assess a legacy project. These commands preserve existing decisions on retry.
New projects get `.opdev/adoption.yaml` with every item pending; no tool stack is
installed automatically. Existing authorities and project-owned files win over
folder defaults. Keep work sequencing in the declared tracker.

Record the assessed repository/component `scope`. For every catalog item fill
`state`, `owner`, `reason`, canonical `references`, existing test `suites`, and
`research` sources (empty if research was not needed). Do not duplicate commands.
Use `implemented` only when the implementation exists and has meaningful evidence.
Use `pending` or `in_progress` for unfinished implementation even after approval;
approval and verification are separate from this state.
Only optional items can be `ignored`, with explicit owner agreement, rationale and
review references. Conditional items may be `not_applicable` with evidence;
mandatory practices cannot be ignored or declared inapplicable. No disposition
waives core/selected-profile requirements. Pending work is not complete adoption.

`opdev adoption status` is read-only. Even `decisions_ready_for_verification` is
not proof of implementation. Run declared checks, inspect real behavior/CI and
confirm that referenced suites cover the claimed practices and components.

## Verify completion

Stage all material files, including the adoption record. Follow [evidence.md](evidence.md)
for a new ledger or direct maintenance. The matching change must have passed
OPDEV-WORK-001 and OPDEV-TEST-002 assertions, each with reviewed evidence like:

```yaml
kind: adoption_review
summary: Describe the actual reviewed scope, decisions, implementation and acceptance evidence.
location: .opdev/adoption.yaml
```

Do not copy that example summary as evidence. In a bootstrap questionnaire, add
the actual entry to shared change evidence and explicitly review each decision.
Do not put adoption approval in durable project evidence. Fingerprints include
adoption decisions; edits require fresh review. The CLI does not authenticate a
reviewer or establish the truth of a link, so inspect the referenced facts.

`adoption prepare-evidence` produces a read-only unresolved worksheet with the
current fingerprint and correct kind/location. Fill its actual summary and work
reference after review, then merge justified assertions into the matching ledger
change. It is not a completed attestation or a full bootstrap answers file.
Distinguish stale fingerprints from missing assertions or incorrect evidence kinds.

Run `opdev adoption check --report <new-external-or-ignored-path.json>`; optionally
add `--remote` for read-only provider auditing. It requires resolved decisions,
fresh review, passing referenced pre-merge suites and all core gates, and checks
that files/evidence did not change during verification. It does not run a remote
pipeline or publish artifacts. Required CI, qualification and recovery evidence
must still exist. Never register this command as a suite that calls itself.

Completion also needs MCD-PIPELINE-001 `delivery_gate` evidence pointing to the
reviewed release/tag pipeline path. Identify its required qualification job,
publication ordering, trigger coverage and actual run; generic pipeline evidence
or a green MR job does not establish that publishing cannot bypass qualification.

Report complete only after it passes and the required integration evidence is
reconciled. Otherwise report the exact remaining decisions, migration work or
errors. Do not change a state to ignored to make a gate pass. Retain results in
the work authority/CI; no permanent completion flag is written.

Report scaffolding, decision approval, implementation, verification and delivery
readiness separately. Say "initialized; adoption incomplete" while gaps remain,
not "converted" or "complete". A green integration result never substitutes for
delivery/compliance or post-integration evidence. Unsupported runtime capabilities
remain an explicit gap; do not install an unpublished pin to hide it.

For ordinary later tasks, reuse project choices and normal gates. Read relevant
adoption decisions without restarting setup or repeating research. Revisit them
when requirements change, compatibility fails, maintenance issues emerge, or the
user asks. A legacy project without a record stays usable; offer explicit adoption
assessment when relevant rather than silently rewriting it.
