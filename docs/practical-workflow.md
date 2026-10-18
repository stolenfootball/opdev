# Practical workflow (source preview)

This opt-in workflow reduces routine coordination and repeated checks. It is
implemented by project schema `5`, policy version `1`. It is not enabled by
updating an existing project's plugin. Schema 1–4 projects keep their selected
policy and verification rules.

Use a source-built CLI advertising `practical.check.v1` in `opdev doctor
--format json`. A matching version number alone does not establish that an
unreleased executable includes this capability. Do not change installed pins or
point production CI at this preview as part of adoption.

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
```

AGENTS.md and CLAUDE.md route to the short guide, preserving unrelated project
instructions. Plans, logs, reports and scratch files do not belong in `.opdev`.
Unknown or unselected files block checks but are never automatically deleted.
The target design also allows selected additional policies; this source preview
does not yet implement that contract and rejects unsupported policy fields.

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

Initialization refuses to overwrite an existing contract. The coordinated
schema-5 migration path is not implemented in this preview; do not manually
replace the manifest to bypass that guard. Existing project catalogs and
external policies are not automatically converted by the proposal. Migration
must explicitly review assurance changes, preserve actual obligations, retire
obsolete active artifacts and retain temporary rollback protection until
verification. Existing Git history is not rewritten.

## Reports and current limits

Human output is concise by default; `--format json` exposes command results,
durations, source identity and review findings. `--report PATH` writes a new
file, never overwrites an existing report. Keep reports in ephemeral CI
artifacts or local state outside source rather than an accumulating ledger.
Preserved command success does not qualify a policy or checkout that changed
during the run; the overall command exits unsuccessfully with the finding.

This preview supports practical initialization, proposal, requirement inspection,
local/staged checks and read-only doctor inspection. Remote doctor inspection,
additional-policy assessment, automated migration and all legacy convenience
commands are not yet adapted. They must not be used to imply complete adoption
or remote qualification. Existing native CI can execute a compatible source
runtime, but published-version installation does not acquire unreleased support.

Whole-session cost and quality comparisons remain necessary. Fewer prompt words
are not measured token savings, and passing regression tests are not proof that
the new workflow is as effective on real product tasks. The design and rationale
are in [the practical workflow contract](../spec/practical-workflow.md).
