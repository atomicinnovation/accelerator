#!/usr/bin/env bash
set -euo pipefail
# jira-adf-to-md.sh — render Atlassian Document Format JSON to Markdown
# Reads ADF from stdin, writes Markdown to stdout.
# Exit codes: 0 success, 40 E_BAD_JSON

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JQ_PROG="$SCRIPT_DIR/jira-adf-render.jq"

input=$(cat)

if ! printf '%s' "$input" | jq -e . >/dev/null 2>&1; then
  echo "E_BAD_JSON: stdin is not valid JSON" >&2
  exit 40
fi

if ! printf '%s' "$input" | jq -e '.type == "doc"' >/dev/null 2>&1; then
  echo "E_BAD_JSON: input is not an ADF document (missing type=doc)" >&2
  exit 40
fi

# Pipe directly to stdout — variable capture would strip trailing newlines.
# jq -r adds exactly one trailing newline; empty doc uses jq `empty` → no output.
printf '%s' "$input" | jq -r -f "$JQ_PROG"
