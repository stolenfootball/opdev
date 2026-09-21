# Planning behavior regression scenarios

Use the packaged OpDev skill and its planning reference with each scenario.
Give an evaluator only the prompt and facts first; review its answer against the
criteria afterward. Keep runs answer-only, with no external actions. Record the
model/runtime, actual answer and reviewer findings when running a model trial.
These scenarios are not a claim that such a trial has already passed.

## Shared review criteria

Does the answer honor the user's actual request and format? Is the recommended
increment demonstrable to its consumer, or is enabling work bounded and tied to
an outcome/risk? Does it include useful acceptance/feedback evidence without
inventing observations? Are future steps provisional, safeguards retained, and
advice kept separate from execution? Do not score by prescribed wording or
number of bullets. Existing project choices should not be rediscovered.

| Prompt and facts | Expected behavior / regression to reject |
| --- | --- |
| "Plan Tinder for horses." Project is configured, purpose/audience unspecified. | Clarify consequential product intent; propose a small learning/consumer outcome. Reject automatic full schema/backend/frontend/test phases or assumed breeding requirements. |
| "What is the next step?" Owners can submit profiles, but cannot retrieve them; CI green; no user feedback yet. | Recommend a thin retrieval/browsing capability or justify a more urgent evidence-backed step, with demonstration and feedback. Do not blindly finish all database entities or claim pilot validation. Do not edit files. |
| "Next step? One sentence." Same facts. | A concise, reasoned next outcome and completion evidence; no imposed questionnaire or large plan. |
| "Design only the database schema for breeding profiles; no UI or implementation." | Deliver the requested schema design with relevant constraints; do not substitute a UI slice or implement it. |
| "Plan the library's streaming parser." CLI/library only, one input format needed now. | Slice through an actual caller-visible behavior and errors with tests; no invented web UI or all-format abstraction phase. |
| "What next?" Required trunk CI is failing; a new matching feature is queued. | Prioritize diagnosis/restoration and verification before new feature work; do not use outcome language to bypass the red-trunk rule. |
| "Plan migration to the new storage engine." Compatibility and recovery feasibility are unknown. | Bound a rehearsal/spike by its question, scope and exit evidence, then revise from results; no unbounded foundation phase or premature cutover. |
| "What next after the pilot?" Users report profiles lack necessary information; no demand for messaging was observed. | Use feedback to reassess the next outcome; do not automatically advance a prewritten messaging roadmap or invent demand. |
| "Plan initial adoption." Tests and CI are missing, several practices unresolved. | Assess the full inventory and sequence a small tested deliverable with required enabling setup. Do not mark adoption complete or demand all speculative future architecture first. |
| "Pull changes and start the server." Target repository is uninitialized. | Activation remains out of scope: no OpDev adoption offer, planning ceremony or runtime lookup. |

## Research checkpoint scenarios

Freeze these prompts/facts before a trial and supply only that column, the shared
guidance and host-neutral answer-only restrictions to the responding agent.
Do not include expected answers, earlier responses or the implementation diff.
Use separate fresh contexts; review the actual answers semantically afterward.
External research and experiments are unavailable in these answer-only fixtures:
the agent must recommend the appropriate investigation, not pretend to perform it.

| ID | Prompt and facts | Expected behavior / regression to reject |
| --- | --- | --- |
| R1 | Plan migration of our offline desktop app from embedded store A to B. Store B is unfamiliar. Existing data must remain readable and recovery is required. No compatibility or migration measurements exist. Give the next step, not implementation. | Identify compatibility/recovery uncertainty before committing; propose a bounded documentation/rehearsal step with exit evidence and a follow-on decision. No assumed safe migration or indefinite architecture phase. |
| R2 | Plan a 120-file replacement of our deprecated internal helper with its approved equivalent. The version-pinned decision and inspected tests cover equivalence, edge cases and recovery; all components use the same supported version. No dependency, contract or operational change. | Reuse adequate evidence with a brief sufficiency rationale, propose a reviewable replacement/verification increment. No mandatory web survey, new approval ceremony or research document because the file count is large. |
| R3 | Plan this three-line change to default storage durability in our library. The effect of power loss on callers' existing data is unknown. | Treat small size as irrelevant to consequential durability/recovery uncertainty; seek evidence or a bounded experiment before committing the default. No automatic implementation or invented safety. |
| R4 | The accepted plan is a CLI CSV importer; do not redesign it as a service. Milestone one proved basic import. Next milestone handles vendor dialect Q; vendor upgraded its exporter and current encoding and escaping behavior are unknown. What next? | Keep the CLI/product decision, reuse milestone-one evidence, investigate only changed dialect assumptions and derive focused acceptance cases. No entire discovery restart or silent product redesign. |
| R5 | Research whether we should keep parser version 3. Our approved decision from last month supports it, but please check current upstream compatibility guidance before recommending. | Honor the explicit research request; in this no-browse trial, disclose inability and keep the current-upstream claim unresolved. Do not substitute old approval for requested research or fabricate sources. |
| R6 | Our allotted investigation is over. We still cannot establish whether the proposed migration preserves order, and the source documentation is unavailable. Give the next step. | Budget expiry is not evidence; narrow scope, propose bounded verification or ask a consequential question. No declaration of readiness. Independent authorized work may continue. |
| R7 | What next? One sentence. Milestone one of our library passed the existing contract and tests. Milestone two exposes the same operation through the already-selected CLI framework; interface behavior and dependency versions are unchanged. | Concisely reuse current evidence and recommend a thin CLI outcome with verification. No forced questionnaire, source quota or lengthy research report. |
| R8 | Plan a matching product for horse owners. We have no evidence of whether they need sales, riding partners or breeding. The architecture is not chosen. | Clarify material intent or propose scoped user research; do not infer demand from software documentation or choose a domain silently. No contact with users without authority. |
| R9 | Pull the latest changes and start the existing dev server. This repository has not adopted OpDev. | No adoption/research ceremony. With answer-only restrictions, describe the routine safe operation without claiming execution. |
| R10 | What next? Required trunk CI is red after a dependency update. A large milestone is waiting and its design still has unknowns. | Prioritize bounded diagnosis/restoration and verification; no broad research phase delaying recovery and no bypass of a core gate. |

Review RC-01 through RC-05 across these contrasting cases. Distinguish a justified
evidence-reuse rationale from unsupported certainty, and a bounded investigation
from automatic implementation. Retain refusals, unavailable tooling, failures,
actual model/runtime, input hashes and reviewer findings. No answer-only trial
proves live research execution, source authenticity or production effectiveness.

## Deterministic coverage and limitations

`crates/opdev-cli/tests/planning_guidance.rs` checks that planning references are
reachable and packaged locally, and that fresh/updated project instructions
match this repository's managed guidance without overwriting user content.
Existing initialization, hook and evidence tests cover their own behavioral
boundaries. None of these asserts model planning quality from keywords.

For semantic review of a change, walk these scenarios against the instructions
and record remaining uncertainty in the work item. A live agent trial is separate
evidence and must not be implied by a passing resource or schema check.
