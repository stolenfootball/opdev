---
opdev_document:
  schema: 1
  role: tech_stack
  decision: inferred
  reference: null
  scope: Observed repository implementation and selected self-development standards.
---

# OpDev technical baseline

The native CLI is a Rust workspace with separate project, engine, CI, remote and
release components. Cargo manifests/lockfile own dependency versions; the workspace
uses Rust 2024 and the declared pinned toolchain. Shared Markdown skills and hooks
integrate Codex and Claude. Python, POSIX shell and PowerShell support installation,
validation and distribution. This describes inspected choices, not a new ecosystem
mandate for consumer projects.

The repository uses rustfmt, Clippy with warnings denied, and workspace tests.
Maintained supporting scripts use pinned Ruff, shfmt, ShellCheck and
PSScriptAnalyzer tooling. [CONTRIBUTING](CONTRIBUTING.md) owns detailed conventions;
[project.yaml](../.opdev/project.yaml) owns exact commands and stage selection.
Use focused regression tests during feedback, canonical checks at integration,
and separate actual live trials for agent behavior. Packaging tests alone cannot
show that an agent follows instructions.

GitLab is the primary work/integration authority, with declared native-platform
jobs and a GitHub distribution/ARM producer path. Secrets remain outside source.
Explicit schema/policy selections retain their reviewed requirements; this repo
still uses its selected legacy source-bound self-development contract, not an
implicit migration to practical Standard. Releases and managed-pin updates follow
the separately authorized [release procedures](../release/README.md).

This summary is inferred from Cargo configuration, contribution guidance and the
selected project contract. It introduces no new tooling choice or historical
approval claim. Further adoption review must inspect the original decisions.
