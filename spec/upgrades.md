# Coordinated upgrades

An upgrade has separate plugin, runtime, project-guidance, project-contract,
adoption/profile, CI and verification states. Updating one MUST NOT imply the
others passed. This applies to both Codex and Claude Code and all software stacks.

## Consumer flow

The shared agent's [upgrade procedure](../plugins/opdev/skills/opdev/references/upgrades.md)
coordinates inspection, explanation, preview, approved application and verification.
Host plugin updates use the host's supported installation path. Runtime updates
use the existing signature-verifying setup or exact-version standalone installer;
there is no separate self-updater or unattended latest-version lookup.

`opdev upgrade` is now read-only by default. `--dry-run` makes that explicit;
`--format json` emits schema-1 structured results. `--plugin-root PATH` inspects
an installed or candidate package's compatibility contract and runtime pin without
executing its scripts. The target is the running CLI's embedded guidance, not an
assertion that its SemVer is the latest release or that a supplied package is
actually installed. The CLI reports its exact executable path.

The preview includes exact before/after guidance text, supported project schema,
adoption gaps, selected assurance profiles, plugin compatibility and locally
declared CI `OPDEV_VERSION` values. It checks root GitLab configuration and all
root GitHub workflow YAML files. Explicit local GitLab includes are resolved;
dynamic/external includes, provider variables, custom script version selection and
remote configuration remain unresolved. CI pins are observations,
never a compatibility/qualification pass, and are never rewritten automatically.
Changing an old download version alone may also require changing its release host
and verification metadata. Custom providers remain explicitly unverified.

After review, `opdev upgrade --apply PLAN_ID` (with the same `--plugin-root`, if
used) applies only managed guidance. The opaque SHA-256 token binds the resolved
root, executable path/version, target guidance, findings and exact inspected input
digests, including project/adoption/evidence files and discovered CI files. It is
an accidental-staleness guard, not a signature or proof of human consent. The CLI
recomputes it before writes, does not deserialize instructions from a plan file,
and rejects a changed target or reviewed input. Review remains a human/agent duty.

Bare `upgrade` previously wrote guidance. This intentional safety change requires
scripts to preview/review and pass `--apply`; it is called out in release notes.
Agents MUST capability-check `upgrade --help` before relying on preview semantics
on an older runtime. Do not probe an old CLI by invoking bare `upgrade`.

## Preservation and failure behavior

UV-01: Doctor/upgrade distinguish selected package path/version/skill digest,
running CLI identity, pin compatibility versus exact version alignment, project
guidance relative to that CLI, and CI declarations. A supplied package is not
proof of installation. A compatible older executable is allowed; do not force a
runtime update merely because versions differ. Same-version source builds can
have different guidance, so use content comparison, not version equality.
UV-02: Session-loaded guidance remains unverified without host evidence. Disk
inspection cannot establish it; after a package change use the host's reload or
fresh context when needed. No automatic restart, network/latest lookup, cache
edit or update occurs. Guidance preview/apply remains a separately reviewed action.

- Preserve project content outside managed markers and existing Claude imports.
  Malformed/duplicate markers, linked targets and stale inputs fail closed.
- Prevalidate both instruction files and stage replacements before committing.
  Each file replacement is atomic; the pair is not a filesystem transaction.
  On a mid-commit I/O failure, report partial application, inspect both files,
  and obtain a fresh preview. Do not restore a whole file over later user edits.
  Do not run concurrent writers during apply; this is not a hostile-writer lock.
- Leave project YAML, documentation locations, commands, evidence, CI, profile
  pins, experiment opt-ins and adoption decisions byte-for-byte unchanged.
  New requirements need explicit assessment, never a fabricated satisfying entry.
- Reject unsupported project schemas without rewriting. Ordinary guidance upgrade
  performs no project/adoption migration; the explicit coordinated interface below
  is separate. Adoption gaps are `migration_required`;
  malformed/unsupported adoption state blocks apply as an assessment error.
  Legacy projects remain unassessed; upgrading is not consent to adoption.
- No runtime installation, downloads, project checks, tracking mutations or
  global filesystem changes occur in the CLI upgrade operation. Git discovery
  performs fixed read-only metadata queries. Do not print unrelated file contents.
