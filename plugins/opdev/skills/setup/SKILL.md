---
name: setup
description: Install or locate the pinned native CLI for the OpDev plugin. Use when the user requests OpDev setup or the activated OpDev skill needs its managed runtime.
---

# Set up OpDev

Resolve the plugin root two directories above this skill directory. Installation
downloads the pinned CLI and a checksum-pinned signature verifier, verifies the
GitLab signature before extraction, and checks the CLI version and compatibility.
It does not need Rust, Node, Python, a global cosign installation, administrator
privileges, or changes to shell profiles. Normal host tool approvals still apply.

On macOS or Linux, run:

```sh
sh <plugin-root>/scripts/runtime.sh --install
```

On Windows, run:

```powershell
powershell -NoProfile -File <plugin-root>/scripts/runtime.ps1 -Mode Install
```

The command prints the installed executable's absolute path on success. Run that
path with `version`, then `plugin verify --contract <plugin-root>/opdev-compatibility.json`.
Use that exact executable in the OpDev workflow. Do not run `opdev init` as part
of setup or install the project's compilers or test runners.

For a read-only lookup, use `--path` or `-Mode Path`. To forward CLI arguments,
use `--run <arguments>` or `-Mode Run <arguments>`. A repeat install verifies the
existing runtime and does not download again. Never represent a failed setup as
successful or bypass signature/compatibility validation.

Storage defaults to `$XDG_DATA_HOME/opdev` (or `$HOME/.local/share/opdev`) on
macOS/Linux and `%LOCALAPPDATA%/opdev` on Windows. `OPDEV_DATA_DIR` selects an
absolute alternative, including a host-provided plugin data directory. Pass the
same value on every invocation. This versioned OpDev-owned store is shared
between agents, survives plugin-cache replacement, and does not modify global
PATH. A host's plugin uninstall does not remove this shared store automatically.

The pin in `runtime.lock` selects an exact version and platform. Keep old runtime
versions while existing sessions use them. The bootstrap rejects an unsupported
platform or a corrupt existing runtime. If recovery is needed, explain the
reported directory and obtain authorization to move it aside before reinstalling.
For an interrupted setup, first verify no installer is still running before
removing its reported stale lock. Never delete the full data root to recover one
version. Explicit uninstall may remove the selected runtime directory once no
session uses it; leave other versions intact.

Requirements: macOS/Linux need standard `sh`, `curl`, `tar`, `awk`, `sed`, `mktemp`,
and `sha256sum` or `shasum`. Linux must run the published GNU binary. Windows needs
PowerShell 5.1+; Windows ARM64 uses the pinned x64 cosign verifier under Windows 11
x64 emulation. Network access is needed for GitLab releases, GitHub verifier
assets, and Sigstore trust data. A cached runtime needs no download.
