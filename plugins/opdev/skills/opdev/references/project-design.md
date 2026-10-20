# Agree on the project before building it

Use during new-project discovery, adoption or upgrade, substantial redesign and
next-step planning. Every OpDev project needs three durable Markdown documents:
`design.md`, `interface.md`, and `tech_stack.md`. Default to `docs/`; use explicit
`design`, `interface`, and `tech_stack` authority paths for a custom location.
Keep them outside `.opdev`. Reuse an existing case-equivalent `DESIGN.md`, never
create a competing lowercase file. These roles are mandatory at every assurance
level, including existing projects; their depth follows the actual software.
They are maintained decisions, not a backlog, transcript or execution ledger.

## Discover, agree, then plan

1. **Understand the goal.** Start from the developer's own intended use. Explore
   users/callers, desired outcomes, full scope and constraints; reflect it back for
   correction. Gather important journeys and hard technical constraints early.
   Do not start by defining the answer through a menu of agent-selected solutions.
2. **Agree on the abstract design.** Propose what the software does, conceptual
   responsibilities, scope, exclusions, success criteria and consequential
   alternatives. Record it in `design.md`; obtain product-design acceptance.
3. **Agree on interaction.** Work through concrete user stories or caller journeys:
   entry points, actions, inputs/outputs, states, errors, cancellation/recovery,
   automation/control, permissions and accessibility as applicable. Show examples
   in `interface.md` and obtain interface-contract acceptance. CLI/API/library,
   batch, embedded and GUI software all have observable consumer contracts.
4. **Agree on technology and standards.** Before recommending, ask about unresolved
   language, platform, hosting, tools, team familiarity and maintenance preferences.
   Reuse supplied answers; "recommend sensible defaults" can delegate selection.
   Research consequential unknowns and propose a coherent stack with rationale,
   alternatives, architecture, storage/integrations, dependencies, security and
   operations. Include coding conventions, formatter, linter, applicable type/static
   checks, testing strategy, focused feedback commands and CI boundaries. Record
   in `tech_stack.md` and obtain technical acceptance. Don't defer all standards to
   an unapproved first coding slice. Configure and test agreed tools in that slice;
   exact executable command vectors belong in the project contract, not duplicated
   inventories. Inapplicable tools need a reason, not a placeholder pass.
5. **Agree on the phased plan and execution scope.** Only after these decisions are
   settled, propose connected milestones with an early usable end-to-end MVP and a
   clear path through all accepted later capabilities. Put milestones, sequencing,
   completion demonstrations and issues in the declared work tracker. Detail the
   nearest milestone; retain later scope now without speculative task inventories.
   Roadmap approval and implementation authorization may be one response. A design
   document or milestone's existence does not grant either.
6. **Deliver within that authority.** At each milestone, inspect changed assumptions,
   create/refine its issues and implement useful slices with focused feedback checks,
   then the selected integration checks. Verify assembled outcomes. Reopen only
   materially affected decisions; do not re-interview or reapprove settled choices.
   MVP completion does not erase later scope or authorize release.

These are decision boundaries, not an irreversible waterfall or fixed question
count. A complete supplied and accepted plan can satisfy multiple boundaries.
Present enough substance that "looks good" has an identifiable object; it does
not accept unshown later choices. Requirements answers, a policy preset, silence,
green tests and agentic self-review are not product/interface/technical acceptance.
Explicit bounded delegation is valid only for its actual scope. Routine repairs
within an accepted contract need no new onboarding. A design-feedback request
does not authorize implementation. An authorized isolated feasibility prototype
may precede technical acceptance but cannot silently become retained product code.

## Existing projects and upgrades

Inspect existing docs, source, manifests, tests and CI before questioning the
developer. Populate all three documents with supported facts and source references;
mark inference as inference. Explain current behavior separately from desired
changes. Infer a formatter from its real config, not from the language's popularity.
Don't replace an adequate stack or standards merely to match preferred defaults.
Ask focused questions where intent, policy, contradictions or consequential gaps
cannot be established. Offer a concise preserve/change/resolve review; related
confirmed existing choices may be accepted together, without three ceremonial turns.

