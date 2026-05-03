#!/usr/bin/env bash
# jira-comment-flow.sh — Manage comments on a Jira issue.
#
# Usage:
#   jira-comment-flow.sh add    KEY [--body TEXT | --body-file PATH]
#                                   [--visibility role:NAME | group:NAME]
#                                   [--render-adf | --no-render-adf]
#                                   [--no-notify] [--print-payload] [--no-editor]
#   jira-comment-flow.sh list   KEY [--page-size N] [--first-page-only]
#                                   [--render-adf | --no-render-adf]
#   jira-comment-flow.sh edit   KEY COMMENT_ID
#                                   [--body TEXT | --body-file PATH]
#                                   [--visibility role:NAME | group:NAME]
#                                   [--render-adf | --no-render-adf]
#                                   [--no-notify] [--print-payload] [--no-editor]
#   jira-comment-flow.sh delete KEY COMMENT_ID [--no-notify] [--describe]
#   jira-comment-flow.sh --help | -h
#
# Exit codes (range 91–99):
#   91  E_COMMENT_NO_SUBCOMMAND   No subcommand provided
#   92  E_COMMENT_BAD_SUBCOMMAND  Unknown subcommand
#   93  E_COMMENT_NO_KEY          No issue key positional argument
#   94  E_COMMENT_NO_BODY         add/edit: no body source available
#   95  E_COMMENT_NO_ID           edit/delete: no comment id argument
#   96  E_COMMENT_BAD_FLAG        Unrecognised flag
#   97  E_COMMENT_BAD_PAGE_SIZE   --page-size not in [1, 100]
#   98  E_COMMENT_BAD_VISIBILITY  --visibility not in form role:NAME or group:NAME
#   99  E_COMMENT_BAD_RESPONSE    .total or .comments|length not an integer
#   11–23, 34 propagated from jira-request.sh (auth/transport/4xx/5xx)
#
# See also: EXIT_CODES.md

_JIRA_COMMENT_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_COMMENT_SCRIPT_DIR/jira-common.sh"
source "$_JIRA_COMMENT_SCRIPT_DIR/jira-body-input.sh"

_jira_comment_usage() {
  cat <<'USAGE'
Usage: jira-comment-flow.sh <subcommand> [args]

Subcommands:
  add    KEY [--body TEXT | --body-file PATH] [--visibility TYPE:NAME]
             [--render-adf | --no-render-adf] [--no-notify] [--print-payload]
  list   KEY [--page-size N] [--first-page-only]
             [--render-adf | --no-render-adf]
  edit   KEY COMMENT_ID [--body TEXT | --body-file PATH] [--visibility TYPE:NAME]
             [--render-adf | --no-render-adf] [--no-notify] [--print-payload]
  delete KEY COMMENT_ID [--no-notify] [--describe]

Options:
  --body TEXT          Inline comment body (Markdown)
  --body-file PATH     Comment body from file (Markdown)
  --visibility TYPE:NAME   Visibility restriction; TYPE is role or group
  --render-adf         Render ADF fields to Markdown in response (default: on)
  --no-render-adf      Return raw ADF in response
  --no-notify          Suppress watcher notifications (adds ?notifyUsers=false)
  --print-payload      Dry-run for add/edit: print payload JSON and exit 0
  --describe           Dry-run for delete: print operation description and exit 0
  --page-size N        Comments per page for list [1..100] (default: 50)
  --first-page-only    Return first page only without paginating
  --no-editor          Disallow $EDITOR fallback for body
  --help, -h           Print this banner and exit 0
USAGE
}

