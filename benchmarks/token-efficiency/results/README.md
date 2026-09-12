# Initial baseline — 11 September 2026

[Recorded results](2026-09-11-codex/summary.json) compare nine synthetic questions
with full and selected input, one trial per case/arm. All 18 exact-answer checks
passed. Codex CLI 0.153.4 requested `gpt-6-astra`, medium effort. The balanced
schedule used seed 17, new processes, and a 120-second limit per trial.

| Observation | Full | Selected |
| --- | ---: | ---: |
| Accepted answers | 9/9 | 9/9 |
| Total reported input + output tokens | 145,725 | 133,866 |
| Tokens per accepted answer | 16,191.7 | 14,874.0 |
| Cached input tokens | 46,592 | 46,592 |
| Uncached input tokens | 98,962 | 87,082 |
| Reported cache-write input tokens | 0 | 0 |
| Reported reasoning output tokens | 0 | 19 |
| Median elapsed seconds | 5.11 | 5.83 |

Selected context reduced total tokens by **8.1%** across this sample. It did
**not** demonstrate lower latency. Cache hits occurred despite fresh processes;
the totals happened to match across arms, but cache conditions were not fixed.
No dollar cost or statistically reliable speed improvement is claimed.

The [static prompts](2026-09-11-codex/payloads.json) range from 142–1,880 proxy
tokens in the full arm and 86–131 in the selected arm (`tiktoken 0.12.0`,
`o200k_base`). Host-reported input is much larger, showing why payload savings
cannot be applied directly to whole requests. The selected report fixtures
already expose blocker lists and the selected evidence fixtures omit history;
production implementations must still demonstrate equivalent selection.

A preceding four-run smoke pilot on the unverified-report and stale-evidence
cases also passed, with 7.5% fewer total tokens. It used an earlier harness
revision and is not pooled into this baseline. Raw events remain private local
artifacts. The checked-in [manifest](2026-09-11-codex/manifest.json) and
[records](2026-09-11-codex/records.jsonl) contain prompt/event hashes and reviewed
usage aggregates, not credentials or host transcripts.

A final accounting audit added the host's optional cache-write and reasoning
counters by reprocessing the same hash-verified events. No model calls were
repeated and all outcomes and total-token counts stayed identical.
[Normalization provenance](2026-09-11-codex/normalization.json) distinguishes
the original execution harness from the updated event parser.

**Decision:** proceed toward experimental report/evidence views, retaining the
full-source reference, and validate them with complete task acceptance tests
before changing the default plugin workflow. One trial per case cannot establish
robust quality equivalence, resumed-task behavior, or production savings. No
instruction-consolidation or session-cache design has been validated by this
experiment. Full development-task effectiveness remains **unverified**.

The first canonical workspace test attempt encountered sandbox denial when two
existing HTTP mock tests bound local sockets. The same canonical command passed
with socket access; no product code or test expectation was changed to make it
pass. The optional tokenizer initially was absent, then was installed into a
temporary virtual environment; default harness tests need no external packages.
