#!/usr/bin/env bash
# linear-create-flow.sh — Create a Linear issue, in one of two modes:
#
#   1. File-first (the /create-linear-issue path):
#        linear-create-flow.sh <work-item-file> [flags]
#      Reads the issue title + description from the work-item file, refuses if
#      the file is already synced (carries a non-empty external_id), creates the
#      issue, then writes the remote-allocated identifier back into the file's
#      external_id frontmatter field (inserting the line if absent).
#
#   2. No-file create-and-return (the /create-work-item dispatcher path):
#        linear-create-flow.sh --title TEXT [--body-file PATH] [flags]
#      Takes explicit content, prints ONLY the bare validated identifier on
#      stdout, performs NO file I/O and NO writeback. Distinguishes pre-create
#      failures (provably before the issueCreate mutation was transmitted — safe
#      to retry) from post-create failures (the request was, or may have been,
#      transmitted — a remote issue may already exist, NOT safe to retry).
#
# Flags:
#   --title TEXT      No-file mode: issue title (selects no-file mode).
#   --body-file PATH  No-file mode: issue description from a file (Markdown).
#   --print-payload   Dry-run: print the operation + input and exit 0 (no API
#                     call, no writeback).
#   --quiet, -q       Suppress INFO stderr lines.
#   --help, -h        Print this banner and exit 0.
#
# In file-first mode the team is the one stored in catalogue.json at
# /init-linear time; the description is the Markdown body below the frontmatter.
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   100 E_CREATE_NO_FILE          no file path supplied or path not readable
#   101 E_CREATE_BAD_FRONTMATTER  missing/unclosed frontmatter
#   102 E_CREATE_ALREADY_SYNCED   external_id is already present (non-empty)
#   103 E_CREATE_NO_TITLE         no title (frontmatter title / --title)
#   104 E_CREATE_BAD_FLAG         unrecognised flag
#   105 E_CREATE_NO_CATALOGUE     catalogue.json missing; run /init-linear
#   106 E_CREATE_BAD_IDENTIFIER   returned identifier failed validation
#   107 E_CREATE_WRITEBACK_FAILED issue created remotely but external_id writeback failed
#   108 E_CREATE_PRE_SEND         no-file mode: failure provably before the mutation (retryable)
#   109 E_CREATE_POST_SEND        no-file mode: request sent/ambiguous (issue may exist; NOT retryable)

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
readonly E_CREATE_PRE_SEND=108
readonly E_CREATE_POST_SEND=109

# Remote identifier shape, e.g. BLA-123. Used to validate the returned
# identifier before it is written anywhere.
readonly LINEAR_IDENTIFIER_RE='^[A-Z][A-Z0-9]*-[0-9]+$'

_linear_create_usage() {
  cat <<'USAGE'
Usage:
  linear-create-flow.sh <work-item-file> [flags]     # file-first: writeback external_id
  linear-create-flow.sh --title TEXT [--body-file PATH] [flags]
                                                       # no-file: print bare identifier

Flags:
  --title TEXT      No-file mode: issue title (selects no-file mode).
  --body-file PATH  No-file mode: issue description from a file (Markdown).
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

# Resolve the team id from the catalogue. Echoes the id on stdout, or returns
# E_CREATE_NO_CATALOGUE.
_linear_team_id() {
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
  printf '%s' "$team_id"
}

# Core create: given title + description, build the payload and (unless
# print_payload) call issueCreate, validate the returned identifier, and print
# the bare identifier on stdout. Shared by both modes.
#
# On failure prints nothing to stdout and returns:
#   - the raw linear-graphql.sh transport code (11/16/20/21/22/34/35/36), or
#   - E_CREATE_BAD_IDENTIFIER (106) when the returned identifier is unusable.
# Callers map these onto their own exit-code contract.
_linear_issue_create() {
  local title="$1" description="$2" team_id="$3" print_payload="$4" quiet="$5"

  local input
  input=$(jq -cn --arg t "$title" --arg d "$description" --arg team "$team_id" \
    '{teamId: $team, title: $t, description: $d}')

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

  # Validate the returned identifier BEFORE printing it — a tampered response
  # must not inject newlines / YAML downstream.
  local identifier
  identifier=$(linear_jq_field "$resp" '.data.issueCreate.issue.identifier')
  if [[ -z "$identifier" ]] || ! [[ "$identifier" =~ $LINEAR_IDENTIFIER_RE ]]; then
    printf 'E_CREATE_BAD_IDENTIFIER: returned identifier %q failed validation\n' \
      "$identifier" >&2
    return $E_CREATE_BAD_IDENTIFIER
  fi

  printf '%s\n' "$identifier"
}

# Map a _linear_issue_create failure code onto the no-file mode's pre/post
# contract. The boundary is drawn conservatively around the ambiguous window:
# only failures where NO mutation could have executed are retryable-pre-create.
#   PRE  (108): server rejected the request before executing the mutation, or
#               no request was sent (auth / no-creds / bad-request / ratelimit /
#               complexity / pre-call validation). Safe to retry.
#   POST (109): the request was, or may have been, transmitted and a response
#               was lost or unusable (bad-response, 5xx, connect/DNS/timeout
#               — linear-graphql.sh collapses connect-refused with read-timeout
#               into code 21, so it is treated conservatively as POST — and a
#               created-but-invalid identifier). A remote issue may exist.
_linear_map_no_file_failure() {
  case "$1" in
    11 | 22 | 34 | 35 | 36) return $E_CREATE_PRE_SEND ;;
    *) return $E_CREATE_POST_SEND ;;
  esac
}

