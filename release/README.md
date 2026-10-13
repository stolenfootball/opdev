# Release operations

The protected GitLab tag pipeline is the only supported way to publish OpDev.
GitLab remains the source and signing authority. Beginning with 0.1.2, the
canonical binary host is [the existing GitHub mirror](https://github.com/stolenfootball/opdev/releases).
Historical GitLab assets remain intact. GitHub Actions artifacts are internal
build handoffs; no independent GitHub tag-triggered publisher is enabled.

Native builders build/test each binary once. The GitLab pipeline runs pinned
cargo-dist over those bytes, preserving legacy archives and adding dist archives,
verified installers, and manifests. It signs the archives and installers with
GitLab OIDC, then publishes the same qualified assets through a GitHub draft.

The publisher checks an exact source tag, a complete inventory, every uploaded
asset digest, and the repository's immutable-release setting. It resumes a
matching incomplete draft, refuses to overwrite mismatched assets/tags, and
accepts a retry of an already-published release only when every digest matches
and the release is immutable. GitLab release notes link to the canonical GitHub
download destination after publication succeeds.

Prerequisites: protect release tags in GitLab; keep `GH_OPDEV_RELEASE_TOKEN`
protected/masked and scoped to the mirror with Contents write permission;
enable immutable releases in the mirror settings. The existing credential also
needs the mirroring/workflow-dispatch permissions used by native builds. The
publisher fails before uploading if immutability is disabled. It creates the
exact GitHub tag only after the source commit is available in the build mirror.
No manual asset uploads, replacement releases, or floating commit selection.

## Candidate and final release

Normal Linux amd64 jobs use the group runners tagged `linux`, `docker`,
`proxmox`; Windows qualification uses `windows`, `powershell`, `hyperv`.
Both Proxmox runners are amd64. The Linux installer asserts `uname -m`.
The required ARM64 check runs on GitHub's standard `ubuntu-24.04-arm` runner in
the existing public mirror, triggered by an exact `opdev-arm64/<SHA>` branch.
The GitLab `installer-arm64` job reads the public API, matches the source SHA,
repository, ref, workflow, successful attempt and native runner label, and waits
at most 20 minutes. Missing/failed/skipped results and API errors block the gate.
It never substitutes amd64 or waives ARM64 because hosted compute is unavailable.

For merge requests, an authorized maintainer pushes that single revision to
`https://github.com/stolenfootball/opdev.git` as `refs/heads/opdev-arm64/<SHA>`
using their existing GitHub login. Ordinary MR jobs receive no write credential.
Protected trunk/tag jobs can perform the same single-ref push using the existing
protected `GH_OPDEV_RELEASE_TOKEN`. Credentials stay out of URLs and arguments.
There is no automatic all-branch mirror; its broader credential/ref scope requires
separate authorization. Keep divergent refs and never force-push to get a pass.
Remove these temporary test branches after their associated integration/release
is complete; retain the workflow run as qualification evidence. For a retry, use
GitHub's visible workflow rerun, then retry the GitLab gate; failed attempts remain
visible. Public standard-runner compute is free; API limits and service availability
can still prevent qualification. Neither job publishes an artifact or release.

A green amd64 job alone cannot qualify the full pipeline. The optional hosted native release
builders retain their platform tags; the normal protected-tag release path still
uses the exact-revision GitHub native-build handoff. Runner recovery does not
change the single GitLab publication path or make an unrun check pass.

1. Merge a green change pipeline to `main` and confirm the resulting trunk
   pipeline is green.
2. Create an annotated candidate tag such as `v0.1.2-rc.1` on that exact trunk
   revision and push it to GitLab.
3. Confirm all six native archives and the plugin archive were smoke-tested,
   reproduced byte-for-byte, included in `SHA256SUMS`, signed, and published
   with the SBOM, manifest, and provenance.
4. Install the candidate archives on representative consumer systems and run
   `opdev version`, `opdev init --dry-run`, and a fixture `opdev check`.
5. If the candidate is accepted, create the final `v0.1.2` tag on the same
   qualified revision. The final tag runs the complete pipeline again; it does
   not promote candidate bytes under a new identity.
6. After the final CLI release is published and qualified, complete
   [managed-runtime pin maintenance](#managed-runtime-pin-maintenance): update
   the source plugin to select that CLI, verify installation and reuse, and
   merge the pin change with its required pre- and post-merge checks. Do not
   report the release task complete while its managed pin still targets the
   previous CLI; report any blocker and keep the release work open.

The separate final build is intentional because the versioned asset names,
source revision, and tag-bound signing identity differ. Within each tag
pipeline, every published archive is built once on its target runner and
promoted unchanged through the GitHub artifact handoff, GitLab evidence,
signing, and GitHub publication.

## Consumer verification

The canonical repository is now `stolenfootball-tools/opdev` on GitLab.
For 0.1.2 and later, download from `https://github.com/stolenfootball/opdev/releases`.
Historical 0.1.0/0.1.1 releases remain at `https://gitlab.com/stolenfootball-tools/opdev/-/releases`.
The GitHub build mirror is `stolenfootball/opdev`.

Releases through `v0.1.1` (including their release candidates) were signed before
the repository rename. Their certificate identity still contains
`stolenfootball-tools/opinionateddevelopment`, as in the example below. Renaming
the repository does not change existing signatures or provenance. Releases
signed after the rename use `stolenfootball-tools/opdev` in the identity;
always use the exact identity for the release being verified.

Download the selected archive, its `.sigstore.json` bundle, and `SHA256SUMS`
from the same release. The signer remains GitLab even when the download host is GitHub. Verify the digest and then the GitLab signing
identity:

```sh
sha256sum -c SHA256SUMS --ignore-missing
cosign verify-blob opdev-0.1.1-x86_64-unknown-linux-gnu.tar.gz \
  --bundle opdev-0.1.1-x86_64-unknown-linux-gnu.tar.gz.sigstore.json \
  --certificate-identity "https://gitlab.com/stolenfootball-tools/opinionateddevelopment//.gitlab-ci.yml@refs/tags/v0.1.1" \
  --certificate-oidc-issuer "https://gitlab.com"
```

Use the matching file names on Windows or macOS. `opdev-release-manifest.json`
records the exact source revision, builder pipeline, artifact digests, SBOM
digest, and the scope limitation of the all-target dependency inventory.

## Recovery

OpDev retains project contracts, adoption decisions, requirement mappings and
local continuation state. Immutable executable versions do **not** make that
state backward compatible. Recovery must preserve later developer work and must
not silently run an older CLI against a newer unsupported schema.

The automated recovery paths are bounded by the failure they address:

- **Interrupted coordinated migration:** retain the private recovery snapshot and
  resume the same reviewed plan with `upgrade --resume FILE --apply ID`.
  The CLI accepts only recorded before/after states. A later edit, changed input
  or forged snapshot stops recovery without replacing that work. Inspect the
  current conflict and prepare a newly reviewed migration; never force the old
  snapshot over it. Successful resume yields the intended readable contracts and
  an idempotent retry. Recovery is forward continuation, not automatic rollback.
- **Failed managed installation:** verification failure must not execute the
  unverified CLI or replace an existing version. Retry the same pin only after
  the cause is corrected. Existing verified versions remain available, but their
  use still needs compatibility with the project and plugin. Windows staging
  cleanup has a bounded four-attempt retry; exhausted cleanup reports partial
  installation distinctly. Do not remove another running installer's lock.
- **Interrupted publication:** the already-authorized publisher resumes the same
  draft and exact artifact inventory, uploads only missing matching bytes and
  verifies the complete inventory before making it public. A conflicting digest,
  moved tag or mutable published release stops the operation. Successful repeated
  publication performs no replacement or duplicate publication. This recovery
  procedure is not authorization to create or publish a release.

For these controlled interruptions, the data-loss target is zero for retained
project content, later edits, prior runtimes and already-uploaded matching assets.
Recovery completion is recognized by the state/digest/compatibility assertions,
not merely exit zero. There is no promised wall-clock recovery SLA: network and
provider availability can block progress. Fail closed and retain the interrupted
state rather than silently extending retries or claiming completion. Tests inject
these failures in disposable fixtures; they do not simulate every filesystem
failure, restore arbitrary user data, or authorize live fault injection.

For a defect requiring new code, a future patch is remediation, **not an already
tested recovery path**. Stop affected use, preserve project data and identify a
compatible safe version or reviewed repair. Only after separate release approval:

1. Mark the affected release and its known impact in GitHub release notes and the GitLab work tracker without deleting
   its evidence.
2. Restore `main` first if its required pipeline is red.
3. Implement the smallest compatible fix with a regression test.
4. Run the normal merge-request and trunk gates.
5. Publish a new candidate and then a new SemVer patch through the tag pipeline.
6. Repeat checksum, signature, installation, and fixture checks before advising
   consumers to upgrade.

Pinning a previous executable is safe only after checking its data/schema and
plugin compatibility; never promise that downgrading reverses a migration.
Exercise affected recovery paths whenever they change. Ordinary regression and
adoption checks do not require a new publication. New release artifacts still
need their own complete qualification and explicit release authorization.

## Managed-runtime pin maintenance

An authorized OpDev CLI release includes this follow-up pin update; do not wait
for a separate developer request. Publish and qualify the final CLI first, then
update `plugins/opdev/runtime.lock` to its version, tag and exact signing
identity. Preserve verifier digests unless the verifier itself is deliberately
updated and qualified. Bump the source plugin version consistently in both host
manifests, the compatibility contract and versioned marketplace metadata, and
reconcile the installation documentation and affected requirements bindings.
Run the offline and live installer checks and complete normal integration.

This is a source-plugin follow-up, not another CLI release: never rewrite the
published tag or immutable plugin archive to include the later pin, and do not
start a recursive release/version-bump cycle. A plugin-only release keeps the
current qualified CLI pin unless a CLI update is part of its scope. Updating
installed plugin caches, consumer CI pins or project policy remains separate
work and requires its own applicable authorization.

Standalone CLI 0.5.0 is published and qualified; source plugin 0.5.1 pins that
immutable GitHub release. No historical release assets or cached runtimes are
replaced. The plugin and CLI versions are independent. Update the lock's
version, tag, signing identity, and verifier digests only as a reviewed change,
then run the offline and live installer suites. Do not point a source-marketplace
plugin at a release that has not been published.

CLI 0.5.0 supplies the newer engineering, clean-adoption and requirements/review
capabilities. The immutable 0.5.0 plugin archive retains its original CLI 0.4.0
pin; refresh the source plugin for the new pin or select a verified compatible
standalone 0.5.0 executable. Updating the source pin does not edit
installed plugin caches, migrate project policy or update consumer CI pins.

The bootstrap scripts and lock are included automatically in the shared plugin
archive. Existing native archives and release signing/publication remain the
canonical path. cargo-dist is enabled for standalone release packaging and verified installers.
The old GitLab-only feasibility probe remains as an upstream regression aid.

## First GitHub candidate and interrupted publication

The first candidate must exercise the exact-tag mirror handoff, immutable draft
publication, download verification on supported platforms, README one-liners,
and a fixture `opdev check`. Generation and offline fixtures do not replace this
qualification. Candidate `v0.1.2-rc.1` completed this path in
[GitLab pipeline 2835650463](https://gitlab.com/stolenfootball-tools/opdev/-/pipelines/2835650463).
All 43 downloaded asset digests and all 15 archive/installer signatures passed.
Live install/reinstall and CLI fixtures passed on macOS ARM64 and Ubuntu 24.04
ARM64; the downloaded macOS x86-64 binary passed under Rosetta. All six native
build/smoke jobs and Windows installer fixtures passed in CI. Post-download
Windows installation was not exercised.

Retrying the same publication job succeeded against the immutable candidate
without rebuilding or replacing assets (job 16411303523). Interrupted-draft
recovery remains covered by offline fixtures; no live interruption was injected.
[Issue 21](https://gitlab.com/stolenfootball-tools/opdev/-/issues/21) records
candidate acceptance and final release validation.

If upload is interrupted, retry the same GitLab publication job: matching assets
are retained, missing assets are uploaded, and the draft becomes public only
after the complete digest inventory matches. A conflicting draft/tag requires
investigation and a new candidate tag; never delete or overwrite assets to force
publication. If GitHub publication succeeds but GitLab release-note creation
fails, retry that note job; do not rebuild or republish the binaries.
