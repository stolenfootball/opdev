# Documentation locations and ownership

The project contract, rather than a folder name, selects authoritative material.
OpDev configuration and evidence live in `.opdev/`. For projects without an
established structure, the recommended locations are `docs/` for human guidance,
`spec/` for design and behavioral contracts, and `release/` for release
engineering inputs, procedures, recovery, and changelog.

These defaults MUST NOT override existing authority locations. Existing
project-owned files, directories, and symlinks MUST NOT be overwritten, moved,
or repurposed automatically to match a default. An explicitly configured
location inside `.opdev/`, an external URL, or another folder remains valid.
Agents MUST inspect ownership and existing content before assigning a purpose.
If an established authority is suitable, reuse it. Otherwise choose an unused
location appropriate to the repository and record it in `authorities` and
`context`. Resolve material ownership ambiguities with the project owner.

Discovery is a read-only proposal. A valid existing contract is returned intact;
an invalid contract fails instead of silently falling back to guesses. Without
a contract, candidate paths must have the expected type. Multiple candidates
for a role produce a warning and leave that authority unselected. A unique
candidate remains an inference requiring review. Absent folders are not created
or asserted to be authorities. Unknown/custom layouts require explicit routing.

New initialization writes `.opdev/project.yaml`, pending `.opdev/adoption.yaml`
decisions, and managed sections of root `AGENTS.md` and `CLAUDE.md`. Legacy projects
start assessment explicitly; see [adoption](adoption.md). Dry-run writes nothing, including for initialized
projects. Neither initialization nor upgrade migrates documentation.
`DELIVERY.md` is not a required filename. Repository-wide agent entry points
remain at the root; detailed project facts belong in their declared authorities.

## Decision and this repository

Keeping human guidance separate makes it discoverable independently of OpDev.
Keeping release instructions beside packaging inputs helps reviewers reconcile
changes. A mandatory scaffold would collide with existing docs sites, language
conventions, generated release directories, and externally maintained manuals.
Putting all documents in `.opdev/` would obscure general project guidance.
Advisory defaults plus explicit routing preserve existing ownership. Revisit the
heuristics if candidate warnings create excessive ambiguity; do not replace
review with silent directory ownership assumptions.

This repository keeps README, AGENTS, and CLAUDE as its root Markdown files.
Contributor and security guidance live in `docs/`; the changelog joins existing
release material in `release/`. Existing `spec/` and `release/` roles are retained.
Current references move with files; historical fingerprint-bound evidence remains
historical. No released archive is rewritten.

The README links directly to contribution and security guidance so access does
not depend on provider-specific automatic discovery. GitHub supports community
files in `docs/`; GitLab-specific automatic discovery is not assumed. Future
moves must review provider settings and links as well as repository references.
