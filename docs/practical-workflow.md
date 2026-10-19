# Practical workflow (source preview)

`adoption next`, `adoption status`, `adoption start` and `adoption check
--preflight` inspect an already configured practical project without creating a
legacy adoption record. `adoption plan` retains its selected level unless an
explicit `--preset` proposes another. These are not product-verification commands.
Use actual stage checks and native required jobs to verify integration; the
practical route deliberately has no all-in-one adoption certificate.

This opt-in workflow reduces routine coordination and repeated checks. It is
implemented by project schema `5`, policy version `1`. It is not enabled by
updating an existing project's plugin. Schema 1–4 projects keep their selected
policy and verification rules.

Use a source-built CLI advertising `practical.check.v1` in `opdev doctor
--format json`. A matching version number alone does not establish that an
unreleased executable includes this capability. Do not change installed pins or
point production CI at this preview as part of adoption.

## Reuse a development preview in CI

An authorized consumer trial need not compile OpDev on every change. The existing
`package-linux-x86_64` producer job retains its verified Linux x86_64 GNU binary,
source SHA, toolchain description and checksum for seven days. It depends on the
required producer checks and verifies the packaged plugin/runtime combination.
This is a job artifact, not a release, installed-plugin update or test-result cache.

Select a successful producer **job ID**, inspect its source commit and toolchain,
and retain the approved binary SHA-256 in the consumer's versioned CI configuration.
Download the raw `target/x86_64-unknown-linux-gnu/release/opdev` artifact from
`/api/v4/projects/85892266/jobs/JOB_ID/artifacts/target/x86_64-unknown-linux-gnu/release/opdev`.
This uses GitLab's [single artifact by job ID API](https://docs.gitlab.com/api/job_artifacts/#download-a-single-artifact-file-by-job-id).
Verify the downloaded bytes against that independently selected digest **before**
making the file executable or running it. Check plugin compatibility afterward.
Use the reviewed HTTPS provider and existing authorized access; do not broaden
credentials. A checksum downloaded beside an untrusted binary is not a trust root.

Pin the exact job ID, source SHA, platform and digest together. Do not use latest
branch artifacts, download-and-execute installers or share writable executable
caches across trust boundaries. Reuse only on a compatible Linux/glibc host; this
job does not provide Windows, macOS or ARM binaries. Missing, expired or mismatched
artifacts fail closed: deliberately select a new verified artifact or explicitly
return to the exact-source build. Never fall back to a mutable branch. Artifact
retention may be extended through the provider's normal retention controls for an
authorized longer trial; no permanent preview registry is needed.

Consumer checks still run on each required revision and stage. Binary reuse saves
tool setup, not product verification; it does not establish release readiness.

## Choose the assurance, not the model effort

| Preset | Intended use | Assurance tradeoff |
| --- | --- | --- |
| Lightweight | Low-consequence work | Appropriate observable checks and truthful reporting; less persistent traceability |
| Standard | Default recommendation for maintained software | Requirement-led implementation and agentic review, regression checks and native integration gates; no per-change approval ledger |
| Assured | Work needing stronger explicit assurance | Source-bound mapping reviews and mapped checks at each selected verification boundary |

The same agent can implement and perform a distinct review pass. None of these
presets authorizes spawning another agent, publishing a release, weakening a
test, or silently dropping an external obligation. A level is a developer
decision, not a fallback selected when a check fails.

Standard deliberately relies more on substantive agent review and native CI,
and less on mechanically validated review records. The CLI cannot prove test
adequacy, authenticate a decision reference, or certify regulatory compliance.
Assured's source binding makes stale reviews detectable; it does not turn an
agent's judgment into proof.

## Native CI and manual checks

Use `opdev check --ci --expected-revision "$CI_COMMIT_SHA"` for GitLab's
pre-merge checkout, adding `--post-merge` for integrated trunk or `--delivery`
for the separate delivery boundary. GitHub uses its actual checked-out SHA,
which may be a synthetic merge commit rather than the PR head. Preserve the
command's exit code when saving its JSON output; no custom report adapter is
needed. This requires `practical.expected-revision.v1`. It does not verify remote
protection settings or substitute for artifact/host-specific qualification.

With `practical.manual-checks.v1`, declare a manual evaluation directly:

