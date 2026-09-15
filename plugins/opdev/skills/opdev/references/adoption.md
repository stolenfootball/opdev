# Adopt OpDev in an existing or new project

An explicit request such as "convert this repo to OpDev" already consents to
assessment. Do not ask again whether to adopt it. Check `opdev adoption --help`:
older compatible runtimes may lack this development capability. Report the gap
and offer a compatible CLI; do not call old initialization complete adoption.

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
accepted decisions on future tasks. Do not research or replace working tools merely
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

Run `opdev init` to create new scaffolding, or `opdev adoption start` to explicitly
assess a legacy project. These commands preserve existing decisions on retry.
New projects get `.opdev/adoption.yaml` with every item pending; no tool stack is
installed automatically. Existing authorities and project-owned files win over
folder defaults. Keep work sequencing in the declared tracker.

Record the assessed repository/component `scope`. For every catalog item fill
`state`, `owner`, `reason`, canonical `references`, existing test `suites`, and
`research` sources (empty if research was not needed). Do not duplicate commands.
Use `implemented` only when the implementation exists and has meaningful evidence.
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

Run `opdev adoption check --report <new-external-or-ignored-path.json>`; optionally
add `--remote` for read-only provider auditing. It requires resolved decisions,
fresh review, passing referenced pre-merge suites and all core gates, and checks
that files/evidence did not change during verification. It does not run a remote
pipeline or publish artifacts. Required CI, qualification and recovery evidence
must still exist. Never register this command as a suite that calls itself.

Report complete only after it passes and the required integration evidence is
reconciled. Otherwise report the exact remaining decisions, migration work or
errors. Do not change a state to ignored to make a gate pass. Retain results in
the work authority/CI; no permanent completion flag is written.

For ordinary later tasks, reuse project choices and normal gates. Read relevant
adoption decisions without restarting setup or repeating research. Revisit them
when requirements change, compatibility fails, maintenance issues emerge, or the
user asks. A legacy project without a record stays usable; offer explicit adoption
assessment when relevant rather than silently rewriting it.
