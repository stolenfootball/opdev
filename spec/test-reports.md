# Structured test evidence

`opdev test-report inspect report.xml [--format json]` is a read-only inspection
of one UTF-8 JUnit file. It works without initializing a project and never runs
commands, changes an evidence ledger, or qualifies a gate. This first increment
does **not** bind a report to execution, revision, CI workflow, or freshness.

## Interpretation and limits

The independent JSON schema version is 1. Output contains `input_sha256` (the
exact bytes, not authenticated provenance), `outcome`, actual `cases`, `failures`,
`errors`, `skipped`, `retry_signals`, `qualification` and fixed `diagnostics`.
`qualification` is always `unverified`. Case error/failure/skip counts are
mutually exclusive, with error then failure taking precedence. Case errors mean
the producer reported a test error, not that OpDev's parser failed.

Supported structure is an unnamespaced `testsuite` or `testsuites` root, nested
suites, named cases directly within suites, and case failure/error/skipped
elements. Suite summary counters are cross-checked, never substituted for cases.
Duplicate suite-name/class/name identities, unsupported elements,
unknown case/suite attributes (including status/result), conflicting results,
inconsistent summaries, disabled
cases, skips and recognized flaky/rerun elements require review. Recognized retry
elements count signals, not attempts; absence of these elements is **not** proof
that retries did not happen. Unknown dialects must not silently produce a pass.

An observed failure/error yields `failed`, even with other uncertainties. An
empty or ambiguous report yields `unverified`. Otherwise `passed` means only
that supported report observations contain no detected failure or uncertainty.
It never means the report is truthful, current, complete, or qualifying.

Exit 0 means report observations passed; exit 1 means failed or unverified.
Unreadable, malformed, oversized or unsupported-root input exits 2 with a
diagnostic on stderr and no JSON document. Human output always states the
qualification limitation. Do not wire this inspection command alone as proof
that a required canonical test command ran successfully.

## Resource and privacy boundaries

Input is limited to 8 MiB, 100,000 XML nodes and 64 levels of nesting. DTD parsing
is disabled and no external entity resolver is installed. The file must be
regular; no network access is performed. Only UTF-8 XML is supported. Output
omits case names, failure bodies, properties and captured stdout/stderr because
these may contain private information. Keep the original report privately for
diagnosis; the digest permits identifying its bytes without echoing them.

## Design decision

Use the established `roxmltree` parser with explicit resource limits rather than
hand-written XML recognition. A bounded tree permits structural checks across
common JUnit variants; streaming would be preferable if larger input becomes a
demonstrated requirement. The existing check report is not this inspection format.
Inspection is optional and separate from canonical command execution and gates.

Sources: [GitLab JUnit reporting](https://docs.gitlab.com/ci/testing/unit_test_reports/)
and [roxmltree parsing options](https://docs.rs/roxmltree/0.21.1/roxmltree/struct.ParsingOptions.html).

Work and remaining acceptance criteria are tracked in
[issue #44](https://gitlab.com/stolenfootball-tools/opdev/-/issues/44), not here.

## Tool-neutral canonical execution

`opdev test-execution --suite SUITE [--root PATH]` runs one declared suite once
and emits JSON observations. No reporter, output format, report path or additional
project configuration is required. The exact canonical argv, working directory
and timeout are preserved. This executes project-controlled code: review the
project contract first.

Before executing, source must be clean and committed, including staged changes
and untracked non-ignored files. The source is checked again after selecting the
command and after execution. This explicit receipt command has stricter source
preconditions than ordinary `opdev check`; it is not a replacement for local
checks while editing. Keep redirected receipts in an already ignored directory
or outside the repository.

[Receipt schema 2](../schema/test-execution-receipt.schema.json) identifies the
suite, command key, digest of the serialized canonical command specification,
opaque attempt ID, local-clock start time, source revisions before/after,
command exit and duration, command outcome and combined observation outcome.
It contains no report fields or report-schema dependency. Command arguments and
logs are omitted for privacy; retrieve the command from the contract at the
recorded revision and retain logs privately. The attempt ID distinguishes local
observations, not authenticated CI runs.

A nonzero command exit yields `failed`, even if source identity also changes.
Timeout or execution failure yields `error`. Otherwise changed or unreadable
post-execution source yields `unverified`; exit zero with unchanged source yields
`passed`. CLI exits are 1, 2, 1 and 0 respectively. Pre-execution errors exit 2
with stderr and no receipt. OpDev does not retry the command.

A passing observation establishes only the command exit and checked source
identity. It does not establish test counts, complete selection, assertions,
effectiveness, internal retries or quarantine completeness. No test report is
read, so even a failed report on disk cannot affect this command's verdict.
Projects must configure runners to propagate failures correctly; this command
does not excuse an observed test failure or waive core requirements.

This is an observation, not an attestation. Source checks cannot detect edits
made and reverted during execution, ignored inputs or environment drift.
Clock accuracy and CI identity are not verified. The receipt retains
`qualification: unverified`, never updates a gate or evidence ledger, and is
not accepted as saved qualification by `opdev check`.

## Gate and policy boundary

Ordinary checks continue to execute declared stage-specific commands and
aggregate their outcomes with core rules and selected blocking extensions.
The [schema-2 acceptance verifier](evidence-ledger.md#schema-2-acceptance-evidence)
uses these current-check suite outcomes alongside reviewed change-bound mappings.
It does not consume saved execution receipts or JUnit observations as qualification.
They do not interpret JUnit or require report-producing tools. Review actual
runner/CI test selection, failure propagation, retries and quarantine controls
during adoption and relevant configuration changes; reuse adequate existing
controls and record decisions in the existing authorities.

Additional project-specific enforcement can use the existing extension
mechanism. Extensions cannot waive core requirements. Do not create a
producer-adapter framework or impose a complete-history attestation by default.
Unknown required evidence stays unverified; an exit-zero observation does not
establish those separately required facts.

## Compatibility decision and reversal trigger

The developer chose tool-independent execution evidence over JUnit gate
integration after reviewing compatibility cost and unproven incremental value.
JUnit is a cross-tool format, but its dialects do not reliably establish complete
test selection or retry history. Keep the bounded read-only inspector as an
optional diagnostic; do not extend its dialect support merely to make gates green.

The removed interfaces (`check --junit SUITE=PATH` and `test-report run --junit`)
existed only in unreleased development builds. They now fail CLI parsing instead
of silently dropping a caller's requested enforcement. Preview callers should
review their intent: use `test-execution --suite SUITE` for command/source
observations, and inspect reports independently when useful. This is not an
equivalent replacement for report-based enforcement. No published project
manifest or check-report schema changes.

Receipt schema 2 deliberately rejects the old report-bound schema-1 shape.
Do not relabel or reinterpret saved version-1 receipts as version 2. Reconsider
report-based gates only after representative real-project trials demonstrate
meaningful additional defects caught, acceptable setup and compatibility costs,
and explicit limits. CI provider identity auditing is a separate capability.
