#!/usr/bin/env bash
# linear-comment-flow.sh — Add a Markdown comment to a Linear issue.
#
# Usage:
#   linear-comment-flow.sh <IDENTIFIER> --body TEXT | --body-file PATH [flags]
#
# Flags:
#   --body TEXT        Inline Markdown comment body.
#   --body-file PATH   Read the Markdown comment body from a file.
#   --print-payload    Dry-run: print operation + input, exit 0.
#   --quiet, -q        Suppress INFO stderr lines.
#   --help, -h         Print this banner and exit 0.
#
# Linear comments are Markdown-native (commentCreate input.body) — no bodyData
# / Prosemirror conversion.
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   90  E_COMMENT_NO_KEY   no issue identifier supplied
#   91  E_COMMENT_NO_BODY  no resolvable comment body
#   92  E_COMMENT_BAD_FLAG unrecognised flag

set -euo pipefail

_LINEAR_COMMENT_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_LINEAR_COMMENT_SCRIPT_DIR/linear-common.sh"

readonly E_COMMENT_NO_KEY=90
readonly E_COMMENT_NO_BODY=91
readonly E_COMMENT_BAD_FLAG=92

_linear_comment_usage() {
  cat <<'USAGE'
Usage: linear-comment-flow.sh <IDENTIFIER> --body TEXT | --body-file PATH

  Adds a Markdown comment to a Linear issue via commentCreate.

Flags:
  --body TEXT        Inline Markdown comment body.
  --body-file PATH   Read the Markdown comment body from a file.
  --print-payload    Dry-run: print operation + input, exit 0.
  --quiet, -q        Suppress INFO stderr lines.
  --help, -h         Print this banner and exit 0.
USAGE
}

_linear_comment() {
  linear_require_dependencies

  local key="" body="" body_file="" body_set=0 print_payload=0 quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help | -h)
        _linear_comment_usage
        exit 0
        ;;
      --body)
        body="$2"
        body_set=1
        shift 2
        ;;
      --body-file)
        body_file="$2"
        shift 2
        ;;
      --print-payload)
        print_payload=1
        shift
        ;;
      --quiet | -q)
        quiet=1
        shift
        ;;
      -*)
        echo "E_COMMENT_BAD_FLAG: unrecognised flag: $1" >&2
        _linear_comment_usage >&2
        return $E_COMMENT_BAD_FLAG
        ;;
      *)
        if [[ -z "$key" ]]; then
          key="$1"
          shift
        else
          echo "E_COMMENT_BAD_FLAG: unexpected positional argument: $1" >&2
          return $E_COMMENT_BAD_FLAG
        fi
        ;;
    esac
  done

  if [[ -z "$key" ]]; then
    echo "E_COMMENT_NO_KEY: issue identifier required" >&2
    return $E_COMMENT_NO_KEY
  fi

  if [[ -n "$body_file" ]]; then
    if [[ ! -r "$body_file" ]]; then
      echo "E_COMMENT_NO_BODY: --body-file not readable: $body_file" >&2
      return $E_COMMENT_NO_BODY
    fi
    body=$(cat "$body_file")
    body_set=1
  fi

  if ((!body_set)) || [[ -z "$body" ]]; then
    echo "E_COMMENT_NO_BODY: supply --body or --body-file with non-empty content" >&2
    return $E_COMMENT_NO_BODY
  fi

  local input
  input=$(jq -cn --arg issue "$key" --arg body "$body" \
    '{issueId: $issue, body: $body}')

  if ((print_payload)); then
    jq -n --arg op "commentCreate" --argjson input "$input" \
      '{operation: $op, input: $input}'
    return 0
  fi

  if ! ((quiet)); then echo "INFO: adding comment to $key" >&2; fi

  local variables
  variables=$(jq -cn --argjson input "$input" '{input: $input}')
  # shellcheck disable=SC2016 # $input is a GraphQL variable
  local query='mutation($input: CommentCreateInput!) {
    commentCreate(input: $input) { success comment { id } }
  }'

  local resp req_exit=0
  resp=$(bash "$_LINEAR_COMMENT_SCRIPT_DIR/linear-graphql.sh" \
    --query "$query" --variables "$variables") || req_exit=$?
  if ((req_exit != 0)); then
    _linear_emit_generic_hint "$req_exit" || true
    return "$req_exit"
  fi
  printf '%s\n' "$resp"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _linear_comment "$@"
fi
