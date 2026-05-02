#!/usr/bin/env bash
# jira-show-flow.sh — Fetch a single Jira issue.
#
# Usage:
#   jira-show-flow.sh <ISSUE-KEY> [flags]
#
# Flags:
#   --fields a,b,c | --fields a   Field tokens (CSV or repeatable, default: *all).
#   --expand a,b,c  Override default expand (default: names,schema,transitions).
#   --comments N    Include last N embedded comments (0 = omit, default 0).
#   --render-adf    Render ADF descriptions/comments to Markdown (default ON).
#   --no-render-adf Keep ADF as-is (JSON objects) in the response.
#   --help, -h      Print this banner and exit 0.
#
# Exit codes:
#   0   success
#   80  E_SHOW_NO_KEY           — no issue key supplied
#   81  E_SHOW_BAD_COMMENTS_LIMIT — --comments not in [0, 100]
#   82  E_SHOW_BAD_FLAG         — unrecognised flag
#
# See also: EXIT_CODES.md

_JIRA_SHOW_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_SHOW_SCRIPT_DIR/jira-common.sh"

_jira_show_usage() {
  cat <<'USAGE'
Usage: jira-show-flow.sh <ISSUE-KEY> [flags]

  Fetches a single Jira issue by key, with optional comment slice and ADF render.

Flags:
  --fields a,b,c | --fields a   Field tokens (CSV or repeatable, default: *all).
  --expand a,b,c  Override default expand (default: names,schema,transitions).
  --comments N    Include last N embedded comments (0 = omit, default 0).
  --render-adf    Render ADF descriptions/comments to Markdown (default ON).
  --no-render-adf Keep ADF as raw JSON objects in the response.
  --help, -h      Print this banner and exit 0.

Example:
  jira-show-flow.sh ENG-1 --comments 5
USAGE
}

_jira_show() {
  local key=""
  local expand="names,schema,transitions"
  local -a field_tokens=()
  local comments=0
  # render_adf defaults ON for show (single-issue reads are for humans;
  # rendered Markdown is the natural output). Search defaults OFF because
  # bulk results don't benefit from per-issue rendering. The asymmetry lives
  # at the helper layer where it is testable, not in SKILL prose.
  local render_adf=1

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help|-h)
        _jira_show_usage
        exit 0
        ;;
      --render-adf)
        render_adf=1; shift ;;
      --no-render-adf)
        render_adf=0; shift ;;
      --fields)
        local -a _fs=()
        IFS=',' read -ra _fs <<< "$2"
        local _t
        for _t in "${_fs[@]}"; do
          [[ -n "$_t" ]] && field_tokens+=("$_t")
        done
        shift 2 ;;
      --expand)
        expand="$2"; shift 2 ;;
      --comments)
        comments="$2"; shift 2
        if ! [[ "$comments" =~ ^[0-9]+$ ]] || (( comments > 100 )); then
          echo "E_SHOW_BAD_COMMENTS_LIMIT: --comments must be an integer between 0 and 100; got '$comments'." >&2
          return 81
        fi
        ;;
      -*)
        echo "E_SHOW_BAD_FLAG: unrecognised flag: $1" >&2
        _jira_show_usage >&2
        return 82
        ;;
      *)
        if [[ -z "$key" ]]; then
          key="$1"; shift
        else
          echo "E_SHOW_BAD_FLAG: unexpected positional argument: $1" >&2
          _jira_show_usage >&2
          return 82
        fi
        ;;
    esac
  done

  if [[ -z "$key" ]]; then
    echo "E_SHOW_NO_KEY: issue key required" >&2
    return 80
  fi

  # Compose the fields query value. Default to *all when no tokens supplied.
  local fields="*all"
  if (( ${#field_tokens[@]} > 0 )); then
    fields=$(IFS=','; printf '%s' "${field_tokens[*]}")
  fi

  # Append comments to expand when requested; this adds fields.comment to the
  # response without a second round-trip.
  local effective_expand="$expand"
  if (( comments > 0 )); then
    effective_expand="${effective_expand},comments"
  fi

  # GET the issue. Keep local and assignment on separate lines so
  # jira-request.sh's exit code is visible to || return $?.
  local issue_json req_exit=0
  issue_json=$(bash "$_JIRA_SHOW_SCRIPT_DIR/jira-request.sh" \
    GET "/rest/api/3/issue/$key" \
    --query "fields=$fields" \
    --query "expand=$effective_expand") || req_exit=$?
  if [[ $req_exit -ne 0 ]]; then
    if [[ $req_exit -eq 11 ]] || [[ $req_exit -eq 12 ]]; then
      echo "Hint: authentication failed — run /init-jira to refresh credentials" >&2
    fi
    return $req_exit
  fi

  # Client-side comment slice: sort embedded comments by .created and keep
  # the last N. Bounded by Atlassian's embedded page (typically ~20).
  if (( comments > 0 )); then
    issue_json=$(printf '%s' "$issue_json" \
      | jq --argjson n "$comments" '
          if (.fields.comment.comments // null) == null then .
          else .fields.comment.comments |=
            (sort_by(.created // "") | .[-($n):])
          end')
  fi

  # Optional ADF render (default ON for show).
  if (( render_adf )); then
    issue_json=$(printf '%s' "$issue_json" | \
      bash "$_JIRA_SHOW_SCRIPT_DIR/jira-render-adf-fields.sh") || return $?
  fi

  printf '%s\n' "$issue_json"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail
  _jira_show "$@"
fi
