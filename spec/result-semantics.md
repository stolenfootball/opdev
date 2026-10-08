# Result and aggregation semantics

Every OpDev rule evaluation produces exactly one result.

## Results

### `passed`

The rule applied, its verifier completed successfully, and all evidence required
by that verifier is present and valid for the evaluated subject.

### `failed`

The rule applied and evidence demonstrates that its requirement was not met.
A failure cannot be overridden by an extension or converted to a warning.

### `unverified`

The rule appears applicable, but OpDev lacks sufficient evidence or permission
to decide whether it passed. Missing remote permissions, missing live evidence,
and stale qualification evidence are unverified rather than passed.

### `not_applicable`

The rule's declared applicability condition is demonstrably false for the
evaluated project or change. A reason and the evidence used to decide
applicability are required.

### `error`

OpDev could not complete the evaluation because the verifier, project command,
provider, or evidence parser failed. Errors are not project failures, but they
block any verdict that depends on the affected rule.

### `migration_required`

A brownfield project does not yet implement the rule and has recorded the gap as
migration work. This permits incremental development but is not compliance and
cannot qualify a delivery governed by that rule.

## Required result record

Every result MUST identify:

- Rule ID and rule-catalog version.
- Result.
- Subject, such as project, revision, artifact digest, environment, or change.
- Verifier and verifier version.
- Evaluation time.
- Evidence references or a reason no evidence was available.
- A concise diagnostic.

Results that execute a command also record the argument vector, exit
classification, duration, and redacted output reference. Secrets and sensitive
values MUST NOT be embedded in result records.

## Reporting execution

### Configuration is not qualification

Evidence has four roles; these are not additional outcome values:

- **Configured:** a validated command, policy, authority or provider setting
  exists. It establishes only that declaration or inspected structure.
- **Observed:** an identified invocation or provider observation produced the
  recorded result. Its subject, environment, stage and limitations still matter.
- **Reviewed:** a named review connects relevant facts and actual assertions to
  the requirement. A review record is a reviewer's claim, not machine proof or
  developer consent.
- **Qualified:** all required observations and reviews for the stated boundary
  are current and sufficient. This is a gate conclusion, not a caller-supplied
  evidence label that can turn an observation into a pass.

The existing report envelope records roles in evidence and diagnostics, with
`verifier` identifying the producing mechanism. `configured` facts remain useful
when the behavioral result is `unverified`. Ledger assertions are reviewed claims
(`evidence_ledger`); command results and authenticated provider records carry
observations. Consumers MUST NOT infer qualification from `kind`, a command exit,
a configuration-inspection pass, or a selected assurance profile alone.

At every evaluation-only boundary, all selected missing canonical suites and
extensions appear as `unverified` checks with their normal blocking policy. No
execution is performed to fill those gaps. Supported same-run reuse supplies only
its authenticated, current-subject results; every uncovered check remains visible.
`--delivery --no-exec` remains rejected by the CLI; the engine's evaluation-only
semantics do not create a publication shortcut.

At pre-merge, `MCD-TEST-001` requires all selected suites to pass and current typed
acceptance evidence to satisfy its requirement. Post-merge applies that rule to
`MCD-TEST-002` instead. A known execution failure/error cannot be replaced by a
generic review or successful local CI inspection. Empty selections do not pass.
Other stages retain their own evidence requirements; this run does not execute
or qualify them. Adequate execution plus acceptance establishes the testing
finding, not CI-exclusive delivery, protection, effectiveness or a release.

### Rule-to-evidence responsibilities

This map interprets catalog 2, not a new per-project questionnaire. Use existing
authorities and only update reviews whose facts changed. The catalog remains the
source of exact applicability and gates. For every row, missing observations or
review are `unverified`, a demonstrated contradiction is `failed`, a broken
verifier is `error`, and a recorded implementation gap is `migration_required`.
The rule diagnostic supplies the next action. None may be relabelled a pass to
avoid a blocker.

