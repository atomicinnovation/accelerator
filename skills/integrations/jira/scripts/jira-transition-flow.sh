#!/usr/bin/env bash
# jira-transition-flow.sh — Transition a Jira issue through its workflow by state name.
#
# Usage:
#   jira-transition-flow.sh [--describe] KEY (STATE_NAME | --transition-id ID)
#            [--resolution NAME]
#            [--comment TEXT | --comment-file PATH]
#            [--no-notify]
#            [--quiet]
#            [--help | -h]
#
# Required:
#   KEY                      Issue key (positional), e.g. "ENG-1"
#
# State target (exactly one required):
#   STATE_NAME               Target workflow state name (case-insensitive match)
#   --transition-id ID       Numeric transition ID; bypasses state name GET
#
# Optional:
#   --resolution NAME        Set resolution field during transition
#   --comment TEXT           Inline comment body (Markdown → ADF)
#   --comment-file PATH      Comment body from file (Markdown → ADF)
#   --no-notify              Suppress watcher notifications (?notifyUsers=false)
#   --describe               Dry-run: print operation description and exit 0
#   --quiet                  Suppress INFO stderr lines
#   --help, -h               Print this banner and exit 0
#
# Exit codes:
#   120 E_TRANSITION_NO_KEY         no issue key positional argument
#   121 E_TRANSITION_NO_STATE       no target state name or --transition-id
#   122 E_TRANSITION_NOT_FOUND      no transition matches the given state name
#   123 E_TRANSITION_AMBIGUOUS      multiple transitions share the state name
#   124 E_TRANSITION_BAD_FLAG       unrecognised flag, conflicting args, or non-numeric --transition-id
#   125 E_TRANSITION_NO_BODY        --comment-file path invalid
#   126 E_TRANSITION_BAD_RESOLUTION empty or whitespace-only resolution value
#   127-129                         reserved
#   11-23, 34 propagated from jira-request.sh (auth/transport/4xx/5xx)
#
# See also: EXIT_CODES.md

_JIRA_TRANSITION_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_TRANSITION_SCRIPT_DIR/jira-common.sh"
source "$_JIRA_TRANSITION_SCRIPT_DIR/jira-body-input.sh"

_jira_transition_usage() {
  cat <<'USAGE'
Usage: jira-transition-flow.sh [--describe] KEY (STATE_NAME | --transition-id ID)
         [--resolution NAME]
         [--comment TEXT | --comment-file PATH]
         [--no-notify] [--quiet] [--help | -h]

Required:
  KEY           Issue key, e.g. "ENG-1"
  STATE_NAME    Target state name (case-insensitive match against available transitions)
    OR
  --transition-id ID   Numeric transition ID (skips state lookup GET)

Optional:
  --resolution NAME   Resolution field value set during transition
  --comment TEXT      Inline comment body (Markdown)
  --comment-file PATH Comment body from file (Markdown)
  --no-notify         Suppress notifications (?notifyUsers=false)
  --describe          Dry-run: print operation description and exit 0
  --quiet             Suppress INFO stderr lines
  --help, -h          Print this banner and exit 0

Examples:
  jira-transition-flow.sh ENG-1 "In Progress"
  jira-transition-flow.sh ENG-1 "Done" --resolution "Fixed"
  jira-transition-flow.sh ENG-1 --transition-id 21 --no-notify
USAGE
}

# _jira_transition_lookup KEY STATE_NAME
# Fetches available transitions for KEY and filters by state name (case-insensitive).
# On 0 matches: prints error to stderr, returns 122.
# On 2+ matches: prints JSON array to stdout, returns 123.
# On 1 match: prints single-element JSON array to stdout, returns 0.
_jira_transition_lookup() {
  local key="$1" state_name="$2"

  local req_exit=0 response=""
  response=$(bash "$_JIRA_TRANSITION_SCRIPT_DIR/jira-request.sh" \
    GET "/rest/api/3/issue/$key/transitions") || req_exit=$?

  if (( req_exit != 0 )); then
    if ! _jira_emit_generic_hint "$req_exit"; then
      case "$req_exit" in
        13) printf 'Hint: issue not found or you do not have permission to see it.\n' >&2 ;;
      esac
    fi
    return "$req_exit"
  fi

  local matches
  matches=$(printf '%s' "$response" | jq -c \
    --arg s "$state_name" \
    '[.transitions[] | select(.to.name | ascii_downcase == ($s | ascii_downcase))]')

  local count
  count=$(printf '%s' "$matches" | jq 'length')

  if (( count == 0 )); then
    printf 'E_TRANSITION_NOT_FOUND: no transition leads to state "%s" from the current state\n' \
      "$state_name" >&2
    return 122
  fi

  if (( count > 1 )); then
    printf '%s\n' "$matches"
    return 123
  fi

  printf '%s\n' "$matches"
  return 0
}

