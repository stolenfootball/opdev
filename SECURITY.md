# Security policy

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitLab's private
vulnerability reporting flow for this project. Include the affected version,
reproduction details, impact, and any suggested mitigation. Avoid including
credentials, personal data, or unrelated secrets.

Maintainers should acknowledge a complete report within five business days,
keep the reporter informed while impact and remediation are assessed, and
coordinate disclosure after a fix or mitigation is available. These are
response targets, not a guarantee that every issue can be resolved within a
fixed period.

## Supported versions

Until OpDev reaches its first stable release, only the latest published release
and the current default branch receive security fixes. Release notes will state
when this policy changes.

## Security boundaries

The CLI treats initialized project content as untrusted:

- discovery does not execute repository-controlled commands;
- configured checks use exact argument vectors without a general-purpose shell;
- checks have time and output bounds and terminate their process group;
- remote audits are read-only and limited to first-class provider hosts;
- extensions cannot replace or weaken core MinimumCD results; and
- release archives and standalone installers are signed by GitLab CI; the
  generated provenance does not claim trusted-builder provenance.

Running an initialized project's canonical commands still executes code chosen
by that project. Review `.opdev/project.yaml` before running checks from an
untrusted repository.

On Windows, Node package managers are commonly installed as batch shims rather
than native executables. OpDev resolves only `npm`, `npx`, `pnpm`, `pnpx`,
`yarn`, and `yarnpkg` through the executable subset of `PATHEXT` in `PATH`; it
does not implicitly search the repository working directory or execute
association-based script types such as PowerShell or JavaScript. The resolved
absolute `.cmd` or `.bat` path is passed to Rust's constrained batch-file
launcher. OpDev never
constructs an unrestricted `cmd /c` string, and Rust rejects arguments it
cannot escape safely. The package manager and project scripts remain
project-controlled code and require the same review as any canonical command.

## Plugin bootstrap trust

The managed-runtime bootstrap trusts the installed plugin source and its reviewed
`runtime.lock`. It verifies the pinned cosign executable digest before execution,
then requires the CLI archive's exact GitLab certificate identity and issuer.
Signature failure never falls back to adjacent checksums. Only the expected
executable is extracted after verification; temporary storage and per-version
installation locks isolate incomplete attempts. Installation uses user-owned
storage and never changes global PATH or initializes a repository.

The runtime receipt detects accidental damage before cached execution. It does
not protect against an actor able to rewrite both executable and receipt in the
same user's data directory. `OPDEV_DATA_DIR` is a trusted caller override, not a
repository-configured value. A damaged runtime requires explicit recovery;
read-only hooks do not replace it or silently select a different executable.
See `spec/installation.md` for dependencies, timeout behavior, and recovery.

Standalone cargo-dist installers verify a checksum-pinned cosign binary and the
archive signature before extraction. The initial installer script is a trust
anchor downloaded over HTTPS. GitHub immutable release settings protect the
published assets, but do not replace consumer signature verification.
