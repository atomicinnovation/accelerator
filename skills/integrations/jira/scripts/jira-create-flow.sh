#!/usr/bin/env bash
# jira-create-flow.sh — Create a Jira issue via POST /rest/api/3/issue.
#
# Usage:
#   jira-create-flow.sh [flags]
#
# Required (--type or --issuetype-id; --project resolved from config if omitted):
#   --project KEY           Project key (or use work.default_project_code)
#   --type NAME             Issue type by name, e.g. "Task"
#   --summary "..."         Single-line summary
#
# Optional:
#   --issuetype-id ID       Override --type with a numeric id (wins if both given)
#   --body "..."            Inline description body (Markdown)
#   --body-file PATH        Read description body from file (Markdown)
#                           (piped stdin is used when neither --body nor --body-file given)
#   --assignee @me|ACCTID  Assignee. @me resolved via site.json. Email not supported.
#   --reporter @me|ACCTID  Reporter. Same rules as --assignee.
#   --priority NAME         Priority name, e.g. "High"
#   --label NAME            Repeatable. Set labels on the new issue.
#   --component NAME        Repeatable. Set components on the new issue.
#   --parent KEY            Parent issue key, e.g. "ENG-99"
#   --custom SLUG=VALUE     Repeatable. Custom field by slug/id.
#                           Use @json:<literal> for arrays/objects.
#   --issuetype-id ID       Numeric issue type id (alternative to --type)
#   --render-adf            No-op on create (response has no ADF body)
#   --no-render-adf         No-op on create
#   --print-payload         Dry-run: print {method,path,queryParams,body} and exit 0
#   --quiet                 Suppress INFO stderr lines
#   --no-editor             Disallow $EDITOR fallback for body
#   --help, -h              Print this banner and exit 0
#
# Note: Jira always notifies on create; --no-notify is not accepted.
#
# Exit codes:
#   100 E_CREATE_NO_PROJECT    project missing and work.default_project_code unset
#   101 E_CREATE_NO_TYPE       --type and --issuetype-id both missing
#   102 E_CREATE_NO_SUMMARY    --summary missing
#   103 E_CREATE_BAD_FIELD     --custom value failed schema coercion or slug unknown
#   104 E_CREATE_BAD_FLAG      unrecognised flag
#   105 E_CREATE_NO_BODY       no body source available
#   106 E_CREATE_NO_SITE_CACHE --assignee/--reporter @me but site.json missing
#   107 E_CREATE_BAD_ASSIGNEE  --assignee value not @me or raw accountId
#   11–23, 34 propagated from jira-request.sh (auth/transport/4xx/5xx)
#
# See also: EXIT_CODES.md

_JIRA_CREATE_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_CREATE_SCRIPT_DIR/jira-common.sh"
source "$_JIRA_CREATE_SCRIPT_DIR/jira-body-input.sh"
source "$_JIRA_CREATE_SCRIPT_DIR/jira-custom-fields.sh"

_jira_create_usage() {
  cat <<'USAGE'
Usage: jira-create-flow.sh [flags]

  Creates a Jira issue via POST /rest/api/3/issue.

Required:
  --project KEY           Project key (or set work.default_project_code in config)
  --type NAME             Issue type by name, e.g. "Task"
  --summary "..."         Single-line summary

Optional:
  --issuetype-id ID       Numeric issue type id (wins over --type if both given)
  --body "..."            Inline description (Markdown)
  --body-file PATH        Description from file (Markdown)
  --assignee @me|ACCTID  Assignee (@me resolved via site.json; email not supported)
  --reporter @me|ACCTID  Reporter (same rules as --assignee)
  --priority NAME         Priority name, e.g. "High"
  --label NAME            Repeatable. Labels to set.
  --component NAME        Repeatable. Components to set.
  --parent KEY            Parent issue key, e.g. "ENG-99"
  --custom SLUG=VALUE     Repeatable. Custom field. Use @json:<literal> for
                          arrays/objects, e.g. --custom sprint=@json:[42]
  --print-payload         Dry-run: print payload JSON and exit 0 (no API call)
  --quiet                 Suppress INFO stderr lines
  --no-editor             Disallow $EDITOR fallback for body
  --help, -h              Print this banner and exit 0

Example:
  jira-create-flow.sh --project ENG --type Task --summary "foo" \
    --body-file plan.md --label needs-review
USAGE
}

