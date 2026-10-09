# Contributing to OpDev

OpDev accepts focused changes that preserve its evidence semantics and remain
applicable to software projects beyond the repository used to motivate them.

Before editing, read `.opdev/project.yaml`, the authorities selected by its
context routes, and the tracked work item. The work item should define the
problem, intended outcome, scope and exclusions, acceptance conditions, evidence,
and material risks. Durable architecture or contract changes belong in `spec/`
and should describe alternatives, rationale, and a reversal trigger.

Self-development uses the CLI built from this checkout (`cargo build --locked
-p opdev-cli`), whose capabilities must match the selected clean-2 contract.
Verify it against `plugins/opdev/opdev-compatibility.json`; do not silently use
an older installed CLI or change the published managed-runtime pin. On Windows,
run full checks from a temporary copy of that verified executable so Cargo can
rebuild the workspace binary without an executable-file lock.

Use Rust 1.97.0 and Python 3 for the offline POSIX installer tests, then run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Behavioral changes require automated tests. Escaped defects require regression
coverage unless a specific, reviewable limitation explains why automation cannot
prevent recurrence. Do not hide flakes with retries; a quarantine needs an owner,
tracked remediation, and an expiry.

### Supporting-script conventions

Maintained Python in `scripts/`, `tests/` and `examples/` uses Ruff's formatter
and high-signal E4/E7/E9/F lint rules. The test-only E402 exception allows disabling
bytecode output before imports. Shell entry points use POSIX syntax, shfmt with
four-space indentation and indented case arms, and ShellCheck warnings/errors.
PowerShell uses four spaces and the explicit formatting/security/correctness
rules in `scripts/PSScriptAnalyzerSettings.psd1`. Frozen `benchmarks/` specimens
are intentionally not reformatted; they are evaluation inputs, not maintained
tooling. A template suppression must explain the generated context, and generated
installer behavior still requires its existing integration tests.

Set up the pinned tools explicitly with `python scripts/setup_style.py`. This
downloads checksum-pinned upstream assets into ignored `target/style-tools`
on the Windows/Linux x64 development and quality hosts; it does not install
globally, edit PATH, or update consumer plugins. Other development hosts need an
explicitly reviewed setup extension, not silently substituted tool versions.
The version/digest inventory is `scripts/style-tools.json`; review updates like
other executable dependencies. Missing tools fail checks rather than installing
automatically or skipping them.

Run `python scripts/check_scripts.py format` and
`python scripts/check_scripts.py lint`. These are canonical local/pre-merge/
post-merge suites alongside Rust's existing commands. Linux checks Python/shell;
the required Windows CI job additionally checks PowerShell, including settings
files. On a Windows development host the same commands check all three languages.
Neither a Linux pass nor a syntax-only test proves the Windows checks passed.
Checks never auto-fix. Keep one-time mechanical formatting separate from behavior
repairs and preserve regression assertions and frozen inputs.

Before requesting integration, stage all material files and bind acceptance
conditions to the actual assertions and current source. Store the bounded review
in this repository's GitLab MR, using review storage policy 2. Do not recreate
`.opdev/evidence.yaml` or an evidence repository. Prepare drafts outside source,
export with `evidence bundle export-review --discussion`, and review before
posting. An agent's claim that its own output is correct is not evidence.

### CI review handoff

The `quality` job runs native `opdev check --ci --review-ci` (adding
`--post-merge` on integrated trunk). No repository-specific helper or separately
provisioned CI secret is required. The MR description selects one
exact comment and independent acceptance identity for each stage. Place a JSON
object between `<!-- opdev-ci-selection:start -->` and
`<!-- opdev-ci-selection:end -->` (without Markdown fences):

```json
{"schema":1,"stages":{"pre_merge":{"revision":"<full source SHA>","note_id":123,"body_sha256":"<whole note SHA-256>","acceptance_sha256":"<reviewed acceptance SHA-256>"}}}
```

The placeholders are not valid selections. Obtain actual identities after posting
the reviewed record. Before merge, use the source head. After merge, add a separate
`post_merge` entry bound to the actual integrated commit and its current-stage
review. Prepare that review from the integrated tree; copying the pre-merge result
does not qualify it. Retry the same failed quality job after the missing selection
is supplied, keeping the failed attempt visible. The native reader never writes reviews,
selects an older passing record or converts a source-head run into post-merge proof.
It refuses fork inputs; extending this credential/trust scope needs separate review.
Tag quality verifies integrated source, not release authorization or delivery readiness.

GitLab jobs use the automatically issued `CI_JOB_TOKEN`, sent as `JOB-TOKEN`.
That short-lived identity takes precedence over developer credentials in CI.
Local explicit-locator checks keep the existing authenticated `glab` fallback;
never export that login into CI. The reader uses supported MR/comment APIs and
the authenticated job endpoint, not general project metadata unavailable to job
tokens. Work excerpts needed by CI must be available through those permissions:
prefer the actual MR's accepted scope; do not silently copy issue text or upgrade
credentials when a required external authority is inaccessible.
Broader remote protection/settings audits remain separate local read-only work.
Missing access fails closed. Keep locators/drafts temporary and outside Git.
The native selector and review checker recheck mutable selections/content.
See [provider integration](../spec/evidence-lifetimes.md#native-ci-review-integration)
for the shared GitHub path and supported context limits.

### Coverage and report lifetime

CI instruments the existing canonical Rust test execution with pinned
`cargo-llvm-cov` and emits LCOV plus a summary without running a second full suite.
Coverage is advisory: no percentage gates and no claim that high coverage proves
assertion quality. The measured scope is instrumented Rust on the Linux quality
runner; it does not measure Python, shell, PowerShell, other native platforms or
live agent effectiveness. Failed checks remain failures, not passing coverage.

Routine check/coverage reports expire after 30 days. GitLab keep-latest retention
is disabled. Keep current review inputs through their active verification and
record useful failure findings/reproductions in the work item; do not archive
every invocation. Published release assets and provenance retain their separate
release lifetime. Remove temporary migration rollback files after verification;
the legacy ledger needs no permanent archive and Git history is not rewritten.

Use conventional commit messages. Keep branches short-lived and integrate only
through CI. Release tags are `vMAJOR.MINOR.PATCH`; their pipeline builds the
artifact once and publishes its SBOM, checksums, release manifest, and provenance
with the same immutable archive.

PowerShell installer tests run with `powershell -NoProfile -File tests/runtime_test.ps1`
on Windows (or `pwsh` on a development machine). Live consumer smoke tests are
separate CI jobs; their network dependencies and limits are documented in
`spec/installation.md`.
