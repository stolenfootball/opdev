# OpDev consumer interface targets

These project-specific targets govern OpDev's CLI, retained reports and agent
guidance, not the software projects using OpDev. They do not claim WCAG conformance
or certify Codex/Claude host interfaces, terminals or assistive technology.

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
- Review representative help, error, blocked-report and decision-question output
  as part of the existing change review, using the proportional review policy
  below. Release review reuses adequate current review; it does not require fresh
  approval for every string. Release execution still needs separate authorization.

Automated coverage is in `crates/opdev-cli/tests/consumer_interface.rs`, with
consent/staleness coverage in `tests/adoption.rs` and the host conversation
scenario in `benchmarks/adoption/decision-review-canary.md`. Run through the
canonical check suite before and after integration. If a user
reports a terminal/assistive-technology barrier, retain the failing scenario and
evaluate it explicitly; plain text alone is not universal accessibility proof.

## Proportional interface review

Agent review is the default for routine help, error messages, spelling,
clarifications and presentation changes that preserve meaning. Check clarity,
actionability, accessibility and consistency with actual behavior. These changes
do not require a separate developer wording-approval round.

Developer review is required when a change materially alters:

- consent or authorization: what the user agrees to or what actions approval permits;
- required developer decisions: the choices the user must resolve or their consequences;
- outcome meaning: what is verified, incomplete, failed or blocked, or what may
  happen next.

For example, explaining an unchanged blocked result in plain language uses agent
review; making that result non-blocking or presenting an unverified result as
verified changes its meaning and requires developer review. Improving a question's
grammar is routine; adding an adoption choice or broadening what its answer
authorizes is material. Classify the actual effect, not whether a string changed.
Approval cannot turn an unverified result into a verified one or waive core rules.

Batch necessary representative output and its semantic changes into the existing
change review. Reuse actual developer approval within its accepted scope; do not
ask again for every message, file or increment. Ask a focused question only when
a material decision remains unresolved or the change exceeds that approval.
Record the actual response and review limitations in the existing work/MR review,
not a new approval registry. Agent review does not require a second agent.

Automated checks remain required where applicable but do not replace material
developer decisions or human evidence for accessibility claims. Approval of this
policy is not proof that later output was reviewed, and a wording clarification
does not supply consent for the action it describes. Revisit this boundary if
routine classifications repeatedly conceal material changes or reported barriers.

## Operational diagnostics

Human check reports explain blocked actions in ordinary language, show the
requirement name alongside its ID, distinguish missing evidence from failed
behavior and broken tooling, and provide a concrete next step. A rule ID, an
unexplained "gate", or an applicability phrase alone is not an explanation.
Daily-cadence advice must not suggest that a merge is blocked when only compliance
is unresolved. Machine outcome names remain stable for automation.

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
