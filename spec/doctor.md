# Read-only readiness diagnostics

Accepted design and implementation scope: [issue 47](https://gitlab.com/stolenfootball-tools/opdev/-/issues/47).
Doctor explains prerequisites before costly work; it is not setup, adoption,
qualification, or a universal project-health verdict. The following conditions
are the acceptance authority for this change.

- **D47-1 Identity:** Report the exact running executable, SHA-256, version,
  current OS/architecture, supported schemas and relevant capabilities. Matching
  versions do not prove matching builds, features, authenticity or CI selection.
- **D47-2 Scope and outcomes:** Human and schema-1 JSON report the same findings
  and exit. Each finding identifies its stable check ID, scope, subject, outcome,
  whether it affects the selected prerequisite exit, observation and next action.
  Project verification always remains unverified. Exit 0 means no blocking
  finding in the inspected prerequisites; 1 means a required prerequisite is
  failed/unverified/migration_required; 2 means an inspection/input error.
  Informational adoption/delivery gaps and unrequested observations do not block
  local prerequisite inspection, waive core rules, or establish readiness.
- **D47-3 Partial inspection:** Missing contracts allow runtime-only inspection
  without adoption. Invalid, unreadable or unsupported contracts yield an error
  while retaining independent observations; dependent checks do not invent tool
  or policy failures. Subdirectories resolve to the nearest project/Git boundary.
- **D47-4 Commands and authorities:** Check declared working directories and
  readable local authority files/directories without folder assumptions. Inspect
  executables in the current environment without invoking them. Reuse canonical
  Windows package-manager shim resolution. Relative paths, incomplete search and
  nested environment behavior remain unverified; a found file does not prove
  launch, versions, architecture compatibility or successful verification.
- **D47-5 Inventory:** Reuse upgrade's plugin compatibility and CI-pin inspectors,
  preserving upgrade preview/apply input binding. Compatible standalone runtimes
  and differing compatible package pins are allowed. CI declarations, including
  matching pins, cannot establish effective runtime compatibility/qualification.
  No plugin root means plugin selection remains unverified, not an error.
- **D47-6 Remote:** Network and credential inspection require explicit --remote.
  Reuse current read-only provider observations; missing visibility is unverified,
  not a pass or proof of absence. Read access is not write permission. A latest
  green pipeline is informational, not exact-revision qualification; concrete red
  blocks. Unsupported/missing provider configuration stays unverified.
- **D47-7 Preservation:** No project command, plugin script, version probe,
  extension, repair, installation, authentication change, environment startup,
  ledger write or project mutation. No arbitrary authority URL fetches. Input
  content reads are bounded to 1 MiB per file and reject special files and linked
  children/junctions. Parse errors do not echo input contents. Reports contain
  local paths and must be reviewed before sharing. This is not protection against
  a concurrent hostile filesystem writer or attestation of executable trust.
- **D47-8 Agent use:** After activation and runtime resolution, use doctor once
  before substantial implementation/verification in a new or changed environment,
  or for setup failures. Capability-check doctor --help on older runtimes; do not
  interpret their unconditional success exit as readiness. Routine work and
  uninitialized-project consent boundaries remain unchanged. Never replace
  missing doctor capability with a fabricated pass or an automatic upgrade.

## Interface and compatibility

`opdev doctor [--root PATH] [--plugin-root PATH] [--remote] [--format human|json]`

There is no --fix, --execute, new project file, tool-specific default or extra
installer. IDs identify checks; subject distinguishes multiple commands,
authorities and inventory observations. JSON is the machine contract; consumers
must not parse human prose. See `schema/doctor.schema.json`.

**Intentional exit migration:** older doctor always returned success after
printing contract gaps. Consumers must capability-check the new flags/schema and
handle 0/1/2 consistently. A missing contract is not a request to initialize one.
Explicit remote inspection without a valid contract is unverified and exits 1
(or 2 when the contract is invalid).

Default exit scope: runtime identity, a present contract, its declared command
and local authority prerequisites, and supplied plugin compatibility. Inspection
errors always exit 2, including unreadable CI inventory. Adoption/delivery gaps,
external authority access, command execution, CI effective runtime and unrequested
remote checks remain informational. Explicit remote settings/access observations
join the selected scope; latest-pipeline unverified remains informational because
that adapter intentionally does not qualify a green snapshot. Failed remote
observations still block. Review the full findings, not just the exit code.

The CLI never probes another executable's capabilities. Missing CLI bootstrap
still belongs to existing host runtime lookup/setup. Exact current-executable
hashes distinguish same-version builds but do not verify release signatures.
OS/WSL hints never imply that a distribution, container or wrapper's inner tools
are available; inspect within the intended environment instead of starting it.

## Architecture and verification

A shared internal read-only inventory serves doctor and upgrade; the latter
retains its guidance previews, input digests and explicit application boundary.
Executable inspection lives beside canonical execution, with the same constrained
Windows shim resolver. It does not change command launch behavior.

Rejected alternatives: ecosystem-specific tool adapters (maintenance and policy
duplication), running arbitrary health scripts (violates read-only expectations),
and universal green/red health (conflates prerequisites and qualification).
Revisit only with a demonstrated consumer requirement and reviewed side effects.

Process fixtures assert source/config preservation, no sentinel execution,
structured outcomes/exits, standalone/compatibility cases, partial reports,
platform uncertainty and representative human output. Provider-unavailable and
red/green distinctions use offline observation fixtures, not live credentials.
Canonical suites run before/after integration. Maintainer review of actual new
wording is required by `consumer-interface.md`; no release is part of this issue.
