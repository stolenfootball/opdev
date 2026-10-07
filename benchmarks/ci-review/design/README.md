# CI design and improvement scenarios

Reusable synthetic cases for the [CI design contract](../../../spec/ci-design.md).
`cases.json` contains model-visible requests and project facts only.
`review.json` is reviewer-only: never include it in a model prompt or workspace.
No case contains consumer code, personal context or real performance measurements.

Freeze the selected baseline/candidate guidance and identical case inputs before
fresh-context trials. Supply only the selected case and necessary project material;
the cases are compact decision scenarios, not runnable pipeline repositories.
Use existing authorized evaluation tooling with explicit model/effort, input hashes,
attempt/output records, timeouts and unchanged-source checks. Verify actual loaded
context and attempted actions, not just final answers or sandbox-denied writes.
Do not overwrite failed attempts, pool contaminated runs, silently switch models,
or infer delegation/provider authorization from this protocol.

Judge the concrete recommendation against the private criteria. Correctly keeping
an adequate pipeline is a positive result; mentioning many optimization words is
not. Compare false positives, missed material problems, permission/coverage errors,
useful actionable advice, uncertainty and unnecessary developer burden. Report
sample size and limits. A manual walkthrough is semantic review, not a fresh-agent
trial; input-shape tests establish neither.

Actual pipeline trials require separately authorized isolated projects, reviewed
runner/secret policy and owned cleanup. Adapt the executable feedback examples,
then deliberately exercise failures and missing work, not just successful runs.
Keep syntax checks, local collector execution, actual provider scheduling and
branch protection observations separate. Measure queue/setup/check time, first
useful feedback, required verification and summed compute on comparable repeated
cold/warm runs. Do not report scenario timings as measured speed improvements.

Evaluation progress and remaining work belong to
[the tracked evaluation](https://gitlab.com/stolenfootball-tools/opdev/-/issues/79),
not this protocol. Broader design/tooling decisions remain linked from
[the parent](https://gitlab.com/stolenfootball-tools/opdev/-/issues/77).
