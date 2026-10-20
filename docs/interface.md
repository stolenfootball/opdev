---
opdev_document:
  schema: 1
  role: interface
  decision: inferred
  reference: null
  scope: Summary of existing CLI and agent interaction contracts.
---

# OpDev interface contract

Developers interact through their coding host and the `opdev` CLI. Initialized
projects load their selected guidance; uninitialized substantive work gets an
early optional adoption offer. Routine status, pulls and dev-server work do not
trigger adoption. Questions discover actual needs, then separate product,
interaction, technical and phased execution decisions. Previously accepted choices
and bounded delegation remain reusable.

CLI commands provide readable text and supported structured reports. Help/version
are noninteractive. Invalid inputs and unsupported capabilities explain the next
action; inspection does not execute checks or authenticate approval. Mutations
require explicit command selection and preserve conflicting user changes.
`documentation status` reports three document roles and decision-reference gaps;
its structural success is not permission to implement.

Checks report actual failed, unrun and uncertain scope. Focused design/feedback
execution does not qualify integration. Required checks still run at their real
boundaries; saved reports cannot qualify changed source. See the exact
[consumer-interface targets](../spec/consumer-interface.md),
[result semantics](../spec/result-semantics.md) and
[practical contract](../spec/practical-workflow.md).

This is a source-derived summary, not new human approval or a universal terminal
accessibility certification. The linked contracts own detailed command behavior.