Preserve the meaning and scope of previous approvals, not the old arrangement of
text. Reorganization neither revokes decisions nor approves new behavior. When
authorized to split mixed documentation, give each concern one authoritative home:

- `design.md`: product purpose, consumers, outcomes, scope and conceptual boundaries.
- `interface.md`: user/caller journeys and observable behavior, including states,
  outputs, errors and recovery.
- `tech_stack.md`: technical decisions, architecture, rationale and engineering
  standards. A component diagram is technical detail; its product responsibility
  may still need a brief explanation in the abstract design.

Move the substantive content, retire its superseded copy and update links and
authority routes, including generic architecture/documentation routes that pointed
to the former mixed owner. Three summaries pointing back to an unchanged mixed master do
not separate concerns. Keep useful, narrowly scoped supporting specifications or
research at their existing owners; link for depth, not to avoid stating the role's
contract. Do not create a fourth catch-all or archive of the old mixed document.
This is not permission to move unrelated files or erase unresolved requirements.

Classify the claims before relocating them: accepted decisions with their actual
scope/reference, observed or inferred facts, proposals, and unresolved questions.
Do not transfer speculative mechanisms wholesale into the technical document under
an accepted heading. Keep material alternatives/questions concise at the relevant
owner and task sequencing in the work tracker. Preserve accepted later scope even
when its implementation details remain open. Ask only where a consequential
ambiguity cannot be resolved from the original decision.

A document-organization task does not authorize filling gaps with new behavior.
Even a sensible extra error rule, control or example must be supported by the
existing contract or visibly proposed; do not place it under inherited acceptance.

Before handing off, read the resulting files against the original material and
decisions: are accepted meanings retained, ownership actually separated, proposals
still distinguishable, and links/routes current? Name a concrete moved concern and
any unresolved distinction in the existing review. A changed heading, valid
metadata or the agent's completion summary does not establish this. Test migrations
by inspecting rewritten artifacts, not only a conversation saying it would split
them. No extra registry, paragraph ledger or automatic semantic gate is required.

An empty repository may have workflow configuration while these documents are
still proposed. Report design/adoption gaps separately: setup is not completed
adoption, technical acceptance, product verification or execution authority. An
upgrade adds missing documents as an explicit documentation migration; ordinary
maintenance need not stop for unrelated retrospective discovery. Do not claim
current adoption complete until all three contain adequate reviewed decisions.

Adding these documents and their authority routes is an ordinary authorized
documentation/configuration edit, not by itself a policy or assurance migration.
Use coordinated policy migration only when the actual contract change requires it.
Don't demand a new approval of unchanged decisions or a full policy-migration
ceremony merely to split an already accepted combined document. An adoption-only
request does not authorize inventing new product milestones; plan the adoption
work and retain existing product scope.

## Document format and status

Use the bundled assets under `assets/project-documents/` as editable starting
points, not finished documents. Preserve project-specific content. Each document
has YAML frontmatter:

```yaml
opdev_document:
  schema: 1
  role: design
  decision: proposed
  reference: null
  scope: Entire product concept; later implementation detail remains provisional.
```

Roles are `design`, `interface`, `tech_stack`. Decisions are `proposed`, `inferred`,
`accepted` or `delegated`. Accepted/delegated needs an actual decision reference
(tracker discussion or identifiable conversation response) and its bounded scope.
Record material unresolved choices in the body, not as accepted by inference.
The reference is attributed information, not authenticated approval. Never invent
a response or use a file's own existence as its decision reference. After a
material change, mark the affected proposal unresolved while retaining the prior
decision's scope in the body; don't carry a blanket accepted label onto new text.

With `planning.documents.v1`, `documentation status` inspects all three files
without commands or writes; `adoption status`/`adoption next` include the findings.
Missing files, unsafe routes, malformed metadata and unreviewed decisions remain
visible. Structural success cannot establish semantic adequacy, current consent,
roadmap approval or implementation authority. Read original decisions, changed
content and current work before acting. On an older CLI, report the capability
gap and inspect these facts directly; don't invent a successful CLI check.

For "what next?", resolve the earliest material unanswered boundary before
recommending product implementation. If all three are settled, inspect roadmap and
execution authority at the work tracker. Do not run full integration CI just to
show a design or ask a preference question. No separate approval ledger is added.
