# Inspect check results

Keep the full evaluation available while loading only the results needed for the
task. On first use, inspect the selected CLI's `check --help`. If it lists
`--report`, prefer:

```sh
opdev check --report <new-file-outside-worktree.json> --format summary
```

Add `--ci` or `--remote` only when that evaluation is appropriate. This runs the
checks once, saves full JSON without replacing a file, and prints compact JSON.
The exit status still follows the requested check gate; inspect all returned
gates before making broader claims. Choose a fresh report path for each run.

For an existing full report, use `opdev report summarize <file.json>` instead of
rerunning checks. Exit 1 means a recorded gate is blocked; exit 2 is a parsing,
validation, or filesystem error. This reads historical results, not current
repository or CI state. Schema-1 reports do not record the evaluation stage;
`stage: null` must not be guessed from the command used to summarize them.

Findings include every non-satisfying rule, and all checks retain their outcomes
and blocking flags. Excerpts have explicit truncation flags. Retrieve omitted
facts or diagnostics from the hashed full artifact using `source_pointer` (a
JSON Pointer), especially when an excerpt is insufficient to diagnose a failure.

Older compatible CLIs lack these commands. Use their existing human check output
or save full JSON and inspect the needed fields. Do not install or replace a
runtime just to obtain a smaller view. Once a capability is supported, report
its execution errors rather than treating them as an old-version fallback.
