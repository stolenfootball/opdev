# Resumable workflow references

Workflow journal schema 1 (`workflow.v1`) is an optional reference protocol, not
a replacement tracker, an approval database or a qualification cache. Keep actual
requirements, policies, decisions and evidence in their existing authorities.
One journal can refer to those sources; do not create one document per event or
copy private conversation history into a public repository. Necessary OpDev-owned
durable metadata may use `.opdev/`; existing authority ownership takes precedence.
No journal, directory or index is created during normal checks or inspection.

## Separate meanings

Records distinguish acceptance review, execution, artifact qualification, policy
decision, implementation approval, execution permission, user feedback and release
authorization. Each names an exact subject, scope, original reference, attributed
actor, observed outcome and retained supporting content. Source/configuration,
stage and optional immutable artifact identities are independent of the event ID.
Subjects have their own schema 1; unknown critical fields/versions are errors.

Human attribution is a claim, not authentication. The CLI rejects an explicitly
agent-attributed human decision, but cannot prove the truth of a caller setting
`human_attributed: true`. Agents must consult the original authorized source and
scope. Matching evidence bytes cannot create consent. Host permission is checked
by the host at execution time; a journal cannot grant it. Permission to run work
is not product feedback or permission to publish.

Every old attempt remains in history. Supersession must identify an earlier
active record of the same kind and scope; it cannot relabel permission as feedback.
Revocation retains the target and its original evidence. Multiple active records
for one kind/scope remain conflicting rather than selecting a favorable result.

## Inspection

`opdev workflow subject --root PROJECT --stage STAGE [--artifact PATH]` generates
mechanical subject fields using the existing staged fingerprint, parsed contract
and optional streamed artifact digest. It performs explicit read-only Git queries,
not project test execution, and writes only stdout. Dirty/unindexed source remains
an error; it does not stage or repair anything. This helper is separate from the
pure inspection command, so inspection never starts hidden processes. A subject
observation is not a promise that the filesystem cannot change afterward.

`opdev workflow inspect --journal PATH --subject PATH --root EVIDENCE_ROOT
[--need KIND] [--json]` reconstructs the view without executing commands, starting
network requests, writing a cache or repairing files. The subject input is an
explicit current observation supplied by the caller, not a newly authenticated
source observation. Obtain it using the existing source/configuration evidence
path; do not relabel an old snapshot as current. Inspection itself deliberately
does not discover Git state or invoke a check. It states that limitation in output.

The view distinguishes recorded, stale, expired, retired, conflicting, unresolved
and not-satisfied activity. These are not new core rule outcomes. `recorded` means
only that supporting file bytes match the supplied subject and declared lifetime.
It is not permission to skip a required stage, a valid execution receipt, an
authenticated human decision or a passed integration gate. The existing evaluator
and same-run execution validator still own qualification. Read the original facts
before reusing completed investigation, verification or a settled decision.

Source/configuration/stage/artifact changes invalidate the conservative full
subject. Missing/changed evidence and expired/future observations stay visible.
Artifact qualification must reference the exact artifact bytes as well as the
original qualification evidence. Referenced files are relative to the explicit
evidence root; traversal, links and unsupported paths do not match. Remote-only
evidence cannot be fetched implicitly: retain a permitted, reviewable observation
or inspect the original source separately. Never copy secrets just to fill a field.

`--need` selects kinds required by the requested outcome. It does not invent new
project requirements, infer release intent or collapse distinct scopes into one
approval. Exit 0 means no selected structural/evidence gaps were observed; exit 1
means gaps remain; malformed/unsupported inputs exit 2. JSON always retains
`qualification: unverified`. Ordinary fresh workflow remains available when the
projection lacks sufficient evidence; do not restart adoption.

## Explicit writes and recovery

`opdev workflow append --journal PATH --event PATH --expected SHA256 --work REF`
adds one event under the expected prior journal digest. Use `--expected absent`
only for an explicitly selected new journal. It never changes the work authority,
updates old events, fabricates a decision or closes a task. All events are checked
before replacement; unsupported versions and stale writers preserve the old file.

The writer uses an OS file lock shared by cooperating writers, rereads under the
lock, checks the expected digest, writes/syncs a temporary file and atomically
replaces the journal. A small sibling lock file contains no workflow facts and
is not a lock-by-presence marker; the OS releases the lock when the process exits.
Do not delete it while writers may be active. Interrupted temporary writes are
not heads and are never adopted automatically. No stale lock takeover is needed.
This protects cooperating writers, not malicious replacement of directories or
non-cooperating edits. Atomic replacement is not a claim of power-loss durability
on every filesystem. Back up the journal and original evidence with their owner.

There is no disposable index in this first version. Reconstruction reads the
journal and original references directly; losing a derived consumer cache must
not lose decisions. Loss of the journal/evidence itself is loss of authoritative
metadata, not permission to invent missing history. Preview any adoption of these
records and preserve existing authorities, branch names, tooling and policies.

Concurrency implementation uses Rust's [native file locks](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)
and the existing [atomic temporary-file persistence](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html#method.persist).
No database, daemon, scheduler or new locking dependency is required.
