# Complete coding-session experiment

## Workflow feedback follow-up (separate protocol)

Issue #60 evaluates workflow changes independently of the historical compact-view
experiment below. Use the existing neutral parcel fixture, session accounting and
independent behavior oracle; do not change historical results or experimental
defaults. The existing grader freezes whole original test bodies (#28), so new
trials must keep those bodies unchanged and add regression tests separately.
It is intentionally stricter than semantic equivalence; retain that limitation.

Before candidate trials, freeze `feedback-protocol.json` in private experiment
storage: schema 1, `identities` (baseline/candidate source, CLI/plugin/guidance
digests, fixture, host/model/effort), `schedule` (case, fresh/resumed context, arm,
repeat), budgets and criteria. Use small fix, CI-only diagnosis, stale evidence
and partial-upgrade cases; contrast genuine native platform risk. Preselect a
maximum feedback-time ratio of 0.85 and completion-time ratio of 1.05, conditional
on all independent behavior, test-preservation and gate-honesty oracles passing.
Run/inspect baseline before candidate; failed or incomplete trials stay recorded.
Do not rerun failures away or pool a pilot into a measured experiment.

Append observations to `feedback-records.jsonl`: schedule identity, exact protocol
SHA-256, outcome, `oracles` (`behavior`, `test_preservation`, `gate_honesty`, each
true/false/null), local `observation_file` basename and digest, and `metrics`.
Metrics are `first_feedback_seconds`, `completion_seconds`, `agent_seconds`,
`command_seconds`, `queue_seconds`, `commands`, `pipelines`, `evidence_edits` and
`failed_attempts`. Measure only observable intervals/counts; unavailable values
are null. Queue, execution and agent intervals can overlap and are not additive.
Retain raw session events privately; counts from command text are proxies, not
automatically observed provider jobs. Actual provider observations supply pipeline
and queue measurements. Review transcripts and changed files independently of
agent claims. A false pass, weakened test or wrong artifact fails acceptance.

`python scripts/opdev_sessions_report.py PRIVATE_DIRECTORY --feedback` validates
schedule/protocol/observation bindings and prints aggregate observations. Missing
comparison metrics, unbalanced/incomplete trials or an oracle failure prevent a
speedup claim. The exporter does not authenticate reviewer assertions. No new
agent framework or mandatory project telemetry is introduced. Live Claude remains
deferred while unavailable; Codex-only findings cannot qualify Claude or general
software-development effectiveness. Fresh answer-only planning probes establish
reasoning in those scenarios, not complete coding-session speed.

## Historical compact-view experiment

This opt-in experiment compares the existing OpDev report/evidence workflow with
compact report and current-evidence guidance. It runs agents against three
small but executable Python changes, five independent repetitions per arm.
The implementation, canonical tests, CLI bytes, fixture revision, model and
effort are identical across arms. Only inspection guidance differs.

The experiment measures a fresh session from investigation through staged code,
agent-written regression tests, local validation and final gate reporting. A
controller independently grades each checkout and pushes its commit to a private
GitLab branch for CI. Controller CI time is reported separately from agent time.
This first experiment does not measure model-driven CI repair, installation,
multi-agent workflows, human interruptions, resumed sessions or compactions.

The [first measured results](results/2026-09-15-codex/README.md) retain all 30
trials, including one strict test-preservation rejection. They do not establish
acceptance-equivalent savings.

The Windows workspace sandbox protects Git metadata. Both arms request staging
through an ignored `.benchmark/stage.request` marker. A trusted controller checks
the changed path allowlist, stages only task-owned paths and returns a receipt.
It offers no arbitrary command execution. Reports go in the ignored `.benchmark`
directory. This instrumentation is identical across arms and its model/tool
overhead remains in measured usage.

The baseline can use ordinary human or JSON reports and read/select the original
ledger. The compact arm receives the shipped compact-view references and may
retrieve full diagnostics. Neither arm is forced to read a whole document.

## Preregistered settings

- Cases: weight-rounding defect, express service across API/receipt/CLI,
  consumer documentation corrected against canonical authority with stale evidence.
- 30 trials: two arms, three cases, five repeats, seed 17.
- Codex 0.154.0, requested gpt-6-astra, medium effort, fresh ephemeral process,
  ignored user config, workspace-write sandbox, Windows elevated sandbox adapter.
- Maximum 600 seconds per trial. Stop before the next trial when measured usage
  reaches 10 million tokens; this is a between-session budget, not a hard cap on
  an in-flight request. No dollar-cost estimate or cap is claimed.
- No automatic model retries. Infrastructure/accounting errors stop the schedule.
  Product failures remain in the denominator and do not stop balanced trials.
- Pilot trials are recorded separately and never pooled into the experiment.

## Independent acceptance

The controller owns acceptance tests outside agent checkouts. It exercises
boundary weights, invalid inputs, API/CLI compatibility, express receipts, and
documentation accuracy as appropriate. Bug/feature trials must add passing
tests that fail against the original implementation. Protected source, policy,
CI, evidence history and project instructions may not be modified. Agents must
report the actual four gate verdicts and cannot invent current evidence.

All model usage, including failed trials, contributes to tokens per accepted
task. Cached input is a subset of input, not an additional charge. Missing usage
is unknown. One terminal turn event includes the complete tool-using session;
multiple completed events are rejected rather than guessing aggregation.
Delegation is prohibited because this adapter cannot verify worker accounting.

Raw events remain private local artifacts. Publish only reviewed aggregates,
hashes and source/CI identities. The fixed seed and manifests make the experiment
reproducible, but provider cache state and model alias resolution are uncontrolled.
Do not infer general software-development effectiveness from this small fixture.

## Reproduce

Use authenticated `glab`, Git, Python 3, a Codex 0.154.0 executable and an OpDev
build with compact views. Create an empty private GitLab project first. The
following placeholders must be replaced with actual paths and project identity:

```text
glab repo create GROUP/EXPERIMENT --private --skipGitInit --defaultBranch main
python scripts/opdev_sessions.py prepare --project GROUP/EXPERIMENT --output PRIVATE_OUTPUT --opdev OPDEV_BINARY --codex CODEX_BINARY
python scripts/opdev_sessions.py run --output PRIVATE_OUTPUT --opdev OPDEV_BINARY --codex CODEX_BINARY --label pilot --pilot
python scripts/opdev_sessions.py run --output PRIVATE_OUTPUT --opdev OPDEV_BINARY --codex CODEX_BINARY --label measured
python scripts/opdev_sessions_report.py PRIVATE_OUTPUT/measured --verify-checkouts --output REVIEWED_RESULTS
```

Create work item 1 in the fixture project with the scope from its handbook before
running agents. Verify the seed pipeline is green. Inspect pilot outcomes before
starting measured trials; a stopped or invalid pilot is not permission to pool
its results into the comparison. Output labels and export directories must be
new. `refresh` is an explicit pre-experiment fixture update used during pilot
setup, not a way to change fixture revisions during a measured schedule.

The exporter verifies raw event hashes, terminal counters and checkout identities,
then exports fingerprints and reviewed fields without host paths or transcripts.
Keep the source traces private for audit. A new CLI/host, fixture, prompt or
accounting implementation requires a separately identified experiment.
