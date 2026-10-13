# Engineering baseline 3

Engineering policy 3 is a versioned, risk-based minimum for ordinary software
development, including solo-maintainer projects. It separates mandatory engineering
outcomes, capability-specific safeguards, and independently selected external or
organization standards. Adequate tools and implementation choices remain local.
An accepted residual risk does not turn a missing or failed required check green.

## Outcomes and everyday workflow

The baseline requires a clear purpose and consumer outcome; assessed risks and
applicability; controlled inputs and reproducible setup; understandable design;
meaningful verification; reviewed integration; safe delivery and operation;
proportionate lifecycle responsibility; and honest evidence and developer authority.
These are outcomes, not nine new forms. Existing requirements, project authorities,
change review and bounded CI reports own the relevant facts.

Plan thin useful slices, including the safeguards needed for each slice. A usable
MVP can exclude future features, not the integrity or security required for its
current use. Run focused checks while iterating; finalize current review and
required checks at integration. A slow pipeline prompts improvement, not another
merge blocker or permission to omit assurance. Measure feedback delay, queue/setup
time, compute, false blocks and unnecessary questions separately from correctness.

## Agentic review is the default

The normal reviewer is an agent. One implementing agent can switch to a distinct
review pass: reread accepted requirements, inspect the actual diff and assertions,
trace meaningful consumer/failure paths, challenge plausible defects, and resolve
or retain concrete findings. Review clear responsibilities and interfaces,
unnecessary coupling or abstractions, and affected user/operator documentation.
No human sign-off, second agent, new review service or universal checklist file is
required. Do not silently spawn agents merely because review is required.

Increase review depth for data loss, security, compatibility, large blast radius,
or uncertain external behavior. State what the agent could not establish. Self-review
is not an independent assessor, and agent agreement or a digest cannot prove
correctness. Selected domain or organization obligations still apply. Ask the
developer only for genuinely unresolved material choices or authority; do not
invent a routine approval checkpoint. A passing review never authorizes release.

## Meaningful verification

Use accepted behavior to derive expected results, not snapshots of whatever the
implementation does. Cover the assembled consumer path and relevant boundaries,
errors and side effects. Critical assertions must distinguish plausible wrong
behavior: an old-defect regression, negative example, property, bounded mutation
or fault injection may suffice. Reuse adequate existing assertions. No fixed
coverage percentage, mutation quota, test pyramid, function size or architecture
style establishes quality by itself.

Policy 3 replaces the absolute deterministic-test duty with controlled repeatable
procedures and appropriate decision rules. For statistical checks, specify the
workload/data, sampling procedure, decision threshold, uncertainty, retry and
stopping rules before results. Keep failed attempts; do not retry until green or
choose a threshold retrospectively. An inconclusive required result is unverified.
This accommodates external systems and probabilistic behavior without treating
unexplained flakiness as acceptable. Effectiveness and correctness remain distinct.

## Capability safeguards 2

Version 2 retains all version-1 capability facts and adds two independent questions.
Missing facts are unknown, not absent. The same acceptance review maps affected
objectives to existing conditions or durable criteria; one meaningful check can
support several objectives without duplicate execution.

| Capability | Additional obligation |
| --- | --- |
| Personal or sensitive data (`sensitive_data`) | `data_handling`: purpose/minimization, retention/deletion, disclosure and third-party processing, including logs, fixtures, reports, exports and model prompts. Transient processing still counts without a database. Verify the relevant declared handling; this is not legal certification. |
| High-consequence behavior (`high_consequence`) | `domain_assurance`: identify consequences, domain obligations, failure containment and the evidence needed for the claimed use. Retain unresolved assurance; an ordinary OpDev pass is not a safety certification or proof an agent has domain competence. |
| Operations (existing) | `operational_objectives` in addition to recovery: meaningful success/failure detection, relevant latency/throughput/resource or capacity limits, response and safe stopping/recovery. Use service SLOs where appropriate; a CLI may instead need bounded memory, useful errors and data integrity. |

Progressive exposure is useful where it can reduce relevant risk, not a mandatory
canary deployment for every library or CLI. Retain required deployment/environment
checks even when earlier source checks can be reused. Unknown impact broadens
investigation; unrelated documentation can remain unaffected with a real review.

