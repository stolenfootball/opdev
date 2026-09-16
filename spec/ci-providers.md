# CI provider boundary

GitHub Actions and GitLab CI are first-class providers in OpDev 0.1. Each adapter owns four responsibilities: the canonical configuration path, baseline rendering, read-only local inspection, and evidence describing change and trunk qualification. Provider-specific syntax remains behind the `CiAdapter` boundary; the rule engine consumes provider-neutral outcomes.

`opdev ci generate` prints a configuration by default and writes only with `--write`. A write uses create-new semantics and refuses to replace an existing CI file. This makes adoption reviewable in brownfield projects. `opdev ci inspect` parses configuration as YAML without executing it. `opdev check --ci` folds those findings into the relevant MinimumCD rule results before recomputing the integration gate.

Generated configurations run for proposed changes and the declared trunk, preserve the JSON report even when a gate blocks, use read-only repository permissions, install the exact versioned OpDev archive, and verify it against the release checksum manifest. Downloads, extraction, and the live evaluator output occur in job-scoped temporary storage outside the repository checkout so generated files cannot invalidate fingerprint-bound change evidence. After evaluation finishes, the job copies the report into the checkout for provider artifact collection and exits with the evaluator's original status. A report-copy failure is itself fatal. GitHub third-party actions are pinned to full commit identifiers. Provider configuration and immutable release publication remain separate authorities.

The GitLab adapter composes with the project's toolchain rather than assuming
that a minimal OpDev-only image can execute arbitrary project commands. It
selects a Debian Trixie variant of an official Rust, Go, Node.js, or Python
image only when the repository declares supported numeric version metadata in a
project-owned version file. Rust, Node.js, and Python retain the declared
version. Go's `go` directive is a minimum requirement and its optional
`toolchain` directive is a preferred toolchain, so OpDev selects the Docker
Official Image's corresponding major-minor Trixie family (for example,
`go 1.24.0` selects `golang:1.24-trixie`) instead of assuming that a
patch-specific Debian tag exists. Mixed ecosystems and custom toolchains
must select their reviewed image explicitly with `--image`; generation fails
instead of guessing when no safe selection is available. The selected image
must provide a POSIX shell, Git, curl, tar, `sha256sum`, and `mktemp`. The
generated job checks those prerequisites and starts the downloaded CLI before
running the project contract. For the current GNU release line, the image must also
provide glibc 2.39 or newer because the published Linux archive is GNU-linked.

This is an OpDev packaging constraint, not a MinimumCD requirement to upgrade a
product's distribution. Generation explains the selected image and x86-64/glibc
prerequisites. Preserve existing product images and review inherited job setup.
The generated GitLab job uses `cache: []` so an inherited compiled cache cannot
cross a libc boundary. Any project-approved cache must include compatibility
boundaries (OS/libc, architecture, toolchain); qualify cold and warm cache runs.
See [GitLab caching](https://docs.gitlab.com/ci/caching/).

Both templates verify a checksum-pinned cosign executable using the maintained
runtime lock, then require the exact tag-bound GitLab certificate identity and
issuer before extraction. Adjacent release checksums alone are insufficient.
Only the expected regular executable is extracted. Verification never falls back
to checksum-only execution. See [Sigstore verification](https://docs.sigstore.dev/cosign/verifying/verify/).

Generated configurations are integration baselines, not complete publishing
pipelines. A capable CLI's `check --ci --delivery` runs declared delivery suites
and delivery extensions and returns the delivery verdict. Require that check as a
non-optional predecessor of publication on the actual tag/release path, bound to
the qualified artifact; an MR/default-branch-only job is insufficient. Review
provider dependency/rules semantics and exercise the release path. Merely finding
strings in YAML is not proof that publication cannot bypass a gate. Local adapter
inspection is supporting configuration evidence, not pipeline execution evidence.

Reducing the GNU baseline or adding musl remains a separately qualified packaging
decision: test every target and native dependency before changing published platform
promises. This repair retains the existing baseline and isolates its consequences.

Additional providers implement the same Rust trait or, in a future external rule-pack protocol, contribute equivalent evidence without altering core rule IDs. Unknown providers remain `unverified`; they never inherit a pass from GitHub or GitLab assumptions.
