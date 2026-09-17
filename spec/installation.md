# Native installation and plugin runtime

The plugin can install a compatible native CLI from the canonical release host
without requiring a Rust toolchain or a global signature-verifier installation.
Plugin 0.2.5 pins the qualified, immutable CLI 0.2.2 GitHub release. The
pin is deliberately separate from the plugin version so a source-marketplace
update never depends on an unpublished archive. The declared CLI compatibility
range MUST accept the pin. Updating the pin is a reviewed source change.

## Activation and consent

POSIX scripts and `runtime.lock` MUST retain LF endings through Windows Git
checkouts and plugin packaging. Plugin-local attributes travel with standalone
copies; repository attributes also cover other shell entry points. Existing
installed caches are not repaired by a source change: refresh from a qualified
package, rather than silently altering cached runtime code. Offline tests exercise
Git checkout conversion with `core.autocrlf=true` and POSIX runtime lookup.

Managed instruction reconciliation MUST reject filesystem links and Git-index
mode `120000` entries, even when Windows exposes them as ordinary files. Preview
and apply both check this boundary. Converting upstream links requires an
explicitly reviewed migration; OpDev does not perform it automatically.

For updates rather than initial setup, follow [coordinated upgrades](upgrades.md).
Plugin installation, selected runtime, project guidance and CI qualification are
separate states. Installation alone never establishes a fully upgraded project.

Skill inspection is applicability detection, not project consent. First inspect
the target repository's contract at its Git root. Configured projects use OpDev
seamlessly; malformed contracts remain errors. In uninitialized projects, routine
pull/status/dev-server operations MUST NOT trigger workflow enforcement, adoption
suggestions, or runtime lookup. For substantive software planning or implementation,
the agent MUST offer adoption once in its first response after minimal local
inspection establishes applicability, before substantive research, planning or
edits. The choice is optional for the developer, not the offer for the agent.
A folder containing only a supplied design qualifies, even without Git. Reading
that design, local instructions and repository state is permitted before the
offer; language/runtime availability probes for stack selection, upstream
browsing, dependency investigation and architecture selection
must not precede it. Do not initialize Git merely to check applicability.
Use a brief neutral question through a permitted host tool or plain chat.
Honor a prior decline in the current conversation. Without acceptance, continue
the original task without OpDev; do not repeatedly ask or block ordinary work.
No runtime lookup, setup, adoption or consent-record write precedes acceptance.
Explicit adoption supplies assessment consent. Explicit setup supplies runtime
installation consent but does not adopt a project.

Resolve the runtime only after activation. If missing, offer setup and wait for
acceptance; plugin installation and compatibility verification do not grant
project or runtime-installation consent. The prompt hook only inspects project
state, never probes or executes a runtime. The skill classifies user intent;
the hook does not attempt natural-language classification with keyword rules.

Regression scenarios: uninitialized pull-and-start, status, unrelated requests,
substantive feature work with acceptance/decline/no answer, explicit adoption,
explicit setup, configured work, nested directories and worktrees. Deterministic
hook tests establish no runtime execution and correct root discovery; they do
not prove that every agent will interpret intent correctly. The
[early-offer canary](../benchmarks/adoption/early-offer-canary.md) separately
checks visible offer timing and consent in fresh host sessions. These regressions
are general activation boundaries, not project-specific exceptions.

## Bootstrap contract

`plugins/opdev/runtime.lock` is trusted package data, not executable code. It
pins the CLI version/tag, exact GitLab signing identity, platform mapping, cosign
version, and each verifier's SHA-256 digest. Verifier pins were obtained from the
upstream cosign v3.1.3 `cosign_checksums.txt`. This trusts the reviewed plugin
source and the pinned verifier digest; downloading an adjacent checksum at
installation time is not a substitute for that trust anchor.

The bootstrap MUST verify the verifier digest before running it, then verify the
CLI archive with cosign's exact certificate identity and GitLab issuer before
extracting or running the CLI. Existing pre-rename signatures retain their old
identity. Only the expected executable is extracted to a private temporary
location. Version and plugin compatibility checks MUST succeed before the
runtime directory is installed atomically. Failed attempts clean their staging
files and lock; a lock left by an uncatchable interruption requires explicit
recovery after checking that no installer is active.

Windows temporary cleanup makes at most four attempts for I/O/access failures,
with visible diagnostics and delays of 100, 200, and 300 milliseconds. Only the
validated staging directory and this invocation's lock may be removed; linked
targets are rejected. Lock disposal/removal is attempted even when staging
cleanup fails. Exhaustion is an error, preserving the original installation
failure or explicitly reporting an installed runtime with incomplete cleanup.
Verification and downloads are never retried by this cleanup policy.

Runtime directories are immutable per release tag and target under an
OpDev-owned data root. macOS/Linux default to `$XDG_DATA_HOME/opdev` or
`$HOME/.local/share/opdev`; Windows defaults to `%LOCALAPPDATA%/opdev`.
`OPDEV_DATA_DIR` selects an absolute alternative. The common root makes skills,
hooks, and different agents resolve the same runtime without relying on
host-specific environment interpolation. It is outside the plugin cache and
survives updates. It is not removed automatically when either agent uninstalls
the plugin. Explicit cleanup must preserve versions still used by sessions.

