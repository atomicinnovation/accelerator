#!/usr/bin/env bash
# linear-init-flow.sh — Linear integration setup orchestration.
#
# Usage:
#   linear-init-flow.sh [--non-interactive] [subcommand]
#
# Subcommands:
#   verify              — resolve token, query viewer { id name }, persist viewer.json
#   list-teams          — query teams { nodes { id name key } }, print as JSON
#   discover --team-id <id>
#                       — query the team's states, persist catalogue.json
#   (none)              — full flow: verify → (discover if --team-id given)
#
# Flags:
#   --non-interactive / -y   fail fast (exit 60) when a value would need prompting
#   --team-id <id>           team UUID to discover (for the bare flow / discover)
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   60  E_INIT_NEEDS_CONFIG  — required config missing in non-interactive mode
#   61  E_INIT_VERIFY_FAILED — viewer verification failed (incl. auth failure)
#   62  E_INIT_NO_TEAM       — selected team not found or has no WorkflowStates

set -euo pipefail

_LINEAR_INIT_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_LINEAR_INIT_SCRIPT_DIR/linear-common.sh"
source "$_LINEAR_INIT_SCRIPT_DIR/linear-auth.sh"

# Flow exit codes are declared as constants so `return $E_*` sites read
# symbolically; these constants are the source of truth and EXIT_CODES.md is
# derived documentation.
readonly E_INIT_NEEDS_CONFIG=60
readonly E_INIT_VERIFY_FAILED=61
readonly E_INIT_NO_TEAM=62

# Default: interactive mode; overridden by --non-interactive
_LINEAR_NON_INTERACTIVE=false
_LINEAR_TEAM_ID=""

# ---------------------------------------------------------------------------
# Gitignore, gitkeep, and absent-accelerator helpers

# Writes rules from LINEAR_INNER_GITIGNORE_RULES into $state_dir/.gitignore.
# Idempotent: each rule is appended only if not already present (grep -qFx).
_linear_ensure_inner_gitignore() {
  local state_dir
  state_dir=$(linear_state_dir) || return 1
  local gi="$state_dir/.gitignore"
  touch "$gi"
  for rule in "${LINEAR_INNER_GITIGNORE_RULES[@]}"; do
    grep -qFx "$rule" "$gi" 2>/dev/null || printf '%s\n' "$rule" >>"$gi"
  done
}

# Ensures $state_dir/.gitkeep exists so the directory is tracked in git when
# all gitignored files (viewer.json, .refresh-meta.json) are absent.
_linear_ensure_gitkeep() {
  local state_dir
  state_dir=$(linear_state_dir) || return 1
  [ -e "$state_dir/.gitkeep" ] || touch "$state_dir/.gitkeep"
}

# Emits a log_warn if the .accelerator/ scaffold has not been initialised.
_linear_warn_if_accelerator_absent() {
  local repo_root
  repo_root=$(find_repo_root) || return 1
  if [ ! -f "$repo_root/.accelerator/.gitignore" ]; then
    log_warn ".accelerator/ is not initialised — run /accelerator:init to set up the scaffold."
  fi
}

# ---------------------------------------------------------------------------
# Transport helper

# Run a GraphQL query via linear-graphql.sh; echo body on stdout, propagate
# exit code. Variables are optional (a JSON object string).
_linear_gql() {
  local query="$1"
  # Default to an empty JSON object. Set this in a separate statement rather
  # than via `${2:-{\}}`: bash 3.2 (the macOS floor) keeps the literal
  # backslash from a brace escaped inside a :-default, yielding "{\}" — invalid
  # JSON — whereas bash 4+ strips it. A plain assignment is unambiguous.
  local variables="${2:-}"
  [ -n "$variables" ] || variables='{}'
  bash "$_LINEAR_INIT_SCRIPT_DIR/linear-graphql.sh" \
    --query "$query" --variables "$variables"
}

# ---------------------------------------------------------------------------
# verify subcommand

