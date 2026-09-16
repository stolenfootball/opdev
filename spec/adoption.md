# Explicit, project-specific adoption

Creating OpDev files is not completion of initialization. New initialization
starts a resumable assessment in `.opdev/adoption.yaml`; every practice in the
versioned [adoption catalog](../rules/adoption.json) MUST be addressed before
completion is claimed. The [record schema](../schema/adoption.schema.json) is
separate from project-manifest schema 1 and the core rule catalog.

## Decisions, not tool stacks

OpDev supplies a fixed assessment structure, safe command execution, evidence
semantics and completion checks. It does not prescribe a formatter, linter,
framework, scanner, or package manager for every language. Discovery is a
read-only proposal, not an inventory guarantee or a claim of implementation.

The agent MUST assess all relevant components, including mixed-language and
unusual projects, and record that scope. Preserve adequate existing conventions,
tools, configuration precedence, commands and CI. Research only unresolved gaps,
using authoritative documentation and project constraints (platform, framework,
compatibility, licensing, maintenance, cost and workflow). Recommend one option
with its rationale; present alternatives when the tradeoff matters. Record the
accepted choice and sources for reuse. Do not repeat research on ordinary tasks
unless requirements change, compatibility fails, maintenance problems appear,
or the user requests reconsideration. Offline uncertainty remains pending.

Present an adoption plan grouped as preserve, add/change, ignore and unresolved.
Ask focused, grouped questions for material choices. An explicit request to adopt
OpDev is consent to begin assessment, not blanket permission to install arbitrary
tools, rewrite CI, reformat the repository, alter protections, or publish software.
Implement the approved plan incrementally. Formatting-only migrations and behavior
changes should remain independently reviewable.

Recommendations MUST satisfy the same core constraints as implementation. User
approval cannot convert a known violation into a valid option. Keeping conflicting
integration/release roles is a deferred migration, not completed adoption. Manual
approval inside CI is distinct from manual distribution outside CI. A qualified
archive alone does not establish CI-exclusive delivery. Diagnostics such as stderr
and exit status can implement proportionate observability, but are not recovery
of a defective release. Local execution or synthetic-fixture status alone cannot
establish inapplicability of recovery, observability or effectiveness.

## Record

### Meaningful decision review

Adoption review MUST distinguish observed facts, developer choices and authority
to implement. Before treating inherited decisions as accepted, inspect their
actual approval sources. Agent summaries, owners, migrated records and issue
states alone do not establish consent. Unavailable or disputed approval remains
unconfirmed; retain history and revisit the affected decisions, not every settled
choice in an ordinary task.

Resolve material choices through focused question rounds before seeking approval
of the resulting implementation plan. Each question SHOULD explain the facts,
valid alternatives, recommendation and consequences. There is no fixed total
question count; ask only when an answer affects a material decision and cannot
be established by inspection or trustworthy existing approval. Non-main naming
choices MUST explicitly offer retaining the name and renaming to `main` unless
already settled by verified consent. Keep role compliance separate from naming.

Use a host question UI only when available and permitted for that question;
otherwise ask in chat and stop for required decisions. Silence, defaults, skipped
answers and timeouts MUST NOT become approval. Partial responses or bounded
delegation authorize only their actual scope. Host permission prompts remain
separate from policy decisions. No host-specific tool or mode is mandatory.

Capability classifications require scoped needs and evidence, not a list of
plausible features or a developer vote. Neither missing evidence nor consent
establishes implementation or inapplicability. Group mechanical repairs apart
from policy selections and evidence reclassifications. Final plan approval MUST
NOT substitute for unanswered material choices. Reuse existing work/decision
references; no new ledger, schema version or claim of authenticated consent is
introduced by this conversational requirement.

`scope` describes the reviewed repository/components. `practices` contains exactly
the catalog IDs. Each decision records:

- `state`: `pending`, `in_progress`, `implemented`, `ignored`, or `not_applicable`.
- `owner`: the accountable reviewer/owner.
- `reason`: the project-specific implementation or disposition rationale.
- `references`: canonical implementation, decision or applicability locations.
- `suites`: existing project test-suite IDs, not copied command definitions.
- `research`: sources consulted for new choices; empty when research was unnecessary.

All decisions begin pending, even when tools are detected. Resolved decisions
require nonblank owner/reason and references. `implemented` automated practices
require executable suites registered for both pre_merge and post_merge. One suite
may cover multiple practices/components if reviewed evidence establishes coverage.
Do not create placeholder tests merely to satisfy the shape of a record.

