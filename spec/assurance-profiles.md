# Assurance profiles

OpDev assurance profiles are exact-version compatibility documents. They map
project evidence to an internal baseline, an external framework, or an evidence
format. They never make a conformance claim merely because a project selects a
profile.

The built-in profiles are:

| Profile | Interpretation |
| --- | --- |
| `opdev-core@1` | Normative OpDev and MinimumCD rule catalog. |
| `minimumcd@1` | Evidence-backed assessment of pinned manifesto source `86665f375a3a7f5c56be22ad67f4aad20348fade`; selected through engineering policy, not the informative profile list. |
| `nist-ssdf-derived@1.1` | Informative mapping to final NIST SP 800-218 SSDF 1.1. |
| `slsa-build-provenance@1.2` | SLSA 1.2 provenance evidence mapping, without a Build level claim. |
| `cyclonedx-sbom@1.5` | CycloneDX 1.5 SBOM evidence supported by the pinned Rust generator. |
| `openssf-osps-baseline-derived@2026.02.19` | Incomplete informative mapping to that exact OSPS Baseline release. |

`opdev profiles` lists the profiles bundled into the current binary. A project
contract must select an exact name and version. `latest` aliases are not
accepted. Unsupported versions produce a project-contract error so an upgrade
cannot silently change assurance semantics.

Mappings use `full`, `partial`, and `gap` coverage labels. These labels describe
the relationship between rules and framework text, not the evaluated state of a
particular project. Even `full` coverage requires evidence-backed rule results.
Passing mapped rules does not establish organization-wide governance, assessor
approval, trusted-builder properties, or third-party certification.

External standards advance independently of OpDev. A new standards version is
introduced as a new profile document and reviewed like a schema migration. Old
profiles remain stable for reproducibility until a future compatibility policy
explicitly removes them.

## Engineering policy 1

Project schema 3 explicitly selects `assurance.engineering.version: "1"` and
records the actual developer decision in `review_reference`. It uses catalog 3.
Project schemas 1/2 retain catalog 2; ordinary initialization and upgrades do not
infer migration. The presence of optional informative profiles never determines
whether baseline requirements apply. Empty profiles cannot disable the baseline.
Legacy `opdev-core@1` and engineering policy cannot be selected together.

Requirements have three interpretations:

- **Baseline:** versioned inputs and reproducible setup/dependencies, appropriate
  formatting and high-signal static checks, secret/dependency safeguards, declared
  authoritative context, one integration trunk, CI and risk-based verification.
  These requirements cannot be marked not applicable or ignored. Adequate
  implementations remain flexible; a missing tool is not a justification.
- **Conditional:** meaningful change/regression verification, review, supported
  behavior, delivery, recovery, configuration, provenance, accessibility,
  operations and other controls apply according to actual capabilities and scope.
  Unknown applicability is unverified, not an exemption. No required applicable
  control has an opt-out. Each rule's applicability and evidence remain explicit.
- **Preferences:** tool vendors, framework choices, trunk spelling and adequate
  project-owned document locations are not pass/fail requirements. Recommendations
  do not override adequate developer choices. There is no per-rule waiver list.

`rule_class` in the core policy module exhaustively assigns every catalog-3 rule.
New baseline rules add evidence obligations, not ecosystem-specific installers.
`OPDEV-BUILD-001` needs versioned input and clean-setup observations;
`OPDEV-STYLE-001` needs actual convention/check coverage and execution review;
`OPDEV-SEC-003` needs reviewed secret/dependency controls and affected observations.
Declarations alone leave these unverified. Revisit changed inputs, dependencies,
tool configuration or contrary observations. Use existing evidence and authorities,
not a new client policy document. Test/assertion adequacy stays independently required.

### MinimumCD mapping and separate assessment

Mapping 1 pins the manifesto's exact upstream commit; `1` is OpDev's mapping
revision, not an upstream MinimumCD version. Its 19 entries cover the nine CD,
six CI and four substantive trunk/branch clauses. Framework updates require a
new explicit version; runtime checks do not fetch changing website text.