_linear_create() {
  linear_require_dependencies

  local file="" title_flag="" body_file="" title_flag_set=0
  local print_payload=0 quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help | -h)
        _linear_create_usage
        exit 0
        ;;
      --title)
        title_flag="$2"
        title_flag_set=1
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

  # --title selects no-file create-and-return mode.
  if ((title_flag_set)); then
    _linear_create_no_file "$title_flag" "$body_file" "$print_payload" "$quiet"
    return $?
  fi

  _linear_create_from_file "$file" "$print_payload" "$quiet"
}

# No-file mode: explicit content in, bare identifier out, no writeback. All
# failures are mapped onto the pre/post-send contract.
_linear_create_no_file() {
  local title="$1" body_file="$2" print_payload="$3" quiet="$4"

  if [[ -z "$title" ]]; then
    printf 'E_CREATE_NO_TITLE: --title is required in no-file mode\n' >&2
    return $E_CREATE_PRE_SEND
  fi

  local description=""
  if [[ -n "$body_file" ]]; then
    if [[ ! -r "$body_file" ]]; then
      printf 'E_CREATE_NO_FILE: --body-file not readable: %s\n' "$body_file" >&2
      return $E_CREATE_PRE_SEND
    fi
    description=$(config_trim_body <"$body_file")
  fi

  local team_id
  if ! team_id=$(_linear_team_id); then
    # Missing catalogue is a pre-transmission config error (no mutation sent).
    return $E_CREATE_PRE_SEND
  fi

  local identifier rc=0
  identifier=$(_linear_issue_create "$title" "$description" "$team_id" \
    "$print_payload" "$quiet") || rc=$?
  if ((rc != 0)); then
    _linear_map_no_file_failure "$rc"
    return $?
  fi

  # print_payload prints the dry-run shape itself and returns 0 with no
  # identifier; pass it straight through.
  if ((print_payload)); then
    printf '%s\n' "$identifier"
    return 0
  fi

  printf '%s\n' "$identifier"
}

# File-first mode: read content from the work-item file, guard on external_id
# presence, create, then write external_id back (inserting if absent).
_linear_create_from_file() {
  local file="$1" print_payload="$2" quiet="$3"

  if [[ -z "$file" ]] || [[ ! -r "$file" ]]; then
    printf 'E_CREATE_NO_FILE: work-item file path required and must be readable\n' >&2
    return $E_CREATE_NO_FILE
  fi

  local fm
  if ! fm=$(config_extract_frontmatter "$file") || [[ -z "$fm" ]]; then
    printf 'E_CREATE_BAD_FRONTMATTER: %s has no parseable frontmatter\n' "$file" >&2
    return $E_CREATE_BAD_FRONTMATTER
  fi

  local external_id title
  external_id=$(printf '%s\n' "$fm" | _linear_fm_field external_id)
  title=$(printf '%s\n' "$fm" | _linear_fm_field title)

  # Already-synced guard: presence-based. Trim surrounding quotes/whitespace;
  # a non-empty remainder means the item already carries a remote identifier.
  # (Same normalisation as the Jira guard and work-item-sync-label.sh.)
  local eid_trimmed
  eid_trimmed=$(printf '%s' "$external_id" | sed "s/^[[:space:]\"']*//; s/[[:space:]\"']*\$//")
  if [[ -n "$eid_trimmed" ]]; then
    printf 'E_CREATE_ALREADY_SYNCED: %s is already synced as %s; nothing created\n' \
      "$file" "$eid_trimmed" >&2
    return $E_CREATE_ALREADY_SYNCED
  fi

  if [[ -z "$title" ]]; then
    printf 'E_CREATE_NO_TITLE: %s has no title field\n' "$file" >&2
    return $E_CREATE_NO_TITLE
  fi

  local team_id
  team_id=$(_linear_team_id) || return $?

  local description
  description=$(config_extract_body "$file" | config_trim_body)

  local identifier rc=0
  identifier=$(_linear_issue_create "$title" "$description" "$team_id" \
    "$print_payload" "$quiet") || rc=$?
  if ((rc != 0)); then
    return "$rc"
  fi

  # --print-payload: the core already emitted the dry-run shape; no writeback.
  if ((print_payload)); then
    printf '%s\n' "$identifier"
    return 0
  fi

  # Write the identifier back into external_id (insert the line if absent). The
  # remote create is not idempotent, so a writeback failure after a successful
  # create is surfaced LOUDLY.
  if ! config_upsert_frontmatter_field "$file" external_id "$identifier" 2>/dev/null; then
    printf 'E_CREATE_WRITEBACK_FAILED: issue was created remotely as %s, but writing external_id back into %s FAILED. Do NOT blindly re-run (it would create a duplicate); set external_id: %s in the file by hand.\n' \
      "$identifier" "$file" "$identifier" >&2
    return $E_CREATE_WRITEBACK_FAILED
  fi

  printf '%s\n' "$identifier"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _linear_create "$@"
fi