| Rule(s) | What configuration establishes | Evidence needed at the applicable boundary; invalidation |
| --- | --- | --- |
| OPDEV-AUTH-001, OPDEV-AUTH-002 | Declared authority locations and work owner; structural finding may pass | Agents resolve content, access and conflicts during planning/review; revisit changed routes or unavailable sources. A path declaration is not content review. |
| OPDEV-WORK-001 | Work routing only | Current intended outcome and accepted conditions in work authority; changed scope invalidates review. |
| OPDEV-DESIGN-001 | Design authority only | Proportionate decision and alternative/risk review for the change; changed assumptions invalidate it. |
| MCD-CI-001 | Selected provider or locally inspectable CI definition | Review actual CI evaluation of changes and trunk, plus requested current provider qualification; changed source, effective CI, trust or protection invalidates the affected finding. |
| MCD-TRUNK-001 | Named trunk | Review actual branch/workflow topology; matching default-branch names alone do not prove it. Changed topology or adoption contradictions require reassessment. |
| MCD-TRUNK-002 | Branch convention | Branch origin/integration/lifetime observations and review; new or changed branches require current evidence. |
| MCD-TRUNK-003 | Cadence target | Dated integration history; time changes the finding. Compliance finding only, not a one-day merge/development timer. |
| MCD-TEST-001, MCD-TEST-002 | Stage-assigned commands | Actual selected-stage execution plus current acceptance review; source, tests, configuration, stage, environment or trust changes invalidate execution/acceptance as applicable. |
| MCD-FLOW-001 | Red-trunk policy | Current trunk run/check observations and actual restoration priority; stale or newer failed/pending runs invalidate an older green result. |
| MCD-COMPAT-001 | Compatibility policy | Affected consumer/contract regression and deliberate migration review; changed supported behavior invalidates it. |
| MCD-DELIVERY-001 | Chosen delivery mode/locator | Review the actual single consumer-facing CI path; a configured locator cannot prove absence of alternate delivery paths. |
| MCD-PIPELINE-001 | Pipeline definition | Observed complete applicable delivery sequence for the artifact/configuration; editing the pipeline or producing new bytes requires appropriate new qualification. |
| MCD-ARTIFACT-001 | Artifact kind/locator | Identified deployable bytes and affected packaging checks; new bytes invalidate the artifact result. |
| MCD-ARTIFACT-002 | Build/promotion policy | Immutable digest chain from one build through promotion, not another build with the same source name; changed bytes or provenance invalidate it. |
| MCD-ENV-001 | Environment names | Production-like qualification where runtime environments exist; changed environment/bytes or exceeded freshness invalidates it. |
| MCD-RECOVERY-001 | Recovery strategy | Automated, tested recovery appropriate to the actual affected delivery; changed recovery behavior/artifacts/environment require affected checks, not automatic release preparation. |
| MCD-CONFIG-001, MCD-CONFIG-002 | Versioned configuration/injection declarations | Versioned behavior/configuration tests and observed external-value injection without rebuild/drift; changed inputs or delivery path invalidate affected evidence. |
| OPDEV-TEST-001 | Listed quality risks | Review that test objectives/acceptance cover applicable risks; changed risks/scope invalidate it. |
| OPDEV-TEST-002, OPDEV-TEST-003 | Test policy | Source-bound inventory, actual assertion review and required current-stage execution; mechanically valid mapping does not prove semantic adequacy. |
| OPDEV-TEST-004 | Regression-or-justification policy | Review defect applicability and regression assertions or a specific limitation for this change; a policy sentence is insufficient. |
| OPDEV-TEST-005 | Retry/quarantine policy | Review actual runner/selection/retry controls and visible observed attempts; changed controls or contrary results invalidate it. No universal runner adapter is implied. |
| OPDEV-TEST-006 | Coverage mode, or no declared collection | Actual selected coverage evidence and review of its limits when collected; changed measurement scope invalidates it. No universal percentage. |
| OPDEV-TEST-007 | External test/environment declarations | Current dependency/environment/variability/freshness observations and reviewed effect on qualification; unavailable required evidence remains unverified. |
| OPDEV-EVAL-001 | Effectiveness objective | Independent effectiveness observations, kept separate from correctness and delivery; changed objective/population/model or freshness requires reassessment. |
| OPDEV-SEC-001, OPDEV-SEC-002 | Security authority/CI policy | Attack-surface and actual trust/permission/input-control review plus affected tests; changed inputs, privileges or architecture invalidate affected claims. |
| OPDEV-SUPPLY-001, OPDEV-SUPPLY-002 | Provenance/SBOM policy | Artifact-bound provenance and shipped-component inventory; new bytes/dependencies invalidate it. |
| OPDEV-A11Y-001 | Accessibility target | Applicable automated and human observations; changed user-facing behavior/target requires proportionate reassessment. |
| OPDEV-OPS-001 | Health/diagnostics authority | Observations that signals support delivery and investigation; declared health paths alone do not pass. |
| OPDEV-AI-001 | Agent-use policy | Review actual agent-authored code, assertions, configuration and claims; author's success summary is insufficient. |
| OPDEV-LEARN-001 | Learning/work authority | Review material observed failures and resulting corrective work or justified disposition; new failures trigger reassessment. |
| OPDEV-EXT-001 | Additive extension structure | Engine aggregation enforces no core overrides; extension execution has separate results. Changed extension mechanism requires regression tests. |

