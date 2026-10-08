# Upgrade OpDev without silently changing a project

Use for requests to upgrade/update OpDev, assess an available upgrade, or migrate
an initialized project. An inspection request permits only inspection. An upgrade
request permits planning the coordinated change, not blindly adopting new rules,
enabling experiments, overwriting project choices or publishing a release.
OpDev upgrades do not imply repository adoption in an uninitialized project.

## Inspect and explain

Identify the intended target release and the host(s) the user wants updated. Read
the actual installed package version, compatibility contract and `runtime.lock`.
Resolve the runtime using the main skill's read-only lookup and report its exact
path/version; distinguish the packaged pin from the selected executable. A
compatible older pin is intentional, not automatically a defect. Never modify
an installed package's lock to force a newer executable. Corruption is an error,
not permission to delete storage or silently fall back.

For newest-release requests, inspect the official release/marketplace metadata
and release notes; do not infer latest from the current binary or a source branch.
Without network access report the known target and unknown latest state. Use the
host's current documented plugin-update interface; do not guess Codex or Claude
commands, edit caches manually or update the other host without scope/consent.

On the selected CLI, inspect `upgrade --help`. The reviewed workflow requires
`--dry-run`, `--apply` and `--plugin-root`. Older bare `upgrade` writes guidance:
do not use it as a probe or pretend it supports preview. If capabilities are
missing, explain the runtime gap and offer a qualified compatible update. The
normal runtime preference still applies; do not install a newer standalone CLI
and then silently select an older managed pin for the remaining work.

For an initialized project with a capable CLI, run:

```text
<selected-cli> upgrade --root <project> --plugin-root <actual-package> --dry-run
```

This is offline and does not run project commands. Explain the inspected package
identity and unknown session-loaded instructions. The
guidance target is the running executable, not inferred from the package version.
Compatible selection and exact pin alignment are different observations.

Explain the guidance diff, plugin/runtime differences, unsupported migrations, preserved profile/adoption
choices, CI pins, opt-ins left unchanged and outstanding verification. Show the
exact target and material changes before asking approval. No new project document
or questionnaire is required; use existing work authorities when appropriate.

## Apply the approved scope

Use the host's supported plugin update/install path for approved host changes.
Use the packaged setup skill for an approved pinned runtime install; standalone
updates use the release's existing signature-verifying exact-version installer.
Retain older managed runtimes for active sessions. Never bypass checksum,
signature, exact-version or compatibility checks. A restart/new session may be
needed for changed skill instructions; do not claim they are active until verified.
An incompatible installation may be inspected for repair but must not run the
normal development workflow.

Re-resolve runtime selection and verify plugin compatibility after updating.
Regenerate the preview with that exact CLI/package: an earlier preview does not
authorize different target guidance. After reviewing the current diff, apply:

```text
<selected-cli> upgrade --root <project> --plugin-root <actual-package> --apply <plan-id>
```

The token protects against stale inputs, not against unreviewed consent. If the
plan changes, explain the delta and obtain approval for any changed scope. Do not
loop regenerating/applying until an error disappears. On a partial write, inspect
all managed instruction files and preserve any later user changes before re-previewing.

CLI apply changes managed guidance only: legacy AGENTS/CLAUDE sections, or the
selected layout's shared `.opdev/guidance.md` and short root pointers. It preserves
unrelated content/imports and does not select the new layout. Review CI pins and related
download host, artifact/signature identity and required runtime capabilities as a
separate proposed diff. Preserve custom jobs, images, provider variables/includes
and all unrelated configuration. Matching version text alone is not qualification.
Use each provider's validation and project pipeline for the approved CI change.
Unsupported schemas or new adoption requirements need an explicit migration plan;
do not replace the contract or auto-mark a practice implemented/ignored. Existing
legacy projects need separate consent before starting assessment.

## Verify and hand off

### Explicit baseline/layout/storage migration

Distinguish a tooling-only update from a full current project migration. The latter
targets [clean adoption](adoption.md#validate-recommendations-before-asking-for-approval),
including mandatory retirement, not just newer version text or thin pointers.
Reuse adequate decisions and implementations; reopen only changed/unverified gaps.

For a requested project migration, first check `upgrade.coordinated-migration.v1`.
Do not interpret a missing capability as permission to install, delete the ledger,
or manually imitate a successful migration. Use the existing work authority for
the decision and preserve all unresolved choices. Build the ephemeral strict
migration request from actual reviewed project policy, not guessed defaults or
an agent-created owner label. `upgrade --migration FILE` previews exact changes;
unknown internal content needs purpose/owner resolution, not cleanup to satisfy
an allowlist. Adequate outside authorities stay intact.

Retain the complete original ledger at the approved archive and verify its exact
retrieval and independent recovery first. Supply its original locator; do not
replace history with a projection of the latest green attempt. Review plugin/local
runtime/CI capability separately, including custom CI/download/signature behavior.
Only actual approved CI edits belong in the request; matching pins do not prove
qualification. Preview does no provider writes or installations.

After scoped developer approval, apply the exact plan with a new external recovery
snapshot. Catalog additions stay pending and previous approval is not reissued for
new policy. Inspect partial writes; use `upgrade --resume RECOVERY --apply
ORIGINAL_PLAN_ID` only with unchanged original/target states. Never overwrite later
work or hide interruption by creating a new approval. Preserve required history,
original decision scope and revocations. Stage the settled result, review current
acceptance, run required checks and verify integration; migration apply alone is
not completed adoption or release permission. No new project migration registry.

With `upgrade.reviewed-retirement.v1`, the same request may select `clean_target`
and exact `cleanup` actions (`move`, `retire_file`, `empty_directory`) covered by its
reviewed retirement list. Classify actual content first. Move preserves exact UTF-8
text and refuses different existing destinations; consolidate mixed content through
its owner before retirement. Empty-directory removal is nonrecursive and limited
to inventoried OpDev/CI namespaces. Current managed files and the legacy ledger
cannot be deleted through generic cleanup; use the ledger's retained-history path.
Actual authority changes need `authority_review_reference` and reviewed content,
not a relabel to suppress a finding. The recovery snapshot is private text recovery,
not a complete ACL/executable-mode/binary backup or durable history authority.
Use a separate scoped mechanism for unsupported transformations. Review removals
and retained content, stage all changes, and finish source-bound cleanup evidence;
keeping obsolete artifacts indefinitely is not completed adoption.

### Outcome reporting

Report plugin installation, actual runtime selection/compatibility, guidance,
project migrations/adoption, CI pins/qualification and project gates separately.
After reviewing command trust, run the declared canonical checks and
`opdev check --ci`; run `opdev adoption check` only when a record exists. Review
fresh evidence against the final staged state, integrate through CI and verify
trunk. Do not silently reuse previous assertions or tests for changed inputs.

If only installation succeeded say "runtime updated; project upgrade incomplete"
and list remaining work. `upgrade` exit 0 and a supported schema do not establish
project checks or complete adoption. No automatic background upgrade checks,
notifications, experiment activation or periodic setup during normal work.