_jira_comment_add() {
  local key="" body_inline="" body_file=""
  local body_inline_set=0 body_file_set=0
  local visibility="" no_notify=0 render_adf=1 print_payload=0 no_editor=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --body)          body_inline="$2"; body_inline_set=1; shift 2 ;;
      --body-file)     body_file="$2"; body_file_set=1; shift 2 ;;
      --visibility)    visibility="$2"; shift 2 ;;
      --no-notify)     no_notify=1; shift ;;
      --render-adf)    render_adf=1; shift ;;
      --no-render-adf) render_adf=0; shift ;;
      --print-payload) print_payload=1; shift ;;
      --no-editor)     no_editor=1; shift ;;
      --*)
        printf 'E_COMMENT_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        return 96 ;;
      *)
        if [[ -z "$key" ]]; then key="$1"
        else printf 'E_COMMENT_BAD_FLAG: unexpected argument: %s\n' "$1" >&2; return 96
        fi; shift ;;
    esac
  done

  if [[ -z "$key" ]]; then
    printf 'E_COMMENT_NO_KEY: issue key required as first positional argument\n' >&2
    return 93
  fi

  if [[ -n "$visibility" ]] && ! [[ "$visibility" =~ ^(role|group):.+ ]]; then
    printf 'E_COMMENT_BAD_VISIBILITY: --visibility must be role:NAME or group:NAME (got: %s)\n' \
      "$visibility" >&2
    return 98
  fi

  local body_src_args=()
  (( body_inline_set )) && body_src_args+=(--body "$body_inline")
  (( body_file_set ))   && body_src_args+=(--body-file "$body_file")
  if (( no_editor )); then
    body_src_args+=(--allow-stdin)
  else
    body_src_args+=(--allow-stdin --allow-editor)
  fi

  local body_md="" body_rc=0
  body_md=$(jira_resolve_body "${body_src_args[@]}") || body_rc=$?
  if (( body_rc != 0 )); then
    printf 'E_COMMENT_NO_BODY: no body source available (use --body, --body-file, stdin, or $EDITOR)\n' >&2
    return 94
  fi

  local adf_doc="{}"
  if [[ -n "$body_md" ]]; then
    local adf_rc=0
    adf_doc=$(printf '%s' "$body_md" \
      | bash "$_JIRA_COMMENT_SCRIPT_DIR/jira-md-to-adf.sh") || adf_rc=$?
    if (( adf_rc != 0 )); then
      printf 'Warning: body Markdown could not be converted to ADF (exit %d); body will be empty\n' \
        "$adf_rc" >&2
      adf_doc="{}"
    fi
  fi

  local payload
  payload=$(jq -n --argjson body "$adf_doc" '{body: $body}')
  if [[ -n "$visibility" ]]; then
    local vis_type="${visibility%%:*}" vis_value="${visibility#*:}"
    payload=$(jq -n --argjson p "$payload" --arg t "$vis_type" --arg v "$vis_value" \
      '$p + {visibility: {type: $t, value: $v}}')
  fi

  local -a query_params=()
  if (( no_notify )); then query_params+=(--query "notifyUsers=false"); fi

  if (( print_payload )); then
    local qp_obj="{}"
    if (( no_notify )); then qp_obj='{"notifyUsers":"false"}'; fi
    jq -n \
      --arg     method "POST" \
      --arg     path   "/rest/api/3/issue/$key/comment" \
      --argjson qp     "$qp_obj" \
      --argjson body   "$payload" \
      '{method:$method, path:$path, queryParams:$qp, body:$body}'
    return 0
  fi

  local tmpfile; tmpfile=$(mktemp)
  trap 'rm -f "$tmpfile"; trap - RETURN' RETURN
  printf '%s' "$payload" > "$tmpfile"

  local req_exit=0 response=""
  response=$(bash "$_JIRA_COMMENT_SCRIPT_DIR/jira-request.sh" \
    POST "/rest/api/3/issue/$key/comment" \
    --json "@$tmpfile" \
    "${query_params[@]+"${query_params[@]}"}") || req_exit=$?

  if (( req_exit != 0 )); then
    if ! _jira_emit_generic_hint "$req_exit"; then
      case "$req_exit" in
        13) printf 'Hint: issue or comment not found, or you do not have permission.\n' >&2 ;;
      esac
    fi
    return "$req_exit"
  fi

  if (( render_adf )); then
    response=$(printf '%s' "$response" \
      | bash "$_JIRA_COMMENT_SCRIPT_DIR/jira-render-adf-fields.sh") || return $?
  fi
  printf '%s\n' "$response"
}

