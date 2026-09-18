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
Explicit check bindings reuse its evidence envelope; neither its schema nor the
project manifest changes.

Sources: [GitLab JUnit reporting](https://docs.gitlab.com/ci/testing/unit_test_reports/)
and [roxmltree parsing options](https://docs.rs/roxmltree/0.21.1/roxmltree/struct.ParsingOptions.html).

Work and remaining acceptance criteria are tracked in
[issue #44](https://gitlab.com/stolenfootball-tools/opdev/-/issues/44), not here.

## Observed canonical execution

`opdev test-report run --suite SUITE --junit target/junit.xml [--root PATH]`
runs exactly one existing canonical suite through the existing bounded process
runner. This executes project-controlled code: review the project contract first.
The suite's argv, working directory and timeout are unchanged. OpDev does not
configure a reporter for the project; the selected command must already produce
the requested XML. The output path is relative to the Git root, not the suite's
working directory. The command emits JSON; there is no new project configuration.

Before executing, source must be clean and committed, including staged changes
and untracked non-ignored files. The output path must not exist (even as a dangling
symlink). No existing file is deleted or overwritten by OpDev. Use a new output
path on every attempt and keep generated reports/redirected receipts in an
already ignored output directory or outside the repository. After execution,
source must still be clean at the same commit. A changed or unavailable source
identity cannot pass. The produced report must be a regular non-symlink file.

The independent [receipt schema 1](../schema/test-execution-receipt.schema.json)
references the checked-in inspection schema; offline validators must register
both schemas instead of fetching their identifier URLs. The receipt identifies
the suite, command key, digest of the serialized canonical
command specification, local-clock start time in Unix milliseconds, source
revisions before/after, command exit and duration,
report observations and digest, and separate command/report/combined outcomes.
It does not copy command arguments or stdout/stderr. Retrieve the command from
the contract at the recorded revision and retain original logs privately.

A failed command or failed report is always `failed`, regardless of other
uncertainty; component outcomes preserve concurrent parser/tool errors. Without
a known failure, verifier errors yield `error`, missing/ambiguous reports or source
changes yield `unverified`, and a successful command plus clear report observations
at unchanged source yields `passed`. Exit codes are respectively 1, 2, 1 and 0.
Pre-execution errors exit 2 with stderr and no receipt. No suite is retried.

This is an observation of one attempt, not authenticated provenance. A trusted
command or another process can write old content to a new file; absent-before
and present-after is not proof of authorship. Source checks do not detect changes
made and reverted during execution, ignored dependencies or environment drift.
Clock accuracy, internal retries, CI identity and full delivery qualification remain unverified.
The standalone receipt never updates core gates or evidence ledgers, and old saved
receipts are not accepted as current qualification. Check ingestion below runs
the command itself; provider verification is a separate capability.

## Opt-in check integration

Development builds accept repeatable `--junit SUITE=PATH` bindings:

```sh
opdev check --ci --junit unit=target/unit.xml --format json --report target/check.json
```

The named suite must already be declared for the selected stage (`local` by
default, `pre_merge` with `--ci`, `delivery` with `--ci --delivery`). Configure
its canonical command to write that JUnit path using the project's chosen test
runner. Paths are relative to the Git root, not the command's working directory.
Bindings do not install reporters or change argv, cwd, timeout or retry policy.
For a persistent requirement, commit the invocation in the project's CI job;
omitting the flag keeps existing command-only checks, so a one-off local binding
does not establish CI enforcement. Old CLIs reject the unknown flag rather than
silently claiming support. No automatic project migration is performed.

Before any commands, duplicate/unknown suite IDs, empty paths and bindings for
another stage are errors. A bound suite is executed once by the same observation
path as `test-report run`, never once for the suite and again for the report.
Clean committed source, absent-before output and unchanged-after revision are
required. Existing output is not deleted or overwritten. Give each attempt fresh
output paths in an ignored directory or an ephemeral CI checkout. A stale path
or dirty source prevents that suite from running and produces a blocking `error`;
other eligible checks may still run. `--no-exec` retains a blocking `unverified`
check rather than dropping the required evidence.

Each bound suite retains an opaque `test_execution_attempt` ID and a
`test_execution_receipt_v1` evidence item. The latter's `summary` is a JSON-encoded
schema-1 receipt, not a path to mutable external evidence. Original stdout/stderr
and private test names are omitted; retain producer logs privately. Receipt
digests, command/source identity, counts and fixed diagnostics survive `--report`
and ordinary full JSON output. A failed command or report stays `failed`;
tool/parser errors stay `error`; missing/empty/ambiguous reports remain
`unverified`. Check exits follow existing semantics: 1 for a blocked selected
gate (including errors in a suite); invalid CLI/bindings exit 2 before evaluation.

**A clean JUnit report is not sufficient to pass this opt-in qualification.**
Internal retry and omitted/quarantined-test completeness are not standardized
by this supported format. Its observations may be `passed`, but its bound check
remains `unverified`. `OPDEV-TEST-005` is also `unverified`: a manifest policy or
saved assertion cannot turn unknown history from this execution into a pass.
Recognized retry signals and skips remain visible; no automatic waiver is added.
This integration is useful for retaining observed evidence and blocking known
failures, not as a turnkey green qualification path for arbitrary JUnit producers.

The attempt ID distinguishes this observation inside the retained report. It is
not a GitHub/GitLab run ID or cryptographic attestation. No CI environment variable
or remote pipeline status is promoted to verified source/run identity. The
trusted canonical command and local Git/filesystem observations are the boundary;
the earlier ignored-input, concurrent-writer and source-change limitations apply.
Provider-wide exact-revision CI policy auditing is separate from this collector.

### Compatibility decision and reversal trigger

Use explicit invocation bindings and the existing evidence envelope rather than
introducing manifest fields that older strict schema-1 readers cannot understand,
importing replayable saved receipts, or executing a second extension command.
This preserves existing projects while offering a concrete failing-evidence path.
Revisit the interface when a producer-specific adapter can demonstrate complete
attempt/quarantine evidence, or repeated CI configuration warrants a reviewed,
versioned project-contract migration. Neither future extension may erase a known
command failure or convert unavailable history into successful qualification.
