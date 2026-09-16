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

OpDev is a stateless CLI and agent plugin. Consumers select an exact version,
and published releases are immutable, so an unsafe release does not require a
data rollback or mutation of existing assets.

Recovery is an on-demand safe roll-forward:

1. Mark the affected release and its known impact in GitHub release notes and the GitLab work tracker without deleting
   its evidence.
2. Restore `main` first if its required pipeline is red.
3. Implement the smallest compatible fix with a regression test.
4. Run the normal merge-request and trunk gates.
5. Publish a new candidate and then a new SemVer patch through the tag pipeline.
6. Repeat checksum, signature, installation, and fixture checks before advising
   consumers to upgrade.

Until the forward fix qualifies, consumers can pin or reinstall the last known
good exact version. This procedure must be exercised during the initial release
candidate and whenever the delivery path materially changes.

## Managed-runtime pin maintenance

Standalone CLI 0.2.1 is published and qualified; source plugin 0.2.3 pins that
immutable GitHub release. No historical release assets or cached runtimes are
replaced. The plugin and CLI versions are independent. Update the lock's
version, tag, signing identity, and verifier digests only as a reviewed change,
then run the offline and live installer suites. Do not point a source-marketplace
plugin at a release that has not been published.

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
