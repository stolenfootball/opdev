# CI provider boundary

Use [CI design and improvement](ci-design.md) when choosing or revising a topology;
adapter generation and inspection do not establish optimality or authorize edits.

## Coordinated feedback examples

`examples/feedback/` provides opt-in neutral GitHub/GitLab wiring examples, not
automatic replacements for custom project YAML. Proposed changes run one
verification path; integrated trunk runs its own verification and builds a
candidate once, with downstream jobs consuming those exact bytes. GitLab rules
exclude ordinary feature pushes, including the first push before an MR exists.
GitHub push triggers are trunk-only. Existing required checks remain distinct;
source/environment changes can legitimately require further verification.

Standalone explicit diagnostics collect scoped environment observations without
producing qualification or release artifacts. See the example README for source
trust, runner/secret policy, ownership and cleanup requirements. Syntax checks
alone do not prove provider behavior; use actual isolated pipeline observations.

## Provider adapters

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

### GitHub inspection and custom layouts

The generation destination `.github/workflows/opdev.yml` is a default, not an
adoption requirement. Inspection inventories all `.yml` and `.yaml` files directly
in `.github/workflows`, including custom filenames, and uses the project's declared
trunk. It does not rename or replace adequate project CI. The adapter's legacy
`inspect` API assumes `main`; project-aware callers use `inspect_for_trunk`.

The supported offline subset connects an event to its job, checkout, runtime
setup and actual gate invocation. PR checks select the PR head, not an assumed
merge ref; trunk push checks select the integrated commit and use `--post-merge`.
Same-commit local reusable workflows are followed without network access. Static
event conditions and literal trunk filters are supported. Disabled, optional,
inspection-only, wrong-stage and disconnected command text do not establish a
required gate. More than one applicable gate is ambiguous rather than permission
to choose a convenient passing candidate.

Runtime setup can be the generated signed-release bootstrap or a locked
`opdev-cli` Cargo build connected to an exact checkout and its directly invoked
debug/release binary. A separate source checkout must name a repository and full
commit identifier. Native POSIX and PowerShell commands are supported. This
recognizes a reviewed source-build alternative; it does not authenticate that
repository's implementation, execute the build, or waive runtime review. Custom
installer scripts, shell wrappers and intermediate actions whose effects cannot
be established remain `unverified`, not grounds to rewrite the project's CI.

Unsupported conditions, path/glob filters, dependency collectors, matrices,
remote or parameterized reuse, environment/path overrides and unresolved setup
remain explicit review limitations. The reader is not a GitHub expression or
shell interpreter. Malformed, missing referenced or linked inputs are errors;
limits produce `unverified`, never a partial success. Limits are 256 directory
entries, 64 workflow files, 1 MiB per file, 4 MiB total, 256 jobs per workflow,
512 steps per inspected job, reusable depth 8 and 128 expansion visits.

A configuration `passed` means only the supported wiring was found. It cannot
erase a failed current check, prove that a provider job ran, or establish branch
protection. Qualification separately requires the correct event, revision,
required job conclusions and reviewed protection policy. In particular, a skipped
job can appear successful to required-check protection; do not substitute its
conclusion for an executed verification. Generated report wrappers retain both
pre-merge and post-merge failures without changing their exit status.

These distinctions follow GitHub's [workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax),
[checkout source selection](https://github.com/actions/checkout), and
[required-check behavior](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks).
Use effective-provider review for unsupported constructs, retaining the precise
limitation; do not label an unknown configuration verified or invent a waiver.

GitLab local inspection supports a single pipeline document or a separate `spec`
header followed by the pipeline. Explicit local includes (string, mapping or
array) are resolved recursively from the repository root, in provider merge
order; root overrides apply last, maps merge and arrays replace. Repeated includes
are applied in order; recursion cycles are errors. Reads reject linked children,
escaping paths, nonregular files and files above 1 MiB, with at most 150 includes
and depth 32, with an 8 MiB total-input budget. Dynamic/conditional/parameterized, glob, external/template/component
includes and input interpolation remain unverified. Missing/malformed inputs are
errors, never false reports of absent controls. Doctor and upgrade inventory use
the same interpretation and bind included files to upgrade previews. This remains
supporting syntax/string evidence, not a GitLab compiler or proof of job execution.
