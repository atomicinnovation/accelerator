#!/usr/bin/env bash
# linear-create-flow.sh — Create a Linear issue from a local work-item file and
# write the remote-allocated identifier back into that file's work_item_id.
#
# Usage:
#   linear-create-flow.sh <work-item-file> [flags]
#
# Flags:
#   --print-payload   Dry-run: print the operation + input and exit 0 (no API
#                     call, no writeback).
#   --quiet, -q       Suppress INFO stderr lines.
#   --help, -h        Print this banner and exit 0.
#
# The issue title and description come from the work-item file's frontmatter
# `title` and the Markdown body below the frontmatter. The team is the one
# stored in catalogue.json at /init-linear time.
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   100 E_CREATE_NO_FILE          no file path supplied or path not readable
#   101 E_CREATE_BAD_FRONTMATTER  missing/unclosed frontmatter or no work_item_id
#   102 E_CREATE_ALREADY_SYNCED   work_item_id is already a remote-format id
#   103 E_CREATE_NO_TITLE         frontmatter has no title
#   104 E_CREATE_BAD_FLAG         unrecognised flag
#   105 E_CREATE_NO_CATALOGUE     catalogue.json missing; run /init-linear
#   106 E_CREATE_BAD_IDENTIFIER   returned identifier failed validation
#   107 E_CREATE_WRITEBACK_FAILED issue created remotely but local writeback failed

set -euo pipefail

_LINEAR_CREATE_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_LINEAR_CREATE_PLUGIN_ROOT="$(cd "$_LINEAR_CREATE_SCRIPT_DIR/../../../.." && pwd)"
source "$_LINEAR_CREATE_SCRIPT_DIR/linear-common.sh"
source "$_LINEAR_CREATE_PLUGIN_ROOT/scripts/config-common.sh"

readonly E_CREATE_NO_FILE=100
readonly E_CREATE_BAD_FRONTMATTER=101
readonly E_CREATE_ALREADY_SYNCED=102
readonly E_CREATE_NO_TITLE=103
readonly E_CREATE_BAD_FLAG=104
readonly E_CREATE_NO_CATALOGUE=105
readonly E_CREATE_BAD_IDENTIFIER=106
readonly E_CREATE_WRITEBACK_FAILED=107

# Remote identifier shape, e.g. BLA-123. A numeric work_item_id means unsynced
# (0047's contract); a remote-format value means already synced.
readonly LINEAR_IDENTIFIER_RE='^[A-Z][A-Z0-9]*-[0-9]+$'

_linear_create_usage() {
  cat <<'USAGE'
Usage: linear-create-flow.sh <work-item-file> [flags]

  Creates a Linear issue from a local work-item file and writes the allocated
  identifier back into that file's work_item_id frontmatter field.

Flags:
  --print-payload   Dry-run: print operation + input, exit 0 (no API call).
  --quiet, -q       Suppress INFO stderr lines.
  --help, -h        Print this banner and exit 0.
USAGE
}

# Extract a top-level frontmatter field value from frontmatter text on stdin.
_linear_fm_field() {
  local key="$1"
  awk -v key="$key" '
    BEGIN { kpat = "^" key ":" }
    $0 ~ kpat {
      v = substr($0, length(key) + 2)
      sub(/^[ \t]+/, "", v); sub(/[ \t]+$/, "", v)
      if (v ~ /^".*"$/ || v ~ /^'"'"'.*'"'"'$/) v = substr(v, 2, length(v) - 2)
      print v; exit
    }
  '
}

