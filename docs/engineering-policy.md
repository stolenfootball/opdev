# Understanding engineering policy

OpDev separates engineering readiness from compliance with an additional standard.
A project chooses suitable tools and practices, but missing tests or safeguards
cannot become passing simply because the project prefers not to use them.

## Inspect before changing policy

On a CLI advertising `policy.preview.v1` in `doctor`, inspect definitions without
changing your selected project policy:

```text
opdev policy explain --engineering 2
opdev policy explain --engineering 2 --rule OPDEV-RECOVERY-001
opdev policy preview --engineering 2 --root . --format json
```

`explain` needs no project. `preview` reads the current initialized project and
shows its selection alongside the proposed definition. Both identify retained,
replaced and external-only rules; neither edits files, executes project checks,
contacts a provider, selects standards or records approval. Exit zero means the
definition was inspected successfully, not that your project passes it.

**Definition preview is not enforcement.** These commands alone do not enable a
Policy 2 project contract, migrate adoption records or change existing check
behavior. Keep using the currently selected project policy until a supported,
explicitly reviewed migration is applied. The preview's definition digest covers
its embedded semantic definition; it is not an approval or reusable test result.

Use the JSON rule's statement, applicability, boundaries and verification routes
to understand the obligation. A conditional rule needs evidence of whether it
applies. Missing configuration does not prove that it does not apply. Unsupported
versions and unknown rule IDs are errors, never a fallback to fewer requirements.

## What the Policy 2 design changes

| Situation | Intended treatment |
| --- | --- |
| Required trunk CI fails | Prioritize repair; block affected integration/delivery. Independent work needs actual dependency and restoration-impact review. |
| A release cannot safely roll back its data | Demonstrate safe tested recovery covering data and external effects. A promise to patch later is insufficient. |
| A developer is iterating on a design | Use meaningful focused feedback checks. Required integration checks still apply before merge and on integrated source. |
| A project needs maintenance branches | Bound supported fixes/lifetime, protect and test that branch, and retain relevant fixes in trunk. |
| An organization requires a standard | Pin the exact supported policy and its enforcement boundaries in addition to the OpDev baseline. |
| Checks pass | Report only the verified boundary. Passing checks never authorize release execution. |

MinimumCD remains distinct. A project may meet OpDev's safe-recovery requirement
and still lack MinimumCD's required rollback evidence. Neither engineering
readiness nor a partial standards mapping establishes full conformance.
Historical rules shown outside the engineering baseline are not automatically
external obligations: the selected standard's exact mapping determines those.
For example, MinimumCD mapping 1 uses `MCD-RECOVERY-002` for literal rollback,
not the older, broader `MCD-RECOVERY-001` recovery statement.

The policy design supports `guidance` (no conformance verdict), `assess` (separate
nonblocking assessment), and `require` (additional blocking requirements at their
selected boundaries). Those choices cannot weaken the baseline. Existing derived
SSDF/OSPS/SLSA mappings are not full certifications. Organization definitions are
intended to be constrained, pinned and additive, not scripts or an exception list.

For exact semantics, rationale, compatibility and evidence responsibilities, see
the [engineering policy specification](../spec/engineering-policy.md). Existing
Policy 1 behavior is documented in [assurance profiles](../spec/assurance-profiles.md).
Implementation progress lives in [GitLab](https://gitlab.com/stolenfootball-tools/opdev/-/issues/106),
not in a repository checklist.