_linear_verify() {
  _linear_warn_if_accelerator_absent

  # Resolve credentials (sets LINEAR_TOKEN in the current shell).
  # Must NOT be called inside $() — command substitution would not propagate
  # the variable assignment back.
  local _cred_err_file _cred_ok
  _cred_err_file=$(mktemp)
  _cred_ok=true
  linear_resolve_credentials 2>"$_cred_err_file" || _cred_ok=false
  if ! $_cred_ok; then
    if $_LINEAR_NON_INTERACTIVE; then
      cat "$_cred_err_file" >&2
      rm -f "$_cred_err_file"
      echo "E_INIT_NEEDS_CONFIG: required Linear token is missing; set linear.token in config.local.md" >&2
      return $E_INIT_NEEDS_CONFIG
    fi
    cat "$_cred_err_file" >&2
    rm -f "$_cred_err_file"
    return 1
  fi
  rm -f "$_cred_err_file"

  # Verify against viewer { id name }
  local viewer_json _rc=0
  viewer_json=$(_linear_gql 'query { viewer { id name } }') || _rc=$?
  if [ "$_rc" -ne 0 ]; then
    # A Bearer-prefixed token fails auth (HTTP 401 → transport exit 11); surface
    # it as an authentication failure rather than a generic verify error.
    echo "E_INIT_VERIFY_FAILED: could not verify Linear credentials (authentication failed?). The personal API key must be sent WITHOUT a 'Bearer' prefix." >&2
    return $E_INIT_VERIFY_FAILED
  fi

  local viewer_id viewer_name
  viewer_id=$(linear_jq_field "$viewer_json" '.data.viewer.id')
  viewer_name=$(linear_jq_field "$viewer_json" '.data.viewer.name')
  if [ -z "$viewer_id" ]; then
    echo "E_INIT_VERIFY_FAILED: no viewer id in response" >&2
    return $E_INIT_VERIFY_FAILED
  fi

  local state_dir
  state_dir=$(linear_state_dir) || return 1

  printf '{"id":%s,"name":%s}\n' \
    "$(printf '%s' "$viewer_id" | jq -R '.')" \
    "$(printf '%s' "$viewer_name" | jq -R '.')" |
    linear_atomic_write_json "$state_dir/viewer.json"

  _linear_ensure_inner_gitignore
  _linear_ensure_gitkeep
}

# ---------------------------------------------------------------------------
# list-teams subcommand

_linear_list_teams() {
  local teams_json _rc=0
  teams_json=$(_linear_gql 'query { teams { nodes { id name key } } }') || _rc=$?
  if [ "$_rc" -ne 0 ]; then
    echo "E_INIT_VERIFY_FAILED: could not list teams" >&2
    return $E_INIT_VERIFY_FAILED
  fi
  printf '%s\n' "$teams_json" | jq '.data.teams.nodes'
}

# ---------------------------------------------------------------------------
# discover subcommand (called inside linear_with_lock)

_linear_do_discover() {
  if [ -z "$_LINEAR_TEAM_ID" ]; then
    echo "E_INIT_NO_TEAM: no --team-id supplied to discover" >&2
    return $E_INIT_NO_TEAM
  fi

  local state_dir
  state_dir=$(linear_state_dir) || return 1

  local vars
  vars=$(jq -cn --arg id "$_LINEAR_TEAM_ID" '{id: $id}')
  local team_json _rc=0
  # shellcheck disable=SC2016 # $id is a GraphQL variable, not a shell expansion
  team_json=$(_linear_gql \
    'query($id: String!) { team(id: $id) { id name key states { nodes { id name type position } } } }' \
    "$vars") || _rc=$?
  if [ "$_rc" -ne 0 ]; then
    echo "E_INIT_NO_TEAM: could not fetch team $_LINEAR_TEAM_ID" >&2
    return $E_INIT_NO_TEAM
  fi

  # Fail if the team is not found or has no states.
  local team_id state_count
  team_id=$(linear_jq_field "$team_json" '.data.team.id')
  state_count=$(printf '%s' "$team_json" |
    jq -r '(.data.team.states.nodes // []) | length' 2>/dev/null || echo 0)
  if [ -z "$team_id" ] || [ "$state_count" -eq 0 ]; then
    echo "E_INIT_NO_TEAM: team $_LINEAR_TEAM_ID not found or has no WorkflowStates" >&2
    return $E_INIT_NO_TEAM
  fi

  # Build catalogue.json = {team:{id,key,name}, workflowStates:[...]}
  local catalogue
  catalogue=$(printf '%s' "$team_json" | jq -c '{
    team: {id: .data.team.id, key: .data.team.key, name: .data.team.name},
    workflowStates: [.data.team.states.nodes[] | {id, name, type, position}]
  }')
  printf '%s\n' "$catalogue" | linear_atomic_write_json "$state_dir/catalogue.json"

  _linear_ensure_inner_gitignore
  _linear_ensure_gitkeep
}

_linear_discover() {
  linear_with_lock _linear_do_discover
}

# ---------------------------------------------------------------------------
# CLI dispatch

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  CMD=""
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --non-interactive | -y)
        _LINEAR_NON_INTERACTIVE=true
        shift
        ;;
      --team-id)
        _LINEAR_TEAM_ID="$2"
        shift 2
        ;;
      verify | list-teams | discover)
        CMD="$1"
        shift
        ;;
      *)
        echo "Unknown option: $1" >&2
        exit 2
        ;;
    esac
  done

  case "$CMD" in
    verify)
      _linear_verify
      ;;
    list-teams)
      _linear_list_teams
      ;;
    discover)
      _linear_discover
      ;;
    "")
      # Full flow: verify, then discover if a team was supplied.
      _linear_verify
      if [ -n "$_LINEAR_TEAM_ID" ]; then
        _linear_discover
      elif $_LINEAR_NON_INTERACTIVE; then
        echo "E_INIT_NEEDS_CONFIG: no --team-id supplied in non-interactive mode" >&2
        exit $E_INIT_NEEDS_CONFIG
      else
        echo "Available teams (select one and re-run with --team-id <id>):" >&2
        _linear_list_teams >&2
      fi
      ;;
  esac
fi
