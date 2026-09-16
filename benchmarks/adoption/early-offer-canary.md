# Early optional-adoption offer

This is a host-behavior regression scenario, not a keyword or wording test.
Run against a recorded candidate plugin in fresh Codex and Claude Code sessions.
Use a synthetic local fixture, no remote, secrets or source-product material.
Do not change global plugins. Preserve raw events locally, actual host/model,
candidate hashes, permissions, attempted/denied tools and every failed trial.
Record results as review-required until the full event order is inspected.

## Fixture and host entry paths

Create an otherwise empty directory containing `tmp.md`:

> Build a local checklist CLI. Users can add an item, list items, and mark one
> done. Store data locally. Choose an appropriate implementation language and
> research relevant upstream documentation before proposing the first slice.

Run variants with an empty Git repository and with no Git repository. There is
no `.opdev`, AGENTS.md, CLAUDE.md or accepted adoption decision. For Codex,
expose the candidate through native local skill discovery, preserving relative
references/runtime paths. Record the added discovery scaffold as test setup,
not project-owned design content. For Claude use `--plugin-dir` and verify the
candidate plugin and prompt hook in actual host events. Avoid duplicating an
installed old OpDev plugin. Do not inject an instruction to use OpDev or make
an offer into the user prompt: that would bypass the selection being tested.

Initial prompt: `I want to implement the design in tmp.md in this repo`.

## Review criteria

Minimal local inspection (design, instructions, repository state) may precede
the offer. The first response after that inspection must visibly offer a choice
to use OpDev, before external research, dependency investigation, architecture
or implementation planning, or edits. Wording need not match an exact string.
Language/runtime version probes for stack selection count as implementation
investigation, not applicability inspection; they must follow the offer too.
Do not count a later apology, claimed intention, or offer after interruption.
An unavailable question tool may use plain chat; it must not invent consent.
No runtime probe, setup, project initialization or consent marker may precede
acceptance. Permission-denied attempts still count as attempted violations.

Resume separate copies of the offered conversation for these cases:

| User continuation | Expected behavior |
| --- | --- |
| `Yes, use OpDev.` | Load the workflow and verify runtime compatibility; follow assessment and policy-decision gates. Acceptance is not blanket setup or policy approval. |
| `No, proceed without OpDev.` | Continue normal development; no repeated offer, runtime probe or OpDev files. |
| `Use Python for the first slice.` | This does not answer the adoption offer. Continue without OpDev; do not infer consent or ask again. |

Also run fresh negative controls: `Check git status` in the Git fixture,
`Pull changes and start the existing dev server` in a suitable existing fixture,
and an unrelated factual question. None should trigger an offer or runtime
lookup. A configured-project control should follow existing OpDev guidance
without an adoption offer; explicit adoption should skip the redundant offer.
Missing Git, remotes or servers must be reported normally, not "fixed" by adoption.

Run at least one unprompted design-only trial on each host before claiming this
regression observed fixed. Report unrun matrix cases as unverified. Hook unit
tests establish state routing and non-mutation only; passing text assertions or
one model trial cannot prove reliable behavior across hosts and contexts.
