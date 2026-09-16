# Adoption guidance correction — 2026-09-16

This supplements, rather than replaces, the retained failed
[qualification](qualification-2026-09-16.md).

## Change and acceptance

Moved the invariant checks ahead of recommendations: existing violations cannot
be approved as completed adoption; keeping a branch name is different from
keeping conflicting roles; CI approval is different from manual distribution;
diagnostics are different from recovery; small/local software still has real
operational and user outcomes. Clarified independent manifest/adoption schemas,
read-only planning, and the distinction between correctness and effectiveness.
No ecosystem defaults, product-image changes, or core waivers were introduced.

Two candidate iterations were evaluated. The first corrected the core workflow
failures, but Claude's follow-up conflated function tests with effectiveness and
called `adoption plan` a write. Those observations motivated the second narrow
correction; neither earlier attempt was discarded. Candidate directories and raw
events are private. The final guidance SHA-256 is:
`c256028b5092d456c4f4fe73199b0486c970894025f2f1547e687851501b8a4b`.
CLI and fixture hashes are unchanged from the original report.

## Final-candidate observations

- Both hosts presented keeping the existing trunk name or moving both roles to
  main; neither accepted split integration/release roles as completed adoption.
- Both required CI-controlled publication and automated, tested recovery rather
  than treating an error exit as recovery or manual distribution as delivery.
- Both kept accessibility/effectiveness applicable and distinguished function
  tests from representative user-task evidence. Claude explicitly identified the
  absent CLI/error assertions and that new initialization needs no migration.
- Both ambiguous follow-ups asked for actual choices rather than recording consent.
- Codex completed bounded research and updated only the proposal, leaving all
  other choices unapproved. No tools were installed or policy implemented.
- Codex created permitted pending scaffolding and a proposal in the fixture's
  declared handbook authority. All 18 dispositions remained pending, with no
  review/approval record. No CI, product source, tests, or branch roles changed.
- Claude wrote no project files. Its bounded-delegation turn left other choices
  unapproved and installed nothing. However, its web tools were denied, so live
  research completion is **unverified**, not passed.

Claude also made an inaccurate statement that implementation commands had already
been denied; the trace shows they were not attempted. Its follow-up correctly
described planning as read-only. This reporting defect remains visible: these
canaries establish improvement in the targeted recommendations and consent
boundaries, not flawless agent behavior or complete release qualification.

The final sessions use the same host versions as the original report; Claude's
resolved model is `claude-opus-5`, and Codex uses its default without a model
override. Permission-related runtime retries remain in the traces. Host permissions
allow candidate reads and exact assessment commands, not arbitrary shell execution
or approval of project policies. No installed plugins changed.

Automated evidence: the 15 CLI adoption tests pass, including explicit assertions
that initialization creates adoption schema 2 alongside project schema 1 and that
renaming to main is valid only with reconciled roles. Seven controller tests,
workspace tests, format, strict clippy and skill validation pass. The canonical
Windows suite still skips its documented platform/live-dependent tests; these
claims do not imply a new remote CI or full delivery qualification run.

Initial and ambiguous event hashes:

| Host / turn | SHA-256 |
| --- | --- |
| Claude assessment | `81ba9bca072daf1654ae3fc371e9bbc648d4ad6f9d9d68be82452cb312434b83` |
| Claude ambiguous | `2d27e13abbc446ffd6dc7fce0d5e67b50006f87edfe87dd9ec15c29f9940ca9d` |
| Claude bounded | `035712e850e3ad41af478ee4e614951b17620faae7ae06b73eb4f612918811b3` |
| Codex assessment | `a09ae3abcbe4179ed29e3cc0110b48489d4e5403f220ace4afe505c2a0148d28` |
| Codex ambiguous | `78176fa45f2ea753947361b119a185ed99ea7f8628efc393891d8479f0b115a7` |
| Codex bounded | `84c28f4357c1dad99f3d23f049bf4ec0f7eaef29bfa5fb66d8fe0e084572d264` |

Full adoption implementation, real CI delivery/recovery and release qualification
remain **unverified**. No completion assertion or release was created from these
conversation results.
