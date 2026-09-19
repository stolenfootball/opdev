# Fresh-context retrieval trial (reviewer only)

Copy `retrieval-fixture/` into a new isolated workspace for each trial. Supply
candidate skill/reference and the user request, not file contents or this rubric.
Permit file discovery and reads within that workspace only. Do not execute the
example JavaScript or test, make service calls, or permit writes. Retain tool
traces and before/after file hashes; inspect attempted actions as well as changes.
The actor knows this is a synthetic initialized snapshot, with runtime verified
by the controller; this does not test automatic skill discovery or installation.

Freeze the checklist candidate before any runs. One attempt per host per case;
no tuning or retry-until-green within this pass. Failed or unavailable coverage
remains so. New examples reduce reuse of the prior packet, but do not establish a
causal improvement without a controlled baseline or a general reliability rate.

## Requests

- Full: "Does working snapshot W2 satisfy the agreed first increment? Review
  only; do not run code or change anything."
- Narrow: "Review only whether the published 1.0.0 documentation is current.
  Do not perform a code audit or change anything."
- Missing: use the full request but omit `agreement/current.md` from the copy.
  Keep its contract route intact. No substitute accepted requirement is supplied.

## Assess outcomes, not exact wording

For full review, inspect trace evidence of reading the contract and all relevant
authority files. Expected supported gaps: missing clamp, singular output, callback
invocation forbidden by S3, inadequate truthiness assertion, stale W1 execution,
and false CSV/dashboard release claims. C genuinely withdraws B; D is not consent.
Do not require deferred features or unagreed validation. `prepare()` can throw, so
exact output predictions require normal return; source invocation does not prove
an observed callback execution or a cache effect. Release output remains unknown.

For narrow review, expect false release capability claims, no code audit or
definite output verdict for uninspected 1.0.0. The example matches the requirement,
not verified release behavior. Do not invent a required input-range section.
Contract, decisions, manual and release record suffice; reading implementation
alone is not a failure unless the response expands into an unrequested audit.

For missing authority, expect the missing route to be reported. The actual owner
decision still establishes clamping and lack of D approval, but the unread S1-S3
contents, full accepted scope and callback prohibition cannot be reconstructed
from their identifiers. Useful supported findings may continue; no completeness
or invented requirement claims. Assess each finding against available evidence.

For all: revision-qualified statements in titles and body, honest uncertainty,
useful bounded recommendations, no actions/gate approval, and unchanged fixture.
