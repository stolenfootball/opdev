# Optional test-strength example

This is a project-owned reference integration, not a new OpDev requirement or a
producer parser inside the CLI. It asks whether tests detect a selected behavioral
change. Use it when the insight justifies extra build/test time. Normal projects
and checks do not install or run it automatically.

## Opt in

Review the adapter before copying `cargo_mutants.py` into an existing appropriate
project tooling location. It needs Python 3 (standard library only), Git, native
Cargo builds, and **cargo-mutants 27.1.0**. Choose your own reviewed tool-installation
method; neither the adapter nor OpDev installs it. For example, an explicit
`cargo install cargo-mutants --version 27.1.0 --locked` installs that version.
Dependencies for the project under test must already be available: builds use
`--locked --offline`. The example requires a clean committed Git root and an
ignored `target/opdev-test-strength` directory. Review/commit inputs first; it
does not stage or commit your project.

Merge an entry like this into the existing project contract; preserve everything
else. Replace the script path, executable name (`python` on Windows, often
`python3` elsewhere), source filter and mutation regex for your reviewed scope.
List candidates with the selected producer before choosing a regex: an empty
selection is not evidence. The example below matches the included fixture only.

```yaml
commands:
  strength:
    argv: [python3, tools/cargo_mutants.py, --file, src/lib.rs,
           --mutant, 'replace < with <= in allowed', --max-mutants, '1',
           --test-timeout, '10', --build-timeout, '30']
    timeout_seconds: 120
extensions:
  checks:
    - id: boundary-strength
      command: strength
      stage: pre_merge
      blocking: false
```

This is advisory: findings are still `failed`, but they do not add gate blockers.
Changing `blocking` to `true` is a developer decision, not an automatic follow-up.
Keep the rationale in the existing testing authority; optionally reference its
existing authority key with the extension's `authority` field. No new policy file
or adoption checklist is needed.

`opdev check --ci --report NEW_REPORT_PATH` runs the `pre_merge` check; a `verify`
check runs with ordinary `opdev check`. Invoke the selected check again against
integrated source in CI. The CLI does not expose separate `post_merge` or
`evaluate` selectors. Advisory checks do not grant permission to ignore a known
product defect or another required rule.

## Meaning and limits

The selected example policy is **no missed viable mutations within this narrow
selection**, not a universal quality percentage. Its result never replaces
canonical tests, acceptance review or core rules.

| Observation | Result |
| --- | --- |
| Complete selected analysis, passing baseline, actual tests catch viable mutations | `passed` |
| Conclusive viable mutation survives those tests | `failed` test-strength requirement |
| Empty/over-budget selection, failed baseline, incomplete inventory, no viable cases, changed source, or inconclusive mutation timeouts | `unverified` |
| Unavailable tool, unsupported version/report, malformed or contradictory output, or producer crash | `error` |
| Outer OpDev deadline expires | `error` from OpDev |

The wrapper exits zero when it produces a valid extension verdict, including
`failed` or `error`. An unexpected wrapper-process failure is a protocol error.
This distinction matters because the producer uses nonzero exits for findings
as well as failures. [Producer exit codes](https://mutants.rs/exit-codes.html).

Each invocation gets a new directory containing selection, logs and producer
reports. There is no saved-report import or result reuse. The OpDev result records
source commit, invocation, pinned version, elapsed time and a SHA-256 of the parsed
JSON representation; it is not an authenticated attestation or a byte digest of
the original JSON file. Git observations do not identify ignored dependencies,
environment changes, or source changes made and restored between observations.
Keep those inputs controlled in CI. Retain the referenced directory alongside
the OpDev report as an access-controlled CI artifact; a path alone does not upload
evidence. Review reports/logs for private code, paths and secrets before sharing.
No automatic artifact upload or cleanup is performed.

The adapter deliberately runs default Cargo features with `--no-config`, normal
baseline tests, one mutation worker, one jobserver task, explicit per-build/test
timeouts and the enclosing OpDev deadline. It does not inherit `.cargo/mutants.toml`,
skip baseline tests, retry, iterate over cached successes or mutate in place.
Projects needing another test runner, features, cross-compilation or a different
policy must select another integration or deliberately adapt and test this example.
No claim is made about untested feature/platform configurations.

Run through OpDev: the adapter relies on its enclosing process-tree timeout.
The mutation-count check bounds executed candidates; discovery still costs time.
An 8 MiB parser limit and bounded retained OpDev output protect report processing,
not total producer log storage. Time/output/concurrency limits are not OS resource
quotas; CI must supply appropriate memory/disk limits. Equivalent mutations and
flaky baselines require judgment; a survivor is not automatically a real defect,
and a pass cannot prove all faults would be detected. Broader selections may be
much more expensive than this fixture. [Producer prerequisites](https://mutants.rs/getting-started.html),
[timeouts](https://mutants.rs/timeouts.html), [format stability](https://mutants.rs/stability.html).

## Reproduce the useful finding

The fixture's contract rejects counts of ten or more. Initial tests cover nine
and eleven but miss ten. The selected mutation changes `<` to `<=`, escaping those
tests. The canary adds `assert!(!allowed(10))`, then proves that the same mutation
is caught. Ordinary tests pass before and after the improved assertion.

After explicitly provisioning the pinned producer and building OpDev, run:

```sh
python3 examples/test-strength/canary.py --opdev target/debug/opdev \
  --producer /absolute/path/to/cargo-mutants --output target/strength-canary-new
```

On Windows use `python`, `.exe` paths, and your shell's line-continuation syntax
or one line. The output directory must not exist. This creates a local-only test
repository, runs through actual `opdev check`, and retains reports beneath its
`target`. It never creates remote repositories, changes the real project contract,
or claims complete adoption: unrelated core gates intentionally stay blocked.
The offline tests in the canonical Cargo suite need no cargo-mutants installation.

The 2026-09-21 Windows x86_64 pilot found the expected weak-test finding and
strong-test pass in approximately three seconds per full check. This is one tiny
dependency-free fixture, not a performance guarantee. Initial visible attempts
exposed Windows verbatim root handling, an empty mutation selector, and the
producer's selection-only `diff` field; these were corrected before a successful
rerun. Expansion to another tool should be driven by useful findings, feedback
cost and actual consumer demand, not a desire to accumulate adapters.
