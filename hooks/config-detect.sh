#!/usr/bin/env bash

# Check for jq dependency (matching vcs-detect.sh pattern)
if ! command -v jq &>/dev/null; then
  echo '{"systemMessage":"WARNING: jq is not installed. Accelerator config detection could not run. Install jq for config support."}'
  exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Run config-summary.sh. Let stderr pass through naturally (matching
# vcs-detect.sh pattern) so warnings reach the terminal without polluting
# the JSON output. If the script fails, discard stdout and continue.
SUMMARY=$("$SCRIPT_DIR/../scripts/config-summary.sh") || SUMMARY=""

# Only output if there's something to report
if [ -n "$SUMMARY" ]; then
  jq -n --arg context "$SUMMARY" '{
    "hookSpecificOutput": {
      "hookEventName": "SessionStart",
      "additionalContext": $context
    }
  }'
fi
