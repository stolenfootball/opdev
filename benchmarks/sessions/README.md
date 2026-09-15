# Complete coding-session experiment

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
