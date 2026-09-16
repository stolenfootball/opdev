# Inspect check results

Use ordinary `opdev check` human output or `--format json` by default. Full report
persistence (`--report PATH`) is stable on CLIs that support it. Do not select
compact output merely because the runtime supports it.

Only when the user or an explicit project decision opts into the compact-context
experiment, inspect `opdev --help` for `--experimental-compact`. If absent, keep
the stable workflow; do not use older ungated compact commands. When present:

```sh
opdev --experimental-compact check --report <new-file-outside-worktree.json> --format summary
```

Add `--ci` or `--remote` only when that evaluation is appropriate. This runs the
checks once, saves full JSON without replacing a file, and prints compact JSON.
The exit status still follows the requested check gate; inspect all returned
gates before making broader claims. Choose a fresh report path for each run.

For an existing full report, use `opdev --experimental-compact report summarize <file.json>` instead of
rerunning checks. Exit 1 means a recorded gate is blocked; exit 2 is a parsing,
validation, or filesystem error. This reads historical results, not current
repository or CI state. Schema-1 reports do not record the evaluation stage;
`stage: null` must not be guessed from the command used to summarize them.

Findings include every non-satisfying rule, and all checks retain their outcomes
and blocking flags. Excerpts have explicit truncation flags. Retrieve omitted
facts or diagnostics from the hashed full artifact using `source_pointer` (a
JSON Pointer), especially when an excerpt is insufficient to diagnose a failure.

Omitting the flag restores the stable workflow; it is not persisted. Token savings
and quality equivalence are not established for general development sessions.

Older compatible CLIs lack these commands. Use their existing human check output
or save full JSON and inspect the needed fields. Do not install or replace a
runtime just to obtain a smaller view. Once a capability is supported, report
its execution errors rather than treating them as an old-version fallback.
