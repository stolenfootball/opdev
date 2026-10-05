# Workflow usability verification

Scope: [issue 52](https://gitlab.com/stolenfootball-tools/opdev/-/issues/52) and
[UX-01 through UX-07](../../spec/workflow-usability.md). These are neutral fixtures,
not consumer-project transcripts or a universal performance benchmark.

## Deterministic regressions

- `crates/opdev-cli/tests/acceptance.rs`: an unindexed archive remains blocked
  and is named in WORK-001/TEST-002/TEST-003; absent, undeclared and wrong-stage
  mapped suites have distinct diagnostics. Existing mutation/staleness, assertion
  adequacy, failed/error and no-execution regressions remain required.
- `crates/opdev-ci/src/lib.rs` and `gitlab.rs`: header/pipeline documents,
  nested/repeated includes, map merge/array replacement, overridden controls,
  malformed documents, cycles, linked/nonregular/escaping files and input bounds.
  Unsupported interpretation remains unverified, not a false missing-control pass.
- `crates/opdev-cli/tests/doctor.rs` and `upgrades.rs`: included version inventory
  and included-file edits invalidating the reviewed upgrade without writes.
- `crates/opdev-cli/tests/adoption.rs`: schema-2 adoption does not imply project
  policy; plan/status report missing schema-2 project qualification with no
  verification claim or writes; requested remote verification stops before suites.
- `crates/opdev-engine/src/command.rs`: a missing executable retains its actual
  program/directory/error, without shell fallback or broader-permission retry.
- Hook/runtime and copied-guidance tests verify resource routing, preservation
  and installer outcomes. They do not prove model classification from keywords.

## Windows observations, 2026-10-01

PowerShell `Restricted` rejected candidate script lookup with exit 1;
`Bypass` limited to the child process resolved the existing managed runtime with
exit 0. Persistent policy was unchanged; organization Group Policy was not
overridden. Offline installer fixtures passed on Windows, including absence,
damage, compatibility and bounded cleanup failures.

Codex CLI 0.154.0 `sandbox --permission-profile :workspace` ran the same
`git --version` canonical argument vector through OpDev successfully. The direct
and OpDev probes used a neutral fixture. This is scoped positive evidence, not
proof for other executables, host versions or policies. The originally reported
`just` launch failure was not reproduced: that executable was unavailable in this
environment. No launch-architecture or permission-bypass repair was inferred.

## Fresh-context semantic review, 2026-10-01

Codex CLI 0.154.0 used its configured default model; no model override. Candidate
skills/references and managed instructions were supplied, not expected answers.
Each advice-only trial used a separate fresh context. Actual answers were reviewed
against `tests/planning-review.md`, not scored by phrase matching.

| Scenario | Observed behavior |
| --- | --- |
| External runner diagnosis | Classified external operations; no development gates/runtime merely because it serves CI; activation on repository changes. |
| Repository-owned CI image change | Applied retained development, scoped compatibility checks and current source-bound evidence; no adoption restart. |
| Retained packaging/startup/recovery called a preview | Required affected package/recovery and CI candidate qualification; no label-based exemption or manual publishing. |
| Library/service/desktop/CLI milestone without release demand | Proposed consumer feedback/next outcomes and applicable candidates; no automatic version/tag/registry ceremony. |
| Existing GitLab adoption, project schema 1/adoption schema 2 | Distinguished the two schemas and requested early observation/actual policy decisions before implementation approval. |

Advice-only results establish interpretation in these sessions, not actual
infrastructure, release or research execution. The generic source questions were
provided alongside current guidance. Default host/personal instructions may still
affect answers; this is not a controlled model comparison.

## Coding-session canary

The local neutral Python parcel-display fixture began with `2 kg to local`.
The first request explicitly asked for two disposable layouts and feedback before
retention. Codex created and ran both under ignored `_preview/`, left product
source/tests unchanged and stopped for feedback. First-preview turn took 46.2s;
there was no production ledger rewrite, full-gate invocation or release action.
This is one observed duration, not a measured speedup.

The next request chose the one-line layout, required exact output
`PACK: 2 kg -> local`, positive weights/nonempty zones, and ValueError for zero,
negative weights or empty zones. In a fresh context with the same checkout,
Codex derived six tests, observed red against old behavior, changed only the
allowed product/tests, then observed green through the declared canonical command.
It ran checks and reported all unresolved project gates rather than claiming
completed adoption. No tag/publication/contract-policy mutation occurred.

Git staging inside that Codex sandbox was denied at `.git/index.lock`, so that
session could not bind production evidence. The denial remained visible; neither
host success nor full adoption is inferred. Controller and CI verification are
separate from that host result. This two-step canary crosses a context reset; it
does not claim uninterrupted chat history or a qualified release.

Candidate CLI digest used in the host sessions:
`abc9f5ea12e1642adeba6725568892d604436b60d104e4f85486edae94e63099`.
Raw neutral outputs and per-input package hashes are retained
locally; review/redaction precedes sharing. Do not publish private consumer logs.

Live Claude execution is deferred at the maintainer's request because account
access is unavailable. Claude package validation and hook/resource fixtures pass;
they do not replace a future live Claude trial. There is no model substitution.

## Provider worksheet and interface review

Read-only worksheets on the OpDev provider repositories observed GitLab trunk,
seven candidate jobs and one protection rule, and GitHub trunk with six jobs.
GitHub protection returned HTTP 404, retained as a diagnostic. Both worksheets
remained unverified, policy unselected and approval review-required. No provider
settings or project decisions were written. Existing remote audit bounds apply.

The maintainer approved the new not-run/local-versus-remote/delivery labels and
condition/suite/stage diagnostic wording in the implementation conversation on
2026-10-01. This is a plain-text clarity review, not assistive-technology or host
interface conformance. Required canonical and pre/post-integration CI checks
remain separate evidence. No release or managed-runtime pin change is requested.

## Faster-feedback follow-up, 2026-10-02

Roadmap #61 tracks issues #55–#60. Deterministic CLI canaries now verify that
execution previews do not launch commands, that selected distinct checks execute
once even with identical argv, and that stage/failure semantics are preserved.
Existing-ledger preparation tests cover unresolved reviews, changed mappings,
source/excerpt/ledger staleness, malformed input, history and current-assertion
preservation. Doctor/upgrade fixtures distinguish disk guidance from unknown
session state and preserve compatible selection and custom content.

Private neutral provider fixtures exercise `examples/feedback/`, never consumer
code. GitHub run 36973345884 passed verify/candidate/consume on the same initial
revision; PR run 36973477055 ran verification with candidate/consume skipped and
no separate feature-push run. GitLab pipeline 2905587103 passed the same three-job
flow; MR pipeline 2905589439 passed verification with no ordinary feature-push
pipeline. GitLab initial job execution/queue seconds were respectively
47.43/6.86 (verify), 48.16/20.33 (candidate), 4.29/35.47 (consume). Post-merge
trunk runs GitHub 36973740215 and GitLab 2905594760 passed the full chain again
for the integrated revision. These are job observations including setup, not
total agent time or a comparison speedup.

Explicit diagnostic GitHub run 36973470538 observed Linux/Python 3.12.3 and
`alpha beta`; its result remained `qualification: unverified`. GitLab diagnostic
pipeline 2905591609 succeeded before any diagnostic-branch integration/package
cycle, observing Linux/Python 3.13.16 and the same unverified diagnostic result.
The first GitLab attempt was rejected for an omitted stage (failed push
pipeline 2905590697 and HTTP 400 on manual creation); the explicit stage and a
regression were added before retrying. No product qualification was inferred.
The probe creates no resources; cleanup is not simulated as if resources existed.

The fresh-context Codex baseline pilot used CLI 0.154.0 and its configured model,
with frozen baseline source d19cd393faf831fe404ebb30788e911aee38ee17 and candidate
planning input. It failed before an answer because the CLI's ChatGPT login rejects
configured `gpt-6-sol`. Elapsed time 3.12s is a failed launch, not feedback latency.
The failed event hash is
`8b68014d52374fe2cf8731c265cbccd032b120160d2b68b7c646ae7c920e2562`.
No candidate trial or performance comparison ran; model choice was requested,
not silently substituted. Live Claude remains deferred. Planning scenarios F1–F4
and the separate feedback report protocol are prepared; live semantic and complete
coding-session acceptance remain unverified until access/model selection permits
a fresh baseline. Historical compact results and defaults are unchanged.

### Authorized model follow-up

The maintainer subsequently selected `gpt-5.6-sol`. A separate 90-second,
zero-retry answer-only protocol ran baseline and candidate F1–F4 once each on
Codex CLI 0.154.0. All eight turns completed without timeout or tool actions.
Protocol SHA-256:
`eb57d993e209a7e3b1b26ad71e467edf50bb6ccddaedb4bb0ce4ffc1c222cccf`.
Raw prompts, events and records remain in local `opdev-feedback-plan/trials-5-6-sol`.
The earlier failed default-model attempt remains part of the history above.

Semantic review of the actual answers found that both arms chose direct focused
feedback for F1, consolidation for F2, preservation of native environments for F3,
and a bounded secret-free reproduction for F4. Candidate F1 explicitly refused
silently bypassing a required integration gate; any proposed policy change would
still need review and could not waive core requirements. Candidate F2 retained
distinct environment coverage while consolidating interdependent changes. F3 kept
both platforms despite identical commands. F4 bounded the Linux probe without
production runner mutation. Wording occasionally described recommendations as
imperatives, but no implementation was attempted in these advice-only sessions.

These samples support the guidance's interpretation, not a causal improvement,
perfect planning reliability, actual CI execution or complete-session speedup.
The separate [CI advisory evaluation](../ci-review/README.md) adds write-authorized
and resumed-decision probes. Live Claude remains explicitly deferred.

### Fresh/resumed task observations, 2026-10-05

The `follow-up-cases.json` fixtures cover CI-only diagnosis, stale evidence and
partial upgrades. A separate batch reused the original neutral parcel fixture
for the small rounding fix. Both freeze baseline `d19cd393faf831fe404ebb30788e911aee38ee17`,
candidate references, Codex CLI 0.154.0, `gpt-5.6-sol` medium effort, 150 seconds
per turn, no automatic retries and the 0.85 feedback/1.05 completion thresholds
before comparison. Fresh/resumed turns share each checkout; a supplied developer
update in the partial-upgrade resume is distinguished from the older disk snapshot.
This tests supplied guidance, not installation or automatic skill activation.

The three-scenario protocol SHA-256 is
`29002f2a7ba3179e09b1b05961b1be3f56a578564c8097255ba7299a13332e76`;
the parcel protocol is
`624b124dd7a548a6cc337e6eb36358477e2f2916b308459cc61925e6e0cd166b`.
Raw inputs, context probes, events and before/after hashes are retained locally
in `opdev-workflow-followup-20261005` and its `-small` sibling. Installed OpDev
skills were excluded using child-process settings and checked in fresh context;
other host instructions were inherited, not claimed hermetic.

Both parcel arms completed fresh and resumed turns. Independent boundary/API
assertions passed, original test bodies were preserved, final suites passed and
new tests detected the original rounding defect under an in-memory substitution.
Candidate resumed reporting explicitly kept integration unverified and delivery
unqualified without remote evidence. These are complete local task turns, not
qualified remote integrations or releases.

The baseline completed all six other turns: it diagnosed case-sensitive import
failure without edits, refused Windows-only Linux qualification, corrected returns
to 14 days while preserving the 7-day damaged-parcel deadline and historical
evidence, and distinguished compatible CLI selection, disk guidance and unknown
session state during partial upgrade. Candidate fresh/resumed CI diagnosis also
completed. Its next fresh stale-evidence turn failed before work with the model
capacity error, so the controller stopped. Candidate stale-evidence/resume and
partial-upgrade coverage remain incomplete. The error is retained, not a zero-time
success or omitted sample.

No speedup is claimed: these samples are small, the broad schedule is incomplete,
and first-useful-feedback, queue and command intervals were not independently
measured. Total invocation time is not substituted for those missing metrics.
Failed pilot/setup attempts remain separate rather than pooled into favorable
comparisons. Historical token-saving defaults and results are unchanged. Final
acceptance and integration remain pending; live Claude is still deferred.

### Explicit later retry, 2026-10-05

After the scheduled attempt again failed for model capacity, the maintainer
explicitly requested another retry. The missing candidate stale-evidence and
partial-upgrade scenarios ran under the unchanged frozen three-scenario protocol,
with separate raw records in `opdev-workflow-followup-20261005-manual-retry`.
No failed record or input was replaced. All four turns completed successfully.

Independent file checks confirmed that only the returns README changed: 14-day
returns, preserved 7-day damaged-parcel reporting, unchanged contract and historical
evidence. The candidate explicitly rejected using that evidence for integration.
Its attempted Git diff check errored because this fixture is not a Git repository;
it disclosed that failure and used a direct consistency check instead.
The partial-upgrade files stayed unchanged. The candidate preserved the compatible
CLI selection, distinguished reported guidance application from independent
verification, and kept active-session state and CI qualification unverified.
Reviewed actions did not install, migrate, contact a provider or manufacture evidence.

| Later candidate turn | Invocation seconds |
| --- | ---: |
| Stale evidence, fresh | 74.22 |
| Stale evidence, resumed | 10.35 |
| Partial upgrade, fresh | 58.11 |
| Partial upgrade, resumed | 16.85 |

These are invocation durations, not separately observed feedback, queue or command
intervals. They do not establish the preregistered improvement threshold; no speedup
is claimed. The planned neutral task coverage now has successful fresh/resumed
observations on both arms, alongside the retained failed attempts. This establishes
scoped behavior and reporting observations, not a clean uninterrupted performance
experiment. The subsequent CI-handoff wording correction and its separately frozen
semantic trials are recorded in the [CI follow-up](../ci-review/2026-10-05-follow-up.md).
Source-bound acceptance and integration remain separate from these local trials.

The maintainer reviewed the new inspection/draft/application wording in the
implementation conversation and replied "Looks good to me" on 2026-10-05.
This records human clarity review, not assistive-technology conformance or a gate
approval. Source-bound automated and integration checks remain required.
