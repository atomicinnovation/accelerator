#!/usr/bin/env bash
# jira-search-flow.sh — Compose JQL, search Jira, paginate.
#
# Usage:
#   jira-search-flow.sh [flags]
#
# Flags:
#   --project KEY           Project key (overrides config default).
#   --all-projects          Search across all accessible projects.
#   --status NAME           Repeatable. Prefix with ~ for NOT IN.
#   --label NAME            Repeatable. ~ negates.
#   --assignee NAME|@me     Repeatable. @me resolves via site.json.
#   --type NAME             Repeatable issuetype filter. ~ negates.
#   --component NAME        Repeatable component filter. ~ negates.
#   --reporter NAME|@me     Repeatable. ~ negates.
#   --parent KEY            Issue key. ~ negates.
#   --watching              Limit to issues the user watches.
#   --jql 'raw'             Raw JQL escape hatch (operator-trusted).
#   --limit N               Page size, 1..100 (paginate beyond).
#   --page-token TOK        Opaque token from a prior response.
#   --fields a,b,c          Field tokens (CSV or repeatable).
#   --render-adf            Render ADF to Markdown via M1 walker.
#   --quiet, -q             Suppress the INFO JQL audit line on stderr.
#   --help, -h              Print this banner and exit 0.
#
# Exit codes:
#   0   success
#   70  E_SEARCH_BAD_PAGE_TOKEN — --page-token failed validation
#   71  E_SEARCH_BAD_LIMIT      — --limit not in [1, 100]
#   72  E_SEARCH_NO_SITE_CACHE  — @me used but site.json missing
#   73  E_SEARCH_BAD_FLAG       — unrecognised flag
#
# See also: EXIT_CODES.md

_JIRA_SEARCH_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_SEARCH_SCRIPT_DIR/jira-common.sh"
source "$_JIRA_SEARCH_SCRIPT_DIR/jira-jql.sh"

_jira_search_usage() {
  cat <<'USAGE'
Usage: jira-search-flow.sh [flags]

  Composes JQL from flag set, searches Jira, paginates.

Flags:
  --project KEY           Project key (overrides config default).
  --all-projects          Search across all accessible projects.
  --status NAME           Repeatable. Prefix with ~ for NOT IN.
  --label NAME            Repeatable. ~ negates.
  --assignee NAME|@me     Repeatable. @me resolves via site.json.
  --type NAME             Repeatable issuetype filter. ~ negates.
  --component NAME        Repeatable component filter. ~ negates.
  --reporter NAME|@me     Repeatable. ~ negates.
  --parent KEY            Issue key. ~ negates.
  --watching              Limit to issues the user watches.
  --jql 'raw'             Raw JQL escape hatch (operator-trusted).
  --limit N               Page size, 1..100 (paginate beyond).
  --page-token TOK        Opaque token from a prior response.
  --fields a,b,c | --fields a   Field tokens (CSV or repeatable).
  --render-adf            Render ADF to Markdown via M1 walker.
  --quiet, -q             Suppress the INFO JQL audit line on
                          stderr (warnings and errors still print).
                          Useful for scripted/loop callers.
  --help, -h              Print this banner and exit 0.

Example:
  jira-search-flow.sh --project ENG --assignee @me \
    --status '~Done' --limit 50
USAGE
}

_jira_search_resolve_me() {
  local state_dir
  state_dir=$(jira_state_dir) || return 1
  local site_json="$state_dir/site.json"
  if [[ ! -f "$site_json" ]]; then
    echo "E_SEARCH_NO_SITE_CACHE: site.json missing; run /init-jira" >&2
    return 72
  fi
  local id
  id=$(jq -r '.accountId // empty' "$site_json")
  if [[ -z "$id" ]] || ! [[ "$id" =~ ^[A-Za-z0-9:_-]+$ ]]; then
    echo "E_SEARCH_NO_SITE_CACHE: accountId in site.json is missing or malformed; run /init-jira to refresh." >&2
    return 72
  fi
  printf '%s' "$id"
}

_jira_search_resolve_field() {
  local token="$1"
  if [[ "$token" =~ ^customfield_[0-9]+$ ]]; then
    printf '%s' "$token"
    return 0
  fi
  local resolved
  if resolved=$(bash "$_JIRA_SEARCH_SCRIPT_DIR/jira-fields.sh" \
                  resolve "$token" 2>/dev/null); then
    printf '%s' "$resolved"
    return 0
  fi
  echo "Warning: field '$token' not in fields.json cache; passing through to Jira. Run /init-jira --refresh-fields if it should resolve." >&2
  printf '%s' "$token"
}

# _jira_search_substitute_me_in <array_name>
# Replaces @me and ~@me entries in the named array with the resolved accountId.
_jira_search_substitute_me_in() {
  local -n _sme_arr="$1"
  local i av
  for i in "${!_sme_arr[@]}"; do
    if [[ "${_sme_arr[$i]}" == "@me" ]]; then
      av=$(_jira_search_resolve_me) || return $?
      _sme_arr[$i]="$av"
    elif [[ "${_sme_arr[$i]}" == "~@me" ]]; then
      av=$(_jira_search_resolve_me) || return $?
      _sme_arr[$i]="~$av"
    fi
  done
}