## Lifecycle

Reuse existing documentation to identify supported versions/platforms, defect and
private vulnerability routes, dependency maintenance ownership, compatibility and
deprecation expectations, and end of support. One person can own all of these.
No public repository, support team, universal support period or new lifecycle file
is required. Review affected promises when changing behavior.

## Version and evidence boundaries

Select engineering `"3"` explicitly in project schema 4 with safeguards version 2
and an actual policy decision reference. Catalog 5 retains historical definitions
for separate exact-version standards. Engineering policies 1 and 2 retain their
meanings; installing a new binary does not migrate a project. Policy preview
accounts for the predecessor rules, replacements and new obligations.

Use `opdev policy explain --engineering 3` for definitions and
`opdev upgrade --engineering-policy 3 --policy-review-reference <decision>` for a
read-only candidate. Existing capability facts remain; new questions start unknown.
Inspect the proposed contract and resolve actual applicability before applying the
reviewed migration. Policy 3 uses clean-adoption target 4 and adoption inventory 3;
the migration reopens testing, review and operational decisions whose duties changed
and leaves new practices pending. An old approval cannot transfer to that new plan.
Policy-2 projects retain clean-3 unless explicitly migrated. Verify both local and
CI support before selecting policy 3; no CLI/plugin installation is implied.

`OPDEV-VERIFY-001` replaces `MCD-TEST-001` and `MCD-TEST-002` in engineering gates;
the originals remain available for the pinned MinimumCD assessment. Its integrated
verification may use [reviewed native-candidate source checks](ci-candidate-reuse.md)
under explicit equivalence and freshness controls. This never establishes literal
post-merge execution, artifact qualification or deployment success. `OPDEV-REVIEW-001` and
`OPDEV-LIFECYCLE-001` add explicit duties. Other policy-2 duties remain. Existing
source, stage, acceptance and execution checks still apply. A mechanically valid
mapping does not establish semantic adequacy or authenticated consent.

Keep material supported guarantees in the requirements catalog, not every function
or implementation detail. Use existing authority references and current change
review instead of per-control duplicates. Routine execution reports expire under
the project retention policy. No new ledger, database, evidence repository or
automatic external-standard update is introduced.

## Research and limits

[NIST SSDF 1.1](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-218.pdf)
supports risk-based implementation and lifecycle security responsibilities.
[NIST Privacy Framework 1.0](https://www.nist.gov/system/files/documents/2020/01/16/NIST%20Privacy%20Framework_V1.0.pdf)
motivates treating data processing more broadly than persistence. These are
precedents, not a claim of complete conformance.

[Google's review guidance](https://google.github.io/eng-practices/review/reviewer/looking-for.html)
supports reviewing design, complexity, tests and documentation. It does not establish
that agent self-review equals independent human review; the solo-agent default is
OpDev's explicit usability tradeoff, with disclosed limits.

[Inozemtseva and Holmes (ICSE 2014)](https://cs.uwaterloo.ca/~rtholmes/papers/icse_2014_inozemtseva.pdf)
found limited association between coverage and effectiveness after controlling for
suite size in their studied Java projects. [Google's mutation-testing experience](https://research.google/pubs/practical-mutation-testing-at-scale-a-view-from-google/)
supports targeted defect-discriminating checks, not a universal mutation mandate.
[The ML Test Score](https://research.google/pubs/the-ml-test-score-a-rubric-for-ml-production-readiness-and-technical-debt-reduction/)
motivates explicit data/model evaluation boundaries; it does not prescribe OpDev's
statistical thresholds.

[DORA small batches](https://dora.dev/capabilities/working-in-small-batches/),
[Hilton et al. (FSE 2017)](https://www.cs.cmu.edu/~mhilton/docs/HiltonFSE17.pdf),
and [SPACE](https://www.microsoft.com/en-us/research/publication/the-space-of-developer-productivity-theres-more-to-it-than-you-think/)
motivate short feedback loops and multidimensional evaluation, not guaranteed
speed or token savings. [SRE objectives](https://sre.google/workbook/implementing-slos/)
and [canarying](https://sre.google/workbook/canarying-releases/) motivate scoped
operational safeguards, not universal uptime targets or rollout machinery.