```yaml
checks:
  - id: usability
    manual: docs/usability-procedure.md
    stages: [evaluation]
```

Keep the actual procedure at that path and retain requirement links to the check
ID. `opdev check --evaluation` reports it as `not_run` and exits nonzero, without
launching a fake failing command. A selected manual check also prevents integration
or delivery from passing if assigned to that boundary. Record actual observations
in the project's existing review process; the CLI does not certify them. Moving
an existing required check to another stage is a policy change, not this repair.

## Daily work

For a small fix, state the intended result and relevant checks, implement it,
review the diff and failure paths, and test. Do not require a new issue, milestone
or design document merely to begin. For a larger assignment, identify an MVP,
connected useful slices, dependencies and deferred scope in the existing tracker.
Research consequential uncertainty rather than doing a mandatory research pass
for every edit. Continue independent authorized work while a decision is pending.

Run focused checks while iterating. A feedback checkpoint is not a merge gate.
Configure one authoritative integration route in GitHub or GitLab, preserving
useful platform coverage. Integrated-source checks may differ from candidate
checks; a candidate result is not a trunk execution. Release remains a separate
developer-authorized operation.

```sh
opdev check --plan
opdev check
opdev check --ci --plan
opdev check --ci
opdev check --ci --post-merge
```

Check options select execution boundaries; they do not attest that the caller
is a trusted CI provider. Protect native jobs and branches using provider
settings. `--ci` execution requires a clean committed checkout and checks that
it stays unchanged. Run local feedback checks while editing instead of using
the integration flag on dirty work.

Checks shared by multiple requirements execute once per command ID in that
invocation. There is no cross-revision result cache and no inference that two
different environment jobs are redundant. Missing stages and `--no-exec`
cannot report verification. `--plan` is a successful inspection, not a test pass.

## Small, explicit files

```text
.opdev/
  project.yaml
  guidance.md
  requirements/       # only explicitly selected current catalogs
    product.json
  policies/           # only explicitly selected additional obligations
    organization.json
```

AGENTS.md and CLAUDE.md route to the short guide, preserving unrelated project
instructions. Plans, logs, reports and scratch files do not belong in `.opdev`.
Unknown or unselected files block checks but are never automatically deleted.
Additional definitions are selected by the contract's `policies` paths. Each
schema-1 definition names `id`, original `source`, `decision_reference`, preserved
`origin`, requirement IDs and execution `stages`. At those stages the mapped
requirements need current source-bound review and execution even under Standard
or Lightweight. Policy files are review inputs: changing them invalidates those
reviews. This enforces the declared mapping, not completeness or certification
against an entire external standard. Review scope against its original authority.

