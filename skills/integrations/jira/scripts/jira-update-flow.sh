#!/usr/bin/env bash
# jira-update-flow.sh — Update an existing Jira issue via PUT /rest/api/3/issue/{key}.
#
# Usage:
#   jira-update-flow.sh KEY [flags]
#
# Required:
#   KEY                       Issue key (positional), e.g. "ENG-1"
#
# At least one mutating flag is required (else exits E_UPDATE_NO_OPS):
#   --summary "..."           Replace summary
#   --body "..."              Replace description (Markdown → ADF)
#   --body-file PATH          Replace description from file (Markdown → ADF)
#   --priority NAME           Replace priority
#   --assignee @me|ACCTID|"" Replace or unassign (empty string = unassign)
#                             Email addresses are NOT supported.
#   --reporter @me|ACCTID     Replace reporter
#   --parent KEY              Replace parent issue key
#   --parent ""               Clear parent
#   --label NAME (repeatable) Replace ALL labels (mutually exclusive with
#                             --add-label/--remove-label)
#   --add-label NAME (rep.)   Add label incrementally via update op
#   --remove-label NAME (rep.) Remove label incrementally via update op
#   --component NAME (rep.)   Replace ALL components (mutually exclusive with
#                             --add-component/--remove-component)
#   --add-component NAME      Add component incrementally
#   --remove-component NAME   Remove component incrementally
#   --custom SLUG=VALUE       Custom field (use @json:<literal> for arrays/objects)
#
# Optional:
#   --no-notify               Suppress watcher notifications (?notifyUsers=false)
#   --render-adf              No-op (PUT 204 has no response body)
#   --no-render-adf           No-op
#   --print-payload           Dry-run: print {method,path,queryParams,body}, exit 0
#   --quiet                   Suppress INFO stderr lines
#   --help, -h                Print this banner and exit 0
#
# Exit codes:
#   110 E_UPDATE_NO_KEY              no issue key positional argument
#   111 E_UPDATE_LABEL_MODE_CONFLICT --label mixed with --add/--remove-label
#                                    or --component mixed with --add/--remove-component
#   112 E_UPDATE_NO_OPS              no mutating flags supplied
#   113 E_UPDATE_BAD_FLAG            unrecognised flag
#   114 E_UPDATE_BAD_FIELD           --custom value failed schema coercion or slug unknown
#   115 E_UPDATE_NO_SITE_CACHE       --assignee/--reporter @me but site.json missing
#   116 E_UPDATE_NO_BODY             --body-file not found or body resolution failed
#   117 E_UPDATE_BAD_ASSIGNEE        --assignee not @me, "" (unassign), or raw accountId
#   11–23, 34 propagated from jira-request.sh (auth/transport/4xx/5xx)
#
# See also: EXIT_CODES.md

_JIRA_UPDATE_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_JIRA_UPDATE_SCRIPT_DIR/jira-common.sh"
source "$_JIRA_UPDATE_SCRIPT_DIR/jira-body-input.sh"
source "$_JIRA_UPDATE_SCRIPT_DIR/jira-custom-fields.sh"

_jira_update_usage() {
  cat <<'USAGE'
Usage: jira-update-flow.sh KEY [flags]

  Updates an existing Jira issue via PUT /rest/api/3/issue/{key}.
  At least one mutating flag is required.

Required:
  KEY                       Issue key (positional), e.g. "ENG-1"

Mutating flags (at least one required):
  --summary "..."           Replace summary
  --body "..."              Replace description (Markdown)
  --body-file PATH          Replace description from file (Markdown)
  --priority NAME           Replace priority
  --assignee @me|ACCTID|"" Replace or unassign (email not supported)
  --reporter @me|ACCTID     Replace reporter
  --parent KEY|""           Replace or clear parent
  --label NAME (rep.)       Replace ALL labels (exclusive with --add/--remove-label)
  --add-label NAME (rep.)   Add label incrementally
  --remove-label NAME (rep.) Remove label incrementally
  --component NAME (rep.)   Replace ALL components
  --add-component NAME      Add component incrementally
  --remove-component NAME   Remove component incrementally
  --custom SLUG=VALUE       Custom field (@json:<literal> for arrays/objects)

Optional:
  --no-notify               Suppress notifications (?notifyUsers=false)
  --print-payload           Dry-run: print payload JSON and exit 0
  --quiet                   Suppress INFO stderr lines
  --help, -h                Print this banner and exit 0

Example:
  jira-update-flow.sh ENG-1 --add-label needs-review --no-notify
  jira-update-flow.sh ENG-1 --summary "revised" --priority High
USAGE
}

