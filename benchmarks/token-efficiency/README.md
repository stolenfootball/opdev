# Token-efficiency benchmark

This benchmark starts the evaluation described in
[the specification](../../spec/token-efficiency.md). It compares full and
selected context on nine synthetic interpretation cases. It does **not** load
the complete OpDev workflow or prove end-to-end savings.

See the [initial baseline and limitations](results/README.md) for the first
18 live trials, measured token reduction, and inconclusive latency results.

Run offline measurements with Python 3:

```sh
python3 scripts/opdev_benchmark.py payloads > /tmp/opdev-payloads.json
python3 tests/benchmark_test.py
```

For optional proxy token counts, install the pinned dependency in an isolated
virtual environment and select an encoding:

```sh
python3 -m venv /tmp/opdev-bench-venv
/tmp/opdev-bench-venv/bin/pip install -r benchmarks/token-efficiency/requirements.txt
/tmp/opdev-bench-venv/bin/python scripts/opdev_benchmark.py payloads --tokenizer o200k_base > /tmp/opdev-token-payloads.json
```

These counts include the shared question and instructions in each prompt.
They exclude host instructions, discovery metadata, tool schemas, and message
framing. They are not exact model tokenizer counts or billing measurements.

A bounded live Codex pilot, using a model and effort supported by your account:

```sh
python3 scripts/opdev_benchmark.py run-codex --model YOUR_MODEL --effort YOUR_EFFORT --case report-unverified --case evidence-stale --repeats 1 --max-runs 4 --timeout 120 --output /tmp/opdev-pilot
python3 scripts/opdev_benchmark.py summarize /tmp/opdev-pilot/records.jsonl
```

The output directory must not exist. Omitting `--case` selects all nine cases;
`--max-runs` must then allow at least 18 trials per repeat. The harness never
silently truncates a balanced schedule to fit the limit. `--seed` defaults to
17. No retry occurs automatically. A new pilot needs a new output directory.
Limits cap trial count and elapsed time per trial; they are not a dollar cap.

The adapter uses `codex exec --json`, a read-only sandbox, disabled approval
requests, ephemeral sessions, an empty temporary working directory, and
`--ignore-user-config`. It reuses existing authentication without reading or
copying credentials. The requested model and effort are explicit. Host global
instructions/discovery and provider alias resolution may still vary; runs on
different machines or dates need environment review, not blind aggregation.
This is controlled context interpretation, not installed-plugin benchmarking.
Tools/delegation are prohibited by the task and invalidate the result if seen.
Raw events are retained so this constraint can be audited.

The adapter follows the documented [Codex JSON event stream](https://learn.chatgpt.com/docs/non-interactive-mode).
It requires the completed-turn input, cached-input, and output counters. Unknown
usage is not a pass. Multiple turns are rejected rather than guessing whether
host counters are cumulative. New adapters need captured, reviewed fixtures and
accounting tests before comparison. Fresh processes may still share provider
prompt caches; inspect recorded cache counters.

`manifest.json` identifies the experiment, `records.jsonl` retains every attempt,
and `summary.json` reports acceptance and paired comparisons. Raw `.jsonl` and
`.stderr` files are private until reviewed. Do not commit raw host traces,
credentials, personal prompts, or production evidence. Keep results outside the
repository; publish only reviewed aggregates with provenance and limitations.