_jira_comment_list() {
  local key="" page_size=50 first_page_only=0 render_adf=1

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --page-size)      page_size="$2"; shift 2 ;;
      --first-page-only) first_page_only=1; shift ;;
      --render-adf)     render_adf=1; shift ;;
      --no-render-adf)  render_adf=0; shift ;;
      --*)
        printf 'E_COMMENT_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        return 96 ;;
      *)
        if [[ -z "$key" ]]; then key="$1"
        else printf 'E_COMMENT_BAD_FLAG: unexpected argument: %s\n' "$1" >&2; return 96
        fi; shift ;;
    esac
  done

  if [[ -z "$key" ]]; then
    printf 'E_COMMENT_NO_KEY: issue key required as first positional argument\n' >&2
    return 93
  fi

  if ! [[ "$page_size" =~ ^[0-9]+$ ]] || (( page_size < 1 || page_size > 100 )); then
    printf 'E_COMMENT_BAD_PAGE_SIZE: --page-size must be an integer in [1, 100] (got: %s)\n' \
      "$page_size" >&2
    return 97
  fi

  local start_at=0
  local accumulated='[]'
  local total=0
  local page_count=0
  local truncated=0
  local MAX_PAGES=20

  while :; do
    if (( page_count >= MAX_PAGES )); then
      truncated=1; break
    fi

    local resp="" req_exit=0
    resp=$(bash "$_JIRA_COMMENT_SCRIPT_DIR/jira-request.sh" \
      GET "/rest/api/3/issue/$key/comment" \
      --query "startAt=$start_at" \
      --query "maxResults=$page_size") || req_exit=$?

    if (( req_exit != 0 )); then
      if ! _jira_emit_generic_hint "$req_exit"; then
        case "$req_exit" in
          13) printf 'Hint: issue or comment not found, or you do not have permission.\n' >&2 ;;
        esac
      fi
      return "$req_exit"
    fi

    local page_comments page_total page_returned
    page_comments=$(printf '%s' "$resp" | jq '.comments')
    page_total=$(printf '%s' "$resp" | jq '.total')
    page_returned=$(printf '%s' "$resp" | jq '.comments | length')

    if [[ ! "$page_total" =~ ^[0-9]+$ ]] || [[ ! "$page_returned" =~ ^[0-9]+$ ]]; then
      printf 'E_COMMENT_BAD_RESPONSE: .total or .comments|length is not an integer\n' >&2
      return 99
    fi

    total="$page_total"
    accumulated=$(jq -n --argjson a "$accumulated" --argjson b "$page_comments" '$a + $b')
    page_count=$(( page_count + 1 ))
    if (( first_page_only )); then break; fi
    if (( page_returned == 0 )); then break; fi
    start_at=$(( start_at + page_returned ))
    if (( start_at >= page_total )); then break; fi
  done

  if (( truncated )); then
    printf 'Warning: truncated comment list at %d pages (page_size=%d). Use --page-size 100 (max) to reduce round-trips, or --first-page-only to fetch only the first page.\n' \
      "$MAX_PAGES" "$page_size" >&2
  fi

  local response
  response=$(jq -n \
    --argjson c     "$accumulated" \
    --argjson t     "$total" \
    --argjson trunc "$truncated" \
    '{startAt: 0, maxResults: ($c | length), total: $t,
      truncated: ($trunc != 0), comments: $c}')

  if (( render_adf )); then
    response=$(printf '%s' "$response" \
      | bash "$_JIRA_COMMENT_SCRIPT_DIR/jira-render-adf-fields.sh") || return $?
  fi
  printf '%s\n' "$response"
}

