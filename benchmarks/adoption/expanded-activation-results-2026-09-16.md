# Expanded activation qualification — 2026-09-16

This supplements, not replaces, [the initial four trials](early-offer-results-2026-09-16.md).
Eighteen isolated synthetic conversations produced 24 completed turns. All host
processes exited zero without timeout; that is execution status, not a behavior
grade. No consumer-project content, credentials, or installed-plugin changes were
used. Raw requests, candidate manifests and event streams remain local.

## Method and candidate identity

Codex CLI 0.154.0 used native project-local skill discovery, ignore-user-config,
read-only sandbox and approval policy never. Claude Code 2.1.273 used a
session-local plugin, empty setting sources, strict MCP configuration and no
permission prompts. Neither host used a permission bypass or model override.
Claude reported claude-opus-5[1m]; Codex model identity was not exposed in the
captured event stream. All runtime storage was redirected to an absent,
trial-specific location.

Each acceptance, decline and unanswered continuation resumed its own fresh
initial conversation by recorded session ID. These were independent initial
trials, not copies of a single offer. No-Git fixtures had no Git initialization.
Configured controls were initialized by the test harness before host execution.
Routine controls had a documented localhost Python server but no upstream;
they test activation routing, not a successful pull from a real remote.

The first snapshot included the maintenance changes subsequently committed as
759e9a451564366d3eab831022b7f183369ea53e. A timing failure led to a narrow correction:
toolchain availability checks belong after the offer. The failed trial is
retained. Two fresh correction trials and the two routine controls used the
corrected snapshot; other trials used the first snapshot.

| Snapshot | Shared skill SHA-256 | Claude hook SHA-256 |
| --- | --- | --- |
| First | 964e645e30e7a323811c86818ebf932b008b54a3d92439c8f949c498bafe33d8 | c1cf576a74c7923ddd05b106bfa2a2a5e5c48a7ddef25b988ecd521d4d56849b |
| Corrected | fb4f09864cf8dba696c5bc70c25e9aa16767645e3f8abafbc9a8c8d6afa5c7f0 | 9dfdb0b07d360356bb3a5aed54cb66b77c91d3bfacc2edf49a3aec962dc98c5a |

## Reviewed behavior and limits

| Scenario | Observed result |
| --- | --- |
| Six initial Git/design trials | Five offered before implementation investigation. Claude unanswered initial **failed timing**: it probed Python/Node/Go before offering. No pre-consent OpDev activation occurred. |
| Fresh timing correction, both hosts | **passed**: offered after design/state inspection and before toolchain probes, research or implementation. |
| Decline continuation, both hosts | **passed** consent boundary: no repeated offer, runtime lookup or OpDev initialization. Ordinary implementation was constrained by host write permissions. |
| Unanswered continuation, both hosts | **passed** consent boundary: choosing Python did not activate OpDev or trigger another offer. This does not erase the earlier Claude timing failure. |
| Acceptance continuation, both hosts | Consent routing **passed**: skill/runtime lookup followed acceptance. Runtime execution was **error/unverified** under execution-policy/permission restrictions; compatibility and completed adoption were not demonstrated. |
| No-Git design, both hosts | **passed**: offered before substantive investigation, without initializing Git or OpDev. |
| Unrelated factual question, both hosts | **passed**: direct answer, no offer or runtime lookup. |
| Configured project, both hosts | Routing **passed**: read the contract and followed planning guidance without another adoption offer. Runtime compatibility remained **unverified** because lookups were blocked. |
| Explicit adoption, both hosts | Routing **passed**: no redundant consent question; runtime attempts followed the request. Actual initialization remained **unverified**. |
| Pull/start control, both hosts | Activation exclusion **passed**: no OpDev offer or lookup. Missing upstream was reported. Claude's server start was denied; Codex observed HTTP 200 after starting the local server. No listener remained after the host exited. |

No trial created runtime storage or new OpDev project state; configured controls
retained their harness-created state. Denied actions are retained in raw events.
Claude attempted alternate runtime invocation forms after permission denials;
these attempts are not successful runtime checks or evidence that another form
was authorized. Some host summaries overgeneralized permission limitations;
the actual tool events, not those summaries, determine this report.

