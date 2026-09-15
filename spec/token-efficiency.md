# Token-efficiency evaluation

OpDev effectiveness evaluation MUST remain separate from correctness and
qualified delivery. A reduction in exposed text cannot establish reduced total
model usage, cost, latency, or equivalent task completion.

## Initial experiment

The repository benchmark in `scripts/opdev_benchmark.py` evaluates context
interpretation, not the complete development workflow. Versioned synthetic cases
live in `benchmarks/token-efficiency/cases.json`. They contain all six rule
outcomes, current evidence located before newer nonmatching entries, missing
current evidence, and authority paths that conflict with conventional folders.

Each case has a full and selected input, the same question, and an exact JSON
answer oracle. Selected inputs are experimental fixtures, not production report,
evidence, or context implementations. Tests independently derive the expected
answers from full facts. Case answers are never included in model prompts.

The benchmark MUST preserve non-satisfying outcomes and missing evidence. It
MUST NOT count an agent's success claim as acceptance. Structured answers are
compared against the oracle, including required keys and exact outcomes.
Synthetic gate labels use `blocked` to describe a non-satisfying aggregate;
they do not introduce a new OpDev rule outcome or report schema.

## Measurement contract

Static measurements identify exact rendered prompt bytes and hashes. Optional
`tiktoken` counts identify encoding and library version and are proxy counts,
not provider billing. Missing token counts remain null.

Live records identify the case, arm, repeat, prompt and event hashes, host
version, requested model and effort, experiment settings, elapsed time, process
status, observed usage, and oracle outcome. The initial Codex adapter accepts
exactly one completed turn with no tool calls. Extra turns, tool use, timeout,
and host errors invalidate the trial. Missing or inconsistent usage is
unverified, never zero. The adapter treats cached input as a subset of total
input and does not add it twice. It does not infer prices or dollar savings.
Reported cache-write and reasoning counters are retained separately, without
adding detail counters to the input/output totals. Missing optional counters
remain null and cannot support a claim of complete billing detail.

Trials use a seeded balanced randomized schedule. A new process is not proof
of a cold provider cache; observed cached usage MUST be retained. No automatic
retry hides a failed attempt. A host error stops the remaining schedule and
leaves an incomplete comparison. Explicit trial count and wall-time limits
bound calls and duration, not dollar spending or exact token consumption.

Comparison groups MUST have matching experiment identities and paired case/
repeat membership. Failed attempts remain in total usage per accepted task.
Missing usage prevents a complete cost denominator. A savings figure is emitted
only for complete pairs with all oracle checks passing. Small pilots remain
exploratory, even when every answer passes. Full development effectiveness
remains unverified regardless of microbenchmark results.

Raw events and stderr are private local artifacts by default. Review them before
sharing; host diagnostics can include local paths and account details. Public
fixtures contain no repository secrets or production evidence. Live trials are
opt-in, use existing host authentication, and never run in required CI. Offline
harness tests run through the canonical workspace suite on POSIX CI, where
Python is already a declared test dependency.

## Design decision

Start with a standalone standard-library Python harness and synthetic fixtures.
This separates the measurement mechanism from the optimization being evaluated,
fits existing repository test tooling, and avoids changing plugin behavior
before a baseline exists. A versioned host event adapter is narrower and easier
to audit than importing a general agent evaluation framework. Optional token
counting does not add dependencies to the CLI or canonical test suite.

Alternatives considered: static counts alone cannot show interpretation quality
or live usage; immediate end-to-end autonomous replay mixes host setup,
repository mutation, billing, and grading before the accounting is tested.
Microbenchmarks sacrifice realism and do not exercise plugin discovery or
activation. Reverse or extend this choice when real task results diverge from
microbenchmark rankings, event schemas change, or worker accounting is needed.

## Next evaluation stages

Before selecting production designs, compare unchanged baseline, thin entry
instructions, and thin instructions plus CLI projections on identical fixture
repository revisions. Keep CLI enforcement and canonical commands identical.
Use independent acceptance tests and inspect filesystem changes, evidence
fingerprints, and report outcomes. Include documentation edits, regression
repair, multi-file changes, failing CI, stale evidence, conflicting folders,
non-development prompts, and resumes after authority changes.

Record exact installed plugin/CLI identities, generated repository instructions,
model/host configuration, all worker usage, tool turns, cache behavior,
compactions, repair rounds, interruptions, and runtime environment. Separate
fresh, warm, and resumed sessions. Do not infer a missing worker's usage from
the coordinator. Randomize order and repeat each task/arm (initially five);
report unsuccessful runs, medians, tail latency, and cost per accepted task.

Use explicit run/cost budgets and controlled workspaces for live task evaluation.
Choose a practical improvement threshold after the baseline distribution is
known. Require no observed acceptance regression or false gate passes. Zero
observed defects in a sample is not proof of zero risk. This broader benchmark
and all production optimizations remain follow-up work under the tracker.

## First coding-session experiment

`scripts/opdev_sessions.py` and `benchmarks/sessions/` extend measurement to
single-agent sessions with actual file edits and tool calls. The baseline and
compact arms use the same executable and repository protocol. Only inspection
guidance differs; baseline agents may select fields without loading whole files.
This is an explicitly activated repository-protocol experiment, not a test of
plugin installation or automatic discovery.

Independent acceptance runs outside agent checkouts, verifies behavior, retains
original tests, and checks added tests against the original defective package.
The controller compares final gate claims to a fresh evaluator report. It records
each candidate on a private GitLab branch and waits for CI. No branch is promoted
as a qualified release. Controller CI time is separate from agent elapsed time;
model-driven CI repair and resumed sessions are outside this first experiment.

The complete tool-using invocation must have exactly one terminal usage record.
The adapter rejects ambiguous multi-turn usage, unknown worker events, missing
counters, host errors, and sessions with no successful commands. Failed product
attempts remain in token totals. A host/accounting failure stops subsequent
trials; it cannot be silently retried or excluded to produce a savings claim.
Raw traces are private and the versioned results contain reviewed aggregates.