_linear_create() {
  linear_require_dependencies

  local file="" print_payload=0 quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help | -h)
        _linear_create_usage
        exit 0
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
        printf 'E_CREATE_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        _linear_create_usage >&2
        return $E_CREATE_BAD_FLAG
        ;;
      *)
        if [[ -z "$file" ]]; then
          file="$1"
          shift
        else
          printf 'E_CREATE_BAD_FLAG: unexpected positional argument: %s\n' "$1" >&2
          return $E_CREATE_BAD_FLAG
        fi
        ;;
    esac
  done

  if [[ -z "$file" ]] || [[ ! -r "$file" ]]; then
    printf 'E_CREATE_NO_FILE: work-item file path required and must be readable\n' >&2
    return $E_CREATE_NO_FILE
  fi

  # Read frontmatter (fails if absent/unclosed) and extract title + work_item_id.
  local fm
  if ! fm=$(config_extract_frontmatter "$file") || [[ -z "$fm" ]]; then
    printf 'E_CREATE_BAD_FRONTMATTER: %s has no parseable frontmatter\n' "$file" >&2
    return $E_CREATE_BAD_FRONTMATTER
  fi

  local wid title
  wid=$(printf '%s\n' "$fm" | _linear_fm_field work_item_id)
  title=$(printf '%s\n' "$fm" | _linear_fm_field title)

  if [[ -z "$wid" ]]; then
    printf 'E_CREATE_BAD_FRONTMATTER: %s has no work_item_id field\n' "$file" >&2
    return $E_CREATE_BAD_FRONTMATTER
  fi

  # Already-synced guard: trim surrounding quotes/whitespace, then test the
  # remote-format shape. A quoted "BLA-123" must still fire the guard.
  local wid_trimmed
  wid_trimmed=$(printf '%s' "$wid" | sed "s/^[[:space:]\"']*//; s/[[:space:]\"']*\$//")
  if [[ "$wid_trimmed" =~ $LINEAR_IDENTIFIER_RE ]]; then
    printf 'E_CREATE_ALREADY_SYNCED: %s is already synced as %s; nothing created\n' \
      "$file" "$wid_trimmed" >&2
    return $E_CREATE_ALREADY_SYNCED
  fi

  if [[ -z "$title" ]]; then
    printf 'E_CREATE_NO_TITLE: %s has no title field\n' "$file" >&2
    return $E_CREATE_NO_TITLE
  fi

  # Team id from the catalogue.
  local state_dir cat team_id
  state_dir=$(linear_state_dir) || return 1
  cat="$state_dir/catalogue.json"
  if [[ ! -f "$cat" ]]; then
    printf 'E_CREATE_NO_CATALOGUE: catalogue.json missing; run /init-linear\n' >&2
    return $E_CREATE_NO_CATALOGUE
  fi
  team_id=$(jq -r '.team.id // empty' "$cat")
  if [[ -z "$team_id" ]]; then
    printf 'E_CREATE_NO_CATALOGUE: catalogue.json has no team id; run /init-linear\n' >&2
    return $E_CREATE_NO_CATALOGUE
  fi

  # Description = the Markdown body below the frontmatter, trimmed.
  local description
  description=$(config_extract_body "$file" | config_trim_body)

  local input
  input=$(jq -cn --arg t "$title" --arg d "$description" --arg team "$team_id" \
    '{teamId: $team, title: $t, description: $d}')

  # --print-payload: dry-run, no API call, no writeback.
  if ((print_payload)); then
    jq -n --arg op "issueCreate" --argjson input "$input" \
      '{operation: $op, input: $input}'
    return 0
  fi

  if ! ((quiet)); then
    printf 'INFO: creating Linear issue "%s" in team %s\n' "$title" "$team_id" >&2
  fi

  local variables
  variables=$(jq -cn --argjson input "$input" '{input: $input}')
  # shellcheck disable=SC2016 # $input is a GraphQL variable
  local query='mutation($input: IssueCreateInput!) {
    issueCreate(input: $input) { success issue { id identifier } }
  }'

  local resp req_exit=0
  resp=$(bash "$_LINEAR_CREATE_SCRIPT_DIR/linear-graphql.sh" \
    --query "$query" --variables "$variables") || req_exit=$?
  if ((req_exit != 0)); then
    _linear_emit_generic_hint "$req_exit" || true
    return "$req_exit"
  fi

  # Validate the returned identifier BEFORE writing it anywhere — a tampered
  # response must not inject newlines / YAML into a tracked file.
  local identifier
  identifier=$(linear_jq_field "$resp" '.data.issueCreate.issue.identifier')
  if [[ -z "$identifier" ]] || ! [[ "$identifier" =~ $LINEAR_IDENTIFIER_RE ]]; then
    printf 'E_CREATE_BAD_IDENTIFIER: returned identifier %q failed validation; local file left untouched\n' \
      "$identifier" >&2
    return $E_CREATE_BAD_IDENTIFIER
  fi

  # Write the identifier back. The remote create is not idempotent, so a
  # writeback failure after a successful create is surfaced LOUDLY.
  if ! config_set_frontmatter_field "$file" work_item_id "$identifier" 2>/dev/null; then
    printf 'E_CREATE_WRITEBACK_FAILED: issue was created remotely as %s, but writing work_item_id back into %s FAILED. Do NOT blindly re-run (it would create a duplicate); set work_item_id: %s in the file by hand.\n' \
      "$identifier" "$file" "$identifier" >&2
    return $E_CREATE_WRITEBACK_FAILED
  fi

  printf '%s\n' "$identifier"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _linear_create "$@"
fi