_jira_comment_edit() {
  local key="" comment_id="" body_inline="" body_file=""
  local body_inline_set=0 body_file_set=0
  local visibility="" no_notify=0 render_adf=1 print_payload=0 no_editor=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --body)          body_inline="$2"; body_inline_set=1; shift 2 ;;
      --body-file)     body_file="$2"; body_file_set=1; shift 2 ;;
      --visibility)    visibility="$2"; shift 2 ;;
      --no-notify)     no_notify=1; shift ;;
      --render-adf)    render_adf=1; shift ;;
      --no-render-adf) render_adf=0; shift ;;
      --print-payload) print_payload=1; shift ;;
      --no-editor)     no_editor=1; shift ;;
      --*)
        printf 'E_COMMENT_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        return 96 ;;
      *)
        if [[ -z "$key" ]]; then key="$1"
        elif [[ -z "$comment_id" ]]; then comment_id="$1"
        else printf 'E_COMMENT_BAD_FLAG: unexpected argument: %s\n' "$1" >&2; return 96
        fi; shift ;;
    esac
  done

  if [[ -z "$key" ]]; then
    printf 'E_COMMENT_NO_KEY: issue key required as first positional argument\n' >&2
    return 93
  fi
  if [[ -z "$comment_id" ]]; then
    printf 'E_COMMENT_NO_ID: comment id required as second positional argument\n' >&2
    return 95
  fi

  if [[ -n "$visibility" ]] && ! [[ "$visibility" =~ ^(role|group):.+ ]]; then
    printf 'E_COMMENT_BAD_VISIBILITY: --visibility must be role:NAME or group:NAME (got: %s)\n' \
      "$visibility" >&2
    return 98
  fi

  local body_src_args=()
  (( body_inline_set )) && body_src_args+=(--body "$body_inline")
  (( body_file_set ))   && body_src_args+=(--body-file "$body_file")
  if (( no_editor )); then
    body_src_args+=(--allow-stdin)
  else
    body_src_args+=(--allow-stdin --allow-editor)
  fi

  local body_md="" body_rc=0
  body_md=$(jira_resolve_body "${body_src_args[@]}") || body_rc=$?
  if (( body_rc != 0 )); then
    printf 'E_COMMENT_NO_BODY: no body source available (use --body, --body-file, stdin, or $EDITOR)\n' >&2
    return 94
  fi

  local adf_doc="{}"
  if [[ -n "$body_md" ]]; then
    local adf_rc=0
    adf_doc=$(printf '%s' "$body_md" \
      | bash "$_JIRA_COMMENT_SCRIPT_DIR/jira-md-to-adf.sh") || adf_rc=$?
    if (( adf_rc != 0 )); then
      printf 'Warning: body Markdown could not be converted to ADF (exit %d); body will be empty\n' \
        "$adf_rc" >&2
      adf_doc="{}"
    fi
  fi

  local payload
  payload=$(jq -n --argjson body "$adf_doc" '{body: $body}')
  if [[ -n "$visibility" ]]; then
    local vis_type="${visibility%%:*}" vis_value="${visibility#*:}"
    payload=$(jq -n --argjson p "$payload" --arg t "$vis_type" --arg v "$vis_value" \
      '$p + {visibility: {type: $t, value: $v}}')
  fi

  local -a query_params=()
  if (( no_notify )); then query_params+=(--query "notifyUsers=false"); fi

  if (( print_payload )); then
    local qp_obj="{}"
    if (( no_notify )); then qp_obj='{"notifyUsers":"false"}'; fi
    jq -n \
      --arg     method "PUT" \
      --arg     path   "/rest/api/3/issue/$key/comment/$comment_id" \
      --argjson qp     "$qp_obj" \
      --argjson body   "$payload" \
      '{method:$method, path:$path, queryParams:$qp, body:$body}'
    return 0
  fi

  local tmpfile; tmpfile=$(mktemp)
  trap 'rm -f "$tmpfile"; trap - RETURN' RETURN
  printf '%s' "$payload" > "$tmpfile"

  local req_exit=0 response=""
  response=$(bash "$_JIRA_COMMENT_SCRIPT_DIR/jira-request.sh" \
    PUT "/rest/api/3/issue/$key/comment/$comment_id" \
    --json "@$tmpfile" \
    "${query_params[@]+"${query_params[@]}"}") || req_exit=$?

  if (( req_exit != 0 )); then
    if ! _jira_emit_generic_hint "$req_exit"; then
      case "$req_exit" in
        13) printf 'Hint: issue or comment not found, or you do not have permission.\n' >&2 ;;
      esac
    fi
    return "$req_exit"
  fi

  if (( render_adf )); then
    response=$(printf '%s' "$response" \
      | bash "$_JIRA_COMMENT_SCRIPT_DIR/jira-render-adf-fields.sh") || return $?
  fi
  printf '%s\n' "$response"
}

