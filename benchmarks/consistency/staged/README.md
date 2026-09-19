# Staged-review prototype

This is an explicitly invoked benchmark, not a distributed plugin runtime or a
new developer requirement. It tests whether candidate generation plus one fresh
verification call improves review quality enough to justify cost. No release,
new CLI command, host permission change or automatic adoption is included.

The [completed pilot results](results-2026-09-18.md) do not justify default
integration. This frozen prototype has documented candidate-binding, startup-error
and failed-call accounting limitations. Address those offline before approving
another live run; do not treat the commands below as a production review service.

## Frozen pilot design

Two new cases were authored independently from the candidate/verifier prompts.
Each host reviews the same source packet under three arms: ordinary concise
review, the current single-pass #46 reference, and staged candidate/verification.
The reviewer-only `rubric.md` is never sent to actors. Run once per case/arm/host:
12 initial calls and at most four verification calls. No prompt changes or retries
after seeing results. This small pilot is not a reliability estimate; it is a
decision point before production integration. It lacks a repeated-trial sample.

Source packets eliminate a known retrieval permission confound without bypassing
permissions. This does **not** test native subagent routing, autonomous authority
retrieval, plugin installation, or a host permissions repair. Those need separate
read-access preflight and end-to-end validation if this design earns adoption.

Use host defaults, existing authentication and read-only/answer-only restrictions.
Each call starts an isolated session in an empty temporary directory outside the
project. Staged verification receives original evidence and candidates, never the
initial model's conversation or private reasoning. It cannot add candidates or
repeat itself. Missing or malformed outputs remain errors, not empty passing
reviews. No cross-provider reviewer or second provider account is required in
the proposed product design; both hosts here are evaluated independently.

## Mechanical boundary

`review_contract.py` uses only Python's standard library for this prototype.
It validates an automatically produced snapshot identifier, source IDs and exact
excerpts, required fields, unique candidates and one explicit assessment per
candidate. It retains missing evidence and separates supported findings from
uncertainty. Its renderer does not ask another model to rewrite verified claims.
It cannot prove source authority, complete coverage, consent or semantic truth.
The tests deliberately include a false semantic claim with mechanically valid
references to document that limit. Developers maintain no new schema or files.

No Python dependency is added to OpDev's shipped native runtime. If the approach
is adopted, qualifying a small Rust implementation and native host integration
is separate work; do not ship the benchmark controller as a review service.

Run the offline mechanical tests:

    python -m unittest discover -s benchmarks/consistency/staged -p test_review_contract.py -v

After explicit authorization for the synthetic payload and provider calls:

    python benchmarks/consistency/staged/pilot.py codex --run-id pilot-1 --allow-synthetic-host-calls
    python benchmarks/consistency/staged/pilot.py claude --run-id pilot-1 --allow-synthetic-host-calls

Run IDs are create-new; reuse refuses to overwrite evidence. Inputs, complete
prompts, host versions, raw events/stderr, answers, structured outputs, rendered
reports and call metadata go under ignored `target/consistency-staged/`. Temporary
host workspaces are recorded in each request. Review raw traces before any public
sharing; only synthetic, reviewed summaries belong in the tracker.

## Compare outcomes

Grade material true findings, false positives, missed issues, scope and revision
accuracy, unknown handling and prohibited actions separately. Do not fail a useful
finding merely for different wording, or call a correct source inference invalid
solely because it was not executed. Retain candidate and verifier judgments to
see whether verification removed true findings, kept false ones, or narrowed claims.
An empty answer cannot win by suppressing recall. Use human-reviewed ground truth;
the acting verifier is not the benchmark grader.

Record wall time and provider-reported token categories, cache use and cost when
available. Null/missing usage is unknown, not zero. Do not compare unlike token
fields across hosts or turn subscription cost estimates into actual charges.
Require an observed material benefit over both baselines before claiming the
extra stage is worthwhile; close ties favor the simpler approach. Review any
proposal to ship separately from permission to run the benchmark.
