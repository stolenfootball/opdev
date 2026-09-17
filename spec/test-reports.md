# Structured test report inspection

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
Duplicate suite-name/class/name identities, unsupported elements, case
unknown case/suite attributes (including status/result), conflicting results, inconsistent summaries, disabled
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
demonstrated requirement. Avoid changing manifest/report schema semantics or
existing gates until execution identity and freshness are implemented and tested.
The existing check report is not this inspection format and remains unchanged.

Sources: [GitLab JUnit reporting](https://docs.gitlab.com/ci/testing/unit_test_reports/)
and [roxmltree parsing options](https://docs.rs/roxmltree/0.21.1/roxmltree/struct.ParsingOptions.html).

Work and remaining acceptance criteria are tracked in
[issue #44](https://gitlab.com/stolenfootball-tools/opdev/-/issues/44), not here.
