# Getting started with the OpDev CLI

Use this guide for a terminal-first workflow or for the details behind the
[agent quick start](../README.md#quick-start). Commands below assume a standalone
`opdev` executable on PATH. With the plugin-managed runtime, ask your agent to
run the equivalent commands using its selected executable.

## Requirements

- Git and a Git-backed project.
- Your project's compilers, package managers, and test runners for its checks.
- Network access for initial installation and any remote provider audit.
- A supported native target: macOS, Windows, or Linux GNU; x86-64 or ARM64.
  Check the selected [release's assets](https://github.com/stolenfootball/opdev/releases)
  before installing. Linux 0.1 releases require glibc 2.39 or newer. Windows
  needs PowerShell 5.1+; ARM64 needs Windows 11 x64 emulation for the signature
  verifier. Linux musl is not a promised published target.

Basic OS download, hashing, and archive tools are required. No Rust toolchain,
administrator privileges, or separately installed signature verifier are needed
for the native installers. See [installation details](../spec/installation.md)
and [managed runtime setup and recovery](../plugins/opdev/skills/setup/SKILL.md).

## Install the standalone CLI

The following installers select published **0.1.2**. Read or download a release
script for review before executing it if required by your trust policy: the
installer itself supplies the verification trust anchors.

macOS or Linux:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/stolenfootball/opdev/releases/download/v0.1.2/opdev-installer.sh | sh
```

Windows PowerShell:

```powershell
powershell -NoProfile -Command "& ([scriptblock]::Create((Invoke-WebRequest -UseBasicParsing 'https://github.com/stolenfootball/opdev/releases/download/v0.1.2/opdev-installer.ps1').Content))"
```

The installers download a checksum-pinned verifier and verify the archive
against its exact GitLab release signing identity before extraction. They
install into a user bin directory and configure user PATH. Open a new terminal:

```sh
opdev version
```

Expect version `0.1.2` for these URLs. To leave PATH unchanged, use
`OPDEV_NO_MODIFY_PATH=1` with the shell installer or the PowerShell installer's
`-NoModifyPath` option. Installation does not initialize a repository.

To select another published version, change the tag in the URL. Re-running an
installer verifies and reinstalls that version; automatic updating is disabled.
See [manual verification and historical releases](../release/README.md#consumer-verification)
and [recovery](../release/README.md#recovery).

The plugin has a separate managed-runtime path: it deliberately pins CLI 0.4.0
in [runtime.lock](../plugins/opdev/runtime.lock), verifies compatibility, and
does not modify PATH. That pin is independent of the plugin's version.

For source development, Rust 1.97 or newer provides a fallback. This installs
from the repository, not necessarily the published release:

```sh
cargo install --locked --git https://gitlab.com/stolenfootball-tools/opdev.git opdev-cli
```

## Initialize a repository

From an existing Git repository, inspect discovery before writing anything:

```sh
cd path/to/your-project
opdev init --dry-run
opdev adoption plan
```

Discovery reads static metadata; it does not run repository-controlled commands
or approve its inferred choices. On a current development build, bare `init`
refuses to write a new project policy. Review the plan and resolve material
choices before supplying the approved complete contract:

```sh
opdev init --project path/to/reviewed-project.yaml --dry-run
opdev init --project path/to/reviewed-project.yaml
```

The proposal is temporary input, not a second contract to keep in the repository.
Use `--legacy-policy` only for intentional older-policy compatibility scaffolding,
not for current adoption. Older published CLIs have different initialization
behavior; check capabilities before using these commands.

Development builds also create `.opdev/adoption.yaml` with every practice
pending for new projects. Re-running initialization preserves existing decisions;
legacy projects are not silently migrated. See the completion workflow below.

### Select current development policy and layout explicitly

Check `opdev init --help` and compatibility first; the historical installer
example above does not supply these newer capabilities. A new project can select
the versioned `clean-1` destination in its reviewed contract: schema 3,
engineering policy 1, the separately selected MinimumCD assessment, layout 1,
external semantic-review storage 1 and capability safeguards 1. See
[current adoption destination](../spec/adoption.md#current-destination).
The older individual engineering/layout flags still support partial scaffolding;
they are not a complete current-adoption contract. A supplied decision reference
is attribution, not proof of developer consent; do not invent an approval.

Engineering policy makes baseline outcomes mandatory while preserving adequate
tools. Applicability depends on real capabilities, not missing configuration.
Selected layout 1 creates the shared `.opdev/guidance.md` and short root pointers,
in addition to the project and pending adoption records. It creates no optional
documents. Review and stage all files actually generated, including adoption and
shared guidance when present; the legacy example below is not their complete list.

For an existing project, `init` preserves its policy. Inspect a policy proposal
through `upgrade --engineering-policy ...`; use the separately reviewed
[coordinated migration](../spec/upgrades.md#explicit-coordinated-migration) when
moving layout, adoption and evidence storage together. It requires the explicit
candidate choices, complete retained history and a verified recovery path.
Neither plugin update nor a successful preview migrates the project.

Review the contract before running checks. Confirm the exact command arguments,
working directories, test suites, authority locations, and CI provider.
Discovery leaves ambiguous facts for review. It does not invent a production-like
environment, artifact, recovery strategy, or coverage threshold.

Once those files reflect the project, stage and commit them through its normal
development workflow:

```sh
git add .opdev/project.yaml AGENTS.md CLAUDE.md
git commit -m "chore: initialize OpDev"
opdev check
```

## Understand the first check

`opdev check` runs configured commands and evaluates requirements. It reports
development, integration, delivery, and compliance separately. The default
command's exit status follows the development gate; `--ci` selects integration.
Always inspect the other gates before making a broader readiness claim.

A new adoption often has blockers:

| Finding | Next action |
| --- | --- |
| A project check fails | Diagnose the command or behavior and add appropriate regression protection. |
| `unverified` | Supply the missing reviewable evidence or access; do not assume a pass. |
| `migration_required` | Track and implement the missing capability, then verify it. |
| `error` | Repair the evaluation, configuration, or tooling problem and run the check again. |

Use `opdev doctor` for read-only prerequisite diagnostics and `opdev rules` to
inspect requirements. Capable CLIs support `doctor --format json` and optional
`--plugin-root PATH`; `--remote` explicitly includes provider observations using
existing credentials. Doctor never executes project commands or repairs setup.
Its exit is 0 for clear inspected prerequisites, 1 for blockers/required unknowns,
and 2 for inspection errors. Passing doctor is not passing tests, completed
adoption or release readiness. Older doctor exits were unconditional: check
`doctor --help` before relying on these flags/semantics. See
[scope and limits](../spec/doctor.md).
Only `passed` and justified `not_applicable` satisfy required rules. Recording
a migration gap is not delivery approval. See [result semantics](../spec/result-semantics.md).

## Complete adoption in development builds

Check `opdev adoption --help` first. Published 0.2.0 has the original assessment;
schema-2 plan/approve/migrate/prepare-evidence are development capabilities.
If unavailable, select a
compatible source-built CLI or leave adoption explicitly incomplete; do not
report older scaffolding as completed assessment.

For an existing project, start an assessment explicitly. New initialization
already creates the record:

```sh
opdev adoption start --dry-run
opdev adoption start
opdev adoption status
```

Review the whole project's scope, including relevant components in mixed-stack
repositories. Preserve adequate tools and conventions. For unresolved gaps,
research suitable options using current primary documentation and project
constraints, then agree on the proposal before implementation. OpDev supplies
a consistent checklist, not a prescribed ecosystem tool stack.

The agent presents preserve/change/ignore/unresolved choices and waits for your
response. You can explicitly delegate bounded choices; asking to adopt is not
blanket delegation. For a non-main trunk, choose whether to keep the name or
rename it to main. Both are supported; a separate integration-to-release branch
promotion workflow still needs migration to one trunk.

With a capable CLI, preview an old record using `opdev adoption migrate`, then
apply the reviewed migration with `--write`. Fill proposed choices and workflow
roles, run `opdev adoption plan`, and review its exact ID before recording your
actual response using `opdev adoption approve --plan ID --reviewer NAME --reference
REF`. Approval does not mark implementation complete. Changed choices need a new
review; `pending` and `in_progress` remain unfinished after approval.

Record an owner, rationale, and evidence references for each disposition in
`.opdev/adoption.yaml`. Implemented automated practices must reference declared
verification suites. Only optional practices may be ignored; conditional ones
may be justified as not applicable. Required practices cannot be waived, and
pending items prevent completion. A resolved checklist is not verification.

After implementation, stage all material changes and review the evidence for
that exact state using the [adoption review contract](../spec/adoption.md).
Then run:

```sh
opdev adoption check --format json
```

Completion requires the current reviewed decisions, successful declared checks,
and all core gates. Local execution does not replace actual CI or delivery
qualification. Save the result in the work item and complete the normal CI
integration workflow. Do not register this command as a project test suite:
it runs the project's verification itself.

Later tasks reuse these decisions; they do not repeat initialization or research
without a relevant change. See [the full specification](../spec/adoption.md)
for resumability, evidence binding, and compatibility.

## Make and verify a change

Start with the project's work item and relevant authorities. Establish the
expected outcome and risks, make a focused change, and add or update regression
coverage for behavior changes. For retained feedback, run focused meaningful
checks and review the direction without finalizing evidence on every tweak.
When the increment is ready for integration, run its required declared checks
and stage **all** material files for the change:

```sh
opdev check
git add -- path/to/changed-file path/to/regression-test
```

If no reviewed assertion is required, skip to your normal integration checks.
If the CLI cannot safely infer a required fact, use the review flow below.
Do not fabricate evidence merely to make a gate pass.

### Review evidence

The following ledger flow is for legacy evidence storage, not for a project
that explicitly selects `assurance.review_storage`. With external storage,
prepare a new candidate outside source (`bootstrap --output`, or `prepare
--ledger-input` / `--ledger-output`), review its actual conditions and assertions,
then explicitly export and retain it at the reviewed authority. Qualification
uses `check --review-locator FILE --review-acceptance-sha256 ID` plus actual
current stage checks. Neither a cached report nor export success proves execution,
consent, retention or authority to delete history. See the
[evidence guidance](../plugins/opdev/skills/opdev/references/evidence.md).

For a legacy-storage project with no `.opdev/evidence.yaml` yet, generate a questionnaire
outside the Git working tree:

```sh
opdev evidence bootstrap > ../opdev-evidence-review.yaml
```

Review and edit the questionnaire. Every decision starts as `review_required`.
Add concrete evidence and choose `passed` or justified `not_applicable` only
where the facts support it. Durable project facts remain separate from
assertions about this staged change.

Preview the candidate ledger, then explicitly write it after review:

```sh
opdev evidence bootstrap --answers ../opdev-evidence-review.yaml
opdev evidence bootstrap --answers ../opdev-evidence-review.yaml --write
git add .opdev/evidence.yaml
opdev check --ci
```

Bootstrap refuses to replace an existing ledger. For direct maintenance, use
`opdev evidence fingerprint` and the [evidence ledger contract](../spec/evidence-ledger.md).
Recheck every assertion you carry forward. Evidence cannot override a concrete
failure, error, or migration requirement.

Unstaged changes and material untracked files block fingerprinting. Any change
to staged paths, contents, or executable bits invalidates change-bound evidence;
only the ledger itself is excluded. Keep temporary questionnaires and reports
outside the repository, or in a deliberately ignored directory.

Commit and push the complete reviewed change through the project's normal pull
or merge request workflow. A local `--ci` check does not replace actual CI or
qualify the project's delivery path.

## Connect CI

For an existing GitHub Actions or GitLab CI configuration:

```sh
opdev ci inspect
```

If no provider configuration exists, generate one baseline:

```sh
opdev ci generate --provider gitlab --write
# Or: opdev ci generate --provider github --write
```

Generation refuses to overwrite an existing provider file. Review and commit
the output. It installs a pinned CLI and evaluates the integration gate; it is
not a complete application build, deployment, or recovery pipeline.

Generated GitLab jobs isolate caches and require a compatible x86-64/glibc 2.39+
environment for OpDev. Preserve product images; review inheritance and test cold
and warm caches when composing with existing CI. Both providers verify the CLI's
signature with a pinned verifier before extraction.

For release qualification, a capable CLI supports `opdev check --ci --delivery`.
It executes delivery-stage suites/extensions and uses the delivery gate for its
exit code; `--no-exec` cannot be combined with it. Wire it as a required predecessor
of publication in the actual release/tag pipeline, with artifact-bound evidence.
A successful integration check alone does not permit publishing or complete adoption.

GitLab generation can infer a toolchain image from supported Rust, Go, Node.js,
or Python version files. For mixed/custom stacks or ambiguous metadata, select
a reviewed image explicitly:

```sh
opdev ci generate --provider gitlab --image registry.example.com/team/toolchain:2026.08 --write
```

Replace that example image with one containing your toolchain, compatible glibc,
a POSIX shell, Git, curl, tar, `sha256sum`, and `mktemp`. The generated job checks
prerequisites and downloads/checksum-verifies the exact CLI outside the checkout.
See [provider boundaries and image selection](../spec/ci-providers.md).

Read-only remote audits are available through `opdev check --remote`. GitLab can
use an authenticated `glab` session when higher-priority provider credentials
are absent. See [credential precedence](../spec/remote-audits.md); never put
credentials in the project contract or evidence ledger.

## Compact inspection in development builds

Compact output is experimental and requires explicit user/project opt-in. Check
the selected CLI's help for `--experimental-compact` and `--report`; version
compatibility alone does not establish every capability. Normal checks use full
human output or `--format json`. These commands apply only after opt-in:

Using a **new** path outside the working tree:

```sh
opdev --experimental-compact check --report ../opdev-check-1.json --format summary
opdev --experimental-compact report summarize ../opdev-check-1.json
opdev --experimental-compact evidence show --current --rule OPDEV-WORK-001
```

The first command retains full JSON and prints a compact view. Summarizing a
saved report does not rerun checks or prove current freshness. Existing report
files are never replaced. Evidence queries require a fully staged state and
return assertions, not gate verdicts. See [compact views](../spec/compact-views.md).

## More help

Use `opdev <command> --help` for exact arguments. The
[specification index](../spec/README.md) links rules, evidence, compatibility,
extensions, and profiles. For questions or bugs, use
[GitLab issues](https://gitlab.com/stolenfootball-tools/opdev/-/issues).