A minimal project contract looks like this (replace the example commands and
decision reference with the project's actual choices):

```yaml
schema: 5
policy_version: 1
preset: standard
trunk: main
decision_reference: "Actual developer decision in the project tracker or session"
commands:
  tests:
    argv: [cargo, test, --locked]
    timeout_seconds: 600
checks:
  - id: regression
    command: tests
    stages: [local, pre_merge]
authorities:
  documentation: docs
requirements: [.opdev/requirements/product.json]
```

Requirements describe enduring observable behavior, not an execution history:

```json
{
  "schema": 2,
  "requirements": [{
    "id": "R1",
    "statement": "Malformed checksum input returns the documented error.",
    "verification": [{
      "check": "regression",
      "path": "tests/checksum.rs",
      "assertion": "malformed_checksum_returns_public_error",
      "inputs": []
    }]
  }]
}
```

The named file must exist and the check must be declared. A locator is a review
aid, not a runner receipt: a green suite does not prove that this particular
test ran. Stage ownership belongs to the check, not duplicated requirement
plans. Include known assertion helpers and fixtures in `inputs`; this is not
automatic dependency discovery.

`opdev requirements inspect` returns the selected definitions and computed
mapping subjects without execution. Assured requires each mapping's actual
review with `reviewer`, `reference`, `rationale`, `subject_sha256`, and
`outcome: passed`. The digest binds the declared commands, stages, requirement,
assertion file and named inputs. Changing those invalidates the review;
unrelated source changes do not. A review never replaces fresh execution.
Known failed reviews block every level.

Use `opdev requirements show R1` to inspect one requirement and its declared
checks, and `opdev requirements schema --version 2` for the catalog JSON Schema.
The [project schema](../schema/practical-project.schema.json) describes the
contract's structural format. Runtime validation additionally checks actual
files, safe paths and cross-document references; schema validation is not
verification.

## New projects and migration

```sh
opdev adoption plan --preset standard
opdev init --project /path/to/reviewed-project.yaml --dry-run
opdev init --project /path/to/reviewed-project.yaml
```

The proposal preserves current command and branch information, but leaves its
decision reference null so it cannot be mistaken for an approved contract.
Review the actual tools, execution boundaries, requirement inventory, existing
obligations and branch choice. Keeping a non-main integration branch is valid.

Initialization installs the reviewed contract and thin entry points without
inventing a product or creating a permanent adoption log. An empty project may
have no checks yet: workflow configuration can succeed while product
verification remains explicitly not run. Selected catalogs must already exist.
`opdev adoption status` inspects that configuration and reports remaining check
or mapping gaps without executing anything or claiming product verification.

Initialization refuses to overwrite an existing contract. Use coordinated
migration for existing repositories or upgrades:

```sh
opdev upgrade --request-schema --request-version 2
opdev upgrade --migration /outside/source/request.json
opdev upgrade --migration /outside/source/request.json --apply PLAN_ID --recovery-output /outside/source/recovery.json
opdev upgrade --resume /outside/source/recovery.json --apply PLAN_ID
```

The schema-2 request supplies the full reviewed `project`, an actual
`assurance_change_reference`, exact replacement `files`, and obsolete `retire`
paths with reasons. `retire_directories` names only inventoried empty directories;
removal is never recursive. Files can replace selected definitions and native CI
configuration; unrelated project files are not migration targets. Managed guide
and root routing updates preserve unrelated instructions.

The preview returns `plan.identity` and exact before/after contents. Apply needs
that identity and a new recovery file outside source. Changed inputs or new
namespace content block it; interrupted runs may resume only the original plan's
before/after states. Later edits are preserved, not overwritten. Recovery writes
are checked before changing source. Writes are atomic per file, not a multi-file
transaction; keep recovery until verification and then remove it.

Current requirement IDs must survive or have explicit `retired_requirements`
decision references. Original additional standards, organization policies and
profiles must be mapped through selected policy definitions with the same
`origin`, or have explicit `retired_obligations` decisions from their original
authority. An engineering MinimumCD selection has identity `minimumcd` and
origin `{"minimumcd":"1"}`; other legacy identities use `standards:NAME`,
`organization_policies:ID` or `profiles:NAME`. Practical policies retain their ID.
Neither a reference nor a digest authenticates that authority. Do not invent a
retirement to obtain green checks. Migration refuses transferred passed mapping
reviews and erased known failed reviews. Review the resulting assertions afresh.

After apply, inspect the actual diff, run applicable checks and qualify native CI.
Configuration migration alone is not completed product verification. Retiring
the old ledger needs no permanent archive or historical-copy verification.
Existing Git history is not rewritten.

## Reports and current limits

Human output is concise by default; `--format json` exposes command results,
durations, source identity and review findings. `--report PATH` writes a new
file, never overwrites an existing report. Keep reports in ephemeral CI
artifacts or local state outside source rather than an accumulating ledger.
Preserved command success does not qualify a policy or checkout that changed
during the run; the overall command exits unsuccessfully with the finding.

`ci generate --provider github` or `gitlab` reuses the signed-runtime installer
and native job templates without review transport. GitLab admits MR pipelines
and pushes to the declared trunk, selecting post-merge checks only for the latter.
Generation never replaces an existing file. `ci inspect` reports local provider
observations, not current remote protection or passing pipelines. Review existing
provider settings and observed required jobs using their native interfaces.
Remote doctor inspection and historical review/ledger commands do not qualify a
practical project. Published-version installation does not acquire unreleased
capabilities; use a reviewed source runtime until a release is authorized.

Whole-session cost and quality comparisons remain necessary. Fewer prompt words
are not measured token savings, and passing regression tests are not proof that
the new workflow is as effective on real product tasks. The design and rationale
are in [the practical workflow contract](../spec/practical-workflow.md).