The versioned baseline is the catalog identified by the binary and result, with
`opdev-core@1` selecting all its rules. No extension, preference or ordinary
developer choice can waive an applicable core requirement. Conditional controls
use demonstrated capabilities, not project labels or absent configuration:
runtime environments (MCD-ENV-001), behavior/environment configuration
(MCD-CONFIG-001/002), actual coverage collection (OPDEV-TEST-006), external tests
(OPDEV-TEST-007), effectiveness objectives (OPDEV-EVAL-001), untrusted CI
(OPDEV-SEC-002), shipped artifacts/components (OPDEV-SUPPLY-001/002), user
interfaces (OPDEV-A11Y-001) and operated software (OPDEV-OPS-001). A justified
inapplicability is not an opt-out.

All sixteen MinimumCD IDs in the table are retained; none is silently removed
by this verifier correction. Single integration trunk, CI-exclusive delivery,
immutable artifact promotion and tested recovery remain baseline requirements.
Maintenance/support branches are not another integration or release-promotion
trunk. Tool choice, adequate implementation alternatives and proportional test
selection remain preferences within the guarantees. Daily integration retains
its separately reported compliance requirement. Full MinimumCD compliance needs
every applicable MinimumCD result, including cadence; local/integration success
or a partial profile mapping is insufficient. Any future baseline/profile split
must honor the rule-removal and migration policy in [compatibility](compatibility.md)
before changing effective requirements, not reinterpret historical reports.

Agent reports MUST distinguish unattempted actions and predicted permission
requirements from observed denials and completed execution. A denial claim MUST
be supported by the matching invocation's tool result. An incomplete invocation
has an unknown result, not an inferred pass or failure. Before reporting actions
or blockers, reconcile claims with available execution evidence; preserve retry
history and qualify missing context. This is a reporting obligation, not a new
rule outcome, user approval step or project ledger. Do not execute mutations or
bypass permissions merely to obtain evidence for a report.

## Aggregation

A required rule contributes to an aggregate verdict as follows:

| Rule result | Aggregate effect |
| --- | --- |
| `passed` | Satisfies the rule. |
| `not_applicable` | Satisfies aggregation only when applicability evidence is valid. |
| `failed` | Blocks the verdict. |
| `unverified` | Blocks the verdict. |
| `error` | Blocks the verdict. |
| `migration_required` | Blocks compliance and qualified delivery, but may permit local development. |