Only catalog practices marked optional may be ignored. Conditional practices may
be not applicable with reviewed justification, but cannot be ignored. Required
practices permit neither disposition. Ignoring an optional practice NEVER waives
an applicable core rule or selected assurance requirement. For example, declining
coverage collection does not waive required behavior tests. A tool failing to
install is a pending/error condition, not an automatic reason to ignore a practice.

Active migration work stays in the work tracker. The adoption record contains
decisions and pointers, not duplicate tool configurations or a separate roadmap.
An absent practice, malformed record, unknown field, or unsupported catalog is not
an implicit opt-out. Future catalog migrations must be explicit and preserve
existing reviewed decisions; catalog upgrades are never automatic.

## Schema 2: approval separate from implementation

New adoption records use schema 2, independently of project-manifest schema 1.
New initialization does not require adoption migration. Inspect the adoption
record's own version before proposing migration. Adoption schema 1 remains readable for inspection and ordinary
work, but cannot complete adoption until explicitly migrated. `adoption migrate`
prints a schema-2 preview; `--write` preserves all old dispositions and references
without inventing approval. No catalog or project-manifest version is changed.

`adoption plan` prints the record, full project contract and a SHA-256 plan ID.
The ID binds scope, choices, rationale, owners, references, suites, research and
workflow roles to the contract. It excludes the approval itself and normalizes
pending/in-progress/implemented as one implementation choice, so approved work
can progress without a new policy decision. Changing an exclusion, rationale,
scope, references or the contract requires a new review. This ID is not the staged
acceptance fingerprint: repository changes still require fresh acceptance evidence.

The optional `review` contains `plan_id`, `reviewer`, `reference` and optionally
`delegation` describing the actual user-granted scope and limits. After a real
developer response, `adoption approve --plan ID --reviewer NAME --reference REF`
records that claim; `--delegation TEXT` records bounded delegated choice.
Approval commands reject stale plans and incomplete assessment metadata.
Agents MUST NOT substitute calling this command for obtaining consent.
An adoption request, an owner string or host command approval is not policy
approval. Hashes and references are tamper/staleness aids, not authenticated human
consent; use host confirmation or independent provider review for that assurance.

The `workflow` records `integration_branches`, `release_source`, `main_option`
(`already_main`, `keep_name`, `rename_to_main`) and reviewed policy/CI `references`.
Exactly one integration branch and the same release source must match the declared
trunk. Offer retaining a non-main name or renaming to main; neither choice may be
silently selected. A branch rename is not a remedy for conflicting GitFlow roles.
No branch deletion, protection change or history rewrite is performed by the CLI.
Role assertions remain reviewed facts, not automatic discovery of every branch.

Missing configuration and software-kind labels do not prove inapplicability.
Accessibility, operational and effectiveness decisions require actual capability
assessment. Optional exclusions need explicit developer agreement. The coverage
collector rule may be N/A when no collection is declared; this never resolves the
separate adoption coverage decision or waives behavioral tests.

Use one adoption work item unless approved sequencing requires independently
owned issues. Closure requires acceptance evidence, not generated files or green
integration CI. Reports distinguish scaffolded, proposed/approved, implemented,
verified and delivery-ready; status never certifies completion.

## CLI workflow

1. `opdev init --dry-run` prints the proposed project contract on stdout and the
   assessment inventory on stderr for new projects. It writes nothing.
2. `opdev init` creates pending adoption state, the project contract and managed
   AGENTS/CLAUDE guidance. It does not install selected tools or execute discovery
   proposals. Exit 0 means scaffolding succeeded, not adoption completed.
3. Review and implement the approved plan. Edit canonical project settings and
   adoption decisions, preserving project-owned instructions and documentation.
4. `opdev adoption status` reports decisions/gaps without executing anything.
   `decisions_ready_for_verification` is not a completion verdict.
5. Stage all material files and review evidence as described below.
6. `opdev adoption check` verifies completion for the evaluated staged state.

`adoption catalog` prints the pinned practice definitions. `status` and `check`
support `--format json`. `check --report PATH` retains the full core report in a
new file; use an external or ignored location. `check --remote` additionally
requests read-only provider auditing. No completion marker is written: retain the
result in the work authority/CI evidence rather than treating a mutable flag as
permanent certification. Ordinary development continues through normal OpDev
checks; do not rerun adoption research on every change.

## Completion evidence

The checker first validates all dispositions, suite references and stages, agent
file presence, and staged freshness. Before it will run project commands, the
matching change in `.opdev/evidence.yaml` MUST contain passed OPDEV-WORK-001 and
OPDEV-TEST-002 assertions with an `adoption_review` evidence entry pointing to
`.opdev/adoption.yaml`. The entry's summary identifies the actual reviewed scope,
accepted choices/opt-outs, implementation and acceptance evidence. This specific
review cannot be replaced by durable project assertions or unrelated generic work
evidence. Use the existing bootstrap review process for a new ledger; add the
entry to the change evidence after explicit review. No new attestation mechanism
or fingerprint exclusion is introduced.