_jira_create_resolve_principal() {
  local value="$1" flag_name="$2" code_no_cache=106 code_bad="$3"
  if [[ "$value" == "@me" ]]; then
    local state_dir
    state_dir=$(jira_state_dir) || return 1
    local site_json="$state_dir/site.json"
    if [[ ! -f "$site_json" ]]; then
      printf 'E_CREATE_NO_SITE_CACHE: %s @me but site.json missing; run /init-jira\n' "$flag_name" >&2
      return "$code_no_cache"
    fi
    local id
    id=$(jq -r '.accountId // empty' "$site_json")
    if [[ -z "$id" ]] || ! [[ "$id" =~ ^[A-Za-z0-9:_-]+$ ]]; then
      printf 'E_CREATE_NO_SITE_CACHE: accountId in site.json is missing or malformed; run /init-jira\n' >&2
      return "$code_no_cache"
    fi
    printf '%s' "$id"
    return 0
  fi
  # Validate raw accountId: must match ^[A-Za-z0-9:_-]+$
  # Reject anything containing @ but not literally @me (catches emails)
  if [[ "$value" =~ @ ]] || ! [[ "$value" =~ ^[A-Za-z0-9:_-]+$ ]]; then
    printf 'E_CREATE_BAD_ASSIGNEE: %s accepts @me or a raw accountId; email addresses are not resolved (got: %s)\n' \
      "$flag_name" "$value" >&2
    return "$code_bad"
  fi
  printf '%s' "$value"
}

