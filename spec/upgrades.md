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
root GitHub workflow YAML files. It does not resolve includes, provider variables,
custom script version selection or remote configuration. CI pins are observations,
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
- Reject unsupported project schemas without rewriting. No automatic project or
  adoption-schema migration exists yet. Adoption gaps are `migration_required`;
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
and preservation/rollback tests exist. Revisit host automation when both hosts
offer a supported, inspectable interface with equivalent consent guarantees.

Regression coverage: `crates/opdev-cli/tests/upgrades.rs`, project bootstrap unit
tests, existing POSIX/PowerShell verified installer suites. Agent procedure review
scenarios live in `tests/upgrade-review.md`; they are not a measured host-session
or live newest-release installation qualification.