| Existing MinimumCD rules | Engineering policy 1 | MinimumCD assessment |
| --- | --- | --- |
| CI-001, TRUNK-001 | Retained mandatory CI and single integration trunk | Required |
| TRUNK-002 | Replaced by OPDEV-BRANCH-001's development/maintenance distinction | Original strict branch finding still required |
| TRUNK-003 | Visible monitored cadence, not a development/integration timer | Required |
| TEST-001, TEST-002, FLOW-001, COMPAT-001 | Retained at their applicable boundaries | Required |
| DELIVERY-001, PIPELINE-001, ARTIFACT-001/002, ENV-001 | Retained delivery controls, not release authorization | Required when applicable |
| RECOVERY-001 | Retained appropriate automated tested recovery | Does not itself prove rollback; RECOVERY-002 is required |
| CONFIG-001/002 | Retained capability-dependent configuration controls | Required when applicable |

All abbreviated rule IDs above use the `MCD-` prefix. No existing rule disappears
from reports. Historical statements retain their IDs; new branch and rollback
requirements have distinct IDs. Every upstream clause needs full mapping coverage
and sufficient current rule findings. Partial/empty mappings and missing findings
cannot establish compliance, even when the mapped subset passes. Required failed
or unexecuted checks also prevent the assessment from passing.

Schema-2 check reports contain the engineering policy identity and a separate
`minimumcd` assessment. Null means not requested, not passed or not applicable.
The six rule outcomes do not change. Operational gates and the compliance gate
describe the engineering baseline; only the separate assessment may support a
MinimumCD claim. Legacy schema-1 reports retain their original aggregate meaning.
Saved reports remain diagnostics, not fresh qualification inputs or certification.

Select `assurance.engineering.minimumcd: "1"` to request the assessment, or omit
it after an explicit reviewed choice. `check --require-minimumcd` additionally
requires that assessment for the exit code; absent selection is an error before
commands run. Ordinary check exits retain their requested operational boundary.
Assessment aggregation launches no commands or network requests: the same verified
observations can support multiple clauses. Missing evidence is reported, not
silently gathered through additional tests, permissions or release operations.

### Maintenance branches

Ongoing feature work has one integration trunk. A maintenance declaration names
an exact branch, supported version and existing policy authority. That authority
must bound its support lifetime and permitted fixes, protect its CI path and
explain how relevant fixes remain in trunk. Prefer fixing and testing trunk first,
then verifying the backport on the actual maintained source. When reproduction
on trunk is impossible, explicitly review the regression risk and fix disposition.

`OPDEV-BRANCH-001` requires actual branch/review/CI evidence; the declaration alone
does not pass it. A second development stream cannot be excused by its name.
One integration trunk is retained even when a reviewed maintenance release source
exists. The stricter MinimumCD branch mapping remains independently assessed;
maintenance permission does not manufacture conformance. Requested remote trunk
qualification still verifies trunk, not a maintenance line; use the existing
identified-run inspection and reviewed evidence for maintained-line CI. Missing
branch-specific execution/protection evidence stays unverified.

### Migration preview

On a CLI advertising `engineering.assessment.v1`, preview without writes:

```text
opdev upgrade --engineering-policy 1 --policy-review-reference <actual-decision-reference>
```

The preview preserves commands, delivery, authorities, branch declarations and
unrelated profile pins. By default it retains assessment intent (legacy contracts
retain a MinimumCD assessment). `--minimumcd-assessment none` explicitly proposes
no assessment; `1` explicitly requests mapping 1. Neither choice waives engineering
requirements. The reference is a caller-attributed decision, not authenticated consent.
Review the candidate against the unchanged original contract and verify the
capability in both local and CI runtimes before explicitly editing the existing
contract. Recheck changed evidence and applicable gates. `--apply` cannot apply
this policy preview; it remains limited to its existing managed-guidance function.
No new project file, automatic installation, consumer migration or release follows.

### Design rationale

Separate versioned policy from observations, borrowing the catalog/profile
distinction from [NIST OSCAL](https://pages.nist.gov/OSCAL/learn/tutorials/control/basic-profile/)
without introducing its full tailoring language. Outcome-focused requirements
follow [NIST SSDF](https://csrc.nist.gov/projects/ssdf); the conditional boundary
is not permission to waive core guarantees. The controlled maintenance distinction
follows [trunk-based development guidance](https://trunkbaseddevelopment.com/branch-for-release/).
Readiness is not release execution, consistent with
[DORA's delivery/deployment distinction](https://dora.dev/capabilities/continuous-delivery/).
[Hilton et al.](https://www.cs.cmu.edu/~mhilton/docs/HiltonFSE17.pdf) document
assurance/speed and flexibility/simplicity tradeoffs, not proof of this design's
performance. Revisit this design if real projects require repetitive declarations,
duplicate execution, or produce unsupported passes; do not solve those failures
by adding waivers or unbounded policy customization.
