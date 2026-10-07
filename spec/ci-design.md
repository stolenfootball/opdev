# Evidence-led CI design and improvement

## Decision and scope

Use focused agent guidance and tested provider patterns to design new CI and
improve existing pipelines. Reuse project commands, CI adapters, execution plans,
run/job observations and reviewed same-run reuse. There is no automatic optimizer,
new mandatory project schema, timing gate or universal pipeline structure.
Efficiency recommendations are advisory; correctness and required verification
remain enforced by existing contracts. The shared
[agent reference](../plugins/opdev/skills/opdev/references/ci-design.md) applies to
both hosts without additional root instruction bulk.

## Behavioral contract

- CD-01: New CI design, adoption discovery, substantial CI changes and observed
  friction load the focused guide. Ordinary tasks do not require a pipeline audit.
  Inspect project capabilities and preserve adequate commands, tools and policies.
- CD-02: New designs use the simplest sufficient topology; existing designs use
  effective configuration, invoked scripts and available run observations. Unknown
  dynamic behavior stays unknown. Do not replace custom workflows automatically.
- CD-03: Consolidation requires genuinely equivalent purpose and execution inputs.
  Preserve native/platform/configuration coverage, source and trust boundaries,
  pre-merge verification and integrated-trunk checks. Prefer simple fresh execution
  ownership before authenticated reuse. Matching command text is insufficient.
- CD-04: Scheduling/setup/cache advice considers critical path, queue, compute,
  runner capacity, compatibility and cold/warm behavior. Caches are reconstructible
  accelerators, not qualification or artifact identity. No blanket sharing,
  cancellation, image migration or measured-speed claim from configuration alone.
- CD-05: Validate required verification negatively as well as positively. Missing,
  skipped, cancelled or failed required work cannot qualify merging. Check event,
  revision, job inventory and protection; offline tests do not prove live provider
  scheduling. Preserve immutable candidate bytes and explicit release authority.
- CD-06: Advice includes evidence, uncertainty, tradeoffs, validation and keeping
  a compliant setup. Honor declines without recurring prompts or extra records.
  Changes need actual scope authority; core failures remain distinct from optional
  optimization. No automatic provider mutation, paid trial or consumer migration.
- CD-07: Distinguish resource/routing checks, example execution, semantic review,
  live-agent effectiveness and provider observations. Measure comparable samples
  before performance claims; keep failure history and unknown results visible.

## Patterns and verification limits

The [feedback examples](../examples/feedback/README.md) demonstrate fresh proposed-
change verification, distinct trunk verification and single candidate consumption.
The GitHub required collector rejects non-successful verification explicitly;
GitLab keeps its required verification unconditional within admitted pipelines.
Neither example configures branch protection, authenticates project evidence or
constitutes a production delivery path. The Python fixture is illustrative, not a
required language or consumer dependency.

`ci_design` tests resolve packaged routes, run the exact example collector against
success and non-success inputs, and check the narrow example wiring. Existing
fixture tests exercise stale/missing verification and altered candidate bytes.
These checks are not provider emulation or agent decision-quality measurements.
Synthetic [review cases](../benchmarks/ci-review/design/README.md) separate prompts
from reviewer expectations. Work order, evaluations and open decisions belong in
[the CI design work item](https://gitlab.com/stolenfootball-tools/opdev/-/issues/77),
not a repository roadmap.

## Alternatives and reversal

Guidance alone has lower maintenance cost but leaves fragile provider patterns
underspecified. A universal CI compiler/optimizer adds significant maintenance and
can mistake textual repetition for redundancy. Prefer focused guidance plus
executable examples; consider narrow read-only assistance only when observed gaps
justify it. Current inspection remains supporting evidence, not an effective-policy
compiler. Revisit the design if trials show repeated unsafe recommendations,
missed material problems, excessive review overhead or recurring factual collection
gaps. A justified no-tooling decision is valid; more automation is not itself value.

## Research basis and limits

- [DORA CI](https://dora.dev/capabilities/continuous-integration/) favors rapid,
  reliable feedback and authoritative downstream artifacts. Its approximate
  ten-minute fast-cycle guidance is not a universal OpDev deadline or permission
  to skip required checks.
- [GitLab efficiency](https://docs.gitlab.com/ci/pipelines/pipeline_efficiency/)
  treats job dependencies, runner capacity, setup and networking as bottlenecks;
  parallelism trades resources/complexity for latency. [Caching guidance](https://docs.gitlab.com/ci/caching/)
  distinguishes caches from artifacts and says availability is not guaranteed.
- [Hilton et al., FSE 2017](https://nomatic.dev/docs/fse17-hilton.pdf) identify
  assurance, security and flexibility tradeoffs through interviews and surveys.
  This supports contextual choices, not one universally optimal topology.
- [Khatami et al., SCAM 2024](https://azaidman.github.io/publications/khatamiSCAM2024b.pdf)
  investigate 83 projects and narrow 22 candidate smells to seven confirmed
  categories through maintainer feedback. Suspicion needs contextual validation;
  unconfirmed does not mean harmless.
- [Urdih et al., EASE 2026](https://arxiv.org/abs/2604.17890) study ten cache-related
  smells and 228 projects. Detector performance on a labelled corpus supports
  bounded checks, not universal cache mandates or guaranteed performance gains.
- [Zheng et al., TOSEM 2025](https://binlin.info/downloads/Zheng2025a.pdf) analyze
  375 failed runs in 260 Java projects and survey 151 developers. Project and
  workflow/environment failures require different diagnoses; results do not
  establish frequencies for every ecosystem.
- [Predictive Test Selection](https://arxiv.org/abs/1810.05286) reports roughly
  halved test infrastructure cost at Facebook while retaining detection of over
  99.9% of faulty changes in that setting. This depended on historical data and
  calibration, not filename heuristics; OpDev does not introduce a selector.
- [GitHub required checks](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks)
  distinguish skipped jobs/workflows and merge-queue events. [Secure use](https://docs.github.com/en/actions/reference/security/secure-use)
  supports least privilege, untrusted-source isolation and immutable action pins.
  Verify current provider/version behavior before changing a real pipeline.
