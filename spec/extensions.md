# OpDev extension protocol 1.0

OpDev v1 supports project-owned command checks as an additive extension mechanism. Extensions are declared in `.opdev/project.yaml`, reference a canonical command by key, and run only at their declared lifecycle stage. The CLI launches each argument vector directly without a shell, applies the declared timeout, contains the subprocess tree, and bounds captured output.

An extension receives one JSON request on standard input conforming to `schema/extension-request.schema.json`. A successful extension process exits zero and writes exactly one JSON response conforming to `schema/extension-response.schema.json` on standard output. Diagnostic logging belongs on standard error. A nonzero exit, timeout, incompatible protocol version, malformed response, or empty summary produces `error`; it never becomes a product `failed` result or an implicit pass.

The response uses the same exhaustive outcomes as core rules. Only `passed` and demonstrably justified `not_applicable` satisfy a blocking extension. Non-blocking extensions remain visible but do not affect a gate.

Extensions cannot target, replace, suppress, reinterpret, or change the applicability of core rule IDs. Their project-local IDs occupy a separate result collection. Core gate aggregation runs independently before additive blocking checks are included. This structural separation enforces `OPDEV-EXT-001` without requiring every project to learn a general policy language.

Versioned declarative rule packs may be added in a later protocol revision. A sandboxed WASM verifier remains a possible future option for portable checks that cannot be expressed declaratively. Native dynamic libraries and unrestricted policy languages are intentionally outside the core extension boundary.

## Optional risk-based test strength

Issue [#48](https://gitlab.com/stolenfootball-tools/opdev/-/issues/48), implementation
scope approved by the maintainer on 2026-09-21, selects the following bounded
consumer outcome: demonstrate a useful testing gap through the existing extension
boundary without introducing a mandatory tool, score, schema or policy file.

- TS-01: A project-selected extension may report findings or strengthen a gate;
  it cannot replace core requirements, canonical suites or acceptance mappings.
  With no extension configured, ordinary behavior remains unchanged.
- TS-02: Extension process failure is `error`, unlike a completed protocol
  response reporting a failed selected requirement. Ordinary suite failure keeps
  its existing meaning. Non-blocking results remain visible without adding gate
  blockers; blocking non-satisfying results block their selected gate.
- TS-03: The optional reference adapter requires a supported producer and fresh,
  source-bound, complete, nonempty viable test evidence for `passed`. Missing or
  inconclusive evidence is `unverified`; unsupported/malformed output or tooling
  failure is `error`. A surviving mutation is a test-strength finding, not proof
  that the original software has a product defect.
- TS-04: Selection, concurrency and time are bounded explicitly; source mutations
  use scratch copies. Preserve diagnostics/evidence and retry history. Retained
  command-output reads and report parsing must not load arbitrarily large files.
  Time/output limits are not CPU, memory or disk quotas.
- TS-05: A real consumer demonstration must show passing ordinary tests that miss
  a selected boundary mutation, then a discriminating assertion catching it.
  Record actual feedback time and limitations, not just a score. Canonical
  regressions exercise findings, errors, incomplete/stale reports and gate isolation.
- TS-06: Guidance recommends stronger testing for a concrete risk or evidence
  gap, with developer-selected scope, tooling, budget and advisory/blocking policy.
  Preserve existing adequate choices; do not auto-install, auto-enable or create
  another mandatory adoption questionnaire or ledger.

The first reference is [cargo-mutants](../examples/test-strength/README.md).
Its producer-specific interpretation is deliberately outside the Rust CLI. Core
only understands the existing extension response. Property-based and compatibility
tests can remain canonical suites if their ordinary command results suffice.
An adapter is appropriate only when a selected requirement needs richer interpretation.

Use the existing testing authority and canonical command configuration. The current
CLI invokes `verify` via `opdev check`, `pre_merge` via `opdev check --ci`, and
`deliver` via `opdev check --ci --delivery`. Other enum stages are not independently
selectable through `check`; this feature adds no scheduler or stage selector.
Re-running `check --ci` on trunk still selects `pre_merge`, not `post_merge`.

The example begins advisory, with blocking an explicit project decision. Its
no-missed policy applies only to the reviewed narrow selection, not all software.
Do not reinterpret `failed` as `passed` to avoid blocking; choose `blocking: false`
when the reviewed policy is advisory. Real-tool canaries are explicit opt-in runs;
the normal regression suite requires no mutation tool or network download.

Alternatives rejected for this increment: producer parsers in core (maintenance
and ecosystem coupling), universal mutation thresholds (misleading applicability),
and a new plugin registry/scheduler (not needed for the consumer outcome). Revisit
only with demonstrated demand, useful findings and acceptable feedback costs.
Recovery is normal reviewed roll-forward; removing a project's optional extension
requires a reviewed policy change and never waives core gates. No project schema,
extension protocol or release change is needed for this opt-in example and the
nonzero-exit correction; both `failed` and `error` already block required checks.
