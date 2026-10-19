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

On a runtime advertising `practical.expected-revision.v1`, CI can add
`--expected-revision` with its exact checkout SHA. Preserve OpDev's native exit
status; do not add a project-specific parser to revalidate its report. Retain
separate product artifact/host qualification. With `practical.manual-checks.v1`,
checks may select `manual: path/to/procedure.md` instead of a command. These are
reported as `not_run`, never a successful observation or a fake command failure.
`check --evaluation` selects that explicit stage. A manual check at a required
integration boundary still prevents a pass; don't move or waive it for green CI.

For authorized development previews, prefer an identified, digest-verified binary
from one successful producer job over rebuilding the same source in every consumer
job. Select exact source, platform, producer job and independently approved digest;
missing or mismatched artifacts fail closed. This reuses tooling, not test results,
and does not authorize release or global installation. During waits, avoid repeated
unchanged-state narration; report meaningful changes or host-required brief updates.

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

When adding or intentionally changing supported behavior, update the affected
selected requirement or add its enduring promise with the actual check and
assertion location as part of completing the implementation, even when another
agent owns CI or merging. A compatibility-only requirement does not cover a new
feature's behavior. Reuse an adequate existing mapping for a repair; do not add
one entry per task, duplicate the specification, or create a catalog merely for
unrelated work. Keep pending design feedback provisional, then reconcile the
affected promises before reporting implementation complete. An empty change to the catalog is valid
only when its existing statements and links still cover the affected promises.

At completion, use the actual diff, selected requirement statements and relevant
test assertions together, not a recollection of the edits or only a file list.
Answer two concrete questions in the existing review or handoff:

- Which requested outcome or necessary dependency justifies each meaningful
  change? Report unrelated discrepancies separately; authority is not scope.
- Which existing or updated requirement and actual assertion cover each new or
  changed supported promise? Name the relevant IDs and test locations, or explain
  why an adequate existing mapping is unchanged. A compatibility statement alone
  does not describe the behavior of a new feature.

Use existing Git diff and project inspection tools to assemble these facts.
Inspect requirement meaning rather than merely whether its file changed. Fix
in-scope omissions before claiming completion; if permissions genuinely prevent
that, report the specific incomplete part. Reuse current test execution when
source and execution conditions are unchanged. This is the implementing agent's
bounded review, not a new command, ledger, mandatory second agent, full-suite
rerun or routine approval. Mechanical validation and an agent's own statement
do not prove semantic completeness. Requested feedback may remain provisional.

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
