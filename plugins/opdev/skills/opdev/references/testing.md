# Testing requirements

Derive tests from behavior, acceptance conditions, and declared quality risks rather than from language-specific quotas.

- Every behavioral change needs automated verification unless existing coverage is demonstrated or a specific limitation is recorded.
- Every escaped defect needs regression protection unless a specific justification explains why it is impractical or harmful.
- Required suites run before integration and again on integrated trunk. Delivery, package, recovery, scheduled, and evaluation suites run at their declared stages.
- A retry remains visible. Quarantine requires an owner, tracked remediation, and an expiry. A flaky or unavailable qualification test cannot silently qualify delivery.
- Coverage identifies untested risk. Use the project-selected mode—reporting, non-regression, changed-code threshold, or critical-module thresholds—without treating percentage alone as test quality.
- Tests that depend on live services, devices, stores, fleets, models, or other external systems must state dependencies, environment, variability, freshness, and whether their result affects deployability or effectiveness.
- Keep deterministic correctness and deployability separate from effectiveness evaluation.

## Proportionate report evidence

During adoption, review the existing runner and CI retry settings, test-selection
filters, allowed-failure controls, and quarantine process. Record the reviewed
controls and developer decisions in the existing testing authority/adoption
record; do not add a parallel policy file. Reuse that review on ordinary tasks,
but revisit affected controls when runner configuration, selection, retries,
quarantine or CI behavior changes, or new failures contradict the review.

Canonical command outcomes are primary. Do not require a particular test runner
or report format. A successful command exit alone does not establish test
counts, complete selection or hidden retry history; review actual controls and
report the scope of the evidence accurately. Do not waive a known failure.

On a CLI that supports `test-execution --help`, `test-execution --suite SUITE`
can collect tool-neutral command/source observations from clean committed source.
It runs the suite once, is separate from ordinary checks, and does not qualify
a gate. Do not run it redundantly merely to generate more paperwork.
`test-report inspect` is an optional read-only JUnit diagnostic, not required
evidence or gate enforcement. Neither command establishes complete retry history.

Do not build runner-specific adapters or demand complete-history attestations
by default. If the project explicitly requires stronger evidence, use its existing
blocking extension checks and reviewed acceptance criteria; unavailable required
evidence stays unverified. Do not waive core rules or stronger selected checks.

Run only canonical commands relevant to the current stage. Preserve their bounded output as evidence, and distinguish a product failure (`failed`) from a verifier failure (`error`). Keep commands as exact argument vectors. On Windows, OpDev may resolve the allowlisted Node package-manager names through `PATH` and executable `PATHEXT` shims; do not replace that constrained behavior with a hand-built shell command.
