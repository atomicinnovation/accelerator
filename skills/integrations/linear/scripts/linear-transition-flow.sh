#!/usr/bin/env bash
# linear-transition-flow.sh — Transition a Linear issue to a target state,
# resolving the state name → UUID from the cached catalogue (no live lookup).
#
# Usage:
#   linear-transition-flow.sh <IDENTIFIER> <STATE-NAME> [flags]
#   linear-transition-flow.sh <IDENTIFIER> --state <STATE-NAME> [flags]
#
# Flags:
#   --describe   Dry-run: print the resolved transition and exit 0.
#   --quiet, -q  Suppress INFO stderr lines.
#   --help, -h   Print this banner and exit 0.
#
# The target state UUID is resolved DIRECTLY from catalogue.json — there is no
# live /transitions-equivalent lookup (the Jira divergence). Matching is
# case-insensitive and trimmed; a display name shared by two catalogue states is
# rejected as ambiguous rather than silently picking one.
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   120 E_TRANSITION_NO_KEY              no issue identifier supplied
#   121 E_TRANSITION_NO_STATE            no target state name supplied
#   122 E_TRANSITION_STATE_NOT_IN_CATALOGUE  state name not in catalogue
#   123 E_TRANSITION_STATE_AMBIGUOUS     two catalogue states share the name
#   124 E_TRANSITION_NO_CATALOGUE        catalogue.json missing; run /init-linear
#   125 E_TRANSITION_BAD_FLAG            unrecognised flag

set -euo pipefail

_LINEAR_TRANS_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_LINEAR_TRANS_SCRIPT_DIR/linear-common.sh"

readonly E_TRANSITION_NO_KEY=120
readonly E_TRANSITION_NO_STATE=121
readonly E_TRANSITION_STATE_NOT_IN_CATALOGUE=122
readonly E_TRANSITION_STATE_AMBIGUOUS=123
readonly E_TRANSITION_NO_CATALOGUE=124
readonly E_TRANSITION_BAD_FLAG=125

_linear_transition_usage() {
  cat <<'USAGE'
Usage: linear-transition-flow.sh <IDENTIFIER> <STATE-NAME> [flags]

  Transitions an issue to a target WorkflowState, resolving the state name to
  its UUID from the cached catalogue (no live lookup).

Flags:
  --describe   Dry-run: print the resolved transition and exit 0.
  --quiet, -q  Suppress INFO stderr lines.
  --help, -h   Print this banner and exit 0.
USAGE
}

_linear_transition() {
  linear_require_dependencies

  local key="" state_name="" describe=0 quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help | -h)
        _linear_transition_usage
        exit 0
        ;;
      --state)
        state_name="$2"
        shift 2
        ;;
      --describe)
        describe=1
        shift
        ;;
      --quiet | -q)
        quiet=1
        shift
        ;;
      -*)
        echo "E_TRANSITION_BAD_FLAG: unrecognised flag: $1" >&2
        _linear_transition_usage >&2
        return $E_TRANSITION_BAD_FLAG
        ;;
      *)
        if [[ -z "$key" ]]; then
          key="$1"
        elif [[ -z "$state_name" ]]; then
          state_name="$1"
        else
          echo "E_TRANSITION_BAD_FLAG: unexpected positional argument: $1" >&2
          return $E_TRANSITION_BAD_FLAG
        fi
        shift
        ;;
    esac
  done

  if [[ -z "$key" ]]; then
    echo "E_TRANSITION_NO_KEY: issue identifier required" >&2
    return $E_TRANSITION_NO_KEY
  fi
  if [[ -z "$state_name" ]]; then
    echo "E_TRANSITION_NO_STATE: target state name required" >&2
    return $E_TRANSITION_NO_STATE
  fi

  local state_dir cat
  state_dir=$(linear_state_dir) || return 1
  cat="$state_dir/catalogue.json"
  if [[ ! -f "$cat" ]]; then
    echo "E_TRANSITION_NO_CATALOGUE: catalogue.json missing; run /init-linear" >&2
    return $E_TRANSITION_NO_CATALOGUE
  fi

  # Resolve name → UUID(s) from the catalogue, case-insensitively and trimmed.
  # Collect ALL matching ids so a shared display name can be flagged ambiguous.
  local ids
  ids=$(jq -r --arg n "$state_name" '
    [.workflowStates[]
     | select((.name | ascii_downcase | gsub("^\\s+|\\s+$"; ""))
              == ($n | ascii_downcase | gsub("^\\s+|\\s+$"; "")))
     | .id] | .[]
  ' "$cat" 2>/dev/null) || ids=""

  local match_count
  match_count=$(printf '%s' "$ids" | grep -c . || true)
  if [[ "$match_count" -eq 0 ]]; then
    echo "E_TRANSITION_STATE_NOT_IN_CATALOGUE: state '$state_name' not in catalogue; run /init-linear to refresh" >&2
    return $E_TRANSITION_STATE_NOT_IN_CATALOGUE
  fi
  if [[ "$match_count" -gt 1 ]]; then
    echo "E_TRANSITION_STATE_AMBIGUOUS: state name '$state_name' matches $match_count catalogue states; cannot disambiguate" >&2
    return $E_TRANSITION_STATE_AMBIGUOUS
  fi
  local state_id
  state_id=$(printf '%s' "$ids" | head -1)

  if ((describe)); then
    jq -n --arg id "$key" --arg state "$state_name" --arg stateId "$state_id" \
      '{operation: "issueUpdate", id: $id, state: $state, stateId: $stateId}'
    return 0
  fi

  if ! ((quiet)); then
    echo "INFO: transitioning $key to '$state_name' (stateId $state_id)" >&2
  fi

  local variables
  variables=$(jq -cn --arg id "$key" --arg s "$state_id" '{id: $id, input: {stateId: $s}}')
  # shellcheck disable=SC2016 # $id/$input are GraphQL variables
  local query='mutation($id: String!, $input: IssueUpdateInput!) {
    issueUpdate(id: $id, input: $input) { success issue { id identifier state { name } } }
  }'

  local resp req_exit=0
  resp=$(bash "$_LINEAR_TRANS_SCRIPT_DIR/linear-graphql.sh" \
    --query "$query" --variables "$variables") || req_exit=$?
  if ((req_exit != 0)); then
    _linear_emit_generic_hint "$req_exit" || true
    return "$req_exit"
  fi
  printf '%s\n' "$resp"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _linear_transition "$@"
fi
