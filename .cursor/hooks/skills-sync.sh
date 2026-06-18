#!/usr/bin/env bash
# postToolUse: re-sync skills/ and agent files after Write or StrReplace.
set -euo pipefail

input=$(cat)
path=$(echo "$input" | jq -r '.tool_input.path // .tool_input.file_path // .tool_input.target_file // empty')

if [[ "$path" == *skills/* ]] || [[ "$path" == */CLAUDE.md ]] || [[ "$path" == */AGENTS.md ]]; then
  bash skills/build.sh
fi

exit 0
