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
  Release-specific preparation and execution MUST require explicit developer
  release authorization. Consumer need may justify a recommendation but MUST NOT
  substitute for consent; implementation/milestone approval, preview acceptance,
  green CI and general completion instructions are not release authorization.
  Reuse explicit authorization within its scope without per-command approval
  rounds. Normal affected package/recovery tests and authorized CI candidates
  remain valid. Milestone completion MUST be reported separately from delivery
  readiness; delivery-only gaps MUST NOT force an unrequested release workflow.

- UX-08: A controlled environment-specific diagnostic MAY obtain bounded feedback
  before integration or packaging, using existing authorized CI/provider tooling.
  Record purpose, environment, scope/time bound, side effects, observations and
  promotion decision in existing work context. Review untrusted source, inherited
  secrets/runner permissions and artifact visibility. Prefer observation; verify
  ownership before cleanup of isolated resources and verify removal afterward.
  Diagnostic success MUST NOT qualify unrelated integration or delivery gates.
  Failed attempts remain visible. No automatic remote execution, runner changes,
  new registry or blanket approval round is implied.

## Retained-work feedback checkpoints

Accepted scope: [issue 76](https://gitlab.com/stolenfootball-tools/opdev/-/issues/76).

- FB-01: Retained work MAY reach a ready-for-feedback checkpoint on a development
  branch/worktree after meaningful focused execution and safety checks, without
  final integration qualification. Agents MUST NOT require full suites, finalized
  acceptance bindings, broad evaluation matrices or installed-host qualification
  merely to request directional feedback. Retention is not disposability.
- FB-02: Check selection MUST follow the current question and material risks,
  preserve explicit project requirements and expose failures/untested scope.
  Relevant unsafe or misleading failures prevent a meaningful feedback claim.
  Broader early checks remain appropriate for unknown impact or consequential
  risks. Repeated feedback repeats focused checks, not automatic completion work.
- FB-03: Optional feedback selections reuse existing command IDs and the testing
  authority, with purpose and escalation conditions. No mandatory manifest field,
  CLI mode, ledger or approval round is introduced. An empty selection is not a
  pass. Declared argument vectors remain unchanged; diagnostic variants are
  disclosed, not substituted for canonical qualification.
- FB-04: Authorized development previews MUST use the declared CI path and bind
  observations to source/configuration, artifact identity and environment. Apply
  safe-use and affected package/startup/recovery checks. Feedback need does not
  authorize live consumer changes, installation, deployment or publication.
  Preview success MUST NOT satisfy merge/release requirements. Existing CI policy
  remains in force until a reviewed change; no skip-CI or allowed-failure bypass.
- FB-05: A sufficiently settled, small useful increment MUST receive current
  source-bound acceptance review and required local/pre-merge and post-merge
  qualification. New source is not qualified by old feedback or green runs.
  Preserve supported reuse constraints and distinct environments. Integrate
  frequently rather than waiting for final product design; replan stalled loops.
  Release remains separately authorized and qualified.
- FB-06: Reports MUST distinguish ready for feedback, ready to merge and ready to
  release without creating new machine outcomes or gates. User feedback indicates
  usefulness, not test correctness. Do not present an early checkpoint as completed
  implementation or universal validation; a failed trial remains visible.

The shared operational guidance is in the packaged planning reference. This
extends UX-06's retained-work handoff rather than relaxing the core catalog or
changing `opdev check` execution. Guidance and generated project instructions
carry the distinction for both hosts; older installed guidance is not silently
updated. Existing projects need an explicit guidance upgrade to refresh their
managed entry points. The plugin and CLI runtime may be updated independently.

Keep work-specific acceptance, observations and progress at the work authority.
Durable scenarios in `tests/planning-review.md` distinguish repeated local
feedback, artifact trials and integration; copied-resource tests establish
distribution, not agent behavior. Revisit this boundary if real sessions still
delay useful feedback or attempt to promote unqualified work. Prefer this small
checkpoint distinction over a universal skip switch or a parallel workflow engine.

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
