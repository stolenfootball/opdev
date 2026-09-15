#!/usr/bin/env bash
set -eu

# Establish project state only; runtime availability cannot grant consent.
project_dir="${CLAUDE_PROJECT_DIR:-$PWD}"
if project_root=$(git -C "$project_dir" rev-parse --show-toplevel 2>/dev/null); then
  project_dir=$project_root
fi
contract="$project_dir/.opdev/project.yaml"
if [ -e "$contract" ] || [ -L "$contract" ]; then
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"The target project has an OpDev contract. For software work, read its .opdev/project.yaml and AGENTS.md and apply the OpDev skill seamlessly. Invalid or unreadable contracts are errors, not absence. Resolve runtime compatibility only when the workflow is needed; offer missing-runtime setup rather than installing without consent. Unrelated prompts need no OpDev action."}}'
else
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"The target project has no OpDev contract. Do not announce or follow OpDev merely because the plugin or skill is available. For routine operations such as pulling changes, checking status, or starting a dev server, do the requested task without an OpDev suggestion or runtime lookup. For substantive software development, offer adoption once and continue the original task without OpDev unless the user accepts. An explicit adoption request already supplies assessment consent. Only after activation resolve the runtime and offer any needed setup. An explicit runtime-setup request does not adopt this project."}}'
fi
