# OpDev consumer interface targets

These project-specific targets were approved by the maintainer for the 0.2.1
patch. They govern OpDev's CLI, retained reports and agent guidance, not the
software projects using OpDev. They do not claim WCAG conformance or certify
Codex/Claude host interfaces, terminals or assistive technology.

## Accessibility

- All native CLI operations are keyboard/argument driven. Help, validation and
  checks must work without interactive input or a pointer. Mutations require
  explicit command arguments rather than timed or defaulted prompt acceptance.
- Redirected help, errors and human reports are readable text without required
  ANSI control sequences. Outcomes and blocked-rule identifiers appear in words,
  not solely as color or symbols. JSON is available for full check reports.
- Instructions require actual answers to material adoption choices. Silence,
  timeout, skipped answers and preselection are not approval. Where a host's
  question UI is unsuitable or unavailable, use the documented chat fallback.
- The release review combines process-level automated checks with a maintainer
  review of representative help, error, blocked-report and decision-question
  output. Record the reviewer's actual response and limitations; approving this
  target is not evidence that the subsequent outputs were reviewed.

Automated coverage is in `crates/opdev-cli/tests/consumer_interface.rs`, with
consent/staleness coverage in `tests/adoption.rs` and the host conversation
scenario in `benchmarks/adoption/decision-review-canary.md`. Run through the
canonical check suite before and after integration. Human review is required
when wording, outcome presentation or consent interaction changes. If a user
reports a terminal/assistive-technology barrier, retain the failing scenario and
evaluate it explicitly; plain text alone is not universal accessibility proof.

## Operational diagnostics

OpDev is an on-demand CLI, not a hosted service. Its relevant failures are invalid
input/configuration, blocked requirements, failed checks, missing tools and
failed or interrupted report persistence. It must expose:

- `version` for runtime identity; help for supported arguments;
- a successful help/version exit, check exit 1 for the selected blocked gate, and
  exit 2 for malformed input or CLI errors, with actionable stderr diagnostics;
- full JSON reports with rule outcomes, blocked gates and available per-check
  diagnostic evidence, retained with `--report` without overwriting prior output;
- a clear distinction between successful inspection and completed adoption, and
  between diagnostics, recovery and release qualification.

The process tests cover help/parse failure, an actual blocked fixture, exact JSON
retention and overwrite refusal. Existing command tests cover failed/missing
executables, timeout and bounded output; compact-view tests cover failed-check
stderr and unchanged full evidence. These are diagnostic signals, not telemetry
or a guarantee that every environment is supported. No telemetry service is
required. Recovery remains the separately qualified procedure in `release/README.md`.

Users reporting failures should retain the exact command, CLI version, exit,
stderr and a private full report where available. Reports can contain project
paths and command output: review/redact before sharing; never publish credentials
or private consumer material. Use a new report path for each attempt.

Revisit these targets if OpDev adds a TUI, web interface, daemon, new output mode
or a reported accessibility barrier. Do not expand conformance claims without
corresponding targets, representative tests and human evidence.
