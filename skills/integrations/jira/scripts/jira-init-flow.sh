#!/usr/bin/env bash
# jira-init-flow.sh — Jira integration setup orchestration.
#
# Usage:
#   jira-init-flow.sh [--non-interactive] [subcommand]
#
# Subcommands:
#   verify          — resolve credentials, verify /rest/api/3/myself, persist site.json
#   discover        — discover projects + fields, persist projects.json + fields.json
#   prompt-default  — prompt for work.default_project_code if unset
#   refresh-fields  — re-run field discovery only (delegates to jira-fields.sh)
#   list-projects   — print cached projects.json .projects array as JSON
#   list-fields     — print cached fields.json .fields array as JSON (delegates to jira-fields.sh)
#   (none)          — full flow: verify → discover → prompt-default
#
# Flags:
#   --non-interactive / -y   fail fast (exit 60) when a value would need prompting
#   --refresh-fields         alias for the refresh-fields subcommand
#   --list-projects          alias for the list-projects subcommand
#   --list-fields            alias for the list-fields subcommand
#
# Exit codes:
#   0   success
#   60  E_INIT_NEEDS_CONFIG  — required config missing in non-interactive mode
#   61  E_INIT_VERIFY_FAILED — /myself verification failed
#
# See also: EXIT_CODES.md

_JIRA_INIT_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_INIT_SCRIPT_DIR/jira-common.sh"
source "$_JIRA_INIT_SCRIPT_DIR/jira-auth.sh"
source "$_JIRA_INIT_SCRIPT_DIR/jira-fields.sh"

# Default: interactive mode; overridden by --non-interactive
_JIRA_NON_INTERACTIVE=false

# ---------------------------------------------------------------------------
# Gitignore, gitkeep, and absent-accelerator helpers

# Writes rules from JIRA_INNER_GITIGNORE_RULES into $state_dir/.gitignore.
# Idempotent: each rule is appended only if not already present (grep -qFx).
_jira_ensure_inner_gitignore() {
  local state_dir
  state_dir=$(jira_state_dir) || return 1
  local gi="$state_dir/.gitignore"
  touch "$gi"
  for rule in "${JIRA_INNER_GITIGNORE_RULES[@]}"; do
    grep -qFx "$rule" "$gi" 2>/dev/null || printf '%s\n' "$rule" >> "$gi"
  done
}

# Ensures $state_dir/.gitkeep exists so the directory is tracked in git when
# all gitignored files (site.json, .refresh-meta.json) are absent.
_jira_ensure_gitkeep() {
  local state_dir
  state_dir=$(jira_state_dir) || return 1
  [ -e "$state_dir/.gitkeep" ] || touch "$state_dir/.gitkeep"
}

# Emits a log_warn if the .accelerator/ scaffold has not been initialised.
# Uses .accelerator/.gitignore as the sentinel created by /accelerator:init.
_jira_warn_if_accelerator_absent() {
  local repo_root
  repo_root=$(find_repo_root) || return 1
  if [ ! -f "$repo_root/.accelerator/.gitignore" ]; then
    log_warn ".accelerator/ is not initialised — run /accelerator:init to set up the scaffold."
  fi
}

# ---------------------------------------------------------------------------
# verify subcommand

_jira_verify() {
  _jira_warn_if_accelerator_absent

  # Resolve credentials (sets JIRA_SITE, JIRA_EMAIL, JIRA_TOKEN in the current shell).
  # jira_resolve_credentials must NOT be called inside $() — command substitution
  # creates a subshell and variable assignments would not propagate back.
  local _cred_err_file _cred_ok
  _cred_err_file=$(mktemp)
  _cred_ok=true
  jira_resolve_credentials 2>"$_cred_err_file" || _cred_ok=false
  if ! $_cred_ok; then
    if $_JIRA_NON_INTERACTIVE; then
      cat "$_cred_err_file" >&2
      rm -f "$_cred_err_file"
      echo "E_INIT_NEEDS_CONFIG: required Jira config is missing; set jira.site and jira.email in accelerator.md" >&2
      return 60
    fi
    cat "$_cred_err_file" >&2
    rm -f "$_cred_err_file"
    return 1
  fi
  rm -f "$_cred_err_file"

  # Verify against /rest/api/3/myself
  local myself_json
  if ! myself_json=$(bash "$_JIRA_INIT_SCRIPT_DIR/jira-request.sh" GET /rest/api/3/myself 2>/dev/null); then
    echo "E_INIT_VERIFY_FAILED: could not reach /rest/api/3/myself" >&2
    return 61
  fi

  local account_id
  account_id=$(printf '%s\n' "$myself_json" | jq -r '.accountId // empty')
  if [[ -z "$account_id" ]]; then
    echo "E_INIT_VERIFY_FAILED: no accountId in /myself response" >&2
    return 61
  fi

  local state_dir
  state_dir=$(jira_state_dir) || return 1

  # Persist site.json with exactly {site, accountId} — no timestamps
  printf '{"site":%s,"accountId":%s}\n' \
    "$(printf '%s' "$JIRA_SITE" | jq -R '.')" \
    "$(printf '%s' "$account_id" | jq -R '.')" \
    | jira_atomic_write_json "$state_dir/site.json"

  _jira_ensure_inner_gitignore
  _jira_ensure_gitkeep
}