_jira_comment_delete() {
  local key="" comment_id="" no_notify=0 describe=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --no-notify) no_notify=1; shift ;;
      --describe)  describe=1; shift ;;
      --*)
        printf 'E_COMMENT_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        return 96 ;;
      *)
        if [[ -z "$key" ]]; then key="$1"
        elif [[ -z "$comment_id" ]]; then comment_id="$1"
        else printf 'E_COMMENT_BAD_FLAG: unexpected argument: %s\n' "$1" >&2; return 96
        fi; shift ;;
    esac
  done

  # Validate required args before --describe short-circuit
  if [[ -z "$key" ]]; then
    printf 'E_COMMENT_NO_KEY: issue key required as first positional argument\n' >&2
    return 93
  fi
  if [[ -z "$comment_id" ]]; then
    printf 'E_COMMENT_NO_ID: comment id required as second positional argument\n' >&2
    return 95
  fi

  local -a query_params=()
  if (( no_notify )); then query_params+=(--query "notifyUsers=false"); fi

  if (( describe )); then
    local qp_obj="{}"
    if (( no_notify )); then qp_obj='{"notifyUsers":"false"}'; fi
    jq -n \
      --arg     method "DELETE" \
      --arg     path   "/rest/api/3/issue/$key/comment/$comment_id" \
      --argjson qp     "$qp_obj" \
      '{method:$method, path:$path, queryParams:$qp, body:null, irreversible:true}'
    return 0
  fi

  local req_exit=0
  bash "$_JIRA_COMMENT_SCRIPT_DIR/jira-request.sh" \
    DELETE "/rest/api/3/issue/$key/comment/$comment_id" \
    "${query_params[@]+"${query_params[@]}"}" >/dev/null || req_exit=$?

  if (( req_exit != 0 )); then
    if ! _jira_emit_generic_hint "$req_exit"; then
      case "$req_exit" in
        13) printf 'Hint: issue or comment not found, or you do not have permission.\n' >&2 ;;
      esac
    fi
    return "$req_exit"
  fi

  return 0
}

_jira_comment() {
  jira_require_dependencies
  local sub="${1:-}"; shift || true
  case "$sub" in
    add)    _jira_comment_add    "$@" ;;
    list)   _jira_comment_list   "$@" ;;
    edit)   _jira_comment_edit   "$@" ;;
    delete) _jira_comment_delete "$@" ;;
    -h|--help)
      _jira_comment_usage
      return 0 ;;
    "")
      _jira_comment_usage >&2
      printf 'E_COMMENT_NO_SUBCOMMAND: a subcommand is required (add, list, edit, delete)\n' >&2
      return 91 ;;
    *)
      printf 'E_COMMENT_BAD_SUBCOMMAND: unknown subcommand: %s\n' "$sub" >&2
      _jira_comment_usage >&2
      return 92 ;;
  esac
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail
  _jira_comment "$@"
fi
