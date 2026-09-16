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
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"The target project has no OpDev contract. For substantive software planning or implementation, including a design-only folder without Git, offer optional adoption once in the first response after minimal local inspection, before substantive research, planning, or edits. Before the offer, inspect only the supplied design, existing instructions, contract presence and repository state. Checking installed language runtimes or package managers to choose a stack is implementation investigation: offer before those probes, upstream browsing or planning. Ask briefly through an available question tool or plain chat. Honor an earlier decline; if declined or unanswered, continue the requested task without OpDev and do not ask repeatedly. Do not announce or follow OpDev, probe its runtime, initialize, or write a consent marker before acceptance. Routine pull/status/dev-server operations and unrelated prompts need no offer or runtime lookup. An explicit adoption request already supplies assessment consent. Only after activation resolve the runtime and offer any needed setup. An explicit runtime-setup request does not adopt this project."}}'
fi
