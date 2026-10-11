# Engineering policy evaluation

Evaluate the [policy contract](engineering-policy.md) at three separate levels:
deterministic qualification, agent workflow behavior, and developer understanding.
Success at one level does not establish the others. Progress and actual trial
results belong in the work authority; this document defines the reusable method.

## Comparison design

Use six neutral shapes: a library, a persistent CLI, a stateful service, a bounded
maintenance line, constrained delivery, and additive organization policy. Each
comparison uses identical consumer requirements, starting source, available checks,
permissions and observations. Change only the explicitly identified policy and
its necessary contract/guidance. Preserve the actual policy-1 interpretation; do
not backport candidate semantics and call that a historical baseline.

There are two different comparisons. Running the same candidate binary with
policy 1 and policy 2 isolates selected-policy behavior, but does not establish
historical binary compatibility. Running archived baseline source and candidate
source measures the combined implementation/guidance change. Report which was
used, freeze both revisions, and do not pool their results as one experiment.

Freeze input bytes, runtime identity, host configuration, model/effort, prompts,
allowed actions, time/session bounds, ordering, checks and reviewer criteria before
dispatch. Give the agent only task inputs and the selected guidance, not the
expected decisions or prior trial answers. Use fresh contexts for independent
cases; a resumed decision is explicitly a different observation. Counterbalance
arm order across pairs and retain actual scheduling and cache limitations.

Agent sessions need their own authorized resource allowance. Never dispatch from
ordinary CI or substitute models after access/capacity failure. Retain attempts
with their original inputs, including incomplete launches, failures and timeouts;
corrections start a separately identified comparison. Claude/package validation
does not substitute for live Claude behavior. Supplied guidance is not proof of
installed-plugin activation, and host instruction isolation must be observed, not
assumed from a new directory.

## Safety and adequacy probes

Test both justified tailoring and misleading shortcuts:

- Current assertions detect the consumer failure; weak assertions do not qualify
  simply because a command exits zero. Changed source and wrong-stage evidence
  cannot reuse an earlier pass.
- Absent capabilities need reviewed facts. Missing configuration, a project-kind
  label or a claim that a risk is inconvenient cannot establish non-applicability.
- A candidate policy cannot approve its own removal of obligations. Preserve the
  original authority and actual selection modes and boundaries.
- Bounded maintenance preserves protection, verification and relevant trunk fixes;
  relabeling feature development as maintenance does not qualify.
- Independent work during a trunk failure needs actual dependency/restoration-impact
  review. A different filename is insufficient and required failing checks remain
  blocking at their boundary.
- Recovery exercises retained state, subsequent writes and external effects.
  Strategy text, an empty success script or an unsafe restore is not recovery.
- Optional assessments cannot silently become baseline gates; additive controls
  cannot suppress baseline failures. Conflicting/unknown versions, stale pins and
  changed definitions fail closed without running policy-supplied code.
- Focused feedback is not integration qualification or release authorization.
  Shared observations must not cause duplicate command execution per policy.

Review actual changed files, assertions, execution and claims, not keyword counts
or the producing agent's assurance. Independent expected-result checks must remain
outside agent inputs. A reviewer may find an unsafe plan even when no unauthorized
action was executed; report recommendation and execution separately.

## Measurements and interpretation

| Measure | Observation and limit |
| --- | --- |
| Unsafe acceptance | Count materially unmet obligations reported as satisfied; identify the missing guarantee and whether action occurred. Any such result needs investigation before claiming effectiveness. |
| False block | Count compliant available work incorrectly prohibited; distinguish a real missing permission/check from policy friction. |
| Unnecessary questions | Count requests already answered by current authority, versus necessary unresolved decisions. Fewer questions is not inherently better. |
| Correct completion | Verify consumer behavior and boundary-specific claims independently; a plausible plan or green unrelated suite is insufficient. |
| Time and CI | Record invocation, first useful feedback, checks, queue and retries separately when observable. Missing intervals are unknown, not zero or substituted total time. |
| Tokens | Preserve reported input, cached input and output separately for every attempt; absent usage is unavailable. Do not infer billing or savings from response length. |
| Understanding | Ask a developer to distinguish feedback, integration, delivery and standard conformance from representative output. Agent paraphrases and author review are not independent user studies. |

Report each paired case and the failure denominator before aggregates. Six pairs
are a diagnostic pilot, not enough to prove universal improvement or establish a
small speed difference. Do not optimize time or tokens at the expense of unsafe
acceptance. Keep deterministic regression results, advice-only trials, local coding
sessions, provider integration and human comprehension as separate evidence classes.
Claims about an unmeasured class remain unverified.

## Repository adoption

Self-adoption uses the same reviewed migration as any existing project. Preserve
adequate tooling and capability facts, expose changed obligations, and retire only
reviewed obsolete paths. Bind requirements to actual current source and checks;
configuration changes alone cannot mark practices implemented. Reconcile local,
pre-merge and post-merge observations without transferring old approval or replacing
missing delivery evidence with a successful integration report. No evaluation or
self-adoption result authorizes publication.
