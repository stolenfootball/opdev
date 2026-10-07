# Claude Code native adapter

After checking the shared [permission scope](delegation.md#permission-scope) for
this assignment, the packaged `opdev-specialist` is a narrow
read-only starting point for investigator, acceptance-reviewer and CI-analyst
assignments. Its presence or automatic host selection is not consent. Recheck
scope before resuming it for a different purpose. Pass the original permission
reference and limits, shared assignment and the resolved shared delegation
reference; a worker must not guess missing source/acceptance identities or scope.
Keep implementation in the controller initially. Its allowed tools are file
reading/search, not shell execution, edits, user questions or recursive dispatch.

The adapter inherits the selected model; verify actual effort/model with the
available host metadata and report unavailable settings. Do not switch model or
relax tools to satisfy an assignment. Partial/interrupted returns stay partial.

Claude plugin subagents do not enforce `permissionMode` from plugin frontmatter;
OpDev deliberately does not set it or pretend it provides a sandbox. Preserve
host permissions and inspect the actual actions. Do not install project/user
agent files or alter global permissions to compensate. Package validation is
not a live execution trial; disclose missing account access or host validation.
