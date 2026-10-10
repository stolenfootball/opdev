# OpDev

**A shared development workflow for Codex, Claude Code, and CI.**

OpDev helps coding agents pick up your project's rules, make tested changes,
and report what is actually ready to merge or deliver. It combines an agent
plugin with a Rust CLI: the plugin guides the work; the CLI runs your checks
and evaluates the evidence; your CI applies the integration gate.

Keep your language, framework, test tools, work tracker, and document locations.
OpDev records those choices in a small, versioned project contract instead of
making every new agent rediscover them.

[Quick start](#quick-start) · [Example session](#a-typical-session) ·
[CLI guide](docs/GETTING_STARTED.md) ·
[Releases](https://github.com/stolenfootball/opdev/releases) ·
[Help](https://gitlab.com/stolenfootball-tools/opdev/-/issues)

## Why use it?

- **Carry context between sessions.** Fresh agents read the same project-owned
  instructions and follow pointers to the relevant design, tests, and work item.
- **Make testing part of the workflow.** Declare canonical commands, regression
  expectations, quality risks, and how flaky tests are handled.
- **Plan useful increments.** Roadmaps and "what next?" answers prioritize
  demonstrable outcomes and feedback, not completed technical layers. Specific
  user requests still control the scope.
- **Keep approval tied to evidence.** Missing or stale evidence blocks the
  affected gate. A successful test run does not imply readiness to deliver.
- **Use one process across tools.** Codex and Claude Code share the protocol;
  GitHub Actions and GitLab CI are first-class providers. The CLI also works
  without an agent.

Current development builds offer an explicitly selected engineering policy:
non-waivable baseline checks, capability-dependent safeguards, and flexible
choices of adequate tools. Meaningful tests, reproducible setup, appropriate
formatting/static checks, one integration trunk and enforced CI remain required.
Delivery requires the applicable immutable-artifact and recovery evidence; it
does not happen without developer authorization. Extensions cannot waive rules.

Under that policy, [MinimumCD](https://minimumcd.org/) is a separately selected
assessment, not another execution of the same tests. Legacy projects retain their
existing MinimumCD-based rules until an explicitly reviewed migration. See
[engineering policy](spec/assurance-profiles.md#engineering-policy-1); a new plugin
does not imply that its installed CLI supports these development capabilities.

In CLI 0.4.0's catalog 2, daily merging is a monitored target, not a deadline
that blocks a later merge. Delays prompt replanning; required tests still have
to pass. The daily requirement remains part of compliance reporting, so a green
merge check alone does not establish full MinimumCD compliance. Released CLIs
continue to use their own catalog until explicitly upgraded.

## Quick start

You need a Git-backed project and a version of Codex or Claude Code with plugin
support. Native runtimes are available for macOS, Windows, and Linux GNU on
x86-64 and ARM64. Initial setup needs network access and basic OS tools, but
not Rust. Your project's own compilers and test runners are still required.
See [platform requirements](docs/GETTING_STARTED.md#requirements) for Linux and
Windows ARM64 constraints.

### 1. Install the plugin for your agent

Run the appropriate commands in a **terminal**. Install separately for each
agent if you use both.

**Codex**

```sh
codex plugin marketplace add https://gitlab.com/stolenfootball-tools/opdev.git
codex plugin add opdev@personal
```

Start a new Codex task after installation or an update.

**Claude Code**

```sh
claude plugin marketplace add https://gitlab.com/stolenfootball-tools/opdev.git
claude plugin install opdev@opdev
```

Restart Claude Code or reload its plugins after installation.

### 2. Set up the runtime

In your **agent's chat**, say:

> Set up OpDev.

The plugin locates a compatible CLI or offers to install its pinned native
runtime. Installation verifies the archive's signature and CLI compatibility,
then stores it in private, versioned storage. Normal tool approvals apply.
Later sessions reuse that runtime.

This step does **not** initialize your repository or add `opdev` to your
terminal's PATH. Prefer a terminal-only workflow? Use the
[standalone CLI installation](docs/GETTING_STARTED.md#install-the-standalone-cli).

### 3. Initialize your project

Open the project with your agent and say:

> Initialize OpDev in this repository. Review the discovered commands and
> document locations with me before adopting them.

The agent reviews material policy, tooling and ownership choices with you before
implementing them. With the current CLI and explicitly selected layout 1, OpDev
creates `.opdev/project.yaml`, pending `.opdev/adoption.yaml`, and one shared
`.opdev/guidance.md`. Short managed sections in `AGENTS.md` and `CLAUDE.md` point
to that guide, preserving unrelated instructions. Legacy layouts retain their
existing guidance until reviewed migration. Existing documentation stays where
it is; initialization does not create or take over a `docs/` folder.

Review and commit those files through your normal development workflow. A first
check may identify missing evidence or delivery setup. That is an adoption
checklist, not a reason to mark unknown requirements as passed. The
[first-check guide](docs/GETTING_STARTED.md#understand-the-first-check) explains
what to do next.

Development builds add an explicit adoption review: every supplied practice
must be implemented, explicitly ignored when optional, or justified as not
applicable. Existing tools are preserved; the agent researches only unresolved
project-specific gaps. Creating files alone is not completed adoption. See the
[adoption guide](docs/GETTING_STARTED.md#complete-adoption-in-development-builds)
for capability checks and the completion gate; published runtimes may not yet
support it.

## A typical session

Once initialized, ask for software work normally—no special command sequence
is needed for every task. For example:

> Add express shipping to the API and CLI. Preserve standard shipping behavior
> and add regression coverage.

OpDev guides the agent to:

1. Read the project contract, relevant design/behavior documents, and work item.
2. Establish the expected behavior, affected consumers, and tests. Record a
   durable design decision only when the change warrants one.
3. Make a focused change and run meaningful checks sufficient to review its
   direction. Iterate here without full qualification for every design tweak.
4. Before merging a coherent increment, review the exact source and assertions
   and run the required checks for that boundary; then verify integrated trunk.
   Explicit engineering policy can assign different justified checks per stage.
5. Report each gate honestly and reconcile CI, project docs and the work item.
   Integration does not authorize a release.

An illustrative handoff might say:

> Express shipping is implemented; compatibility and regression tests pass.
> Integration is blocked: this change still needs reviewed evidence.
> Delivery remains blocked until the declared recovery path is qualified.

That distinction is deliberate. OpDev does not treat an agent's confidence,
old approval, or green unit tests as proof of delivery readiness.

For a fresh agent, the repository instructions restore the process. In an
uninitialized project, substantive development may prompt an adoption offer;
without acceptance, the original task continues without OpDev. Routine requests
such as pulling changes or starting a dev server do not prompt adoption. If a
required CLI or integration is missing or incompatible after activation, it reports the problem
and offers setup rather than silently continuing without the protocol.

## What lives in your repository?

| File | Purpose |
| --- | --- |
| `.opdev/project.yaml` | Project commands, document and tracker locations, testing policy, delivery requirements, and context routes. |
| `.opdev/adoption.yaml` | Practice dispositions and review provenance; pending records do not mean completed adoption. |
| `.opdev/guidance.md` | With selected layout 1, the single shared managed guide, reloaded after a context reset. |
| `AGENTS.md` and `CLAUDE.md` | With layout 1, short pointers to the guide alongside existing instructions/imports. Legacy roots retain their earlier format. |
| `.opdev/evidence.yaml` | Legacy reviewed evidence only. Preserve it until a reviewed storage migration verifies complete history retrieval and recovery. |

The contract points to your existing sources of truth. Design notes can live in
your chosen folder or declared external authority. There is no required project
template, document relocation, or replacement test framework.

The selected strict layout permits optional durable knowledge under
`.opdev/docs/`: `design.md`, `development.md`, `testing.md`, `delivery.md`,
`specs/`, `decisions/`, and referenced `assets/`, **only as needed**. No miscellaneous
scratch/archive directory, raw transcripts, work backlog or growing execution
history belongs there. Initialization does not create empty documentation.
Legacy locations remain valid until an explicit migration; do not tidy them
automatically. Small changes can stay in the existing work item.

Typed local drafts and attempts live in CLI-owned state outside product source.
With separately selected review-storage policy, exact semantic reviews are
retrieved from the reviewed external archive; retained reports do not replace
current execution or authenticate developer consent. Export alone never permits
deleting a legacy ledger. See [storage and migration](spec/upgrades.md#explicit-coordinated-migration).

Public documentation retains its project/ecosystem locations, and the root
README should link to development guidance when it exists. Both root agent
entry points remain. See [documentation layout](spec/documentation-layout.md).

Discovery recognizes common Cargo, npm, Python, Go, infrastructure, documentation,
and plugin repositories. Other stacks can declare their commands and authorities
explicitly; discovery support is not an allowlist of software you can use.

## What the checks mean

OpDev evaluates the selected versioned policy and the checks configured by your project.
It reports four separate gates:

| Gate | Question |
| --- | --- |
| Development | May ordinary implementation proceed? |
| Integration | May this change enter trunk? |
| Delivery | May this identified artifact be delivered through the declared path? |
| Compliance | Is there sufficient evidence for the selected policy? With engineering policy, MinimumCD has its own separately requested assessment. |

Rule outcomes are `passed`, `failed`, `unverified`, `not_applicable`, `error`,
and `migration_required`. Only `passed` and justified `not_applicable` satisfy
a required rule. Missing evidence is `unverified`; an evaluation problem is
`error`; a recorded adoption gap is `migration_required`. None is a hidden pass.

Some gaps permit incremental development, but they cannot qualify a delivery
or compliance claim. See [result semantics](spec/result-semantics.md).

## CI, trust, and limits

Use `opdev ci inspect` to review existing GitHub Actions or GitLab CI files.
For a project without CI, OpDev can generate a baseline without overwriting an
existing configuration. You still need to supply and qualify your project's
build, delivery, and recovery path. See [connect CI](docs/GETTING_STARTED.md#connect-ci).

Discovery is static; running checks executes commands selected by the project.
Review the contract before running checks in an untrusted repository. Remote
audits are read-only, and extensions cannot replace core verdicts. OpDev does
not certify software or replace engineering judgment.

**Status:** OpDev is pre-1.0. See [published releases](https://github.com/stolenfootball/opdev/releases)
for available binaries. Source plugin 0.4.2 pins qualified CLI 0.4.0. Updating
the plugin does not migrate existing project records or CI pins. The
[compatibility policy](spec/compatibility.md) explains their separate versions.
GitLab is the source and CI authority; GitHub hosts current binary releases.

The 0.3.0 line adds change-bound acceptance evidence, advisory consistency review,
test-execution receipts, exact remote CI qualification, readiness diagnostics and
proportionate planning research. Review the [migration notes](release/CHANGELOG.md)
before upgrading required CI: acceptance ledgers need explicit schema-2 review,
remote qualification needs a reviewed policy, and `doctor` exit codes changed.
Existing project files are not silently migrated. Compact report and evidence
views remain behind the explicit
`--experimental-compact` flag, disabled by default. Agents use full output unless
you or your project explicitly opt in; omit the flag to return to stable behavior.
Older CLIs do not have this flag. The
[30-session evaluation](benchmarks/sessions/results/2026-09-15-codex/README.md)
found lower total token usage, but did not establish equivalent-quality savings
or general development effectiveness. See [compact views](spec/compact-views.md)
for capabilities and fallback behavior.

## Learn more

To upgrade an initialized project, ask your agent to "upgrade OpDev." It inspects
the installed plugin/runtime, explains the target and project changes, and seeks
approval before applying them. CLI 0.2.0 supports `opdev upgrade
--dry-run` and reviewed `--apply PLAN_ID`; older released CLIs do not support
this flow. Installation, project guidance and CI qualification are separate steps.
See [upgrades](spec/upgrades.md); project choices and experimental opt-ins are preserved.

- [Getting started with the CLI](docs/GETTING_STARTED.md): installation,
  initialization, a first change, evidence review, and CI.
- [Normative specification](spec/README.md): lifecycle, rules, and authority order.
- [Requirements and verification](docs/requirements-and-verification.md): durable
  product guarantees, test relationships, current observations and explicit migration.
- [Evidence ledger](spec/evidence-ledger.md): review and freshness requirements.
- [Acceptance evidence](spec/evidence-ledger.md#schema-2-acceptance-evidence): development
  CLIs bind reviewed requirements and assertions to the current change and suite
  execution; older ledgers need explicit migration before these checks qualify.
- [Consistency review](spec/consistency-review.md): ask whether implementation,
  tests and documentation agree with accepted requirements. Advisory findings
  reuse your existing authorities and do not authorize edits or gate approval.
- [Test execution evidence](spec/test-reports.md): development builds can record
  tool-neutral canonical command/source observations without requiring a reporter.
  Optional JUnit inspection remains separate from checks and gate qualification.
- [Remote CI evidence](spec/remote-audits.md): development builds can verify an
  explicit GitHub/GitLab run and required jobs. Reviewed project-schema-2 policy
  additionally binds `check --remote` to current trunk, check producers and merge
  policy. Missing policy or provider evidence remains unverified; nothing is
  adopted or changed automatically.
- [Experiments](spec/experiments.md): keep stable releases independent of opt-in
  work, with a standard record, configuration tests, and a cleanup decision.
- [Extensions](spec/extensions.md) and [assurance profiles](spec/assurance-profiles.md):
  additional project checks and versioned guidance.
- [Release operations](release/README.md): verification, supported assets, and recovery.
- [Changelog](release/CHANGELOG.md): notable changes.

## Help and contributing

Ask questions, report bugs, or propose improvements through
[GitLab issues](https://gitlab.com/stolenfootball-tools/opdev/-/issues).
For suspected vulnerabilities, follow the private reporting instructions in
[SECURITY.md](docs/SECURITY.md).

Contributions are welcome; start with [CONTRIBUTING.md](docs/CONTRIBUTING.md)
for development prerequisites and required checks.

Maintained by Opinionated Development contributors. Licensed under
[Apache 2.0](LICENSE).
