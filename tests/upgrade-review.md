# Upgrade procedure review scenarios

Review against the packaged upgrade reference. These are manual instruction
checks, not claims of measured Codex/Claude end-to-end execution.

| Situation | Required behavior |
| --- | --- |
| "Are upgrades available?" | Inspect metadata and state; no installation or project writes. |
| "Upgrade OpDev" on old managed pin | Inspect `upgrade --help`; do not run old bare `upgrade` as a preview; explain capabilities and offer verified target. |
| New standalone binary, older managed runtime | Report actual precedence; do not call the older selected CLI upgraded. |
| Plugin installed, no project contract | Installation may proceed within consent; no project adoption. |
| Custom GitLab includes or GitHub script-based pins | Preserve configuration; report unresolved effective version and propose a separately reviewed diff. |
| Offline inspection | Show known target and unknown newest-release state; no invented availability. |
| Declined runtime update | Preserve runtime and report limitations; no fallback install or cache editing. |
| Damaged or incompatible managed runtime | Report path/problem; no deletion, signature bypass or normal workflow activation. |
| Guidance changed after review | Stale apply rejected; re-preview and review changed scope. |
| Pending/legacy adoption or future schema | No fabricated dispositions, erased decisions or automatic assessment. |
| Installed successfully, tests fail/unavailable | Installation updated; project upgrade incomplete, with exact outcomes. |
| User requests only Claude update | Do not mutate Codex installation or unrelated project state. |
