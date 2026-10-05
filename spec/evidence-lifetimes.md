# Evidence lifetimes and conservative reuse

OpDev does not implement a general cross-revision test cache or infer unaffected
files from filenames, comments, documentation labels or agent judgment. Source,
configuration and stage remain whole-subject bindings by default. A newer source
or unknown dependency requires fresh qualification or an explicitly unverified
result. A cache hit is never a requirement-adequacy judgment.

The supported narrow case is [same-run CI execution](execution-reuse.md): reviewed
producer/input/environment policy can explicitly exclude an evidence ledger that
is not a command input. Execution remains bound to the exact producer and run;
new test conditions still need current assertion review. No automatic exclusion
is inferred, and the default includes the ledger. Fresh execution remains usable.

| Fact | Lifetime and invalidation |
| --- | --- |
| Assertion review | Exact source, complete acceptance inventory and current assertions; new conditions invalidate review even when execution is unchanged |
| Canonical execution | Exact reviewed input/configuration/environment identities, provider producer/run/attempt and stage; newer attempts and unavailable evidence cannot fall back to old green |
| Artifact qualification | Exact retained artifact bytes and separate qualification evidence; replacement or missing bytes cannot inherit old qualification |
| Developer decision | Original attributed scope, authority, revocation/supersession and applicable expiry; permission is not feedback or release authority |
| Remote observation | Explicit observation context and freshness, rechecked through the provider when qualification requires it; local journals cannot refresh or authenticate it |

The [workflow reference projection](workflow-records.md) applies conservative
subject and content checks independently to each record and explains the changed
identity. Review records compare a separately supplied complete acceptance
digest. A changed review need not erase unchanged execution history, but the
projection itself never qualifies that execution. Existing core evaluation and
provider validators remain responsible for gates and current source observations.

All supporting references must be available and unchanged. Expired, future-dated,
conflicting, revoked and superseded facts stay visible. Cooperative concurrent
updates use the journal's expected-head check; an older observation cannot replace
new facts. No derived index is required or authoritative. Historical diagnostic
receipts retain their meaning and are not upgraded into qualifying executions.

This intentionally favors broader rechecking over an unproven scoped cache.
Introduce narrower dependencies only for a demonstrably useful case with a
reviewed complete input boundary, negative/mutation tests against fresh checks,
explicit compatibility and developer policy. Neither test removal nor automatic
policy migration is a valid optimization. Qualification, delegation and inspection
remain separate; unsupported consumers use the fresh execution path.