_jira_transition() {
  jira_require_dependencies

  local key=""
  local state_name=""
  local transition_id=""
  local resolution="" resolution_set=0
  local comment_text="" comment_text_set=0
  local comment_file="" comment_file_set=0
  local no_notify=0
  local describe=0
  local quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help|-h)
        _jira_transition_usage; exit 0 ;;
      --describe)
        describe=1; shift ;;
      --transition-id)
        transition_id="$2"; shift 2 ;;
      --resolution)
        resolution="$2"; resolution_set=1; shift 2 ;;
      --comment)
        comment_text="$2"; comment_text_set=1; shift 2 ;;
      --comment-file)
        comment_file="$2"; comment_file_set=1; shift 2 ;;
      --no-notify)
        no_notify=1; shift ;;
      --quiet|-q)
        quiet=1; shift ;;
      -*)
        printf 'E_TRANSITION_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        _jira_transition_usage >&2
        return 124 ;;
      *)
        if [[ -z "$key" ]]; then
          key="$1"; shift
        elif [[ -z "$state_name" ]]; then
          state_name="$1"; shift
        else
          printf 'E_TRANSITION_BAD_FLAG: unexpected positional argument: %s\n' "$1" >&2
          _jira_transition_usage >&2
          return 124
        fi ;;
    esac
  done

  # --- Post-parse validation ---

  if [[ -z "$key" ]]; then
    printf 'E_TRANSITION_NO_KEY: issue key required as first positional argument\n' >&2
    return 120
  fi

  # Both STATE_NAME and --transition-id supplied → conflict
  if [[ -n "$state_name" && -n "$transition_id" ]]; then
    printf 'E_TRANSITION_BAD_FLAG: STATE_NAME and --transition-id are mutually exclusive\n' >&2
    _jira_transition_usage >&2
    return 124
  fi

  # --transition-id must be a positive integer
  if [[ -n "$transition_id" ]] && ! [[ "$transition_id" =~ ^[0-9]+$ ]]; then
    printf 'E_TRANSITION_BAD_FLAG: --transition-id must be a positive integer, got: %s\n' \
      "$transition_id" >&2
    return 124
  fi

  # At least one of STATE_NAME or --transition-id is required
  if [[ -z "$state_name" && -z "$transition_id" ]]; then
    printf 'E_TRANSITION_NO_STATE: target state name or --transition-id is required\n' >&2
    return 121
  fi

  # Validate --comment-file path
  if (( comment_file_set )); then
    if [[ "$comment_file" == -* ]]; then
      printf 'E_TRANSITION_NO_BODY: --comment-file path must not begin with "-": %s\n' \
        "$comment_file" >&2
      return 125
    fi
    if [[ -L "$comment_file" ]]; then
      local resolved
      resolved=$(readlink -f "$comment_file" 2>/dev/null || true)
      case "$resolved" in
        /dev/*|/proc/*|/sys/*)
          printf 'E_TRANSITION_NO_BODY: --comment-file resolves to a device path: %s\n' \
            "$comment_file" >&2
          return 125 ;;
      esac
    fi
    if ! [[ -f "$comment_file" && -r "$comment_file" ]]; then
      printf 'E_TRANSITION_NO_BODY: --comment-file not found or not readable: %s\n' \
        "$comment_file" >&2
      return 125
    fi
  fi

  # Validate --resolution value
  if (( resolution_set )); then
    local trimmed="${resolution//[[:space:]]/}"
    if [[ -z "$trimmed" ]]; then
      printf 'E_TRANSITION_BAD_RESOLUTION: --resolution value must not be empty or whitespace-only\n' >&2
      return 126
    fi
  fi

  # --- --describe branch ---

  if (( describe )); then
    local resolution_json="null"
    if (( resolution_set )); then
      resolution_json=$(jq -n --arg v "$resolution" '$v')
    fi
    local comment_bool="false"
    if (( comment_text_set || comment_file_set )); then
      comment_bool="true"
    fi

    if [[ -n "$transition_id" ]]; then
      # Known ID: no network call needed
      jq -n \
        --arg     key   "$key" \
        --argjson state "null" \
        --arg     tid   "$transition_id" \
        --argjson res   "$resolution_json" \
        --argjson com   "$comment_bool" \
        '{"key":$key,"state":$state,"transition_id":$tid,"resolution":$res,"comment":$com}'
      return 0
    fi

    # STATE_NAME path: resolve eagerly via GET
    local lookup_rc=0 matches=""
    matches=$(_jira_transition_lookup "$key" "$state_name") || lookup_rc=$?
    if (( lookup_rc == 123 )); then
      printf '%s\n' "$matches"
      return 123
    fi
    if (( lookup_rc != 0 )); then
      return "$lookup_rc"
    fi

    local resolved_id
    resolved_id=$(printf '%s' "$matches" | jq -r '.[0].id')

    jq -n \
      --arg     key   "$key" \
      --arg     state "$state_name" \
      --arg     tid   "$resolved_id" \
      --argjson res   "$resolution_json" \
      --argjson com   "$comment_bool" \
      '{"key":$key,"state":$state,"transition_id":$tid,"resolution":$res,"comment":$com}'
    return 0
  fi

  # --- Live path ---

  # Resolve transition ID (skip GET if --transition-id already supplied)
  if [[ -z "$transition_id" ]]; then
    local lookup_rc=0 matches=""
    matches=$(_jira_transition_lookup "$key" "$state_name") || lookup_rc=$?
    if (( lookup_rc != 0 )); then
      if (( lookup_rc == 123 )); then
        printf '%s\n' "$matches"
      fi
      return "$lookup_rc"
    fi
    transition_id=$(printf '%s' "$matches" | jq -r '.[0].id')
  fi

  # Resolve comment body → ADF (if supplied)
  local adf_comment=""
  if (( comment_text_set || comment_file_set )); then
    local body_src_args=()
    if (( comment_text_set )); then body_src_args+=(--body "$comment_text"); fi
    if (( comment_file_set )); then body_src_args+=(--body-file "$comment_file"); fi

    local body_md="" body_rc=0
    body_md=$(jira_resolve_body "${body_src_args[@]}") || body_rc=$?
    if (( body_rc != 0 )); then
      printf 'E_TRANSITION_NO_BODY: comment body resolution failed\n' >&2
      return 125
    fi

    local adf_rc=0
    adf_comment=$(printf '%s' "$body_md" \
      | bash "$_JIRA_TRANSITION_SCRIPT_DIR/jira-md-to-adf.sh") || adf_rc=$?
    if (( adf_rc != 0 )); then
      printf 'E_TRANSITION_NO_BODY: failed to convert comment to ADF\n' >&2
      return 125
    fi
  fi

  # Build POST body using incremental-merge pattern
  local transition_obj fields_obj update_obj payload

  transition_obj=$(jq -n --arg id "$transition_id" '{"id": $id}')

  fields_obj="{}"
  if (( resolution_set )); then
    fields_obj=$(jq -n --arg r "$resolution" '{"resolution": {"name": $r}}')
  fi

  update_obj="{}"
  if [[ -n "$adf_comment" ]]; then
    update_obj=$(jq -n --argjson body "$adf_comment" \
      '{"comment": [{"add": {"body": $body}}]}')
  fi

  payload=$(jq -n \
    --argjson transition "$transition_obj" \
    --argjson fields     "$fields_obj" \
    --argjson update     "$update_obj" \
    '{transition: $transition} +
      (if $fields == {} then {} else {fields: $fields} end) +
      (if $update == {} then {} else {update: $update} end)')

  # Build query params
  local -a query_params=()
  if (( no_notify )); then
    query_params+=(--query "notifyUsers=false")
  fi

  if ! (( quiet )); then
    printf 'INFO: transitioning issue %s\n' "$key" >&2
  fi

  local tmpfile; tmpfile=$(mktemp)
  trap 'rm -f "$tmpfile"; trap - RETURN' RETURN
  printf '%s' "$payload" > "$tmpfile"

  local req_exit=0
  bash "$_JIRA_TRANSITION_SCRIPT_DIR/jira-request.sh" \
    POST "/rest/api/3/issue/$key/transitions" \
    --json "@$tmpfile" \
    "${query_params[@]+"${query_params[@]}"}" || req_exit=$?

  if (( req_exit != 0 )); then
    if ! _jira_emit_generic_hint "$req_exit"; then
      case "$req_exit" in
        12) printf 'Hint: you do not have the TRANSITION_ISSUES permission on this project.\n' >&2 ;;
        13) printf 'Hint: issue not found or you do not have permission to see it.\n' >&2 ;;
      esac
    fi
    return "$req_exit"
  fi
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail
  _jira_transition "$@"
fi