_jira_update_resolve_principal() {
  local value="$1" flag_name="$2" code_bad="$3"
  if [[ "$value" == "@me" ]]; then
    local state_dir
    state_dir=$(jira_state_dir) || return 1
    local site_json="$state_dir/site.json"
    if [[ ! -f "$site_json" ]]; then
      printf 'E_UPDATE_NO_SITE_CACHE: %s @me but site.json missing; run /init-jira\n' "$flag_name" >&2
      return 115
    fi
    local id
    id=$(jq -r '.accountId // empty' "$site_json")
    if [[ -z "$id" ]] || ! [[ "$id" =~ ^[A-Za-z0-9:_-]+$ ]]; then
      printf 'E_UPDATE_NO_SITE_CACHE: accountId in site.json is missing or malformed; run /init-jira\n' >&2
      return 115
    fi
    printf '%s' "$id"
    return 0
  fi
  if [[ "$value" =~ @ ]] || ! [[ "$value" =~ ^[A-Za-z0-9:_-]+$ ]]; then
    printf 'E_UPDATE_BAD_ASSIGNEE: %s accepts @me, "" (unassign), or a raw accountId; email addresses are not resolved (got: %s)\n' \
      "$flag_name" "$value" >&2
    return "$code_bad"
  fi
  printf '%s' "$value"
}

