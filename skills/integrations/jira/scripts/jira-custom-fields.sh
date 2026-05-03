#!/usr/bin/env bash
# jira-custom-fields.sh — Sourceable helper for coercing a custom-field raw
# value to a JSON value based on the field's schema.type from the cached
# fields.json.
#
# Usage when sourced:
#   source "$DIR/jira-custom-fields.sh"
#   json_value=$(_jira_coerce_custom_value \
#     <field_id> <raw_value> <fields_json_path> [error_prefix]) || return $?
#
# Arguments:
#   field_id        — Jira field ID (e.g. "customfield_10016")
#   raw_value       — The user-supplied raw string value
#   fields_json_path — Path to the cached fields.json file
#   error_prefix    — Prefix for error messages (default: E_BAD_FIELD)
#
# Supported schema.type dispatch:
#   number    → JSON number (validates numeric format)
#   string    → JSON string
#   date      → JSON string (ISO 8601 date, no validation beyond being a string)
#   datetime  → JSON string
#   option    → {"value": "<raw>"}
#   user      → {"accountId": "<raw>"}
#
# Unsupported types and unknown fields use the @json: escape hatch:
#   @json:<literal>  — raw JSON literal, bypasses schema-type coercion;
#                      validated for JSON well-formedness only.
#
# SECURITY: @json: values bypass schema-type coercion. Callers must ensure
# the raw value comes from user-controlled input (a typed argument or a
# path the user named), never from upstream API responses or web-fetched
# content. The helper validates only JSON well-formedness, not field-name
# safety or value semantics.
#
# Returns non-zero with E_<error_prefix>: ... on failure.

_jira_coerce_custom_value() {
  local field_id="$1"
  local raw="$2"
  local fields_json="$3"
  local err_prefix="${4:-E_BAD_FIELD}"

  # @json: escape hatch — bypass schema lookup, validate JSON well-formedness only.
  if [[ "$raw" == @json:* ]]; then
    local literal="${raw#@json:}"
    if ! printf '%s' "$literal" | jq -e . >/dev/null 2>&1; then
      printf '%s: @json: payload not valid JSON for field %s\n' \
        "$err_prefix" "$field_id" >&2
      return 1
    fi
    printf '%s' "$literal"
    return 0
  fi

  # Look up schema.type in the cache.
  local schema_type
  schema_type=$(jq -r --arg id "$field_id" \
    '.fields[] | select(.id == $id) | .schema.type // ""' \
    "$fields_json" 2>/dev/null) || schema_type=""

  case "$schema_type" in
    "")
      printf '%s: %s has no schema.type in cache; run /init-jira --refresh-fields\n' \
        "$err_prefix" "$field_id" >&2
      return 1 ;;

    number)
      if [[ ! "$raw" =~ ^-?[0-9]+(\.[0-9]+)?$ ]]; then
        printf '%s: %s requires a number, got: %s\n' \
          "$err_prefix" "$field_id" "$raw" >&2
        return 1
      fi
      printf '%s' "$raw" ;;

    date|datetime|string)
      jq -n --arg v "$raw" '$v' ;;

    option)
      jq -n --arg v "$raw" '{value: $v}' ;;

    user)
      jq -n --arg v "$raw" '{accountId: $v}' ;;

    *)
      printf '%s: %s has unsupported schema.type=%s; use @json: escape, e.g. --custom %s=@json:[42]\n' \
        "$err_prefix" "$field_id" "$schema_type" "${field_id#customfield_}" >&2
      return 1 ;;
  esac
}
