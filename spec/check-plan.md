# Verification execution preview

Issue #55: a developer can inspect the commands a check selects, then use that
check as the single canonical handoff execution instead of running the suite
again merely to obtain an OpDev verdict.

- PLAN-01: `opdev check --plan [--ci [--delivery]] --format human|json`
  reads the project contract and selects the same ordered suites/extensions as
  the corresponding check. It executes no commands/extensions, reads no ledger,
  contacts no provider, writes no files and grants no qualification.
- PLAN-02: Report each check's ID, kind, command key, literal argv, working
  directory, effective timeout and blocking status. Include selected suite and
  extension stages. Shared selectors and extension timeout resolution serve both
  preview and execution. Distinct declarations remain distinct even with equal
  argv. Default timeout is the executor's default. Executable resolution, nested
  commands, environment suitability and source freshness are not established.
- PLAN-03: JSON schema 1 has `kind: check_plan` and
  `qualification: unverified`, with no gate verdict. Exit 0 means preview success;
  invalid arguments/contracts or read errors exit 2. `--remote`, `--no-exec`,
  saved check reports and experimental summary output do not combine with plan.
  Ordinary checking retains failure, stale-evidence and current-execution rules.
- PLAN-04: Generic counting-command canaries compare declared selection with
  actual execution, including duplicate argv, stage exclusion, failing commands,
  directory/argument preservation and extension timeout overrides. Preview must
  leave its command marker absent and ledger bytes unchanged.

Arguments and paths can contain private project values; review preview output
before sharing. A plan is not a cached result or promise that commands will run.
This feature does not merge pre-integration and integrated-trunk verification.

Use focused tests during edits, then finalized acceptance evidence and a normal
check at handoff. Do not precede it with an identical full suite solely for a
second report. A new source/configuration/environment or a substantive diagnostic
question can justify a new execution. No automatic equivalence inference or
saved-report cache is introduced.