_jira_update() {
  jira_require_dependencies

  local key=""
  local summary="" priority=""
  local body_inline="" body_file=""
  local body_inline_set=0 body_file_set=0
  local assignee="" assignee_set=0
  local reporter="" reporter_set=0
  local parent="" parent_set=0
  local -a set_labels=() add_labels=() remove_labels=()
  local -a set_components=() add_components=() remove_components=()
  local -a customs=()
  local no_notify=0 print_payload=0 quiet=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help|-h)
        _jira_update_usage; exit 0 ;;
      --summary)
        summary="$2"; shift 2 ;;
      --body)
        body_inline="$2"; body_inline_set=1; shift 2 ;;
      --body-file)
        body_file="$2"; body_file_set=1; shift 2 ;;
      --assignee)
        assignee="$2"; assignee_set=1; shift 2 ;;
      --reporter)
        reporter="$2"; reporter_set=1; shift 2 ;;
      --priority)
        priority="$2"; shift 2 ;;
      --parent)
        parent="$2"; parent_set=1; shift 2 ;;
      --label)
        set_labels+=("$2"); shift 2 ;;
      --add-label)
        add_labels+=("$2"); shift 2 ;;
      --remove-label)
        remove_labels+=("$2"); shift 2 ;;
      --component)
        set_components+=("$2"); shift 2 ;;
      --add-component)
        add_components+=("$2"); shift 2 ;;
      --remove-component)
        remove_components+=("$2"); shift 2 ;;
      --custom)
        customs+=("$2"); shift 2 ;;
      --no-notify)
        no_notify=1; shift ;;
      --render-adf|--no-render-adf)
        shift ;;
      --print-payload)
        print_payload=1; shift ;;
      --quiet|-q)
        quiet=1; shift ;;
      -*)
        printf 'E_UPDATE_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        _jira_update_usage >&2
        return 113 ;;
      *)
        if [[ -z "$key" ]]; then
          key="$1"; shift
        else
          printf 'E_UPDATE_BAD_FLAG: unexpected positional argument: %s\n' "$1" >&2
          _jira_update_usage >&2
          return 113
        fi ;;
    esac
  done

  # Validate required key
  if [[ -z "$key" ]]; then
    printf 'E_UPDATE_NO_KEY: issue key required as first positional argument\n' >&2
    return 110
  fi

  # Validate label-mode exclusivity
  if (( ${#set_labels[@]} > 0 )) && \
     (( ${#add_labels[@]} + ${#remove_labels[@]} > 0 )); then
    printf 'E_UPDATE_LABEL_MODE_CONFLICT: --label and --add-label/--remove-label are mutually exclusive. Use --label to replace all labels at once, or --add-label/--remove-label to add and remove individually.\n' >&2
    return 111
  fi

  # Validate component-mode exclusivity
  if (( ${#set_components[@]} > 0 )) && \
     (( ${#add_components[@]} + ${#remove_components[@]} > 0 )); then
    printf 'E_UPDATE_LABEL_MODE_CONFLICT: --component and --add-component/--remove-component are mutually exclusive.\n' >&2
    return 111
  fi

  # Resolve assignee / reporter
  if (( assignee_set )) && [[ -n "$assignee" ]]; then
    assignee=$(_jira_update_resolve_principal "$assignee" "--assignee" 117) || return $?
  fi
  if (( reporter_set )) && [[ -n "$reporter" ]]; then
    reporter=$(_jira_update_resolve_principal "$reporter" "--reporter" 117) || return $?
  fi

  # Resolve body if provided
  local body_md=""
  if (( body_inline_set || body_file_set )); then
    local body_src_args=()
    if (( body_inline_set )); then body_src_args+=(--body "$body_inline"); fi
    if (( body_file_set ));   then body_src_args+=(--body-file "$body_file"); fi

    local body_rc=0
    body_md=$(jira_resolve_body "${body_src_args[@]}") || body_rc=$?
    if (( body_rc != 0 )); then
      printf 'E_UPDATE_NO_BODY: body resolution failed\n' >&2
      return 116
    fi
  fi

  # Convert body Markdown → ADF
  local adf_doc=""
  if [[ -n "$body_md" ]]; then
    local adf_rc=0
    adf_doc=$(printf '%s' "$body_md" | bash "$_JIRA_UPDATE_SCRIPT_DIR/jira-md-to-adf.sh") || adf_rc=$?
    if (( adf_rc != 0 )); then
      printf 'Warning: body Markdown could not be converted to ADF (exit %d); description unchanged\n' \
        "$adf_rc" >&2
      adf_doc=""
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
    cf_id=$(bash "$_JIRA_UPDATE_SCRIPT_DIR/jira-fields.sh" resolve "$cf_slug" 2>/tmp/update-fields-err.tmp) \
      || cf_id_rc=$?
    if (( cf_id_rc != 0 )); then
      cat /tmp/update-fields-err.tmp >&2
      printf 'E_UPDATE_BAD_FIELD: field slug "%s" not found; run /init-jira --refresh-fields to update field cache\n' \
        "$cf_slug" >&2
      return 114
    fi

    local cv cv_rc=0
    cv=$(_jira_coerce_custom_value "$cf_id" "$cf_raw" "$fields_json" "E_UPDATE_BAD_FIELD") || cv_rc=$?
    if (( cv_rc != 0 )); then
      return 114
    fi

    custom_fields_obj=$(jq -n \
      --argjson o "$custom_fields_obj" \
      --arg k "$cf_id" \
      --argjson v "$cv" \
      '$o + {($k): $v}')
  done

  # Build fields_obj (set semantics)
  local fields_obj="{}"

  if [[ -n "$summary" ]]; then
    fields_obj=$(jq -n --argjson o "$fields_obj" --arg v "$summary" '$o + {summary: $v}')
  fi

  if [[ -n "$adf_doc" ]]; then
    fields_obj=$(jq -n --argjson o "$fields_obj" --argjson v "$adf_doc" '$o + {description: $v}')
  fi

  if [[ -n "$priority" ]]; then
    fields_obj=$(jq -n --argjson o "$fields_obj" --arg v "$priority" '$o + {priority: {name: $v}}')
  fi

  if (( assignee_set )); then
    if [[ -z "$assignee" ]]; then
      fields_obj=$(jq -n --argjson o "$fields_obj" '$o + {assignee: {accountId: null}}')
    else
      fields_obj=$(jq -n --argjson o "$fields_obj" --arg v "$assignee" '$o + {assignee: {accountId: $v}}')
    fi
  fi

  if (( reporter_set )) && [[ -n "$reporter" ]]; then
    fields_obj=$(jq -n --argjson o "$fields_obj" --arg v "$reporter" '$o + {reporter: {accountId: $v}}')
  fi

  if (( parent_set )); then
    if [[ -z "$parent" ]]; then
      fields_obj=$(jq -n --argjson o "$fields_obj" '$o + {parent: null}')
    else
      fields_obj=$(jq -n --argjson o "$fields_obj" --arg v "$parent" '$o + {parent: {key: $v}}')
    fi
  fi

  if (( ${#set_labels[@]} > 0 )); then
    local labels_arr
    labels_arr=$(printf '%s\n' "${set_labels[@]}" | jq -R . | jq -s .)
    fields_obj=$(jq -n --argjson o "$fields_obj" --argjson v "$labels_arr" '$o + {labels: $v}')
  fi

  if (( ${#set_components[@]} > 0 )); then
    local comps_arr
    comps_arr=$(printf '%s\n' "${set_components[@]}" | jq -R '{name: .}' | jq -s .)
    fields_obj=$(jq -n --argjson o "$fields_obj" --argjson v "$comps_arr" '$o + {components: $v}')
  fi

  # Merge custom fields into fields_obj
  if [[ "$custom_fields_obj" != "{}" ]]; then
    fields_obj=$(jq -n --argjson o "$fields_obj" --argjson c "$custom_fields_obj" '$o + $c')
  fi

  # Build update_obj (op-list semantics)
  local update_obj="{}"

  if (( ${#add_labels[@]} + ${#remove_labels[@]} > 0 )); then
    local labels_ops="[]"
    local lbl
    for lbl in "${add_labels[@]+"${add_labels[@]}"}"; do
      labels_ops=$(jq -n --argjson o "$labels_ops" --arg v "$lbl" '$o + [{add: $v}]')
    done
    for lbl in "${remove_labels[@]+"${remove_labels[@]}"}"; do
      labels_ops=$(jq -n --argjson o "$labels_ops" --arg v "$lbl" '$o + [{remove: $v}]')
    done
    update_obj=$(jq -n --argjson o "$update_obj" --argjson v "$labels_ops" '$o + {labels: $v}')
  fi

  if (( ${#add_components[@]} + ${#remove_components[@]} > 0 )); then
    local comps_ops="[]"
    local comp
    for comp in "${add_components[@]+"${add_components[@]}"}"; do
      comps_ops=$(jq -n --argjson o "$comps_ops" --arg v "$comp" '$o + [{add: {name: $v}}]')
    done
    for comp in "${remove_components[@]+"${remove_components[@]}"}"; do
      comps_ops=$(jq -n --argjson o "$comps_ops" --arg v "$comp" '$o + [{remove: {name: $v}}]')
    done
    update_obj=$(jq -n --argjson o "$update_obj" --argjson v "$comps_ops" '$o + {components: $v}')
  fi

  # Assemble payload: include fields/update only when non-empty
  local payload
  payload=$(jq -n \
    --argjson fields "$fields_obj" \
    --argjson update "$update_obj" \
    '{} +
      (if $fields == {} then {} else {fields: $fields} end) +
      (if $update == {} then {} else {update: $update} end)')

  if [[ "$payload" == "{}" ]]; then
    printf 'E_UPDATE_NO_OPS: no fields specified to update\n' >&2
    return 112
  fi

  # Build query params
  local -a query_params=()
  if (( no_notify )); then
    query_params+=(--query "notifyUsers=false")
  fi

  # --print-payload dry-run
  if (( print_payload )); then
    local qp_obj="{}"
    if (( no_notify )); then
      qp_obj='{"notifyUsers":"false"}'
    fi
    jq -n \
      --arg     method "PUT" \
      --arg     path   "/rest/api/3/issue/$key" \
      --argjson qp     "$qp_obj" \
      --argjson body   "$payload" \
      '{method: $method, path: $path, queryParams: $qp, body: $body}'
    return 0
  fi

  if ! (( quiet )); then
    printf 'INFO: updating issue %s\n' "$key" >&2
  fi

  # Write payload to tmpfile and PUT
  local tmpfile; tmpfile=$(mktemp)
  trap 'rm -f "$tmpfile"' RETURN
  printf '%s' "$payload" > "$tmpfile"

  local req_exit=0
  bash "$_JIRA_UPDATE_SCRIPT_DIR/jira-request.sh" \
    PUT "/rest/api/3/issue/$key" \
    --json "@$tmpfile" \
    "${query_params[@]+"${query_params[@]}"}" || req_exit=$?

  if (( req_exit != 0 )); then
    if ! _jira_emit_generic_hint "$req_exit"; then
      case "$req_exit" in
        13) printf 'Hint: issue not found or you do not have edit permission.\n' >&2 ;;
      esac
    fi
    return "$req_exit"
  fi
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -euo pipefail
  _jira_update "$@"
fi