_jira_search() {
  local project="" all_projects=0
  local -a status_vals=() label_vals=() assignee_vals=()
  local -a type_vals=() component_vals=() reporter_vals=()
  local -a parent_vals=() text_vals=()
  local watching=0
  local limit=50 page_token=""
  local -a field_tokens=()
  local raw_jql=""
  local render_adf=0
  local quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help|-h)
        _jira_search_usage
        exit 0
        ;;
      --project)
        project="$2"; shift 2 ;;
      --all-projects)
        all_projects=1; shift ;;
      --status)
        status_vals+=("$2"); shift 2 ;;
      --label)
        label_vals+=("$2"); shift 2 ;;
      --assignee)
        assignee_vals+=("$2"); shift 2 ;;
      --type)
        type_vals+=("$2"); shift 2 ;;
      --component)
        component_vals+=("$2"); shift 2 ;;
      --reporter)
        reporter_vals+=("$2"); shift 2 ;;
      --parent)
        parent_vals+=("$2"); shift 2 ;;
      --watching)
        watching=1; shift ;;
      --text)
        text_vals+=("$2"); shift 2 ;;
      --jql)
        raw_jql="$2"; shift 2 ;;
      --limit)
        limit="$2"; shift 2
        if ! [[ "$limit" =~ ^[0-9]+$ ]] || (( limit < 1 || limit > 100 )); then
          echo "E_SEARCH_BAD_LIMIT: --limit must be a positive integer between 1 and 100; got '$limit'. Use --page-token to paginate beyond 100 results." >&2
          return 71
        fi
        ;;
      --page-token)
        page_token="$2"; shift 2
        if [[ ${#page_token} -gt 4096 ]] || [[ "$page_token" =~ [[:cntrl:][:space:]] ]]; then
          echo "E_SEARCH_BAD_PAGE_TOKEN: --page-token contains invalid characters or exceeds maximum length" >&2
          return 70
        fi
        ;;
      --fields)
        local -a _fs=()
        IFS=',' read -ra _fs <<< "$2"
        field_tokens+=("${_fs[@]}")
        shift 2
        ;;
      --render-adf)
        render_adf=1; shift ;;
      --quiet|-q)
        quiet=1; shift ;;
      *)
        echo "E_SEARCH_BAD_FLAG: unrecognised flag: $1" >&2
        _jira_search_usage >&2
        return 73
        ;;
    esac
  done

  # Resolve @me in principal arrays
  _jira_search_substitute_me_in assignee_vals || return $?
  _jira_search_substitute_me_in reporter_vals || return $?

  # Default project from config when neither --project nor --all-projects given
  if [[ -z "$project" && "$all_projects" -eq 0 ]]; then
    local repo_root
    repo_root=$(find_repo_root 2>/dev/null) || repo_root=""
    if [[ -n "$repo_root" ]]; then
      local default_project
      default_project=$(cd "$repo_root" && \
        "$_JIRA_SEARCH_SCRIPT_DIR/../../../../scripts/config-read-value.sh" \
        "work.default_project_code" "" 2>/dev/null) || default_project=""
      project="$default_project"
    fi
  fi

  # Build jql_compose args
  local -a compose_args=()
  [[ -n "$project" ]] && compose_args+=(--project "$project")
  if (( all_projects )); then compose_args+=(--all-projects); fi
  local v
  for v in "${status_vals[@]+"${status_vals[@]}"}";    do compose_args+=(--status "$v");    done
  for v in "${label_vals[@]+"${label_vals[@]}"}";      do compose_args+=(--label "$v");     done
  for v in "${assignee_vals[@]+"${assignee_vals[@]}"}"; do compose_args+=(--assignee "$v"); done
  for v in "${type_vals[@]+"${type_vals[@]}"}";        do compose_args+=(--type "$v");      done
  for v in "${component_vals[@]+"${component_vals[@]}"}"; do compose_args+=(--component "$v"); done
  for v in "${reporter_vals[@]+"${reporter_vals[@]}"}"; do compose_args+=(--reporter "$v"); done
  for v in "${parent_vals[@]+"${parent_vals[@]}"}";    do compose_args+=(--parent "$v");    done
  if (( watching )); then compose_args+=(--watching); fi
  for v in "${text_vals[@]+"${text_vals[@]}"}";        do compose_args+=(--text "$v");      done
  [[ -n "$raw_jql" ]] && compose_args+=(--jql "$raw_jql")

  local jql
  jql=$(jql_compose "${compose_args[@]}") || return $?

  if ! (( quiet )); then echo "INFO: composed JQL: $jql" >&2; fi

  # Resolve fields
  local -a resolved_tokens=()
  local tok
  for tok in "${field_tokens[@]+"${field_tokens[@]}"}"; do
    [[ -z "$tok" ]] && continue
    resolved_tokens+=("$(_jira_search_resolve_field "$tok")")
  done
  local fields_array
  if (( ${#resolved_tokens[@]} > 0 )); then
    fields_array=$(printf '%s\n' "${resolved_tokens[@]}" | jq -R . | jq -s .)
  else
    fields_array='[]'
  fi

  # Build request body
  local body
  body=$(jq -n \
    --arg    jql        "$jql" \
    --argjson fields    "$fields_array" \
    --argjson maxResults "$limit" \
    --arg    pageToken  "$page_token" \
    '{
       jql: $jql,
       fields: $fields,
       fieldsByKeys: false,
       maxResults: $maxResults
     } + (if $pageToken == "" then {} else {nextPageToken: $pageToken} end)')

  local response
  response=$(bash "$_JIRA_SEARCH_SCRIPT_DIR/jira-request.sh" \
    POST /rest/api/3/search/jql --json "$body") || return $?

  if (( render_adf )); then
    response=$(printf '%s' "$response" | \
      bash "$_JIRA_SEARCH_SCRIPT_DIR/jira-render-adf-fields.sh") || return $?
  fi

  printf '%s\n' "$response"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail
  _jira_search "$@"
fi
