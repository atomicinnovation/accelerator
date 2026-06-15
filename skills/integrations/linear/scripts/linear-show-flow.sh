#!/usr/bin/env bash
# linear-show-flow.sh — Fetch a single Linear issue by identifier.
#
# Usage:
#   linear-show-flow.sh <IDENTIFIER> [flags]
#
# Flags:
#   --comments N   Include the last N comments (default: all returned).
#   --help, -h     Print this banner and exit 0.
#
# Linear descriptions and comment bodies are Markdown-native — no ADF render.
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   80  E_SHOW_NO_KEY    — no issue identifier supplied
#   81  E_SHOW_BAD_FLAG  — unrecognised flag
#   82  E_SHOW_NOT_FOUND — no issue matches the identifier

set -euo pipefail

_LINEAR_SHOW_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_LINEAR_SHOW_SCRIPT_DIR/linear-common.sh"

readonly E_SHOW_NO_KEY=80
readonly E_SHOW_BAD_FLAG=81
readonly E_SHOW_NOT_FOUND=82

_linear_show_usage() {
  cat <<'USAGE'
Usage: linear-show-flow.sh <IDENTIFIER> [flags]

  Fetches a single Linear issue by identifier (e.g. BLA-123).

Flags:
  --comments N   Include the last N comments (default: all returned).
  --help, -h     Print this banner and exit 0.
USAGE
}

_linear_show() {
  local key=""
  local comments=-1

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help | -h)
        _linear_show_usage
        exit 0
        ;;
      --comments)
        comments="$2"
        shift 2
        if ! [[ "$comments" =~ ^[0-9]+$ ]]; then
          echo "E_SHOW_BAD_FLAG: --comments must be a non-negative integer; got '$comments'" >&2
          return $E_SHOW_BAD_FLAG
        fi
        ;;
      -*)
        echo "E_SHOW_BAD_FLAG: unrecognised flag: $1" >&2
        _linear_show_usage >&2
        return $E_SHOW_BAD_FLAG
        ;;
      *)
        if [[ -z "$key" ]]; then
          key="$1"
          shift
        else
          echo "E_SHOW_BAD_FLAG: unexpected positional argument: $1" >&2
          _linear_show_usage >&2
          return $E_SHOW_BAD_FLAG
        fi
        ;;
    esac
  done

  if [[ -z "$key" ]]; then
    echo "E_SHOW_NO_KEY: issue identifier required" >&2
    return $E_SHOW_NO_KEY
  fi

  local variables
  variables=$(jq -cn --arg id "$key" '{id: $id}')

  # shellcheck disable=SC2016 # $id is a GraphQL variable
  local query='query($id: String!) {
    issue(id: $id) {
      id identifier title
      state { name }
      assignee { name }
      description
      comments { nodes { body } }
    }
  }'

  local issue_json req_exit=0
  issue_json=$(bash "$_LINEAR_SHOW_SCRIPT_DIR/linear-graphql.sh" \
    --query "$query" --variables "$variables") || req_exit=$?
  if [[ $req_exit -ne 0 ]]; then
    return $req_exit
  fi

  # A null issue means the identifier did not resolve.
  local issue_id
  issue_id=$(linear_jq_field "$issue_json" '.data.issue.id')
  if [[ -z "$issue_id" ]]; then
    echo "E_SHOW_NOT_FOUND: no issue matches '$key'" >&2
    return $E_SHOW_NOT_FOUND
  fi

  # Optional client-side comment slice: keep the last N comments.
  if ((comments >= 0)); then
    issue_json=$(printf '%s' "$issue_json" | jq --argjson n "$comments" '
      if (.data.issue.comments.nodes // null) == null then .
      else .data.issue.comments.nodes |= .[-($n):]
      end')
  fi

  printf '%s\n' "$issue_json"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _linear_show "$@"
fi