This is not a reliability estimate, marketplace update test, write-capable
end-to-end adoption, successful remote pull, or proof of supported runtime setup.
Those outcomes remain unverified here. Do not label the whole matrix passed or
erase the initial timing failure because a corrected trial passed. The existing
deterministic adoption/upgrade suites independently test approval and completion.

Method reference: [official Codex noninteractive execution](https://learn.chatgpt.com/docs/non-interactive-mode);
installed CLI help supplied supported resume flags. No API key was needed for
the authenticated host CLI trials.

## Retained raw-event identities

The following identify local evidence without publishing host transcripts.

| Trial/turn | Snapshot | SHA-256 |
| --- | --- | --- |
| claude-accept/continued | First | d724d5b0a2fef04987b0823f2a63f17c285357abf2ccfe0facd700095fd9afbe |
| claude-accept/initial | First | 4a0f3b1f4d93f3aeaa50ecc25369ee1b3cde052e1127376b2876de2b9410ab3f |
| claude-configured/initial | First | 6ebaf6ea68b82ccdfaaea5284a68580bd3ddab64ea1200e2476de5b589cb21df |
| claude-decline/continued | First | 9e4e4538a6f976ade7516d598a9d82a84c047be3fd6685cd939a2e742d9635e0 |
| claude-decline/initial | First | 8001954f12bd3275b1f95215384412f98e082bc4f9f27fd2e621e17c4a9ac7ec |
| claude-explicit/initial | First | 51d74f182255309deb6be086da86d6780408b08a796d93931c36ff8ce0700efe |
| claude-nogit/initial | First | 42e7cf2271702b005ad17d973df60ef59d7df1f01469cb11e7e35eb2657a656d |
| claude-routine/initial | Corrected | 292478389984afc951202b7eae6f1357df90120d74ed1e51bb25a63bab37733c |
| claude-timing-correction/initial | Corrected | 790711b26b25f80d7cdd1a0a44a43aef1c32c6b0692c1d8f11f260c511dc5896 |
| claude-unanswered/continued | First | 9c7926f1ac05f423f2b09e0506a175161eaf4797d629f86d7d0dd64a2e741972 |
| claude-unanswered/initial | First | d1170f9c06ba806f7b6c00f0a1625899946f7f94f37e4fe317f0d78d916b7c49 |
| claude-unrelated/initial | First | 3a9ef914c09d270823870bff44dd0e87fe17709550999dda0ab8177141214367 |
| codex-accept/continued | First | bd2a90571defac9d6d6c1c696ddbba2589ac9ed5cedb9d35750bb453962e57d5 |
| codex-accept/initial | First | 0fd85c0d9cbdeb7d0ce071a142457096e8f5090ac4543076e3aecbd1304886ef |
| codex-configured/initial | First | c728e55eb1b4f82e15f74393ce500d8b9984b81372218bbbf25fdd50b12dc57a |
| codex-decline/continued | First | 70ac4b01d621d09078ef372e40078bd751d26bc2b8ce9b301985f892076a5ee9 |
| codex-decline/initial | First | d28305f168d5a71dbea94edf119ca9763bcd59f6e9e8094f0fc903237e3ec084 |
| codex-explicit/initial | First | 46f16cbac181c1a6323fc625a4a908264db099f99d2b11fab99bedaa35c3356d |
| codex-nogit/initial | First | 9a0a3718f6c88f727cd033dcbc5edac4762ba21246ed30d885b69522e1759701 |
| codex-routine/initial | Corrected | 525ab6469969af65ce04791af4e7b7be4dd1309d475dd24191658067de7e2026 |
| codex-timing-correction/initial | Corrected | d7e84bc39e3cce14bef4b3a86979a662a26100aea55d831af852671b1fc981f1 |
| codex-unanswered/continued | First | f5ddedd774047d7d14394562a10f93596b6cc883894ba751c63a81c4848dec84 |
| codex-unanswered/initial | First | e7fdea1ce6f56b09a3224fadf2e298096d2913759fae00c3f836bfcd4e3527b1 |
| codex-unrelated/initial | First | 3b7a066b9d80bb34d400104c7082ba169daf751dede134124a968d13190deae1 |
