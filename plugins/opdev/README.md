# OpDev agent plugin

This directory is one shared skill package for Codex and Claude Code. Codex reads `.codex-plugin/plugin.json`; Claude Code reads `.claude-plugin/plugin.json`, the same `skills/opdev/SKILL.md`, and a `UserPromptSubmit` hook that contributes only initialization state.

The hook does not enforce policy or modify the project. In an initialized repository, `.opdev/project.yaml` and the managed `AGENTS.md` block remain authoritative. In an uninitialized repository, the skill asks before running `opdev init`. If the CLI is unavailable, it offers installation rather than attempting it.

For local Claude Code testing, run `claude --plugin-dir ./plugins/opdev`.

For repository-marketplace installation:

```sh
codex plugin marketplace add https://gitlab.com/stolenfootball-tools/opdev.git
codex plugin add opdev@personal
claude plugin marketplace add https://gitlab.com/stolenfootball-tools/opdev.git
claude plugin install opdev@opdev
```

Start a fresh Codex task or reload Claude Code plugins after installation. CLI
installation and release packaging are described at the repository root.

The plugin declares its supported CLI range in `opdev-compatibility.json`.
Claude's prompt hook and the shared skill run `opdev plugin verify` before OpDev
is activated; missing, malformed, and incompatible contracts fail closed.

## Managed runtime setup

Version 0.1.2 adds the `setup` skill. Ask the agent to set up OpDev after plugin
installation; the OpDev skill also handles missing CLI setup on first use under
normal host approvals. The current runtime pin is published CLI 0.1.1, compatible
with this plugin. Existing standalone installations remain supported.

For manual setup from a plugin checkout:

```sh
sh plugins/opdev/scripts/runtime.sh --install
sh plugins/opdev/scripts/runtime.sh --run version
```

```powershell
powershell -NoProfile -File plugins/opdev/scripts/runtime.ps1 -Mode Install
powershell -NoProfile -File plugins/opdev/scripts/runtime.ps1 -Mode Run version
```

These commands install only a private, versioned runtime. They do not initialize
a repository or add a global `opdev` command. See [setup](skills/setup/SKILL.md)
for supported environments, storage overrides, and explicit cleanup.
