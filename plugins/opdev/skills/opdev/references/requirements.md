# Durable requirements and current verification

Use this reference only when the project explicitly selects
`assurance.requirements.version: 1` and the CLI supports
`requirements.catalog.v1`. It needs project schema 3, strict layout 2 and MR/PR
review storage 2. Missing capability needs an upgrade offer, not an improvised
ledger or automatic installation. Existing projects retain their selected policy.

Read current project policy and relevant capability JSON files in
`.opdev/requirements/`. Use `requirements schema`, `inspect`, `show ID`, `bind`
and `diff --base REVISION` for read-only preparation. These commands do not run
tests, approve mappings, authenticate decisions or qualify gates. They inspect
staged Git sources; stage intended files without sweeping in unrelated work.

Separate requirements (promises), verification design (why checks establish
them), observations (what happened), and authorization (what the developer
permitted). None substitutes for another. One requirement owns inline normative
text OR references its existing exact specification fragment. Stable IDs survive
file moves. Group capability records meaningfully; do not create a file per helper.

Derive observable criteria from accepted behavior, including important negative
outcomes and side effects. Reuse meaningful tests. Each criterion needs exactly
one plan per supported configuration/required stage; all members must establish
it together. Green component tests do not establish an assembled consumer outcome.
An empty inventory/criterion list/plan is not a pass.

Inspect actual assertions and runner controls. Describe each link's assertion and
discriminating case; include known helper/fixture/selection inputs. Unknown/shared
dependencies broaden review. A declared dependency list or refreshed digest does
not establish semantic adequacy. Record actual agent/human attribution honestly;
replace current mapping judgments instead of appending a history ledger.

Mapping review binds the guarantee, criterion, applicability, configuration,
stage, assertion/input sources and relevant command/policy. An unrelated product
implementation change may preserve that review but needs fresh execution. Use
normal required `check` runs, not extra catalog-specific runs. A saved report or
green run for another snapshot/stage cannot qualify current work.

Keep current change impact, one-off conditions, decisions and observations in the
existing MR/PR. Its `acceptance.requirements` selects candidate/baseline catalog
identities and explains impact/scope and actual authorization. Baseline must match
the provider-observed MR/PR target snapshot; do not choose an easier old baseline.
Safeguard objectives may reference catalog criterion IDs. Review removals and
applicability reductions against accepted promises and actual developer scope;
stronger assertions are not automatically weakening. Do not duplicate all enduring
criteria in every MR or omit one-off conditions because they are not catalog data.

Suite assurance proves canonical command outcomes, not individual test selection
or complete retry history. Required case-level assurance is unverified without a
supported observation; no automatic downgrade, mandatory JUnit or runner adapter.
Known skips, filter gaps and quarantines remain assurance gaps, not waivers.
Manual observations need exact current plan/member subject, timestamp, result,
actual observer, observed behavior, context/limits and reference in the MR/PR;
apply the method's age and recheck limits. Mapping review is not that observation.

Keep future phases/milestones/progress at the work authority; inventory the first
useful supported slice for new projects. Assess existing promises and tests for
brownfield adoption; old green suites do not prove inventory completeness.
Upgrade through reviewed full-contract migration. Its explicit capability inputs
start with unverified reviews. Preserve useful facts, not obsolete active ledgers;
temporary recovery protection ends after verified migration, with no mandatory
archive or rewritten Git history. No automatic consumer migration or release.

Continue focused feedback loops. At integration, reconcile the catalog, one-off
conditions, actual test assertions and current execution. Report missing/stale
review alongside known failures. CI detail remains bounded under project policy;
the current catalog and ordinary Git history are not a growing execution ledger.
