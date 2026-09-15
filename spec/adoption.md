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

## Record

`scope` describes the reviewed repository/components. `practices` contains exactly
the catalog IDs. Each decision records:

- `state`: `pending`, `implemented`, `ignored`, or `not_applicable`.
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
existing reviewed decisions; no automatic version update is implemented in v1.

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

The checker then executes pre-merge suites and extensions using the existing
engine, inspects CI, optionally audits the provider, requires every referenced
suite to pass and all four core gates to pass. It rechecks staged/evidence
freshness after execution. A record or passing command alone cannot qualify
delivery. Real pre/post-integration CI, artifacts and recovery still need the
evidence required by their core rules; local execution does not become remote CI.
Do not register `adoption check` as a suite that it would recursively execute.

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
their contract and does not add a record; `upgrade` still refreshes only managed
guidance. Use `opdev adoption start --dry-run`, then `opdev adoption start` after
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