`adoption prepare-evidence` prints an unresolved partial review worksheet with
the staged fingerprint and required `adoption_review` kind/location. It never
writes the ledger or supplies a passing decision/summary. Complete the actual
review and merge justified assertions into the matching ledger change; the partial
worksheet is not a full `evidence bootstrap --answers` input. Diagnostics separate
fingerprint freshness from missing or wrongly classified assertions.

The checker then executes pre-merge suites and extensions using the existing
engine, inspects CI, optionally audits the provider, requires every referenced
suite to pass and all four core gates to pass. It rechecks staged/evidence
freshness after execution. A record or passing command alone cannot qualify
delivery. Real pre/post-integration CI, artifacts and recovery still need the
evidence required by their core rules; local execution does not become remote CI.
Do not register `adoption check` as a suite that it would recursively execute.

Completion additionally requires a passed MCD-PIPELINE-001 result with a
`delivery_gate` evidence entry pointing to the reviewed release/tag pipeline
dependency path. Its summary must identify the required qualification job and
publication ordering, including trigger/rule coverage and the actual run reviewed.
A generic pipeline assertion or green integration-only job is not this review.
This remains a reviewable claim, not automatic proof of provider execution.

Exit 0 from `check` means completion verified for this state under the existing
reviewed-evidence trust model. Exit 1 means pending decisions, insufficient/stale
review, failed checks or blocked gates. Exit 2 means malformed/unsupported input
or tooling failure. `status` exits 0 when inspection succeeds, even while pending.
If prerequisites are unresolved, `check` executes no project commands. Ignoring
practices never changes core result aggregation. Review references are project
claims, not proof of reviewer identity or reference authenticity; source isolation,
meaningful assertions and complete component coverage require competent review.

## Legacy projects, interruption and runtime compatibility

Existing projects without a record remain `legacy_unassessed`. `init` preserves
their contract and does not add a record; [upgrade](upgrades.md) previews and
explicitly applies only managed guidance. Use `opdev adoption start --dry-run`, then `opdev adoption start` after
choosing to assess the project. Neither command resets existing decisions.

New initialization writes unresolved adoption state atomically before the project
contract. If later writes fail, the next init recognizes the partial adoption and
preserves its record. Repair conflicting agent markers explicitly, then retry.
Initialization is resumable, not an all-files transaction. Malformed existing
adoption state fails closed and is never silently replaced.

These commands are development capabilities, not present in every published CLI.
Agents must check `opdev adoption --help` before promising this workflow. If the
selected runtime lacks it, offer a compatible CLI or report the capability gap;
do not call old scaffolding full adoption or install an unqualified runtime pin.
Existing ordinary workflows remain usable; no release or runtime pin is changed
by this implementation.

## Decision and reversal trigger

The generalized gap was that absent checks could vanish from discovery, leaving
no distinction between deliberate exclusion and oversight. Choose one catalog,
one project decision record, project-specific research and existing evidence
gates. Reject fixed ecosystem stacks, repeated research on settled decisions,
automatic opt-outs and a permanent boolean completion marker. This adds a small
review obligation while avoiding a language/plugin installation framework.

Revisit discovery helpers when real adoption demonstrates repeated missed facts;
revisit evidence adapters when manual review becomes the bottleneck. Neither
change should prescribe tools or infer that omitted practices are satisfied.

## Audit-driven acceptance

[Work item 37](https://gitlab.com/stolenfootball-tools/opdev/-/issues/37) tracks the
approval and brownfield-CI regression. The design follows reviewed plan/apply
separation ([Terraform workflow](https://developer.hashicorp.com/terraform/intro/core-workflow)),
stale-review invalidation ([GitLab approvals](https://docs.gitlab.com/user/project/merge_requests/approvals/settings/)),
and incremental migration without a compliance claim
([MinimumCD brownfield](https://beyond.minimumcd.org/docs/migrate-to-cd/brownfield/)).
Avoid a new provider-specific approval service or pretending a local hash proves
human identity. Revisit authenticated approval adapters when a supported host can
provide verifiable, portable response provenance.

Deterministic fixtures cover approval/staleness/delegation, legacy preservation,
non-main trunks versus GitFlow roles, missing-config applicability, unresolved
evidence preparation and delivery-versus-integration exit codes. Host-agent
behavior needs separate transcript/outcome evaluation on both Codex and Claude:
no unsolicited implementation before choices, no fabricated approval, no issue
proliferation/closure, correct scope under delegation. Use neutral disposable
projects. Local unit tests are not those live agent canaries or release qualification.