A stored executable digest detects accidental runtime damage before reuse. It
does not defend against a local actor who can rewrite both the executable and
its receipt; user-owned runtime storage has the same local trust boundary as
other installed developer tools. A corrupt runtime is reported, not silently
executed, overwritten, or replaced by a different binary.

Read-only lookup and command execution do not download dependencies. Only an
explicit bootstrap installation operation downloads. The activated skill may
request it through normal host approvals when no compatible CLI is available;
the prompt hook remains read-only. A compatible standalone CLI remains usable.
Setup MUST NOT initialize a repository, change global PATH or shell profiles,
install project-specific tools, or bypass a denied host permission.

## Platform and dependency boundary

The lock covers Windows, macOS, and Linux GNU on x86-64 and ARM64. Basic OS shell,
HTTPS download, hashing, and archive facilities are prerequisites. Windows uses
PowerShell 5.1 or newer; Windows ARM64 requires Windows 11 x64 emulation for the
upstream x64 cosign binary. The CLI's own system-library requirements still
apply. Unsupported or unusable binaries fail setup before installation.

Initial verification requires access to GitHub archives (GitLab for historical
0.1.0/0.1.1 pins), GitHub cosign binaries,
and Sigstore trust metadata. Reuse of a valid installed runtime requires no
network. Downloads have bounded timeouts and HTTPS-only redirects. Neither bootstrap
silently retries a failed download; a new installation attempt is explicit. Signature failure never
falls back to checksum-only installation.

## Standalone cargo-dist installation

Releases beginning with 0.1.2 use the existing public GitHub mirror as the
canonical binary host. GitLab remains the source, qualification, signing, and
publication authority. Historical GitLab releases remain available, including
historical managed 0.1.1 runtimes. Managed bootstraps and generated CI adapters select GitLab
for 0.1.0/0.1.1 and GitHub for subsequent versions.

The release pipeline pins cargo-dist 0.32.0 by binary checksum. A generic dist
package identifies the GitHub mirror and stages already-built native binaries;
it MUST NOT compile them again. Existing archive layouts/names are preserved
as additional assets, while cargo-dist creates its expected archive layout.
Packaging verifies that every extracted executable equals the input byte for
byte. Because cargo-dist preserves wall-clock archive metadata, the existing
deterministic OpDev packager normalizes its archive layout before signing;
installer and manifest digests are refreshed for those final bytes. Global installers and refreshed manifest/checksums describe the final
assets. GitLab signs both legacy and cargo-dist archives and the installers.

Generated scripts receive the versioned verification extensions in
`release/installers`. Generation requires exact, single upstream insertion
points and MUST fail on template drift. The extension verifies a pinned cosign
binary digest, then the downloaded archive's exact GitLab signing identity and
issuer before extraction. Verification failure prevents installation. These
extensions are necessary because upstream PowerShell lacks checksum checks and
neither upstream installer checks our signatures. Optional cargo-dist updater
installation is disabled until its verification path is separately qualified.
The generated scripts retain the upstream MIT notice; OpDev additions are
Apache-2.0 and the upstream license ships with releases.

Standalone installers use cargo-dist's user installation directory, PATH
handling, and overwrite semantics. Reinstallation downloads and verifies the
selected version again. This differs deliberately from immutable cached plugin
runtimes. No repository is initialized. Download overrides do not override the
pinned signature identity or verifier hashes.

The accepted alternative was GitLab-first plus GitHub fallback, which generated
successfully but would require synchronizing two download inventories. Pure
GitLab generation still fails in cargo-dist 0.32.0's GitHub-only receipt path;
the historical probe remains for tracking upstream support. Extending only our
handwritten installer would forgo cargo-dist packaging and installer maintenance.
The user selected GitHub hosting and cargo-dist, accepting a bounded, tested
verification extension. Reconsider the extension when upstream provides
fail-closed archive authenticity checks, and reconsider hosting if the mirror
cannot preserve exact source tags and immutable qualified assets.

## Acceptance and qualification

Canonical workspace tests run the offline POSIX bootstrap cases. PowerShell has
an equivalent offline suite run on a Windows CI runner. Cases cover successful
install and reuse, missing-runtime lookup, corrupt verifier/signature/runtime,
wrong versions, compatibility failure, argument handling, and lock cleanup.
Linux x86-64 and ARM64 and Windows CI also install the actual pinned release into
temporary storage. macOS is exercised locally before review; full platform
release qualification remains part of the protected tag pipeline. A successful
bootstrap of CLI 0.1.1 does not qualify a new CLI release.

Standalone qualification generates all six target packages and both installers,
runs shell/PowerShell signature-failure fixtures, and checks interrupted draft
publication and immutable retry behavior. The first live GitHub candidate, `v0.1.2-rc.1`, passed this qualification;
[issue 21](https://gitlab.com/stolenfootball-tools/opdev/-/issues/21) records
the exact pipelines, signature and inventory checks, and consumer environments.
Each subsequent release still requires its own tag-bound qualification.
