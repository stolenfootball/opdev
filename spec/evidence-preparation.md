# Preparing an existing acceptance ledger

EP-01: `opdev evidence prepare --input ACCEPTANCE.yaml --work AUTHORITY` MUST
calculate staged reference hashes and the current fingerprint, leaving every
mapping and review unverified. Input is the existing schema-2 acceptance shape;
omit `sha256`, mapping `outcome` and `review`. Provide scope/rationale, condition
IDs/statements/authorities with source paths/excerpts, and explicit verification
mappings with assertion, discriminating case, target path/excerpt and suite or
automation limitation. The command cannot infer omitted requirements or adequacy.

Output is a temporary schema-1 draft containing the fingerprint, ledger byte
digest, work authority and acceptance payload. Save outside the worktree or in an
already ignored working directory. No new tracked authority is needed. No suites,
extensions, network access or ledger writes occur when preparing a draft.

EP-02: `--draft DRAFT.yaml` previews the candidate ledger and current acceptance
subject digest. Review each mapping and set its actual outcome; review completeness,
scope and assertions, then record the actual reviewer/reference/rationale/outcome
and preview's subject digest. Changed mappings require another review/digest.
The CLI supplies the hash, not consent or a semantic verdict. Failed/unverified
mappings remain unsatisfied; only explicitly reviewed facts should be asserted.

EP-03: `--draft DRAFT.yaml --write` MUST reject an unresolved/stale review, changed
ledger, changed staged state, bad reference or absent excerpt before writing.
It atomically replaces the single ledger file; do not run concurrent writers.
The staleness guard is not an authenticated approval or hostile-writer lock.
Atomic replacement may fail across filesystems (for example an external Git
directory); report the error without a non-atomic fallback.

EP-04: Preserve historical entries, durable assertions and unrelated current
assertions. New fingerprints append an entry with no inferred rule assertions;
an existing exact-current entry updates only its acceptance, and its work must
agree. Schema-1 migration remains separately reviewed. Serialization may normalize
YAML formatting/comments; semantic history is preserved. Invalid drafts never
rewrite the ledger. Normal checks still require current canonical execution;
neither a preview nor successful application qualifies a gate.

Manual schema-2 edits remain supported. New ledgers use `evidence bootstrap`, not
this existing-ledger helper. The temporary draft format reuses acceptance's
existing schema and validation rather than introducing another project policy.
