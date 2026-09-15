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

## Deterministic coverage and limitations

`crates/opdev-cli/tests/planning_guidance.rs` checks that planning references are
reachable and packaged locally, and that fresh/updated project instructions
match this repository's managed guidance without overwriting user content.
Existing initialization, hook and evidence tests cover their own behavioral
boundaries. None of these asserts model planning quality from keywords.

For semantic review of a change, walk these scenarios against the instructions
and record remaining uncertainty in the work item. A live agent trial is separate
evidence and must not be implied by a passing resource or schema check.
