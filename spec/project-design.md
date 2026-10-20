# Project design and acceptance boundaries

Every OpDev project has three maintained documents outside `.opdev`: `design.md`,
`interface.md`, `tech_stack.md`. Default location is `docs`; explicit authority
keys of the same names (without extension) select repository-relative files.
Case-equivalent existing filenames are valid. Existing projects populate them
from inspected sources and preserve adequate choices; missing intent or material
conflicts need developer input. Current implementation is evidence of what exists,
not evidence that the developer approved it. A small library needs concise caller
and technical contracts, not an invented UI or enterprise process.

Each concern has one authoritative home. Authorized migration of mixed material
moves product scope to design, observable interaction to interface, and technical
architecture/standards to tech_stack, removing superseded duplication and repairing
routes/links. Three summaries pointing back to an unchanged mixed master are not
adequate separation. Narrowly scoped supporting specs/research remain valid; a
fourth catch-all is not a substitute. Preserve approved meaning and attribution,
not the old layout. Distinguish accepted, inferred, proposed and unresolved claims;
neither moving speculative detail nor a file-level label accepts it. Preserve
accepted later scope while keeping task sequencing at the work authority.

Review source-to-result preservation and concern ownership in the actual artifacts.
This is semantic review, not a deterministic CLI guarantee. Behavioral migration
trials must inspect generated files as well as the conversation; structural tests
and heading matches cannot establish this property.

Product acceptance, interface acceptance and technical acceptance are distinct.
They precede the phased implementation proposal and its execution authorization.
Gather user journeys and hard constraints during discovery; refine these together,
not as irreversible waterfall phases. Reuse complete approved briefs and bounded
delegation. Reopen only affected material decisions. A requirements answer, preset,
file, issue, self-review or green check does not approve unshown choices.

The roadmap has an early usable end-to-end MVP, connected later capabilities and
assembled outcome checks. Preserve all accepted later scope in the work tracker;
detail issues near each milestone rather than inventing distant implementation.
Within approved scope, continue without repetitive permission. Release remains
separately authorized. Authorized isolated feasibility probes can precede technical
acceptance, but do not become retained implementation without the relevant decision.

## Structural inspection

`planning.documents.v1` provides read-only `documentation status --format json`
and embeds its report in `adoption status`/`adoption next` for both policy routes.
It reads no providers and executes no project commands. Document frontmatter uses
`opdev_document` with schema 1, matching role, nonempty scope, decision enum
`proposed|inferred|accepted|delegated`, and a reference for the latter two.
Unrelated frontmatter may coexist; unknown OpDev decision fields are errors.
There must be body text, not only headings. This establishes structure, not
substantive adequacy, completeness, freshness or authenticity of a decision.

Each role has a distinct filename; fallback uses the documentation directory or
`docs` when undeclared. A documentation-file authority needs explicit role routes.
Traversal, absolute paths, `.opdev`/`.git` placement, linked inputs, nonregular or
oversized files are refused. Reuse the bounded regular-file reader. Inspection
never relocates content, manufactures templates or rewrites policy.

Exit 1 from `documentation status` means missing/malformed documents or unreviewed
attributed decisions. Exit 0 means the three decision references are recorded,
**not approval or permission to implement**. Next-step advice names the earliest
unresolved role, then directs the agent to actual decisions, roadmap and execution
scope. Ordinary adoption status remains successful read-only inspection even when
it reports gaps. Practical adoption preflight exits 1 for document gaps; current
legacy adoption verification blocks before execution on those gaps. Explicit
`--legacy-assessment` preserves historical policy assessment, never current adoption
completion. Ordinary integration checks are not retroactively new design gates.

Changing material content requires revisiting its attributed decision; an agent
must not retain an accepted blanket label over a new proposal. No signed consent,
approval ledger, content-hash machinery or transcript archive is introduced.
References point to actual decisions in existing work/conversation authorities.
Read them after context resets. Configuration and adoption/design completeness
remain separate, so new projects can configure workflow while design is pending.

## Rationale and reversal

Prose alone left agents recommending implementation after only product/interface
agreement. Three discoverable documents and structural feedback make the missing
technical boundary visible without pretending a CLI can evaluate design or consent.
Reject automatic stack substitution and approval-by-inference. Reject three forced
question rounds when an existing accepted baseline supplies the answers.

The ordering follows discovery before construction and story conversation before
confirmation, not a fixed-duration methodology: [GOV.UK discovery](https://www.gov.uk/service-manual/agile-delivery/how-the-discovery-phase-works),
[GOV.UK alpha](https://www.gov.uk/service-manual/agile-delivery/how-the-alpha-phase-works),
and [Agile Alliance's three Cs](https://agilealliance.org/glossary/three-cs/).
The document names/metadata are OpDev's own convention, not requirements of those
sources. Revisit if trials show repeated settled questions, stale decisions,
ceremonial paperwork or ignored missing boundaries. Test actual behavior separately
from routing and structural parser tests; neither establishes general agent reliability.
