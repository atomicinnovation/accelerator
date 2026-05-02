#!/usr/bin/env bash
set -euo pipefail
# jira-render-adf-fields.sh — Walk a Jira API JSON document and render
# all ADF-bearing fields to Markdown in place.
#
# Usage: jira-render-adf-fields.sh < input.json
# Reads JSON from stdin; writes transformed JSON to stdout.
#
# Both single-issue (GET /issue/{key}) and search-response
# ({issues:[...]}) shapes are handled.
#
# The --render-adf flag on jira-search-flow.sh and jira-show-flow.sh
# is implemented exactly once here.
#
# Exit codes:
#  0  success
# 90  E_RENDER_BAD_INPUT — stdin is not valid JSON
#
# Test seams (honoured only when ACCELERATOR_TEST_MODE=1):
#   ACCELERATOR_JIRA_FIELDS_CACHE_PATH_TEST — override the fields.json path
#   ACCELERATOR_JIRA_ADF_RENDERER_TEST      — override the jira-adf-to-md.sh path

_JIRA_RENDER_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_RENDER_SCRIPT_DIR/jira-common.sh"

# _render_at_jq_path <json_doc> <path_json_array>
# If the value at <path> is an ADF doc object (type == "doc"), renders it to
# Markdown and returns the modified document.  Otherwise returns unchanged.
# The type-predicate gate makes the filter idempotent: a second pass over
# already-rendered strings short-circuits without re-spawning the renderer.
_render_at_jq_path() {
  local json="$1"
  local path_arr="$2"  # JSON array: e.g. ["fields","description"]

  local is_adf
  is_adf=$(printf '%s' "$json" | jq -r --argjson p "$path_arr" \
    'if (getpath($p) | type == "object" and .type == "doc") then "true" else "false" end' \
    2>/dev/null) || is_adf="false"

  if [[ "$is_adf" != "true" ]]; then
    printf '%s' "$json"
    return 0
  fi

  # Resolve the ADF renderer path (test seam: ACCELERATOR_JIRA_ADF_RENDERER_TEST)
  local _renderer
  if [[ "${ACCELERATOR_TEST_MODE:-}" == "1" \
     && -n "${ACCELERATOR_JIRA_ADF_RENDERER_TEST:-}" ]]; then
    _renderer="$ACCELERATOR_JIRA_ADF_RENDERER_TEST"
  else
    _renderer="$_JIRA_RENDER_SCRIPT_DIR/jira-adf-to-md.sh"
  fi

  local adf_json rendered
  adf_json=$(printf '%s' "$json" | jq --argjson p "$path_arr" 'getpath($p)')
  # NOTE: keep local and assignment on separate lines so the renderer's exit
  # code is visible to || rendered=""; combining would silently swallow it.
  rendered=$(printf '%s' "$adf_json" | bash "$_renderer" 2>/dev/null) || rendered=""

  printf '%s' "$json" | jq --argjson p "$path_arr" --arg md "$rendered" 'setpath($p; $md)'
}

# _render_issue <issue_json> [custom_field_id...]
# Renders all ADF-bearing fields in a single issue JSON object.
_render_issue() {
  local json="$1"
  shift
  local -a custom_ids=("$@")

  # Scalar ADF paths
  json=$(_render_at_jq_path "$json" '["fields","description"]')
  json=$(_render_at_jq_path "$json" '["fields","environment"]')

  # Custom textarea fields from the cache
  local id
  for id in "${custom_ids[@]+"${custom_ids[@]}"}"; do
    json=$(_render_at_jq_path "$json" "[\"fields\",\"${id}\"]")
  done

  # Comment bodies: iterate by index so we can build numeric-indexed paths
  local count
  count=$(printf '%s' "$json" | jq -r '(.fields.comment.comments // []) | length' \
    2>/dev/null) || count=0
  local i
  for (( i = 0; i < count; i++ )); do
    json=$(_render_at_jq_path "$json" "[\"fields\",\"comment\",\"comments\",$i,\"body\"]")
  done

  printf '%s' "$json"
}

main() {
  local input
  input=$(cat)

  # Validate JSON
  if ! printf '%s' "$input" | jq -e . >/dev/null 2>&1; then
    echo "E_RENDER_BAD_INPUT: stdin is not valid JSON" >&2
    exit 90
  fi

  # Resolve the custom-fields cache path
  # Test seam (strict equality — "true" / "false" / "" do NOT activate):
  local fields_cache
  if [[ "${ACCELERATOR_TEST_MODE:-}" == "1" \
     && -n "${ACCELERATOR_JIRA_FIELDS_CACHE_PATH_TEST:-}" ]]; then
    fields_cache="$ACCELERATOR_JIRA_FIELDS_CACHE_PATH_TEST"
  else
    local state_dir
    state_dir=$(jira_state_dir 2>/dev/null) || state_dir=""
    fields_cache="${state_dir}/fields.json"
  fi

  # Collect IDs of custom textarea fields from the cache
  local -a custom_ids=()
  if [[ -f "$fields_cache" ]]; then
    while IFS= read -r id; do
      [[ -n "$id" ]] && custom_ids+=("$id")
    done < <(jq -r \
      '.fields[] | select(.schema.custom == "com.atlassian.jira.plugin.system.customfieldtypes:textarea") | .id' \
      "$fields_cache" 2>/dev/null)
  fi

  local result="$input"

  # Dispatch: search response (has "issues" key) vs single issue
  local has_issues
  has_issues=$(printf '%s' "$result" | jq -r 'if has("issues") then "true" else "false" end' \
    2>/dev/null) || has_issues="false"

  if [[ "$has_issues" == "true" ]]; then
    # Search response: iterate issues[] by index
    local issue_count
    issue_count=$(printf '%s' "$result" | jq -r '.issues | length' 2>/dev/null) || issue_count=0
    local i issue_json rendered_issue
    for (( i = 0; i < issue_count; i++ )); do
      issue_json=$(printf '%s' "$result" | jq ".issues[$i]")
      rendered_issue=$(_render_issue "$issue_json" "${custom_ids[@]+"${custom_ids[@]}"}")
      result=$(printf '%s' "$result" | jq --argjson i "$i" --argjson v "$rendered_issue" \
        '.issues[$i] = $v')
    done
  else
    # Single issue
    result=$(_render_issue "$result" "${custom_ids[@]+"${custom_ids[@]}"}")
  fi

  printf '%s\n' "$result"
}

main
