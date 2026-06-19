#!/usr/bin/env bash
# linear-search-flow.sh — Compose a Linear IssueFilter, search, paginate.
#
# Usage:
#   linear-search-flow.sh [flags]
#
# Flags:
#   --state NAME        Filter by WorkflowState name (resolved to its UUID via
#                       the catalogue; case-insensitive). Repeatable not
#                       supported — a single state per search.
#   --assignee NAME     Filter by assignee display name.
#   --label NAME        Filter by label name.
#   --text STR          Free-text match on issue title (case-insensitive).
#   --limit N           Page size (first:), 1..250 (default 50). Pagination
#                       follows all pages regardless.
#   --quiet, -q         Suppress the INFO filter audit line on stderr.
#   --help, -h          Print this banner and exit 0.
#
# There is NO --team flag: the team is implied by the catalogue (single-team
# scoping, fixed at /init-linear time).
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   70  E_SEARCH_BAD_FLAG     — unrecognised flag
#   71  E_SEARCH_BAD_LIMIT    — --limit not a positive integer in [1, 250]
#   72  E_SEARCH_NO_CATALOGUE — --state used but catalogue.json missing
#   73  E_SEARCH_BAD_STATE    — --state value not found in the catalogue

set -euo pipefail

_LINEAR_SEARCH_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_LINEAR_SEARCH_SCRIPT_DIR/linear-common.sh"

readonly E_SEARCH_BAD_FLAG=70
readonly E_SEARCH_BAD_LIMIT=71
readonly E_SEARCH_NO_CATALOGUE=72
readonly E_SEARCH_BAD_STATE=73

_linear_search_usage() {
  cat <<'USAGE'
Usage: linear-search-flow.sh [flags]

  Composes a Linear IssueFilter from flags, searches the catalogue's team,
  paginates, and emits the merged result.

Flags:
  --state NAME        WorkflowState name (resolved to UUID via catalogue).
  --assignee NAME     Assignee display name.
  --label NAME        Label name.
  --text STR          Free-text match on title (case-insensitive).
  --limit N           Page size, 1..250 (default 50).
  --quiet, -q         Suppress the INFO filter audit line.
  --help, -h          Print this banner and exit 0.
USAGE
}

# Resolve a WorkflowState name to its UUID via catalogue.json (case-insensitive,
# trimmed). Prints the UUID on success.
_linear_search_resolve_state() {
  local name="$1"
  local state_dir
  state_dir=$(linear_state_dir) || return 1
  local cat="$state_dir/catalogue.json"
  if [[ ! -f "$cat" ]]; then
    echo "E_SEARCH_NO_CATALOGUE: catalogue.json missing; run /init-linear" >&2
    return $E_SEARCH_NO_CATALOGUE
  fi
  local id
  id=$(jq -r --arg n "$name" '
    [.workflowStates[]
     | select((.name | ascii_downcase | gsub("^\\s+|\\s+$"; ""))
              == ($n | ascii_downcase | gsub("^\\s+|\\s+$"; "")))
     | .id]
    | (if length == 0 then "" else .[0] end)
  ' "$cat" 2>/dev/null) || id=""
  if [[ -z "$id" ]]; then
    echo "E_SEARCH_BAD_STATE: state '$name' not found in catalogue; run /init-linear to refresh" >&2
    return $E_SEARCH_BAD_STATE
  fi
  printf '%s' "$id"
}

_linear_search() {
  local state_name="" assignee="" label="" text=""
  local limit=50 quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help | -h)
        _linear_search_usage
        exit 0
        ;;
      --state)
        state_name="$2"
        shift 2
        ;;
      --assignee)
        assignee="$2"
        shift 2
        ;;
      --label)
        label="$2"
        shift 2
        ;;
      --text)
        text="$2"
        shift 2
        ;;
      --limit)
        limit="$2"
        shift 2
        if ! [[ "$limit" =~ ^[0-9]+$ ]] || ((limit < 1 || limit > 250)); then
          echo "E_SEARCH_BAD_LIMIT: --limit must be a positive integer between 1 and 250; got '$limit'" >&2
          return $E_SEARCH_BAD_LIMIT
        fi
        ;;
      --quiet | -q)
        quiet=1
        shift
        ;;
      *)
        echo "E_SEARCH_BAD_FLAG: unrecognised flag: $1" >&2
        _linear_search_usage >&2
        return $E_SEARCH_BAD_FLAG
        ;;
    esac
  done

  # Build the IssueFilter incrementally with jq conditional merges.
  local filter='{}'
  if [[ -n "$state_name" ]]; then
    local state_id
    state_id=$(_linear_search_resolve_state "$state_name") || return $?
    filter=$(jq -cn --argjson f "$filter" --arg id "$state_id" \
      '$f + {state: {id: {eq: $id}}}')
  fi
  if [[ -n "$assignee" ]]; then
    filter=$(jq -cn --argjson f "$filter" --arg a "$assignee" \
      '$f + {assignee: {name: {eqIgnoreCase: $a}}}')
  fi
  if [[ -n "$label" ]]; then
    filter=$(jq -cn --argjson f "$filter" --arg l "$label" \
      '$f + {labels: {name: {eq: $l}}}')
  fi
  if [[ -n "$text" ]]; then
    filter=$(jq -cn --argjson f "$filter" --arg t "$text" \
      '$f + {title: {containsIgnoreCase: $t}}')
  fi

  if ! ((quiet)); then echo "INFO: composed IssueFilter: $filter" >&2; fi

  local variables
  variables=$(jq -cn --argjson filter "$filter" --argjson first "$limit" \
    '{filter: $filter, first: $first}')

  # shellcheck disable=SC2016 # $cursor/$filter/$first are GraphQL variables
  local query='query($cursor: String, $filter: IssueFilter, $first: Int) {
    issues(first: $first, after: $cursor, filter: $filter) {
      nodes { id identifier title updatedAt state { name } assignee { name } }
      pageInfo { hasNextPage endCursor }
    }
  }'

  bash "$_LINEAR_SEARCH_SCRIPT_DIR/linear-graphql.sh" \
    --query "$query" --variables "$variables" --paginate .data.issues
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _linear_search "$@"
fi
