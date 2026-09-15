# Initialization and installation

## Uninitialized project

Ask once, in plain language: “This looks like software-development work. OpDev is installed but this project is not initialized. Would you like me to initialize it?” Continue the original task without OpDev if the user declines.

An explicit request to adopt or convert the project already supplies consent.
After consent, follow [adoption.md](adoption.md) for the full assessment and
completion workflow. Check CLI capability first; scaffolding alone is not completion.

Initial discovery and scaffolding:

1. Run `opdev init --dry-run` and show material inferences, authority conflicts, and migration gaps. Follow [documentation ownership](project-contract.md#documentation-locations-and-ownership); existing project locations take precedence over suggested folder defaults.
2. If the proposal is reasonable, run `opdev init`.
3. Review `.opdev/project.yaml` with the user where delivery, recovery, coverage, or project kind remains uncertain.
4. Do not overwrite an existing CI configuration. Use `opdev ci generate --provider github|gitlab` for review, then repeat with `--write` only after approval. GitLab generation infers an official image from exact project toolchain metadata; for mixed or custom stacks, review and pass `--image` explicitly. Never accept an image guess that does not contain the project's canonical command toolchain.

Development CLIs create `.opdev/project.yaml`, pending `.opdev/adoption.yaml`, and
managed sections in `AGENTS.md` and `CLAUDE.md` for new projects. They preserve
content outside OpDev markers. Existing projects without an adoption record require
explicit `opdev adoption start`; `opdev upgrade` refreshes only managed guidance.

## CLI unavailable

Inform the user before substantive software development and offer, rather than perform, one of these choices:

- Use the packaged setup skill for the pinned verified runtime, or obtain a compatible native release through the project's documented installation path. Confirm it supplies the capabilities required for the task.
- For a Rust development environment, install from the source repository with `cargo install --git https://gitlab.com/stolenfootball-tools/opdev.git --locked opdev-cli`.
- Continue without OpDev for this task.

After installation, verify with `opdev version`; then return to the initialization flow. Never claim installation succeeded without running that check.