- Preview/apply exit 0 means that operation succeeded, not full upgrade completion.
  Reported compatibility/assessment blockers exit 1; command/input/I/O errors
  exit 2. `project_verification` remains `unverified` until separately established.

## Verification and rationale

Run compatibility verification with the actually selected runtime. Review the
project's commands, run the relevant canonical checks and `opdev check --ci`, and
run `opdev adoption check` when an adoption record exists. Preserve failed,
unverified, migration-required and error outcomes. Evidence must be reviewed and
rebound to the final staged state; upgrading never refreshes assertions. Integrate
project changes through CI and verify trunk before claiming completion.

The accepted design uses a small deterministic CLI and host-neutral orchestration
instead of an all-in-one self-updater: host cache lifecycles, installation trust,
project decisions and provider-specific CI are different authorities. Automatic
CI regeneration would destroy customizations; generic schema migration would
invent intent. No extra persistent project file is needed for this workflow.
Revisit automated migrations only when a versioned deterministic transformation
and preservation/recovery tests exist. The bounded interface below supports the
reviewed current transition; it is not a generic schema repair engine. Revisit host automation when both hosts
offer a supported, inspectable interface with equivalent consent guarantees.

Regression coverage: `crates/opdev-cli/tests/upgrades.rs`, project bootstrap unit
tests, existing POSIX/PowerShell verified installer suites. Agent procedure review
scenarios live in `tests/upgrade-review.md`; they are not a measured host-session
or live newest-release installation qualification.

## Explicit coordinated migration

Development capability `upgrade.coordinated-migration.v1` adds `upgrade
--migration REQUEST.json [--plugin-root PATH]`. It previews a complete explicit
schema-3 project with engineering, layout and external review storage already
selected by the developer. `schema/migration-request.schema.json` describes the
strict ephemeral input: schema 1, actual `decision_reference`, full `project`,
`ci_review_reference`, optional exact local `ci` replacements and optional original
history `ArchiveLocator`. References document scope; strings do not authenticate
consent. This is not a new file to maintain in product `.opdev` or a default on
ordinary upgrades. No release version or installed host change is selected.

Read-only preview inventories all bounded `.opdev` contents (including ignored
files), root instructions, parsed adoption, local provider CI/includes and optional
plugin metadata. Unknown directories, unsafe links, conflicting ownership and
unsupported schemas remain blockers. Existing outside authorities stay unchanged
unless `authority_review_reference` records the actual content-ownership migration
decision. A string is not proof of consent or content adequacy. Classify and split
mixed content through its owner; reviewed text moves below do not infer consolidation.
No filename-based cleanup or adoption restart is performed. Namespace limits are 4096 entries/32 levels,
8 MiB per input and 32 MiB total namespace bytes. Exceeding a limit fails rather
than accepting a partial inventory or trimming old evidence.

The candidate diff includes deterministic shared guidance and thin root pointers,
preserving unrelated text/imports. Existing adoption practice dispositions, scope,
workflow and references survive; new catalog practices start pending. A policy
change does not transfer old approval to different choices: the original record
remains in the recovery snapshot and current review becomes unresolved. An absent
assessment blocks this transition until explicit adoption assessment exists.
The command cannot complete adoption, assert applicability or invent verification.

CI replacements are exact supplied text for already inspected GitLab/GitHub YAML
targets, not generated jobs or guessed version substitutions. Ordinary guidance
upgrade still never edits CI. Included paths outside the supported migration
targets need a separate reviewed edit. Plugin compatibility, local runtime identity,
declared CI pins, unknown dynamic configuration and actual qualification are separate
observations; matching versions or a caller's review reference never proves CI.

For storage policy 2, the obsolete active ledger is removed after reviewed
migration with temporary interruption/rollback protection. No external archive,
historical-copy verification or archival commit is required. Existing Git history
is not rewritten. Retire the temporary recovery file after migration verification
and when rollback is no longer needed; it is not a permanent evidence authority.
Late file edits still stop removal and original adoption approval is not transferred.

