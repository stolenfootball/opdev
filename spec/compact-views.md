# Compact report and evidence views

These CLI views reduce exposed context without changing evaluation, evidence
selection, or gate requirements. They are projections, not a second evaluator
or an evidence approval mechanism. Full check artifacts and the original ledger
remain authoritative.

## Check artifacts and summaries

`opdev check --report PATH` writes the complete existing schema-1 JSON report to
a new file. It MUST NOT replace an existing file or symlink. An existing output
path is rejected before running project checks. A race to create the destination
fails rather than overwriting it. Parent directories are not created implicitly.
Prefer a path outside the working tree so the artifact does not become unindexed
input for a later evidence fingerprint. A write failure is an error, never a
successful evaluation claim.

`--format summary` requires `--report PATH`. The requested local or CI checks
execute once, and the full report is saved before printing the compact JSON
view. Existing human and JSON presentations remain available. Check exit status
is unchanged: 0 when the selected development/integration gate passes, 1 when it
blocks, and 2 for a CLI error. Other gates may still block when that selected
gate passes; consumers MUST inspect the returned gate results.

`opdev report summarize PATH` reads a full saved report without locating a
project, running commands, or auditing providers. It returns 0 only when all
recorded gates pass, 1 when any recorded gate blocks, and 2 when the file cannot
be read or validated. Its broader exit criterion is explicit because a saved
schema-1 report does not identify the gate requested by the original command.

Summary schema 1 uses `kind: check_summary` and includes:

- original subject, catalog version, evaluation timestamp, and every gate;
- counts for all six rule outcomes;
- every non-satisfying rule, including its exact ID and outcome;
- every check's ID, kind, blocking flag, affected gates, outcome and duration;
- bounded diagnostic/summary excerpts and output excerpts for non-satisfying
  checks, each with an explicit truncation flag;
- the canonical full-report path and SHA-256 of its exact bytes;
- JSON Pointers locating the original rule and check records.

Excerpts are limited to 512 Unicode scalar values per field. The number of
findings and checks is not capped: a context budget MUST NOT suppress blockers.
Supporting evidence, successful check logs, full diagnostics, and applicability
justifications remain available in the referenced artifact. A consumer needing
those details MUST retrieve them rather than infer them from an abbreviated
view. A file changed after summarization no longer matches its recorded hash.

All summaries identify themselves as saved evaluations whose freshness has not
been revalidated. `stage` is null because the original report schema does not
record it. A consumer MUST NOT invent a stage, repository revision, or current
remote status from the command used to summarize a historical file.

The summarizer accepts only the supported full-report schema and current
embedded catalog version, with the complete catalog-ordered rule results.
Unknown/missing/reordered rules, duplicate IDs within a check kind, and gate
records inconsistent with the source outcomes are errors. Suite and extension
checks may share an ID; their kinds and JSON Pointers disambiguate them.
Validation uses the same gate aggregation as evaluation. It does not authenticate
an artifact or independently verify its asserted facts. Summaries cannot be
loaded as full reports. An incompatible historical report needs its originating
CLI, not guessed migration or silent reevaluation.

## Current evidence

`opdev evidence show --current [--rule ID]` is a read-only query. It requires an
initialized project, validates the complete original ledger, and obtains the
fingerprint through the existing staged-index implementation. Unindexed
material, malformed ledgers, unsupported schemas, duplicate assertions, and
unknown requested rule IDs are errors. `--current` is required to make the
query scope explicit; historical selection is not implemented.

The JSON view has `kind: current_evidence`, schema 1, the root, ledger path,
current fingerprint, and separate `project` and `change` scopes. Only the exact
matching change is included; ledger order does not establish freshness. Rule
filtering applies to both scopes after validation and fingerprint selection.
It never merges change assertions with durable facts or turns them into final
rule results.

An absent ledger is represented by `ledger_present: false`, empty project
assertions and `change: null`. A valid ledger without matching change evidence
also has `change: null`; durable facts remain visible. A requested rule absent
from both selected scopes is named in `missing_requested_rule`. Missing evidence
remains unverified; a successful query exit means the read completed, not that
any gate passed. The existing evaluator alone applies eligible assertions to
otherwise unverified results and cannot override concrete failures or errors.

The query MUST NOT rewrite, prune, approve, or copy historical assertions into a
new fingerprint. Subsequent staged changes invalidate the view. Existing
bootstrap review and explicit ledger maintenance requirements remain intact.

## Decision and compatibility

The repeated context benchmark in work item #24 found lower token use for
selected report/evidence inputs with no clear latency benefit. Its synthetic
preselected inputs did not validate a production selector. This increment
therefore adds deterministic projection tests plus process-level acceptance:
a real check captures a failure, saves full diagnostics, and can be inspected
again without rerunning commands. Evidence tests cover old, missing, invalid,
and newly staged states, scope preservation, and rule filtering.

An optional CLI presentation and offline artifact reader preserve existing
consumers and allow rollback simply by choosing human/full JSON output. Changing
the full-report schema or replacing the ledger would increase migration cost.
Deleting historical evidence or omitting non-satisfying findings would weaken
reviewability. Keeping input scopes separate avoids implying that project
assertions override evaluator failures.

The plugin still supports older compatible runtimes. Its on-demand references
probe command help and use the existing workflow when the capability is absent;
command execution errors on a capable runtime are not fallback signals. No
runtime pin or compatibility requirement changes merely to enable a smaller
view. Revisit the projection design if real tasks require enough fallback reads
to erase the savings, or if a future report schema can preserve explicit stage
and revision identity. Full development-task effectiveness remains unverified.
