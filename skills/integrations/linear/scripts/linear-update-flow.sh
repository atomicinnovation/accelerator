#!/usr/bin/env bash
# linear-update-flow.sh — Update fields on an existing Linear issue.
#
# Usage:
#   linear-update-flow.sh <IDENTIFIER> [flags]
#
# Flags (at least one mutating flag required):
#   --title TEXT        New title.
#   --description TEXT  New description (Markdown).
#   --state NAME        New WorkflowState (resolved to its UUID via catalogue).
#   --assignee-id ID    New assignee user UUID.
#   --priority N        New priority (Linear integer 0-4).
#   --print-payload     Dry-run: print operation + id + input, exit 0.
#   --quiet, -q         Suppress INFO stderr lines.
#   --help, -h          Print this banner and exit 0.
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   110 E_UPDATE_NO_KEY       no issue identifier supplied
#   111 E_UPDATE_NO_OPS       no mutating flags supplied
#   112 E_UPDATE_BAD_FLAG     unrecognised flag
#   113 E_UPDATE_NO_CATALOGUE --state used but catalogue.json missing
#   114 E_UPDATE_BAD_STATE    --state value not found in the catalogue

set -euo pipefail

_LINEAR_UPDATE_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_LINEAR_UPDATE_SCRIPT_DIR/linear-common.sh"

readonly E_UPDATE_NO_KEY=110
readonly E_UPDATE_NO_OPS=111
readonly E_UPDATE_BAD_FLAG=112
readonly E_UPDATE_NO_CATALOGUE=113
readonly E_UPDATE_BAD_STATE=114

_linear_update_usage() {
  cat <<'USAGE'
Usage: linear-update-flow.sh <IDENTIFIER> [flags]

  Updates fields on an existing Linear issue via issueUpdate.

Flags (at least one required):
  --title TEXT        New title.
  --description TEXT  New description (Markdown).
  --state NAME        New WorkflowState (resolved to UUID via catalogue).
  --assignee-id ID    New assignee user UUID.
  --priority N        New priority (0-4).
  --print-payload     Dry-run: print operation + input, exit 0.
  --quiet, -q         Suppress INFO stderr lines.
  --help, -h          Print this banner and exit 0.
USAGE
}

# Resolve a WorkflowState name → UUID via catalogue.json (case-insensitive).
_linear_update_resolve_state() {
  local name="$1"
  local state_dir
  state_dir=$(linear_state_dir) || return 1
  local cat="$state_dir/catalogue.json"
  if [[ ! -f "$cat" ]]; then
    echo "E_UPDATE_NO_CATALOGUE: catalogue.json missing; run /init-linear" >&2
    return $E_UPDATE_NO_CATALOGUE
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
    echo "E_UPDATE_BAD_STATE: state '$name' not found in catalogue; run /init-linear to refresh" >&2
    return $E_UPDATE_BAD_STATE
  fi
  printf '%s' "$id"
}

_linear_update() {
  linear_require_dependencies

  local key="" title="" description="" state_name="" assignee_id="" priority=""
  local title_set=0 desc_set=0 print_payload=0 quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help | -h)
        _linear_update_usage
        exit 0
        ;;
      --title)
        title="$2"
        title_set=1
        shift 2
        ;;
      --description)
        description="$2"
        desc_set=1
        shift 2
        ;;
      --state)
        state_name="$2"
        shift 2
        ;;
      --assignee-id)
        assignee_id="$2"
        shift 2
        ;;
      --priority)
        priority="$2"
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
        echo "E_UPDATE_BAD_FLAG: unrecognised flag: $1" >&2
        _linear_update_usage >&2
        return $E_UPDATE_BAD_FLAG
        ;;
      *)
        if [[ -z "$key" ]]; then
          key="$1"
          shift
        else
          echo "E_UPDATE_BAD_FLAG: unexpected positional argument: $1" >&2
          return $E_UPDATE_BAD_FLAG
        fi
        ;;
    esac
  done

  if [[ -z "$key" ]]; then
    echo "E_UPDATE_NO_KEY: issue identifier required" >&2
    return $E_UPDATE_NO_KEY
  fi

  if ((!title_set)) && ((!desc_set)) && [[ -z "$state_name" && -z "$assignee_id" && -z "$priority" ]]; then
    echo "E_UPDATE_NO_OPS: supply at least one of --title/--description/--state/--assignee-id/--priority" >&2
    return $E_UPDATE_NO_OPS
  fi

  # Build the IssueUpdateInput incrementally.
  local input='{}'
  if ((title_set)); then
    input=$(jq -cn --argjson i "$input" --arg t "$title" '$i + {title: $t}')
  fi
  if ((desc_set)); then
    input=$(jq -cn --argjson i "$input" --arg d "$description" '$i + {description: $d}')
  fi
  if [[ -n "$state_name" ]]; then
    local state_id
    state_id=$(_linear_update_resolve_state "$state_name") || return $?
    input=$(jq -cn --argjson i "$input" --arg s "$state_id" '$i + {stateId: $s}')
  fi
  if [[ -n "$assignee_id" ]]; then
    input=$(jq -cn --argjson i "$input" --arg a "$assignee_id" '$i + {assigneeId: $a}')
  fi
  if [[ -n "$priority" ]]; then
    if ! [[ "$priority" =~ ^[0-9]+$ ]]; then
      echo "E_UPDATE_BAD_FLAG: --priority must be an integer (0-4); got '$priority'" >&2
      return $E_UPDATE_BAD_FLAG
    fi
    input=$(jq -cn --argjson i "$input" --argjson p "$priority" '$i + {priority: $p}')
  fi

  if ((print_payload)); then
    jq -n --arg op "issueUpdate" --arg id "$key" --argjson input "$input" \
      '{operation: $op, id: $id, input: $input}'
    return 0
  fi

  if ! ((quiet)); then echo "INFO: updating issue $key" >&2; fi

  local variables
  variables=$(jq -cn --arg id "$key" --argjson input "$input" '{id: $id, input: $input}')
  # shellcheck disable=SC2016 # $id/$input are GraphQL variables
  local query='mutation($id: String!, $input: IssueUpdateInput!) {
    issueUpdate(id: $id, input: $input) { success issue { id identifier } }
  }'

  local resp req_exit=0
  resp=$(bash "$_LINEAR_UPDATE_SCRIPT_DIR/linear-graphql.sh" \
    --query "$query" --variables "$variables") || req_exit=$?
  if ((req_exit != 0)); then
    _linear_emit_generic_hint "$req_exit" || true
    return "$req_exit"
  fi
  printf '%s\n' "$resp"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _linear_update "$@"
fi