_jira_create() {
  jira_require_dependencies

  local project="" type_name="" type_id="" summary=""
  local body_inline="" body_file=""
  local body_inline_set=0 body_file_set=0
  local assignee="" reporter="" priority="" parent=""
  local -a labels=() components=() customs=()
  local render_adf=1 print_payload=0 quiet=0 no_editor=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help|-h)
        _jira_create_usage; exit 0 ;;
      --project)
        project="$2"; shift 2 ;;
      --type)
        type_name="$2"; shift 2 ;;
      --issuetype-id)
        type_id="$2"; shift 2 ;;
      --summary)
        summary="$2"; shift 2 ;;
      --body)
        body_inline="$2"; body_inline_set=1; shift 2 ;;
      --body-file)
        body_file="$2"; body_file_set=1; shift 2 ;;
      --assignee)
        assignee="$2"; shift 2 ;;
      --reporter)
        reporter="$2"; shift 2 ;;
      --priority)
        priority="$2"; shift 2 ;;
      --label)
        labels+=("$2"); shift 2 ;;
      --component)
        components+=("$2"); shift 2 ;;
      --parent)
        parent="$2"; shift 2 ;;
      --custom)
        customs+=("$2"); shift 2 ;;
      --render-adf|--no-render-adf)
        shift ;;
      --print-payload)
        print_payload=1; shift ;;
      --quiet|-q)
        quiet=1; shift ;;
      --no-editor)
        no_editor=1; shift ;;
      *)
        printf 'E_CREATE_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        _jira_create_usage >&2
        return 104 ;;
    esac
  done

  # Resolve project from config if not supplied
  if [[ -z "$project" ]]; then
    local repo_root
    repo_root=$(find_repo_root 2>/dev/null) || repo_root=""
    if [[ -n "$repo_root" ]]; then
      local default_project
      default_project=$(cd "$repo_root" && \
        "$_JIRA_CREATE_SCRIPT_DIR/../../../../scripts/config-read-value.sh" \
        "work.default_project_code" "" 2>/dev/null) || default_project=""
      project="$default_project"
    fi
  fi

  # Validate required args
  if [[ -z "$project" ]]; then
    printf 'E_CREATE_NO_PROJECT: --project not given and work.default_project_code not configured\n' >&2
    return 100
  fi

  if [[ -z "$type_name" && -z "$type_id" ]]; then
    printf 'E_CREATE_NO_TYPE: --type or --issuetype-id is required\n' >&2
    return 101
  fi

  if [[ -z "$summary" ]]; then
    printf 'E_CREATE_NO_SUMMARY: --summary is required\n' >&2
    return 102
  fi

  # Resolve assignee / reporter
  if [[ -n "$assignee" ]]; then
    assignee=$(_jira_create_resolve_principal "$assignee" "--assignee" 107) || return $?
  fi
  if [[ -n "$reporter" ]]; then
    reporter=$(_jira_create_resolve_principal "$reporter" "--reporter" 107) || return $?
  fi

  # Resolve body via jira_resolve_body
  local body_src_args=()
  if (( body_inline_set )); then body_src_args+=(--body "$body_inline"); fi
  if (( body_file_set ));   then body_src_args+=(--body-file "$body_file"); fi
  if (( no_editor )); then
    body_src_args+=(--allow-stdin)
  else
    body_src_args+=(--allow-stdin --allow-editor)
  fi

  local body_md=""
  local body_rc=0
  body_md=$(jira_resolve_body "${body_src_args[@]}") || body_rc=$?
  if (( body_rc != 0 )); then
    printf 'E_CREATE_NO_BODY: no body source available (use --body, --body-file, stdin, or $EDITOR)\n' >&2
    return 105
  fi

  # Convert body Markdown → ADF
  local adf_doc="{}"
  if [[ -n "$body_md" ]]; then
    local adf_rc=0
    adf_doc=$(printf '%s' "$body_md" | bash "$_JIRA_CREATE_SCRIPT_DIR/jira-md-to-adf.sh") || adf_rc=$?
    if (( adf_rc != 0 )); then
      printf 'Warning: body Markdown could not be converted to ADF (exit %d); description will be empty\n' \
        "$adf_rc" >&2
      adf_doc="{}"
    fi
  fi

  # Resolve --custom slug → id + coerce value
  local fields_json=""
  local custom_fields_obj="{}"

  local cf
  for cf in "${customs[@]+"${customs[@]}"}"; do
    if [[ -z "$fields_json" ]]; then
      fields_json="$(jira_state_dir)/fields.json"
    fi
    local cf_slug="${cf%%=*}" cf_raw="${cf#*=}"
    local cf_id cf_id_rc=0
    cf_id=$(bash "$_JIRA_CREATE_SCRIPT_DIR/jira-fields.sh" resolve "$cf_slug" 2>/tmp/create-fields-err.tmp) \
      || cf_id_rc=$?
    if (( cf_id_rc != 0 )); then
      cat /tmp/create-fields-err.tmp >&2
      printf 'E_CREATE_BAD_FIELD: field slug "%s" not found; run /init-jira --refresh-fields to update field cache\n' \
        "$cf_slug" >&2
      return 103
    fi

    local cv cv_rc=0
    cv=$(_jira_coerce_custom_value "$cf_id" "$cf_raw" "$fields_json" "E_CREATE_BAD_FIELD") || cv_rc=$?
    if (( cv_rc != 0 )); then
      return 103
    fi

    custom_fields_obj=$(jq -n \
      --argjson o "$custom_fields_obj" \
      --arg k "$cf_id" \
      --argjson v "$cv" \
      '$o + {($k): $v}')
  done

  # Build labels JSON array
  local labels_json="[]"
  if (( ${#labels[@]} > 0 )); then
    labels_json=$(printf '%s\n' "${labels[@]}" | jq -R . | jq -s .)
  fi

  # Build components JSON array
  local components_json="[]"
  if (( ${#components[@]} > 0 )); then
    components_json=$(printf '%s\n' "${components[@]}" | jq -R '{name: .}' | jq -s .)
  fi

  # Build issuetype object: --issuetype-id wins over --type
  local issuetype_json
  if [[ -n "$type_id" ]]; then
    issuetype_json=$(jq -n --arg id "$type_id" '{id: $id}')
  else
    issuetype_json=$(jq -n --arg name "$type_name" '{name: $name}')
  fi

  # Assemble payload via jq
  local payload
  payload=$(jq -n \
    --arg     project        "$project" \
    --arg     summary        "$summary" \
    --argjson issuetype      "$issuetype_json" \
    --argjson description    "$adf_doc" \
    --argjson labels         "$labels_json" \
    --argjson components     "$components_json" \
    --argjson custom_fields  "$custom_fields_obj" \
    --arg     assignee       "$assignee" \
    --arg     reporter       "$reporter" \
    --arg     priority       "$priority" \
    --arg     parent         "$parent" \
    '{
      fields: (
        {
          project:     {key: $project},
          summary:     $summary,
          issuetype:   $issuetype,
          description: $description
        }
        + (if ($labels | length) > 0    then {labels:     $labels}                else {} end)
        + (if ($components | length) > 0 then {components: $components}            else {} end)
        + (if $assignee != ""            then {assignee:   {accountId: $assignee}} else {} end)
        + (if $reporter != ""            then {reporter:   {accountId: $reporter}} else {} end)
        + (if $priority != ""            then {priority:   {name: $priority}}      else {} end)
        + (if $parent   != ""            then {parent:     {key: $parent}}         else {} end)
        + $custom_fields
      )
    }')

  # --print-payload: emit dry-run shape and exit without calling the API
  if (( print_payload )); then
    jq -n \
      --arg     method "POST" \
      --arg     path   "/rest/api/3/issue" \
      --argjson qp     '{}' \
      --argjson body   "$payload" \
      '{method: $method, path: $path, queryParams: $qp, body: $body}'
    return 0
  fi

  if ! (( quiet )); then
    printf 'INFO: creating issue in project %s (type: %s)\n' \
      "$project" "${type_id:-$type_name}" >&2
  fi

  # Write payload to tmpfile and POST
  local tmpfile; tmpfile=$(mktemp)
  trap 'rm -f "$tmpfile"' RETURN
  printf '%s' "$payload" > "$tmpfile"

  local req_exit=0 response
  response=$(bash "$_JIRA_CREATE_SCRIPT_DIR/jira-request.sh" \
    POST /rest/api/3/issue --json "@$tmpfile") || req_exit=$?

  if (( req_exit != 0 )); then
    if ! _jira_emit_generic_hint "$req_exit"; then
      case "$req_exit" in
        13) printf 'Hint: check the project key is correct and you have create-issue permission.\n' >&2 ;;
      esac
    fi
    return "$req_exit"
  fi

  printf '%s\n' "$response"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail
  _jira_create "$@"
fi
