# First coding-session comparison

The 30-session experiment completed on 2026-09-15 with full usage accounting,
no measured retries and no infrastructure/accounting errors. Compact guidance
used less total context, but the frozen acceptance rule rejected one compact
trial. **Acceptance-equivalent savings remain unverified.**

Work: [issue 26](https://gitlab.com/stolenfootball-tools/opdev/-/issues/26).
Method and reproduction: [experiment protocol](../../README.md).

## Observed results

| Metric | Baseline | Compact |
| --- | ---: | ---: |
| Completed sessions | 15 | 15 |
| Accepted by frozen grader | 15 | 14 |
| Total input + output tokens, including failures | 3,875,227 | 3,012,945 |
| Tokens per accepted task, including failed attempts | 258,348 | 215,210 |
| Cached input tokens (subset of input) | 3,336,704 | 2,482,432 |
| Uncached input tokens | 508,302 | 501,986 |
| Output tokens | 30,221 | 28,527 |
| Median agent seconds | 98.1 | 89.6 |
| Slowest agent seconds | 140.5 | 141.0 |
| Successful candidate GitLab pipelines | 15 | 15 |
| False gate claims | 0 | 0 |

Total measured usage was 6,888,172 tokens. The descriptive total-token difference
is 22.25%; the tokens-per-accepted-task difference is 16.70%. These are not
acceptance-equivalent savings claims: `token_reduction_fraction` remains null
in the machine-readable summary. Uncached input decreased only 1.24%, so total
token reduction must not be interpreted as a comparable billing reduction.
Provider cache state was uncontrolled; no dollar savings are inferred.

| Scenario | Accepted baseline / compact | Tokens per accepted baseline / compact |
| --- | ---: | ---: |
| Express API, receipt and CLI | 5 / 5 | 250,906 / 211,695 |
| Started-kilogram regression | 5 / 4 | 259,765 / 269,226 |
| Documentation and stale evidence | 5 / 5 | 264,375 / 175,514 |

Compact used fewer total tokens in 14 of the 15 matched case/repetition pairs.
The rounding case nevertheless used more tokens per accepted task under the
frozen rule. Pair identities are task/repetition labels, not matched model
random seeds, and do not establish statistical significance.

## The rejected trial

`rounding-compact-4`, commit `c92dbe5faf672c34b0862369231dcb79dbcf6882`,
failed only `existing_tests_preserved`. The grader compares complete function
ASTs. The candidate retained every original invalid-input value and assertion,
but added `False`, `1000.0`, `"1000"` and `None` to an existing test's input list.
Manual diff review found a monotonic coverage extension, not weakened tests.

Behavior, canonical tests, new-test mutation protection, scope, staging,
gate/evidence honesty and CI all passed. Its 227,218 tokens and failed verdict
remain in the original results. We did not alter the grader or rerun the trial
after observing the failure. [Issue 28](https://gitlab.com/stolenfootball-tools/opdev/-/issues/28)
tracks aligning the preservation oracle and prompt in a separately identified
future experiment. An offline test records this strict grading limitation.

Every trial accurately reported all four fixture gates as blocked. This was
expected: the fixture deliberately lacks current reviewed change evidence and
delivery qualification. Passing benchmark acceptance or candidate CI does not
mean the candidate was qualified for delivery.

## Audit and identities

- Frozen harness commit: `2c02d38dafeda41861911ce26cdd6b982fdd8068`.
- OpDev source revision: `824535f97af2f75c653f89aaad6fddad2a98939b`; CLI 0.1.2.
- Fixture seed: `642a7547551c8e33a559a43d4db6fcd42960d074`.
- Codex CLI 0.154.0; requested `gpt-6-astra`, medium effort; Windows x86-64,
  Python 3.13.0; one fresh ephemeral process per session; seed 17.
- The same executable, fixture, canonical tests and enforcement were used in
  both arms. Only inspection guidance differed. Baseline agents could select
  fields from ordinary reports/ledgers; compact agents could retrieve full data.
- Export verified both executable hashes, raw event hashes, terminal token
  counters, schedule identity, clean candidate checkouts, commit identities and
  evidence fingerprints. GitLab was re-queried to verify all 30 candidate
  pipeline IDs, commit SHAs and successful statuses.
- Manual review checked documentation preservation and code/test diffs. A trace
  scan found no observed sibling/controller access or delegation. Command/read
  counts in the JSON are lexical proxies, not exact information-retrieval or
  repair-round measurements.

[manifest.json](manifest.json), [schedule.json](schedule.json),
[records.json](records.json) and [summary.json](summary.json) retain sanitized
measurements and source/CI identities. Raw transcripts, stderr and local
candidate checkouts remain private, ignored local audit artifacts. The private
GitLab canary is disposable; its links may cease resolving after cleanup.

## Pilots and limitations

Setup probes and pilots are excluded, not erased. They exposed an outdated
Codex executable, missing Windows sandbox configuration, protected Git metadata,
an initially unsupported web-search event, and a single-arm summary bug. The
measured harness was frozen only after those were addressed and tested. Both
arms used the same constrained controller-staging protocol; no sandbox bypass
was used. No pilot token usage is pooled into the table above.

This is one small synthetic Python repository, one requested model/host/OS,
and explicitly activated repository instructions. It covers investigation,
edits, tests, local validation, staging and honest final reports, followed by
controller-owned GitLab CI. It does not evaluate installed-plugin discovery,
Claude Code, large projects, model-driven CI repair, resumed/compacted sessions,
human interruptions or worker accounting. Agent time excludes controller CI
time and is descriptive of a shared workstation, not a controlled speed claim.

The OpDev harness MR also encountered an unrelated Windows installer cleanup
flake: job 16516389618 failed removing locked `cosign.exe`; the one visible retry,
16516431965, passed. [Issue 27](https://gitlab.com/stolenfootball-tools/opdev/-/issues/27)
remains open. This was not a measured-session retry or a runtime fix.

Next: resolve issue 28 before a new balanced acceptance-equivalence experiment;
then extend coverage to another stack and Claude Code. General development
effectiveness remains `unverified`; this work changes no production defaults.
