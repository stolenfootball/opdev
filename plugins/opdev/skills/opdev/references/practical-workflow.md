# Practical workflow (schema 5)

This route never overrides an older contract. Use actual runtime capabilities;
unsupported operations are not permission to hand-roll a replacement.
Before using this source-preview route, confirm the selected executable advertises
`practical.check.v1` in `doctor --format json`. Version compatibility alone is
insufficient. If unavailable, report the preview capability gap and preserve the
existing policy; do not update a managed pin or install a preview without consent.

## Levels and development

Lightweight favors brief acceptance and relevant checks for low-consequence work.
Standard adds maintained behavioral/regression tests, requirement-led agentic
review and authoritative integration checks. Assured adds explicitly source-bound
assurance across the selected scope. The developer selects the level; none
certifies defect-free software or authenticates consent.

Read commands, checks and relevant authority locations. Identify requested
behavior and important failure cases. A small task can use its MR without another
issue. Larger goals use connected useful slices, MVP phases and milestones;
preserve accepted future requirements. Research material uncertainty and reuse
adequate tools rather than re-investigating the stack on every task.

Implement with focused checks. Add meaningful behavioral and regression tests;
explain genuine automation limitations. Review assertions and consumer-visible
outcomes, not only test names or green suites. Record concrete decisions/findings
in the existing change discussion. Standard needs no manual acceptance digest,
selected review comment, ledger or extra reviewer.

`check --plan` inspects selected commands without execution. `check` runs local
feedback checks; `check --ci` selects pre-merge checks; `check --ci --post-merge`
selects integrated-source checks. These can differ. `--no-exec` never qualifies
missing executions. Multiple links to one canonical command need one execution.
Do not run the full suite immediately before asking OpDev to run it again.

For requested design feedback, use relevant focused checks and wait for that
feedback without merging or running full integration CI merely to show the draft.
A required failure blocks integration. Never substitute an older green report or
lower the selected level to hide it. Continue genuinely independent authorized
work while resolving the failure.

Native GitHub/GitLab required jobs own remote integration status. A local stage
selection is not remote execution or proof of integrated trunk. Changed merge
content or relevant execution conditions require fresh verification. Distinct
platform coverage is not duplication. Delivery checks do not grant release
permission; CI changes need actual implementation authority.

## Requirements and safeguards

Selected `.opdev/requirements/*.json` files own enduring promises, not task
status. Schema 2 links observable statements to check IDs and assertion locations
without a copy per stage. The link is a verification design, not a runner receipt.
Never infer individual test execution from a suite exit code. Review narrowed
requirements and removed assertions against developer intent. Strengthening an
assertion is not automatically weakening coverage.

Affected data, security, public-interface, distribution and high-consequence
behavior needs appropriate safeguards at every level. Unrelated work does not
escalate just because these capabilities exist. Preserve additional obligations.
Unsupported stronger verification remains unavailable, not a Standard pass.

## Adoption and upgrades

Explicit adoption authorizes assessment. Inspect existing tools and instructions,
propose a level and concrete changes, and ask only material questions. Reuse
actual decisions. Preserve adequate tools, authority locations and non-main
branch names; offer renaming without requiring it. Never manufacture approval
or mark a required gap optional to complete adoption.

For a new configuration, `init --project FILE --dry-run` previews the reviewed
full contract and managed instructions. `init --project FILE` applies it without
replacing existing policy. An empty repository can finish workflow setup without
inventing sample product code. Report product verification as not performed.

Existing contracts need a supported reviewed migration. Explain assurance being
reduced and preserve external obligations unless their authority changes them.
On a runtime advertising `practical.migration.v1`, use `upgrade --request-schema
--request-version 2`, then `upgrade --migration FILE` to inspect the exact proposal.
Apply only its reviewed identity with temporary recovery outside source; resume
the original snapshot after interruption, never overwrite a later edit. Preserve
original obligations and requirements or record their actual retirement decisions.
Do not transfer passed mapping reviews into the new contract.
Remove superseded active artifacts after verification; keep temporary recovery
protection and ordinary Git history, not an archive of obsolete OpDev files.
If any reviewed migration input changes, do not apply the stale plan: inspect a
new preview. Keeping a non-main trunk is valid. Ledger retirement does not require
a permanent archive.
Missing migration support is a limitation, not permission to force init or fake
approval. Existing policy remains until its replacement is supported and selected.

Only selected contract, short guidance, current requirements and policy definitions
belong in `.opdev`. No permanent adoption log, roadmap, reports or scratch archive.
Temporary state belongs outside source; unfamiliar project files are not automatic
cleanup targets.

Report actual observations and limits. Check results do not establish adequate
semantic review, consent, complete adoption, external conformance or release
authorization. Continue within the developer's scope, stopping only at their
boundary or a genuine blocker, not merely because feedback is ready.
