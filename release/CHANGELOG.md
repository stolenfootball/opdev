# Changelog

All notable changes will be recorded here. OpDev follows Semantic Versioning once
the public compatibility boundary stabilizes.

## 0.2.1 / Plugin 0.2.2

- Fix adoption review: ask relevant material questions before implementation
  approval, verify inherited consent, respect bounded delegation, and preserve
  the choice to keep a non-main trunk name. Use supported host question tools
  with chat fallback; unanswered questions never become approval.
- Separate adoption approval from implementation with plan-bound review,
  explicit schema-1-to-2 adoption migration and unresolved evidence preparation.
  Existing records are preserved, not automatically migrated. CLI 0.2.0 cannot
  read schema-2 adoption records; use CLI 0.2.1 before choosing that migration.
  Project-manifest and evidence schemas are unchanged.
- Correct applicability, integration-versus-delivery qualification and action
  reporting. Diagnostics do not substitute for release recovery; missing evidence
  is not proof of implementation or inapplicability.
- Harden generated CI installation with signature verification, explicit libc
  compatibility checks and isolated caches. Preserve project images and choices.
- Keep compact/token-reduction views behind explicit experimental opt-in.
- The bundled plugin retains its qualified CLI 0.2.0 pin during publication;
  new adoption capabilities require standalone CLI 0.2.1 until the subsequent
  managed-runtime pin update is qualified. No historical assets are replaced.

## Plugin 0.2.1

- Pin the qualified CLI 0.2.0 and route managed downloads to GitHub while
  preserving historical GitLab routing and exact GitLab signature verification.
  Require CLI >=0.2.0, <0.3.0; preserve older cached runtimes for active sessions.
- Version agent plugins independently from the CLI. This source-marketplace
  update does not replace the immutable v0.2.0 release archives.

## 0.2.0 - 2026-09-16

This minor release changes bare `upgrade` to preview-only and requires an
experimental flag for compact views. Project and evidence schemas are unchanged.
The managed plugin pin remains on published CLI 0.1.1 until the new runtime is
independently published and qualified; new CLI commands require standalone 0.2.0.

- Keep token-reduction views experimental and off by default. Compact check
  summaries, saved-report summaries and current-evidence queries require
  `--experimental-compact`; agents require explicit opt-in. Full human/JSON
  checks and full report persistence remain stable. Older compact scripts must
  add the flag. General quality-equivalent token savings remain unverified.

- Make `opdev upgrade` preview-only by default; apply managed guidance with a
  reviewed `--apply PLAN_ID`. Inspect plugin/runtime compatibility, adoption and
  CI pins without changing project choices. Existing automation that relied on
  bare `upgrade` writing files must migrate to preview/review/apply. Add a shared
  Codex/Claude upgrade procedure and reject stale or unsafe guidance writes.

- Apply outcome-based planning to plans, roadmaps, task breakdowns and next-step
  recommendations, preserving explicit user requests and bounded enabling work.

- Gate agent activation on target-project state and consent before runtime setup.
  Uninitialized pull/status/dev-server tasks proceed without an OpDev suggestion;
  prompt hooks no longer probe runtimes before the workflow is needed.

- Add explicit adoption assessment and completion checks with a versioned practice
  inventory, project-specific research, preserved decisions, and reviewed opt-outs.
  New initialization starts pending; existing projects opt into assessment explicitly.

- Adopt a mechanism-neutral experiment lifecycle with shared agent guidance,
  a standard record, and read-only `opdev experiment validate` checks. Preserve
  existing project schemas, evidence gates, and stable release defaults.

- Organize human guidance and release history into folders; preserve configured
  authorities and surface discovery conflicts without modifying project-owned
  documentation. Keep initialization dry-run read-only on existing projects.

## 0.1.2 - 2026-09-10

- Add cargo-dist packaging and signature-verifying one-line installers, with immutable GitHub releases published exclusively by GitLab CI. Preserve historical GitLab downloads and the plugin runtime pin.

- Add plugin-managed CLI setup with a pinned signature verifier, exact release
  identity verification, versioned storage, and offline failure tests.
- Add Linux and Windows consumer-install CI checks and a reproducible cargo-dist
  0.32.0 probe documenting the GitLab-only hosting blocker.

- Rename the canonical GitLab repository to `stolenfootball-tools/opdev` and
  update installation links, generated CI downloads, and future signing identity.
  Rename the GitHub build mirror to `stolenfootball/opdev` while preserving
  historical release identities.

## 0.1.1 - 2026-08-31

- Correct GitHub and GitLab generated installers, GitLab runtime/OAuth behavior,
  Windows npm command shims, evidence bootstrap sizing, Go image selection, and
  CI report isolation discovered by the initial private canaries.
- Add a packaged plugin-to-CLI compatibility contract with fail-closed first-use
  verification in the shared skill and Claude Code prompt hook.
- Make Codex contract validation and strict Claude Code plugin validation
  required CI checks and smoke-test compatibility from the packaged artifact.
- Keep the GitHub native-build mirror synchronized to the qualified revision and
  remove ephemeral `gitlab-release/*` branches after artifact handoff.

## 0.1.0 - 2026-08-30

- Define the 37-rule OpDev and MinimumCD catalog with strict result semantics.
- Add cross-platform Rust project discovery, initialization, checks, and reports.
- Add GitHub Actions and GitLab CI generation, inspection, and read-only audits.
- Add safe project checks, versioned assurance profiles, and bound evidence.
- Add shared Codex and Claude Code plugin behavior with persistent project guidance.
- Add deterministic release checksums, CycloneDX association, and SLSA-compatible
  provenance without a SLSA Build level claim.
- Add deterministic `.tar.gz` and `.zip` packaging with normalized metadata,
  safe path handling, and reproducibility checks.
- Add native release qualification for Windows, Linux GNU, and macOS on x86-64
  and ARM64, plus independently packaged Codex and Claude Code plugin files,
  using an immutable GitHub-builder-to-GitLab-publisher handoff.
- Add keyless Sigstore signatures for every distributed archive and a tested
  safe roll-forward recovery procedure.
