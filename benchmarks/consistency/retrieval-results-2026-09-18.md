# Bounded checklist/retrieval pass, 2026-09-18

Disposition: keep #46 open and the candidate unmerged. The final draft checklist
did not establish sufficient semantic reliability in this pass. No additional
prompt tuning or retry-until-green was performed. This is a separate follow-up to
[earlier trials](results-2026-09-18.md), not a replacement for those results.

## Candidate and method

The maintainer approved one bounded improvement-and-validation pass on new
examples, including fresh-context authority retrieval. The skill-creation guidance
kept the change to a final draft checklist (scope, revision identity, observed vs.
inferred claims, and actual requirements), replacing the prior heading-specific
paragraph rather than adding a fixture-specific implementation rule.

The candidate reference SHA-256 is
`00fd21824afd9961fc7310badf39dd8c0a49c5732042f32a13522dbb9bd9851f`;
shared skill is unchanged at
`0ff20c682482f911e792b8e79d22dd6e9cc55855749789f7226209caeec67ee1`.
It was frozen before all six runs. Actors received candidate guidance and a user
request, not file contents, expected answers, or the reviewer rubric. Each got a
new copy of `retrieval-fixture/`; the missing case omitted the routed contract
file. The reviewer-only protocol is [retrieval-review.md](retrieval-review.md).

Same host versions as the prior pass: Codex CLI 0.154.0 (model not reported in its
stream) and Claude Code 2.1.274 (stream reports `claude-opus-5[1m]`). Codex used an
ephemeral read-only sandbox with approval policy never and ignored user config.
Claude used restricted mode with only Read/Glob/Grep offered, no settings sources,
hooks, MCP configuration, slash commands, or session persistence; prompts requiring
approval were denied. Normal host authentication was retained. The controller
authorized only local discovery/reads within each synthetic workspace, no source
execution, service calls, or writes. No private consumer data was used.

Artifacts are in ignored `target/consistency-trials/<host>-retrieval-<case>/`:
request (prompt, argv, candidate and all initial file hashes), events, stderr and
result (final hashes). All six hosts exited zero within 180 seconds; all fixture
bytes/file inventories were unchanged. Exit zero and no writes do not establish
semantic success. The task-local controller is not shipped as product tooling.

## Outcomes

| Case | Codex | Claude |
| --- | --- | --- |
| Full W2 increment | Unverified: contract read blocked by execution policy | Retrieval passed; semantic accuracy failed |
| Published documentation only | Unverified: contract read blocked by execution policy | Scope/release distinction passed; unsupported extra finding failed |
| Missing contract authority | Unverified: contract read blocked before missing-file case could be exercised | Missing-source recognition passed; semantic precision failed; attempted unavailable Bash tool |

Codex's JSON streams contain no tool-event records, but stderr independently
records execution-policy rejection of `Get-Content .opdev/project.yaml` (two
attempts in full, one in each other case). The responses correctly withheld
findings and reported unverified inspection. The initial review of JSON alone
could not corroborate the claimed denial; subsequent stderr inspection did.
No inference is made about Codex's review quality from these blocked reads, and
no permission changes or reruns were made to bypass the denial.

Claude full trace reads the contract first, then all six authority files. It
finds the clamp, pluralization, prohibited callback, inadequate assertion, stale
execution and false release claims; it respects the superseding owner decision
and deferred features. However:

- The response labels static behavior "observed" and says callback side effects
  fire on every render, although no callback execution or side effects were seen.
- It predicts returned strings without qualifying that `prepare()` must return
  normally at those claims. A separate limits section does not correct them.
- It infers that source compatibility guarantees an omitted argument, which the
  supplied acceptance text did not establish as a separate requirement.

Claude narrow reads contract, decisions, manual and release record, not code or
tests. It correctly limits the example's status to agreement with requirements,
not proven release behavior, and identifies false shipped CSV/dashboard claims.
But it also calls "planned for a future release" a wording defect on the basis
that deferred work is not committed. No accepted rule forbids describing future
proposal items as planned; this is an optional wording preference, not a supported
currency defect. This remaining false positive prevents an overall pass.

Claude missing detects the absent contract, retrieves remaining authorities, and
does not reconstruct S2/S3 from their identifiers. It usefully recovers the clamp
decision from the owner's words and retains missing execution/release limits.
It nevertheless says either W2 or the release manual must be wrong because their
outputs differ, when different revisions need not have identical requirements.
It describes unreleased W2 as shipping withdrawn behavior and generalizes normal
callback return from callability. These are unsupported conclusions. It also
attempted a local directory listing with unavailable Bash; the tool returned
"No such tool available" and no shell command executed. Its summary should have
reported the failed attempt, not merely said no commands ran. Read-only file
access stayed within the fixture; no mutation was observed.

## Evidence identity

Event-stream SHA-256 (stderr and complete file manifests retained alongside):

| Trial | Digest |
| --- | --- |
| codex-retrieval-full | `695596f4cede804a9dd85d86e1aa822044ba1a442876f07c65c6d661a56b5740` |
| claude-retrieval-full | `dd72b2d8282e9ff5276112e2644d3ec2b665510014599208ffc7391ba8f44736` |
| codex-retrieval-narrow | `bf7f50e17e5842f7ac95fd0092a185e25339edf764cfbf71374a06528abebb28` |
| claude-retrieval-narrow | `b33da5ffe60b009c4b74282e4772e9aa62ad3578eff7a24f53f54b63a5aa9689` |
| codex-retrieval-missing | `17e377174babbe28e2420e4ce27501d7495681af68403b398a724a2e54bde8ec` |
| claude-retrieval-missing | `ff99af5a34d913a19e92a08b2371aa270af4ae61696cb932f847e99fca5be92a` |

## Validation and decision boundary

Canonical format, Clippy and workspace tests passed for the candidate; the test
run used host access, with the same four environment-gated tests ignored and the
timeout subprocess helper exercised by its parent test. Plugin and skill validators
passed. These checks establish packaging/structural properties, not semantic
review correctness. No integration CI or post-merge qualification was run.

This is one attempt per new case per host, not a controlled before/after comparison
or a reliability estimate. Automatic plugin routing, marketplace installation,
real tracker retrieval and Codex file-retrieval behavior remain unverified.

Recommendation: defer completion of #46 rather than treating known accuracy
failures as a disclaimer-only pass or adding a bespoke review engine. Preserve
the candidate and fixtures for a later evaluation. A maintainer may instead
explicitly choose a narrower advisory scope and revised acceptance, but that is
a decision still pending, not an approval inferred from permission to test.
No core gate is waived. No release, merge, managed-plugin update or consumer
project change was performed.
