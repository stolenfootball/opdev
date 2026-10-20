# From an idea to an approved implementation plan

OpDev separates three questions: what are we building, how will someone use it,
and how will we implement and maintain it? Agreeing on one does not automatically
answer the others.

1. Describe the need in your own words. The agent asks relevant follow-ups and
   reflects its understanding before choosing a solution.
2. Review the abstract product design: behavior, scope, constraints and tradeoffs.
3. Work through real user stories or caller journeys, then review the interface
   contract, including errors and recovery—not just a list of screens or endpoints.
4. Share technical preferences or ask for recommendations. Review the proposed
   stack, architecture, formatting, linting, applicable static checks, testing,
   focused checks, CI and operational standards together.
5. Review a phased roadmap with an early useful MVP and a path to all accepted
   later capabilities. Authorize the implementation scope. The agent then works
   milestone by milestone, creating detailed issues as they become useful.

These are decision boundaries, not six mandatory meetings. A sufficiently complete
approved brief can cover several at once. Later discoveries can revise affected
decisions without reopening everything. Repairs inside an agreed design remain
direct. You can delegate technical selection explicitly; the agent must honor its
limits. Routine implementation review is agentic, not another human sign-off.

## The three required documents

Every project, at every assurance level, maintains:

| Document | Enduring contents |
| --- | --- |
| `docs/design.md` | Consumers, problems/outcomes, product behavior, conceptual boundaries, constraints and accepted scope. |
| `docs/interface.md` | Journeys and observable inputs, outputs, states, errors, permissions, accessibility and recovery. This also applies to CLI, API, library and batch software. |
| `docs/tech_stack.md` | Stack/architecture choices and rationale; conventions, formatter/linter, relevant static checks, tests, CI, dependencies and operations. |

Custom directories use the `design`, `interface` and `tech_stack` authority keys
in `.opdev/project.yaml`. Practical contracts use path strings; older contracts
use `{kind: path, location: ...}`. An existing `DESIGN.md` may serve the design
role through an explicit route; do not make a conflicting lowercase copy.
Keep these files outside `.opdev`. Each concern has one authoritative home:
the product design should not remain a mixed master containing the real interface
and technical contracts while the other files merely summarize or link to it.
Narrowly scoped supporting specifications and research can supply deeper detail;
links connect owners rather than creating competing copies. Exact command argument
vectors live in the project contract. Milestones, issues, sequence and progress
live in the work tracker—not in these documents.

The plugin includes draft templates in `skills/opdev/assets/project-documents/`.
Replace their prompts with actual project facts. Each file includes a small
decision header, for example:

```yaml
---
opdev_document:
  schema: 1
  role: tech_stack
  decision: inferred
  reference: null
  scope: Existing Python library and its inspected development tools.
---
```

Use `proposed` for a proposal, `inferred` for observed existing choices, or
`accepted`/`delegated` with an actual decision reference and bounded scope. A label
is not proof of approval. Material new proposals cannot inherit a blanket approval
from old content. Preserve prior decisions and identify what remains unresolved.

## Adopting an existing project or upgrading

OpDev inspects the existing code, documentation, manifests, tests and CI and fills
in what those sources support. It keeps adequate tools and approved decisions.
It asks about consequential gaps, conflicts and preferences—not facts it can read.
For example, a configured formatter can be documented; an absent linter is a
question/recommendation, not silent permission to install the agent's favorite.
You can review related preservation choices together rather than repeat the entire
new-project interview. Missing documents remain an adoption gap until populated
and their relevant decisions reviewed. Adding document structure does not invalidate
earlier genuine approvals or require replacing the stack.

When splitting an existing mixed document, move the substantive content to its
role, retire superseded duplication, and update links and authority routes.
Check generic architecture/documentation routes as well as the three role keys.
Preserve the meaning of approved decisions, not an obsolete text arrangement.
Separate confirmed choices, inferred current facts, proposals and open questions.
Do not turn every speculative mechanism into an accepted technical requirement
merely by moving it. Keep relevant uncertainties concise; retain accepted later
scope and leave sequencing in the tracker. No fourth catch-all or archive is needed.
New error rules or other plausible details still need support or a proposal label;
document organization does not authorize filling design gaps on the user's behalf.
Review the actual resulting documents against their sources, not just filenames,
metadata or an agent's summary of what it changed.
Adding these documents alone is not an assurance-policy migration. Already
authorized document organization can proceed without reapproving unchanged
choices. Adoption alone does not authorize new product capabilities.

`opdev documentation status` reports missing documents, invalid metadata and
unreviewed decisions without executing commands or changing anything. Adoption
status/next includes the same findings. A successful structural result does not
prove content adequacy, current consent, a roadmap approval or permission to code.
Read the referenced decisions. An empty repository can be workflow-configured
while design and adoption are still incomplete; that is not product verification.

Use focused review while discussing designs. Full integration CI belongs at the
settled integration boundary, not before you can answer a stack question. Keep
release authorization separate from roadmap, milestone and implementation approval.
