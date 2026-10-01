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
