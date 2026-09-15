# Experimental work

Read this when introducing, changing, evaluating, promoting, or abandoning an
experiment, or when a release must exclude unfinished behavior. Do not require
an experiment record for ordinary changes or initialize empty registries.

Clarify whether the user means **not enabled** or **not shipped** when material.
Select runtime opt-in, build-time exclusion, branch by abstraction, disconnected
components, or an isolated prototype to suit the software. Preview channels do
not themselves isolate features. Preserve one integration trunk and one automated
delivery path; an experiment never waives MinimumCD or permits a red trunk.

Use the existing work authority. Adapt [experiment.yaml](experiment.yaml) there,
or have the work item link to a canonical record file. Do not duplicate status in
static design docs or assume a `docs/` folder. Retain the standard fields: ID,
work, owner, review date, purpose, promotion/abandonment criteria, isolation and
stable default, recovery, supported configurations/test suites, and cleanup.
For new repository-owned work with no established location, propose a file in
`.opdev/experiments/` only when needed. The date and suite IDs in the template
must be replaced, not copied as acceptance evidence.

Require actual stable and supported experimental configuration tests before and
after integration. A suite may cover multiple configurations, but verify that it
does. All-features testing is not evidence for the disabled configuration. Select
interaction tests by risk; do not create an uncontrolled matrix of binary variants.
Assess shared code, startup, dependencies, persistent data and security effects.
A hidden feature is not proof of isolation, authorization, or safe recovery.

Check `opdev experiment --help` before using the development-only record validator.
When available, run `opdev experiment validate PATH --root ROOT`. It is read-only:
it checks structure, review date and suite declarations, not behavior or gates.
Older published CLIs may lack it. Report that limitation and apply the existing
work/testing/evidence protocol; do not invent a validator or claim it ran. If
automated validation is required for this task, offer a compatible development
CLI or stop for that capability rather than silently skipping it.

Review outcomes separately: correctness, deployability, effectiveness. Bind
reviewed change evidence to the exact staged state through the ordinary ledger.
Map the record and evidence to existing work/design, testing, compatibility,
configuration, artifact, environment and recovery rules. A valid record alone
cannot satisfy them. Different build variants need independently qualified
artifact identities and unchanged promotion. Runtime configuration changes follow
the reviewed, versioned CI path too.

At the review date, promote, abandon, or explicitly extend with evidence and a new
date. Do not silently extend, automatically enable, or remove code based on a date.
Finish by removing temporary switches/old code or deliberately making an option
permanently supported with an owner and tests. Record unresolved gaps honestly;
unrelated work does not authorize resolving or hiding them. Permanent product
options and operational kill switches have their own support lifecycle.
