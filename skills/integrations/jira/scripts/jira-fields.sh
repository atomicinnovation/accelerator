#!/usr/bin/env bash
# jira-fields.sh — Custom-field discovery, slug generation, and cache management.
#
# Usage (CLI):
#   jira-fields.sh refresh               — fetch fields from Jira and cache locally
#   jira-fields.sh resolve <name-or-id>  — resolve field name/slug/id to a Jira field ID
#   jira-fields.sh list                  — print cached fields as a JSON array
#
# Also sourceable: source this file to use jira_field_slugify in other scripts.
# The BASH_SOURCE guard prevents CLI dispatch when sourced.
#
# Exit codes:
#   0   success
#   50  E_FIELD_NOT_FOUND     — no field matches the given query
#   51  E_FIELD_CACHE_MISSING — fields.json absent; run refresh or /init-jira
#   52  E_FIELD_CACHE_CORRUPT — fields.json present but not valid JSON
#   53  E_REFRESH_LOCKED      — another refresh holds the lock (from jira-common.sh)
#
# See also: EXIT_CODES.md

_JIRA_FIELDS_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_FIELDS_SCRIPT_DIR/jira-common.sh"
source "$_JIRA_FIELDS_SCRIPT_DIR/jira-auth.sh"

# ---------------------------------------------------------------------------
# Slug generation
#
# Uses LC_ALL=C so character-class matching is locale-independent — a UTF-8
# field name produces the same slug on every platform. Uses tr rather than
# bash ${var,,} to stay compatible with macOS /bin/bash 3.2.

jira_field_slugify() {
  local s
  s=$(LC_ALL=C printf '%s' "$1" | tr '[:upper:]' '[:lower:]')
  s=$(LC_ALL=C printf '%s' "$s" \
    | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//; s/[^a-z0-9]+/-/g; s/^-+//; s/-+$//')
  printf '%s\n' "$s"
}

# ---------------------------------------------------------------------------
# Internal helpers

_fields_do_refresh() {
  local state_dir
  state_dir=$(jira_state_dir) || return 1

  if ! jira_resolve_credentials 2>/dev/null; then
    echo "E_REQ_NO_CREDS: no resolvable credentials" >&2
    return 22
  fi

  local raw_json
  if ! raw_json=$(bash "$_JIRA_FIELDS_SCRIPT_DIR/jira-request.sh" GET /rest/api/3/field); then
    echo "E_FIELD_REFRESH_FAILED: could not fetch /rest/api/3/field" >&2
    return 1
  fi

  local fields_json
  if ! fields_json=$(printf '%s\n' "$raw_json" | jq --arg site "$JIRA_SITE" \
    '{site: $site, fields: [.[] |
       {id, key, name,
        slug: (.name | ascii_downcase
                      | gsub("[^a-z0-9]+"; "-")
                      | ltrimstr("-")
                      | rtrimstr("-"))}
       + (if (.schema.custom or .schema.type) then
             {schema: ((if .schema.custom then {custom: .schema.custom} else {} end)
                      + (if .schema.type  then {type:   .schema.type}  else {} end))}
           else {} end)
    ]}'); then
    echo "E_BAD_JSON: could not parse /rest/api/3/field response" >&2
    return 1
  fi

  printf '%s\n' "$fields_json" | jira_atomic_write_json "$state_dir/fields.json"

  local ts
  ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  printf '{"lastRefreshed":"%s"}\n' "$ts" \
    | jira_atomic_write_json "$state_dir/.refresh-meta.json"
}

_fields_refresh() {
  jira_with_lock _fields_do_refresh
}

_fields_resolve() {
  local query="$1"
  local state_dir
  state_dir=$(jira_state_dir) || return 1
  local cache="$state_dir/fields.json"

  if [[ ! -f "$cache" ]]; then
    echo "E_FIELD_CACHE_MISSING: fields.json not found; run 'jira-fields.sh refresh' or '/init-jira'" >&2
    return 51
  fi

  if ! jq empty < "$cache" 2>/dev/null; then
    echo "E_FIELD_CACHE_CORRUPT: fields.json is not valid JSON" >&2
    return 52
  fi

  # Search in priority order: name → slug → id → key. head -1 picks first match.
  local result
  result=$(jq -r --arg q "$query" '
    (.fields[] | select(.name == $q) | .id),
    (.fields[] | select(.slug == $q) | .id),
    (.fields[] | select(.id   == $q) | .id),
    (.fields[] | select(.key  == $q) | .id)
  ' "$cache" 2>/dev/null | head -1) || result=""

  if [[ -z "$result" ]] || [[ "$result" == "null" ]]; then
    echo "E_FIELD_NOT_FOUND: no field matching '$query' in cache" >&2
    return 50
  fi

  printf '%s\n' "$result"
}

_fields_list() {
  local state_dir
  state_dir=$(jira_state_dir) || return 1
  local cache="$state_dir/fields.json"

  if [[ ! -f "$cache" ]]; then
    echo "E_FIELD_CACHE_MISSING: fields.json not found; run 'jira-fields.sh refresh' or '/init-jira'" >&2
    return 51
  fi

  jq '.fields' "$cache"
}

# ---------------------------------------------------------------------------
# CLI dispatch (only when executed, not sourced)

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail

  cmd="${1:-}"
  case "$cmd" in
    refresh)
      _fields_refresh
      ;;
    resolve)
      shift
      if [[ $# -lt 1 ]]; then
        echo "Usage: jira-fields.sh resolve <name-or-id>" >&2
        exit 2
      fi
      _fields_resolve "$1"
      ;;
    list)
      _fields_list
      ;;
    *)
      echo "Usage: jira-fields.sh <refresh|resolve <name-or-id>|list>" >&2
      exit 2
      ;;
  esac
fi
