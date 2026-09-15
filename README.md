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
- **Keep approval tied to evidence.** Missing or stale evidence blocks the
  affected gate. A successful test run does not imply readiness to deliver.
- **Use one process across tools.** Codex and Claude Code share the protocol;
  GitHub Actions and GitLab CI are first-class providers. The CLI also works
  without an agent.

OpDev is opinionated about delivery: [MinimumCD](https://minimumcd.org/)
requirements are mandatory. Use one integration trunk, restore red CI first,
deliver through CI, build an immutable artifact once, and have a tested recovery
strategy. You choose how your project meets those requirements. Extensions can
add checks, but cannot waive core rules.

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

The agent reviews the project contract and initialization with you. OpDev
creates `.opdev/project.yaml` and managed guidance in `AGENTS.md` and
`CLAUDE.md`, preserving unrelated content. Existing documentation stays where
it is; initialization does not create or take over a `docs/` folder.

Review and commit those files through your normal development workflow. A first
check may identify missing evidence or delivery setup. That is an adoption
checklist, not a reason to mark unknown requirements as passed. The
[first-check guide](docs/GETTING_STARTED.md#understand-the-first-check) explains
what to do next.

## A typical session

Once initialized, ask for software work normally—no special command sequence
is needed for every task. For example:

> Add express shipping to the API and CLI. Preserve standard shipping behavior
> and add regression coverage.

OpDev guides the agent to:

1. Read the project contract, relevant design/behavior documents, and work item.
2. Establish the expected behavior, affected consumers, and tests. Record a
   durable design decision only when the change warrants one.
3. Make a focused change and run the project's declared checks.
4. Review any evidence that cannot be inferred automatically, tied to the exact
   staged change where required.
5. Report each gate honestly and reconcile the work with CI and project docs.

An illustrative handoff might say:

> Express shipping is implemented; compatibility and regression tests pass.
> Integration is blocked: this change still needs reviewed evidence.
> Delivery remains blocked until the declared recovery path is qualified.

That distinction is deliberate. OpDev does not treat an agent's confidence,
old approval, or green unit tests as proof of delivery readiness.

For a fresh agent, the repository instructions restore the process. In an
uninitialized software project, the plugin asks before adopting OpDev. If a
required CLI or integration is missing or incompatible, it reports the problem
and offers setup rather than silently continuing without the protocol.

## What lives in your repository?

| File | Purpose |
| --- | --- |
| `.opdev/project.yaml` | Project commands, document and tracker locations, testing policy, delivery requirements, and context routes. |
| `AGENTS.md` | Persistent instructions for fresh agents, alongside your existing guidance. |
| `CLAUDE.md` | Imports the shared `AGENTS.md` guidance for Claude Code. |
| `.opdev/evidence.yaml` | Optional reviewed facts; change-specific assertions are bound to the staged Git index. |

The contract points to your existing sources of truth. Design notes can live in
your chosen folder or declared external authority. There is no required project
template, document relocation, or replacement test framework.

Discovery recognizes common Cargo, npm, Python, Go, infrastructure, documentation,
and plugin repositories. Other stacks can declare their commands and authorities
explicitly; discovery support is not an allowlist of software you can use.

## What the checks mean

OpDev evaluates 37 core rules and the checks configured by your project.
It reports four separate gates:

| Gate | Question |
| --- | --- |
| Development | May ordinary implementation proceed? |
| Integration | May this change enter trunk? |
| Delivery | May this identified artifact be delivered through the declared path? |
| Compliance | Is there sufficient evidence for the selected assurance profile? |

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

**Status:** OpDev is pre-1.0. The standalone release is
[0.1.2](https://github.com/stolenfootball/opdev/releases/tag/v0.1.2); the source
plugin deliberately pins compatible CLI 0.1.1. The
[compatibility policy](spec/compatibility.md) explains their separate versions.
GitLab is the source and CI authority; GitHub hosts current binary releases.

Development builds on `main` also include compact report and evidence views;
published CLIs may not have them yet. The
[30-session evaluation](benchmarks/sessions/results/2026-09-15-codex/README.md)
found lower total token usage, but did not establish equivalent-quality savings
or general development effectiveness. See [compact views](spec/compact-views.md)
for capabilities and fallback behavior.

## Learn more

- [Getting started with the CLI](docs/GETTING_STARTED.md): installation,
  initialization, a first change, evidence review, and CI.
- [Normative specification](spec/README.md): lifecycle, rules, and authority order.
- [Evidence ledger](spec/evidence-ledger.md): review and freshness requirements.
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
