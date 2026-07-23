#!/usr/bin/env bash
set -euo pipefail

# jira-resolve-fields.sh — read-only resolver for the Jira create field set.
#
# The SINGLE source of truth for two mappings, shared by the user-facing
# /create-jira-issue work-item-file mode AND the /create-work-item dispatcher's
# Jira branch, so the two entry points can never map the same work item to a
# different issue type or project.
#
#   1. kind → issue type:  story→Story, bug→Bug, task/spike→Task, epic→Epic;
#      anything else → Task (reported as the "default" source).
#   2. --project resolution, in precedence order:
#        a. an explicit --project flag,                       (source: flag)
#        b. work.default_project_code from config,            (source: config)
#        c. the project code embedded in a project-coded id.  (source: id)
#      Unresolvable → E_RESOLVE_NO_PROJECT, naming work.default_project_code.
#
# Two input modes:
#   --file <work-item-file> [--project KEY]
#       Read kind / id / external_id from the file. If external_id is already
#       present (non-empty after trimming quotes/whitespace) → E_RESOLVE_ALREADY_SYNCED
#       (re-pushing a synced item would create a duplicate). Used by the skill.
#   --kind KIND [--project KEY] [--id ID]
#       Resolve from explicit values (no file, no already-synced guard). Used by
#       the dispatcher, which holds the draft in memory before any file exists.
#
# Output (stdout, one tab-separated line):
#   <issue_type>\t<issue_type_source>\t<project>\t<project_source>
#   issue_type_source ∈ {mapped, default}; project_source ∈ {flag, config, id}
#
# Exit codes (see EXIT_CODES.md):
#   0   success
#   108 E_RESOLVE_NO_PROJECT       project unresolvable; names work.default_project_code
#   109 E_RESOLVE_ALREADY_SYNCED   --file already carries a non-empty external_id
#   2   usage error

_JRF_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_JRF_PLUGIN_ROOT="$(cd "$_JRF_SCRIPT_DIR/../../../.." && pwd)"
source "$_JRF_PLUGIN_ROOT/scripts/config-common.sh"

readonly E_RESOLVE_NO_PROJECT=108
readonly E_RESOLVE_ALREADY_SYNCED=109

_jrf_usage() {
  cat <<'USAGE' >&2
Usage:
  jira-resolve-fields.sh --file <work-item-file> [--project KEY]
  jira-resolve-fields.sh --kind KIND [--project KEY] [--id ID]
USAGE
}

# Extract a top-level frontmatter field from frontmatter text on stdin (first
# match wins; surrounding quotes stripped).
_jrf_fm_field() {
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

# kind → "<issue_type>\t<source>". source is "mapped" or "default". The trailing
# newline matters: the caller reads this with `read`, which returns non-zero on
# an unterminated line and would trip `set -e`.
_jrf_issue_type() {
  case "$1" in
    story) printf 'Story\tmapped\n' ;;
    bug) printf 'Bug\tmapped\n' ;;
    epic) printf 'Epic\tmapped\n' ;;
    task | spike) printf 'Task\tmapped\n' ;;
    *) printf 'Task\tdefault\n' ;;
  esac
}

# Extract a project code from a project-coded id (e.g. PROJ-0042 → PROJ). Prints
# nothing for a bare-numeric/legacy id.
_jrf_project_from_id() {
  local id="$1"
  if [[ "$id" =~ ^([A-Za-z][A-Za-z0-9]*)-[0-9]+$ ]]; then
    printf '%s' "${BASH_REMATCH[1]}"
  fi
}

_jrf_main() {
  local mode="" file="" kind="" project_flag="" id=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --file)
        mode="file"
        file="$2"
        shift 2
        ;;
      --kind)
        mode="${mode:-kind}"
        kind="$2"
        shift 2
        ;;
      --project)
        project_flag="$2"
        shift 2
        ;;
      --id)
        id="$2"
        shift 2
        ;;
      --help | -h)
        _jrf_usage
        exit 0
        ;;
      *)
        _jrf_usage
        return 2
        ;;
    esac
  done

  local external_id=""
  if [[ "$mode" == "file" ]]; then
    if [[ -z "$file" || ! -r "$file" ]]; then
      printf 'jira-resolve-fields.sh: --file path required and must be readable\n' >&2
      return 2
    fi
    local fm
    if ! fm=$(config_extract_frontmatter "$file") || [[ -z "$fm" ]]; then
      printf 'jira-resolve-fields.sh: %s has no parseable frontmatter\n' "$file" >&2
      return 2
    fi
    kind=$(printf '%s\n' "$fm" | _jrf_fm_field kind)
    id=$(printf '%s\n' "$fm" | _jrf_fm_field id)
    external_id=$(printf '%s\n' "$fm" | _jrf_fm_field external_id)

    # Already-synced guard (presence-based, same normalisation as the Linear
    # guard and work-item-sync-label.sh).
    local eid_trimmed
    eid_trimmed=$(printf '%s' "$external_id" | sed "s/^[[:space:]\"']*//; s/[[:space:]\"']*\$//")
    if [[ -n "$eid_trimmed" ]]; then
      printf 'E_RESOLVE_ALREADY_SYNCED: %s already carries external_id %s; nothing to create\n' \
        "$file" "$eid_trimmed" >&2
      return $E_RESOLVE_ALREADY_SYNCED
    fi
  elif [[ "$mode" != "kind" ]]; then
    _jrf_usage
    return 2
  fi

  local issue_type issue_type_source
  IFS=$'\t' read -r issue_type issue_type_source < <(_jrf_issue_type "$kind")

  # Resolve the project in precedence order: flag → config → id project code.
  local project="" project_source=""
  if [[ -n "$project_flag" ]]; then
    project="$project_flag"
    project_source="flag"
  else
    local cfg
    cfg=$("${ACCELERATOR_BIN:-$_JRF_PLUGIN_ROOT/bin/accelerator}" config work default_project_code) || cfg=""
    if [[ -n "$cfg" ]]; then
      project="$cfg"
      project_source="config"
    elif [[ -n "$id" ]]; then
      local from_id
      from_id=$(_jrf_project_from_id "$id")
      if [[ -n "$from_id" ]]; then
        project="$from_id"
        project_source="id"
      fi
    fi
  fi

  if [[ -z "$project" ]]; then
    printf 'E_RESOLVE_NO_PROJECT: cannot resolve a Jira project — pass --project, set work.default_project_code, or use a project-coded id\n' >&2
    return $E_RESOLVE_NO_PROJECT
  fi

  printf '%s\t%s\t%s\t%s\n' "$issue_type" "$issue_type_source" "$project" "$project_source"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _jrf_main "$@"
fi
