#!/usr/bin/env bash
set -eu

project_dir="${CLAUDE_PROJECT_DIR:-$PWD}"
compatibility_contract="${CLAUDE_PLUGIN_ROOT}/opdev-compatibility.json"

managed_status=0
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    cli=$(powershell -NoProfile -File "${CLAUDE_PLUGIN_ROOT}/scripts/runtime.ps1" -Mode Path 2>/dev/null) || managed_status=$?
    if [ "$managed_status" -eq 0 ]; then cli=$(cygpath -u "$cli"); fi
    ;;
  *) cli=$(sh "${CLAUDE_PLUGIN_ROOT}/scripts/runtime.sh" --path 2>/dev/null) || managed_status=$? ;;
esac
if [ "$managed_status" -eq 3 ]; then cli=$(command -v opdev || true); fi

if [ "$managed_status" -ne 0 ] && [ "$managed_status" -ne 3 ]; then
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"The OpDev managed runtime could not be validated. For software-development work, invoke the OpDev skill and report the runtime lookup error. Do not silently fall back to another CLI or delete the managed runtime."}}'
elif [ -z "$cli" ]; then
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"The OpDev plugin is installed, but the OpDev CLI is unavailable. If and only if the user prompt clearly requests software development, invoke the OpDev skill and its packaged setup skill to install the pinned managed runtime using normal host approvals. Do not initialize the repository or interrupt unrelated tasks."}}'
elif ! "$cli" plugin verify --contract "$compatibility_contract" >/dev/null 2>&1; then
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"The installed OpDev CLI is incompatible with this OpDev plugin or its compatibility contract could not be verified. For software-development work, invoke the opdev skill, report the compatibility failure, and offer to install a compatible CLI. Do not apply OpDev until verification succeeds."}}'
elif [ -f "$project_dir/.opdev/project.yaml" ]; then
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"This project is initialized for OpDev. For software-development work, invoke the opdev skill, read .opdev/project.yaml and AGENTS.md, and apply them seamlessly without asking whether to use OpDev."}}'
else
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"The OpDev plugin and a compatible CLI are installed, but this project is not initialized. If and only if the user prompt clearly requests software development, ask whether they want to initialize OpDev before substantive development. Do not ask for unrelated tasks."}}'
fi