An aggregate verdict passes only when every applicable required rule is either
`passed` or validly `not_applicable`.

## Gates

Rules may participate in one or more gates:

- `development`: whether ordinary local implementation may proceed.
- `integration`: whether a change may enter trunk.
- `delivery`: whether an artifact may be delivered through OpDev.
- `compliance`: whether the project may claim the selected OpDev or external
  profile.

A broken required trunk pipeline blocks new feature development even when local
development checks would otherwise pass. Repair, diagnosis, rollback, and work
needed to restore the pipeline remain permitted.

## Freshness and identity

### Daily integration target (catalog 2)

Daily integration remains the MinimumCD compliance requirement MCD-TRUNK-003,
but it no longer participates in development or integration decisions. A missed
or unverified day prompts a review of the blocker and the next small tested
change, not a retrospective merge prohibition or an approval exception.
Its actual result remains visible and still blocks a compliance claim when
unsatisfied. OpDev's operational gates therefore do not enforce every MinimumCD
practice: a successful merge gate is not proof of MinimumCD compliance.

Keep one integration branch, small changes and short-lived branches. Do not use
MCD-TRUNK-002 as a substitute one-day timer; assess branch origin, scope, drift,
reintegration and cleanup. All other applicable checks retain their blocking
behavior. This change neither executes a merge nor authorizes delivery.

This corrects a deadlock where historical delay prevented the integration needed
to recover. A generic override mechanism was rejected because it would introduce
approval and exception machinery without improving test confidence. Revisit the
policy if observed branch drift or conflicts increase; do not infer improvement
merely from fewer blocked reports.

The CLI's default exit follows development; `check --ci` follows integration.
With capability `check.post-merge-integration.v1`, `check --ci --post-merge`
executes the selected integrated-trunk suites/extensions and follows integration,
not delivery. Selected blocking post-merge checks contribute to both integration
and delivery, including unavailable reuse evidence. Delivery-only rules remain
visible and blocking for delivery, without preventing otherwise verified
integration. Pre-merge and post-merge executions are not interchangeable.
In post-merge evaluation-only mode, selected checks without verified execution
are reported as `unverified`, not omitted, even without a reuse policy.
`check --ci --delivery` executes the delivery stage and follows the delivery gate;
it cannot be combined with `--no-exec`. Inspect all reported gates before making
broader claims. An integration-only baseline does not enforce the release path.
Gate success is evidence, not developer authorization to release.

This separates integrated-source verification from an artifact's readiness to
ship: coupling both exits previously forced delivery work into ordinary milestone
completion. Reusing the integration aggregate with stage-selected checks keeps
failed post-merge suites and extensions blocking without creating a fifth gate
or weakening delivery rules. Ignoring the nonzero exit or adding a blanket waiver
would also hide genuine failures and was rejected. Regression tests cover exit
selection, all check outcomes, missing execution and rejected reuse; shared
guidance separately requires explicit release authorization. Revisit this split
if a required integrated-source risk cannot be represented by the existing
integration rules and selected post-merge checks, not merely because a delivery
gate remains blocked when release was not requested.

Declaring a trunk name or matching a provider default branch does not establish
the single-trunk workflow. Review branch roles and release source. A known
contradiction in the adoption workflow remains `migration_required` despite a
generic evidence assertion. Likewise project kind or omitted effectiveness risk
metadata cannot establish accessibility, operations or effectiveness exclusions;
absent reviewed capability evidence these rules remain `unverified`.

Evidence is valid only for the subject it identifies. Artifact qualification
MUST identify the artifact digest. Revision-only evidence cannot be silently
reused for a different artifact. Time-sensitive or external evidence MUST state
its freshness policy.

## Extensions

Extensions may produce additional results and may strengthen a gate. They MUST
NOT:

- Replace a core result.
- Change a core rule's applicability.
- convert `failed`, `unverified`, `error`, or `migration_required` to `passed`.
- Suppress required evidence.

