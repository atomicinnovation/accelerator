#!/usr/bin/env bash
set -euo pipefail
# jira-md-to-adf.sh — compile Markdown to Atlassian Document Format JSON
# Reads Markdown from stdin, writes ADF JSON to stdout.
# Exit codes: 0 success, 41 E_ADF_UNSUPPORTED_*, 42 E_ADF_BAD_INPUT

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOKENISER="$SCRIPT_DIR/jira-md-tokenise.awk"
ASSEMBLER="$SCRIPT_DIR/jira-md-assemble.jq"

tmpfile=$(mktemp)
trap 'rm -f "$tmpfile"' EXIT

awk -f "$TOKENISER" > "$tmpfile" || exit $?
jq -R -s -f "$ASSEMBLER" < "$tmpfile"
