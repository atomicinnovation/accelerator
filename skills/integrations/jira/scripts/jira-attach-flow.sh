#!/usr/bin/env bash
# jira-attach-flow.sh — Upload one or more local files as attachments to a Jira issue.
#
# Usage:
#   jira-attach-flow.sh [--describe] KEY FILE [FILE...]
#            [--quiet]
#            [--help | -h]
#
# Required:
#   KEY          Issue key (positional), e.g. "ENG-1"
#   FILE         One or more file paths to upload as attachments
#
# Optional:
#   --describe   Dry-run: print operation description and exit 0
#   --quiet      Suppress INFO stderr lines
#   --help, -h   Print this banner and exit 0
#
# Exit codes:
#   130 E_ATTACH_NO_KEY      no issue key positional argument
#   131 E_ATTACH_NO_FILES    no file paths supplied
#   132 E_ATTACH_FILE_MISSING a named file does not exist or is not readable
#   133 E_ATTACH_BAD_FLAG    unrecognised flag
#   134-139                  reserved
#   11-23, 34 propagated from jira-request.sh (auth/transport/4xx/5xx)
#
# See also: EXIT_CODES.md

_JIRA_ATTACH_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_ATTACH_SCRIPT_DIR/jira-common.sh"

_jira_attach_usage() {
  cat <<'USAGE'
Usage: jira-attach-flow.sh [--describe] KEY FILE [FILE...]
         [--quiet] [--help | -h]

Required:
  KEY    Issue key, e.g. "ENG-1"
  FILE   One or more local file paths to upload

Optional:
  --describe   Dry-run: print operation description and exit 0
  --quiet      Suppress INFO stderr lines
  --help, -h   Print this banner and exit 0

Examples:
  jira-attach-flow.sh ENG-1 ./screenshot.png
  jira-attach-flow.sh ENG-1 ./logs.txt ./debug.json
USAGE
}

_jira_attach() {
  jira_require_dependencies

  local key=""
  local -a files=()
  local describe=0
  local quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help|-h)
        _jira_attach_usage; exit 0 ;;
      --describe)
        describe=1; shift ;;
      --quiet|-q)
        quiet=1; shift ;;
      --)
        shift
        while [[ $# -gt 0 ]]; do
          if [[ -z "$key" ]]; then key="$1"
          else files+=("$1")
          fi
          shift
        done ;;
      -*)
        printf 'E_ATTACH_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        _jira_attach_usage >&2
        return 133 ;;
      *)
        if [[ -z "$key" ]]; then
          key="$1"; shift
        else
          files+=("$1"); shift
        fi ;;
    esac
  done

  # --- Post-parse validation ---

  if [[ -z "$key" ]]; then
    printf 'E_ATTACH_NO_KEY: issue key required as first positional argument\n' >&2
    return 130
  fi

  if [[ ${#files[@]} -eq 0 ]]; then
    printf 'E_ATTACH_NO_FILES: at least one file path is required\n' >&2
    return 131
  fi

  # --- File validation (runs before --describe short-circuit) ---

  local ten_mb=$((10 * 1024 * 1024))
  for path in "${files[@]}"; do
    if [[ "$path" == -* ]]; then
      printf 'E_ATTACH_FILE_MISSING: file path must not begin with "-": %s\n' \
        "$path" >&2
      return 132
    fi
    if [[ -L "$path" ]]; then
      local resolved
      resolved=$(readlink -f "$path" 2>/dev/null || true)
      case "$resolved" in
        /dev/*|/proc/*|/sys/*)
          printf 'E_ATTACH_FILE_MISSING: file path resolves to a device path: %s\n' \
            "$path" >&2
          return 132 ;;
      esac
    fi
    if ! [[ -f "$path" && -r "$path" ]]; then
      printf 'E_ATTACH_FILE_MISSING: file not found or not readable: %s\n' \
        "$path" >&2
      return 132
    fi
    local size
    size=$(wc -c < "$path" 2>/dev/null || echo 0)
    if (( size > ten_mb )); then
      printf 'Warning: %s is %.1f MB — Jira Cloud'\''s default limit is 10 MB; upload may fail\n' \
        "$path" "$(echo "scale=1; $size / 1048576" | bc)" >&2
    fi
  done

  # --- --describe branch ---

  if (( describe )); then
    local files_json
    files_json=$(printf '%s\n' "${files[@]}" | jq -R . | jq -s .)
    jq -n \
      --arg     key   "$key" \
      --argjson files "$files_json" \
      '{"key":$key,"files":$files}'
    return 0
  fi

  # --- Live path ---

  if ! (( quiet )); then
    printf 'INFO: uploading %d file(s) to %s\n' "${#files[@]}" "$key" >&2
  fi

  local -a multipart_args=()
  for path in "${files[@]}"; do
    multipart_args+=(--multipart "file=@${path}")
  done

  local req_exit=0 response=""
  response=$(bash "$_JIRA_ATTACH_SCRIPT_DIR/jira-request.sh" \
    POST "/rest/api/3/issue/$key/attachments" \
    "${multipart_args[@]}") || req_exit=$?

  if (( req_exit != 0 )); then
    if ! _jira_emit_generic_hint "$req_exit"; then
      case "$req_exit" in
        12) printf 'Hint: you do not have the CreateAttachments permission on this project.\n' >&2 ;;
        13) printf 'Hint: issue not found or you do not have permission to see it.\n' >&2 ;;
      esac
    fi
    return "$req_exit"
  fi

  printf '%s\n' "$response"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail
  _jira_attach "$@"
fi
