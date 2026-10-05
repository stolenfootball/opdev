# Development workflow usability

Accepted scope: [issue 52](https://gitlab.com/stolenfootball-tools/opdev/-/issues/52).
These requirements clarify task boundaries and diagnostic evidence; core result
aggregation, consent and exact-source qualification remain authoritative.

- UX-01: Blocked acceptance diagnostics MUST retain underlying index/source
  failures and identify the condition, mapped suite and selected stage when
  execution is absent. No files are auto-ignored and no undeclared checks run.
- UX-02: GitLab inspection MUST support an optional spec header and explicit
  nested repository-local includes with provider merge order. Repeated includes
  are distinct from cycles; overriding arrays replace prior arrays. Missing or
  malformed files are errors, unsupported/dynamic interpretation is unverified.
  The gate and doctor/upgrade inventory share interpretation. Inventory binds
  every read include to its upgrade snapshot. Local syntax/string observations
  do not establish effective job scheduling or qualification.
- UX-03: Windows lookup MUST distinguish absent, policy-blocked, damaged and
  incompatible runtimes. A documented process-scoped invocation of the reviewed
  plugin may be used; persistent policy and organization restrictions are not
  changed. Spawn diagnostics identify the program, working directory and OS
  failure; no automatic broader-permission retry is allowed. A representative
  successful sandbox probe establishes only the observed executable/environment.
- UX-04: Adoption plan/status MUST expose remote qualification prerequisites
  before implementation approval, independently of adoption-record schema. A
  remote worksheet contains observations and unresolved choices, not selected or
  approved policy. No network access without an explicit remote request. Local
  verification, remote qualification and delivery readiness remain distinct.
- UX-05: Determine requested task scope before resolving the runtime. External
  host/runner/storage/network/account servicing alone does not activate the
  development lifecycle. Project-owned source/configuration/contract changes do;
  mixed tasks transition at that boundary. Ordinary authorization and accurate
  observed-result reporting apply to operational work.
- UX-06: Isolated disposable local exploration MAY reach feedback using only
  the execution and safety checks needed for its purpose, without production
  acceptance records or full project suites. Keep its purpose, temporary location,
  actual checks and discard/promotion intent in existing work/conversation context.
  No new registry or persistent project policy is required. Do not label retained
  production edits or distributed experiments as disposable. Retained work needs
  accepted conditions, meaningful tests, current review bindings and canonical
  gates before integration; distribution uses the declared CI path. Initial
  requirements precede implementation, but final fingerprints/review digests
  follow settled source. Material edits invalidate evidence as before.
- UX-07: Milestone completion MUST NOT automatically request a version bump,
  tag, release notes or registry publication. Ordinary integration establishes
  development progress; a CI candidate supplies package bytes for a meaningful
  consumer trial; publication creates a deliberate supported checkpoint. Required
  MR/trunk checks remain applicable. Package/recovery qualification follows
  affected behavior and actual delivery. Reuse evidence only for unchanged
  artifact identity, configuration and applicable freshness; new bytes are not
  qualified by old observations.

- UX-08: A controlled environment-specific diagnostic MAY obtain bounded feedback
  before integration or packaging, using existing authorized CI/provider tooling.
  Record purpose, environment, scope/time bound, side effects, observations and
  promotion decision in existing work context. Review untrusted source, inherited
  secrets/runner permissions and artifact visibility. Prefer observation; verify
  ownership before cleanup of isolated resources and verify removal afterward.
  Diagnostic success MUST NOT qualify unrelated integration or delivery gates.
  Failed attempts remain visible. No automatic remote execution, runner changes,
  new registry or blanket approval round is implied.

## Verification and effectiveness

Regression fixtures exercise source/stage diagnostics, equivalent CI layouts,
overrides, malformed inputs, bounds, consent and upgrade preservation. Fresh host
scenarios evaluate task classification and timing of feedback, adoption choices,
promotion and publication separately from deterministic resource tests. Use only
neutral fixtures, retain actual outputs and limitations, and do not claim model
behavior from phrase matching. Compare first-preview time, suite execution,
evidence rewrites and debugging separately; one probe is not a universal speedup.

## Design and reversal

Use existing provider adapters, evidence schemas, work authorities and host
guidance. Reject a universal skip-check switch, a custom toolchain/launcher
framework and a new exploration ledger: each creates maintenance or weakens the
meaning of qualification. Unsupported provider interpretation remains explicit.
Revisit bounded local-include coverage when real consumers require another form;
revisit exploration guidance if host trials promote unqualified changes or still
delay feedback. Release/pin changes follow normal immutable qualification later.
