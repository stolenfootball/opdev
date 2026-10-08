# Resumable workflow references

Workflow journal schema 1 (`workflow.v1`) is an optional reference protocol, not
a replacement tracker, an approval database or a qualification cache. Keep actual
requirements, policies, decisions and evidence in their existing authorities.
One journal can refer to those sources; do not create one document per event or
copy private conversation history into a public repository. Necessary OpDev-owned
durable metadata may use `.opdev/`; existing authority ownership takes precedence.
No journal, directory or index is created during default checks or inspection.

## CLI-owned local continuation

Capability `state.local.v1` provides one resolver for both agent integrations.
`opdev state resolve --root PROJECT` observes Git metadata and prints locations
without creating them, invoking project commands, contacting a provider or moving
an installation. These locations are local aids, not project policy:

| Platform | Essential state | Reconstructible cache |
| --- | --- | --- |
| Windows | `%LOCALAPPDATA%/opdev/state` | `%LOCALAPPDATA%/opdev/cache` |
| macOS | `~/Library/Application Support/opdev/state` | `~/Library/Caches/opdev` |
| Other Unix | `$XDG_STATE_HOME/opdev` or `~/.local/state/opdev` | `$XDG_CACHE_HOME/opdev` or `~/.cache/opdev` |

Relative XDG values are ignored. An explicit `OPDEV_STATE_DIR` must be absolute;
it changes only the state location. Source, Git metadata, essential state and cache
must not overlap. Existing runtime resolution and `OPDEV_DATA_DIR` remain unchanged.
There is no automatic relocation, cleanup, daemon, database, or account.

Operating-system application virtualization can redirect an otherwise identical
logical user-data path into host-specific storage. The resolver reports observed
locations; using the same CLI code does not prove physical sharing across hosts.
Preserve existing records and inspect each host's resolved location. A shared
`OPDEV_STATE_DIR` requires a permitted location deliberately selected for those
hosts; do not disable isolation, silently import another host's history or mistake
missing local state for missing original tracker decisions.

The resolver derives repository/worktree keys from canonical common-Git and
worktree-Git directories plus physical metadata (Windows creation time; Unix
device/inode and creation time when available). Linked worktrees share a repository
key but not a worktree key; separate clones do not share histories. Branch switches
retain the local worktree key, while changed source makes its context stale. Moves,
filesystem restore or directory recreation can change or recycle physical identity:
these keys are lookup aids, not global identity, authentication or proof of freshness.
Never import another clone's decisions or qualify execution using a local key.

Each worktree resolves `context.json`, `context-history/`, `drafts/` and `runs/`
under `state/repositories/<repository-id>/worktrees/<worktree-id>/`. Only requested
data is created; resolving paths does not scaffold these directories. Newly created
Unix directories use mode 0700; Windows uses the user's inherited ACL. Existing
permissions are not rewritten. Reject links/reparse points and traversal in state
paths; checks protect cooperative use, not hostile concurrent directory replacement.

`state context --input FILE --expected SHA256|absent` stores a schema-1 derived
pointer with only `subject` (the existing workflow subject), original `work` and
bounded `references`. It has no decision, approval, policy, transcript or gate
fields. The supplied source/configuration subject must be current. Cooperative
locking and an expected-byte digest prevent stale updates; old pointer bytes are
retained by digest before atomic replacement. An unsupported existing schema or
unknown field is not silently repaired. Interrupted temporary writes are not heads.

`state inspect` reads this pointer and current source, returning `missing`, `stale`
or `references_only`, always with `qualification: unverified`. This avoids historical
log scanning but does not fetch or authenticate references. Read original sources
and scope; remote revocation, conflict and expiry cannot be inferred from a URL or
unchanged pointer. Where retained workflow events are already used, the existing
`workflow inspect` projection detects their revocations/conflicts and content drift.
Do not create a second journal merely to populate context.

`evidence prepare --input FILE --work REF --retain-draft` saves the existing
mechanically prepared draft beneath a new local draft directory and emits its path.
All mapping/review decisions still start unverified, including caller-supplied
passed claims. Preparation does not approve, execute checks or change the ledger.
Use the same reviewed draft application path afterward; stored drafts are not proof.

`check --retain-state` executes the ordinary requested check once and records a
separate attempt directory. Immutable `start.json` precedes execution and records
schema 1, the staged source/configuration/stage subject, command-plan hashes, CLI
version/bytes, local clock and OS/architecture. It does not dump environment variables
or claim a complete environment fingerprint. This optional retention requires
stageable source identity; it does not silently stage files. Default checks retain
their existing editing behavior.

The full diagnostic `report.json` and separate schema-1 `completion.json` are
written without replacing prior attempts. Completion binds report bytes and the
post-check subject. Failed checks retain their actual report. An error or killed
process can leave only the start or an uncompleted report: absence of completion
means unfinished/unknown, not passed or a manufactured failed test. Clock accuracy,
ignored input changes and environment drift remain limitations. A storage error
is an error, never permission to claim a retained result.

`state attempt <id>` reads one attempt, rejects unsupported records, unsafe names
and changed/missing completed-report bytes, and distinguishes a completed observation
from unfinished/interrupted execution. No prior attempt is selected as a fallback.
Saved reports remain diagnostics: ordinary check never reads them as qualification.
There is no cross-revision execution reuse, automatic upload or release authority.

Unexported reports, draft work and context history are essential local state until
their owner deliberately retains/discards them; they are not cache. There is no
cleanup command that could erase sole evidence or an active attempt/runtime.
Private reports may contain project output: inspect before sharing. Loss of this
state is not loss of the original tracker decisions and does not authorize inventing
missing evidence. Portable retained evidence has its own trust/retention boundary.

Design choice: extend canonical checks and existing reference protocols rather
than introduce a second execution wrapper or approval database. Revisit identity
or storage mechanisms if supported filesystems cannot maintain isolated safe
continuation; do not hide that limitation with automatic history import. Platform
roles follow [XDG](https://specifications.freedesktop.org/basedir/latest/) and
[Apple's filesystem guidance](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html).

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
[--need KIND] [--acceptance-sha256 DIGEST] [--json]` reconstructs the view without executing commands, starting
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
Acceptance-review records additionally name `acceptance_sha256`, the complete
reviewed inventory. Missing or changed current inventory leaves that review stale
even when product source and execution are unchanged. Other record kinds do not
become stale merely from that inventory edit; this does not qualify their claims.
Obtain the current digest from the existing acceptance evidence operation, never
by hashing only the conditions a worker happened to inspect.
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
Normal success and error exits explicitly unlock before dropping the file; a
guard also attempts release during unwinding. This prevents a duplicated handle
from retaining a completed writer's lock merely because another handle remains
open. Real contention still fails immediately: there is no waiting loop or hidden
acquisition retry. A release error reports that a write may already have committed;
inspect the current head rather than assuming failure means nothing changed.
Process death still relies on OS handle lifetime, not guard execution. This
hardening addresses a documented handle-lifetime risk, not proof of the cause of
every intermittent contention report.

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
