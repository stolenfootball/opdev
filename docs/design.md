---
opdev_document:
  schema: 1
  role: design
  decision: inferred
  reference: null
  scope: Summary of the existing supported OpDev design; not a new policy selection.
---

# OpDev product design

OpDev helps developers and coding agents plan, implement and verify software with
explicit decision authority and proportionate checks. A native CLI validates
contracts and observes execution; shared Codex/Claude guidance handles discovery,
design, implementation and review. It serves software beyond any single language,
UI type or CI provider. One agent is the default.

The [practical contract](../spec/practical-workflow.md) defines explicit assurance
levels; [engineering policy](../spec/engineering-policy.md) preserves the meanings
of older selected obligations. Updating guidance does not migrate project policy.
Requirements, test design, execution and authorization are distinct. A green suite
does not prove semantic completeness or consent. Work stays in the project tracker;
durable documentation stays outside the strict `.opdev` configuration namespace.

The [project-design workflow](project-design.md) separates product, interface and
technical acceptance before phased implementation. Useful end-to-end outcomes,
early MVPs and retained later scope guide planning. Feedback, integration and release
are separate boundaries; publication needs explicit authorization.

This summary is inferred from maintained specifications and the repository contract,
not a retrospective claim that every historical choice was personally approved.
Detailed specifications remain authoritative. Current work and accepted sequencing
remain in [GitLab](https://gitlab.com/stolenfootball-tools/opdev/-/issues).
