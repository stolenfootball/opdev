# Native installation and plugin runtime

The plugin can install a compatible native CLI from the canonical GitLab release
without requiring a Rust toolchain or a global signature-verifier installation.
Version 0.1.2 of the plugin initially pins the already-published CLI 0.1.1. The
pin is deliberately separate from the plugin version so a source-marketplace
update never depends on an unpublished archive. The declared CLI compatibility
range MUST accept the pin. Updating the pin is a reviewed source change.

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

Initial verification requires access to GitLab archives, GitHub cosign binaries,
and Sigstore trust metadata. Reuse of a valid installed runtime requires no
network. Downloads have bounded timeouts and HTTPS-only redirects. Neither bootstrap
silently retries a failed download; a new installation attempt is explicit. Signature failure never
falls back to checksum-only installation.

## cargo-dist decision

cargo-dist 0.32.0 was evaluated with `hosting = ["simple"]`, GitLab release URLs,
and shell/PowerShell installers. Actual generation fails in
`InstallReceipt::from_metadata` because it requires GitHub hosting. A GitLab
repository metadata URL is rejected earlier as well. The reproducible probe is
in `release/cargo-dist-probe` and `scripts/probe_cargo_dist.py`; its reported
`error` describes unavailable generation, not a passing installer gate.

Alternatives were adding a second GitHub consumer release channel, maintaining a
cargo-dist fork, or rewriting generated installer templates. All would add
maintenance or weaken the established distribution boundary. This increment
therefore uses a small reviewed bootstrap over the existing signed archives.
Revisit cargo-dist when the probe can generate GitLab-only installers without
patches. At that point its nested tar layout must be handled additively, preserving
legacy archive names/layout and packaging already-built binaries without rebuilds.

## Acceptance and qualification

Canonical workspace tests run the offline POSIX bootstrap cases. PowerShell has
an equivalent offline suite run on a Windows CI runner. Cases cover successful
install and reuse, missing-runtime lookup, corrupt verifier/signature/runtime,
wrong versions, compatibility failure, argument handling, and lock cleanup.
Linux x86-64 and ARM64 and Windows CI also install the actual pinned release into
temporary storage. macOS is exercised locally before review; full platform
release qualification remains part of the protected tag pipeline. A successful
bootstrap of CLI 0.1.1 does not qualify a new CLI release.
