#!/usr/bin/env bash
set -euo pipefail

# Reads the markdown body (project context) from accelerator config files.
# Outputs the team context first, then local context, separated by a blank line.
# If no config files exist or bodies are empty, outputs nothing.
#
# Usage: config-read-context.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

OUTPUT=""
while IFS= read -r config_file; do
  body=$(config_extract_body "$config_file")
  trimmed=$(printf '%s\n' "$body" | config_trim_body)
  if [ -n "$trimmed" ]; then
    if [ -n "$OUTPUT" ]; then
      OUTPUT="$OUTPUT"$'\n\n'"$trimmed"
    else
      OUTPUT="$trimmed"
    fi
  fi
done < <(config_find_files)

if [ -n "$OUTPUT" ]; then
  echo "## Project Context"
  echo ""
  echo "The following project-specific context has been provided. Take this into"
  echo "account when making decisions, selecting approaches, and generating output."
  echo ""
  printf '%s\n' "$OUTPUT"
fi