For legacy storage policy 1, an existing ledger is retired only with an independently selected exact archive
locator in the chosen storage repository. The provider must return every original
byte, with a supported original ledger schema. No old assertion is rewritten as a
current success. Retention/access/protection ownership remains an explicit reviewed
obligation: observed retrieval alone does not prove future availability. A missing,
expired, changed or unsupported archive stops the migration. Preserve oversized
history and explicitly plan its supported retention; never truncate it to fit.

`upgrade --migration REQUEST.json --apply PLAN_ID --recovery-output NEW_FILE`
applies only the reviewed exact proposal. Before any project write it creates and
independently rereads a new bounded recovery snapshot outside source/Git storage.
It includes original/target bytes and inspected identities, not just file paths.
The snapshot may contain private configuration/evidence: retain it at an appropriate
private location and do not paste it into public work items. No provider writes,
installation, execution qualification, history re-publication or release occurs.

Writes are atomic per file, not a multi-file transaction. Project, adoption,
managed routing and supplied CI replacements precede retiring the old active ledger.
Each step rechecks the inventory and all original/target states; linked paths or
later edits stop the operation. Exclude concurrent writers during application;
these checks do not provide hostile-writer filesystem locking. On interruption,
inspect the actual diff and original snapshot; do not run `init`, blindly roll back,
or regenerate approvals to conceal partial state.

`upgrade --resume RECOVERY.json --apply ORIGINAL_PLAN_ID` continues only when every
target still equals its original or approved final bytes and untouched inputs
remain unchanged. Changed snapshots, runtimes, roots and unknown targets fail.
For storage policy 1 it rechecks historical availability before retiring its path. Applied files are
no-ops, so repeating continuation is safe; a fresh preview of the applied request
has no changes. Later developer edits are preserved and require a new reviewed
resolution, never automatic rollback. Keep recovery until required qualification
and, for storage policy 1 only, independent durable retention are confirmed; the CLI performs no snapshot
cleanup. A successful apply remains `qualification: unverified`.

### Reviewed retirement toward clean adoption

Capability `upgrade.reviewed-retirement.v1` adds optional `clean_target`, `cleanup`
and `authority_review_reference` to the same ephemeral request. `clean_target`
selects the [versioned destination and retirement decisions](adoption.md#current-destination)
in the existing adoption record; it requires the complete policy bundle and clears
old approval when changed. Leaving it absent preserves existing target decisions.
This does not reassess every unchanged practice or claim completion.

Each explicit cleanup action has `path`, `kind`, nonblank `reason`, and optional
`destination`. Its source must be covered by the reviewed target retirement list:

- `move` copies a bounded regular UTF-8 file's exact content to a new destination
  before retiring the original. An existing destination must match exactly; no
  overwrite/merge is guessed. Consolidate different content in an ordinary reviewed
  change first, then use `retire_file` with its retained replacement declared.
- `retire_file` removes only that exact reviewed obsolete file after the mandatory
  external recovery snapshot retains its original bytes. This snapshot is recovery,
  not a substitute for externally retained history where that is required.
- `empty_directory` removes only a named inventoried OpDev/CI directory after its
  explicitly selected children are gone. Removal is nonrecursive; unplanned or
  later content stops application. Other directories require a separate reviewed edit.

Portable contained paths, no links/reparse points, protected current agent/policy
files, duplicate/cyclic destinations and Git metadata are checked before writes.
Legacy `.opdev/evidence.yaml` cannot use this generic cleanup: use the dedicated
ledger-removal path with the selected storage policy's safeguards. Only storage
policy 1 requires authenticated history retrieval. All unselected content/ownership findings
remain blockers. The same snapshot and before/after checks support interruption
and idempotent continuation; later edits are preserved, not rolled back. Snapshot
copies contain text bytes, not a complete filesystem metadata/ACL backup; projects
needing executable-mode, ACL, binary or encoding migration use a reviewed separate
mechanism with appropriate recovery rather than treating this as a general mover.
Stage the settled result: an obsolete path remaining in Git's index still blocks
clean adoption, even after the working file is gone. Reconcile references, semantics,
capabilities, required checks and integration after application. No automatic release.