# ---------------------------------------------------------------------------
# discover subcommand (called inside jira_with_lock)

_jira_do_discover() {
  # Resolve credentials (needed for JIRA_SITE in projects.json)
  jira_resolve_credentials 2>/dev/null || {
    echo "E_INIT_VERIFY_FAILED: credential resolution failed during discover" >&2
    return 61
  }

  local state_dir
  state_dir=$(jira_state_dir) || return 1

  # Discover projects
  local raw_projects
  if ! raw_projects=$(bash "$_JIRA_INIT_SCRIPT_DIR/jira-request.sh" GET /rest/api/3/project 2>/dev/null); then
    echo "E_INIT_VERIFY_FAILED: could not fetch /rest/api/3/project" >&2
    return 61
  fi

  local projects_json
  projects_json=$(printf '%s\n' "$raw_projects" | jq --arg site "$JIRA_SITE" \
    '{site: $site, projects: [.[] | {key, id, name}]}') || {
    echo "E_BAD_JSON: could not parse /rest/api/3/project response" >&2
    return 1
  }

  printf '%s\n' "$projects_json" | jira_atomic_write_json "$state_dir/projects.json"

  # Discover fields (directly, not via subprocess, to avoid lock re-entrancy)
  _fields_do_refresh
}

_jira_discover() {
  jira_with_lock _jira_do_discover
}

# ---------------------------------------------------------------------------
# prompt-default subcommand

_jira_prompt_default() {
  local repo_root
  repo_root=$(find_repo_root) || return 1

  # Check if work.default_project_code is already set
  local current
  current=$(cd "$repo_root" && "$_JIRA_INIT_SCRIPT_DIR/../../../../scripts/config-read-value.sh" \
    "work.default_project_code" "" 2>/dev/null) || current=""

  if [[ -n "$current" ]]; then
    return 0
  fi

  if $_JIRA_NON_INTERACTIVE; then
    echo "E_INIT_NEEDS_CONFIG: work.default_project_code is not set; re-run without --non-interactive to be prompted" >&2
    return 60
  fi

  # Interactive: prompt (not exercised in automated tests)
  local state_dir
  state_dir=$(jira_state_dir) || return 1
  local projects_file="$state_dir/projects.json"
  if [[ -f "$projects_file" ]]; then
    echo "Available projects:" >&2
    jq -r '.projects[] | "  \(.key)  \(.name)"' "$projects_file" >&2
  fi
  printf 'Enter default project key: ' >&2
  local key
  read -r key
  if [[ -z "$key" ]]; then
    echo "E_INIT_NEEDS_CONFIG: no default project key provided" >&2
    return 60
  fi
  echo "Note: add 'work: {default_project_code: $key}' to accelerator.md" >&2
}

# ---------------------------------------------------------------------------
# list-projects subcommand

_jira_list_projects() {
  local state_dir
  state_dir=$(jira_state_dir) || return 1
  local cache="$state_dir/projects.json"

  if [[ ! -f "$cache" ]]; then
    echo "E_FIELD_CACHE_MISSING: projects.json not found; run 'jira-init-flow.sh' or 'jira-init-flow.sh discover'" >&2
    return 51
  fi

  jq '.projects' "$cache"
}

# ---------------------------------------------------------------------------
# CLI dispatch

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail

  CMD=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --non-interactive|-y) _JIRA_NON_INTERACTIVE=true; shift ;;
      --refresh-fields)     CMD="refresh-fields"; shift ;;
      --list-projects)      CMD="list-projects";  shift ;;
      --list-fields)        CMD="list-fields";    shift ;;
      verify|discover|prompt-default|refresh-fields|list-projects|list-fields)
        CMD="$1"; shift ;;
      *) echo "Unknown option: $1" >&2; exit 2 ;;
    esac
  done

  case "$CMD" in
    verify)
      _jira_verify
      ;;
    discover)
      _jira_discover
      ;;
    prompt-default)
      _jira_prompt_default
      ;;
    refresh-fields)
      bash "$_JIRA_INIT_SCRIPT_DIR/jira-fields.sh" refresh
      ;;
    list-projects)
      _jira_list_projects
      ;;
    list-fields)
      bash "$_JIRA_INIT_SCRIPT_DIR/jira-fields.sh" list
      ;;
    "")
      # Full flow
      _jira_verify
      _jira_discover
      _jira_prompt_default
      ;;
  esac
fi
