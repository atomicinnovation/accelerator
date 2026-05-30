#!/usr/bin/env bash
# JSONL composition helpers shared by atomic-common.sh (for
# atomic_jsonl_remove_by_key) and skills/config/migrate/scripts/
# interactive-lib.sh (for write_session_record).
#
# Single source of truth: the writer (jsonl_compose_record) and the
# remover (atomic_jsonl_remove_by_key) MUST agree on the escape rules
# for the JSON-string subset, otherwise the anchored-prefix match
# breaks.
#
# Source this file from a shell script:
#
#   source "$SCRIPT_DIR/jsonl-common.sh"

# jsonl_json_escape <value>
#   Escape <value> per JSON string-value rules. The only mandatory
#   transforms are: backslash, double-quote, NL, CR, TAB, and control
#   characters (\u00XX). Higher-plane UTF-8 is preserved as-is. The
#   function emits the escaped value (without surrounding quotes) on
#   stdout.
jsonl_json_escape() {
  local value="$1"
  # Order matters: backslash first, then quote, then control chars.
  # All other transforms operate on the post-backslash state.
  value=${value//\\/\\\\}
  value=${value//\"/\\\"}
  value=${value//$'\n'/\\n}
  value=${value//$'\r'/\\r}
  value=${value//$'\t'/\\t}
  value=${value//$'\b'/\\b}
  value=${value//$'\f'/\\f}
  # Remaining ASCII control chars (0x00–0x1F) — emit as \u00XX. Bash 4+
  # pattern substitution handles each control character byte individually.
  local out="" ch i
  if [[ "$value" == *[[:cntrl:]]* ]]; then
    for ((i = 0; i < ${#value}; i++)); do
      ch="${value:$i:1}"
      case "$ch" in
        [[:cntrl:]])
          out+=$(printf '\\u%04x' "'$ch")
          ;;
        *)
          out+="$ch"
          ;;
      esac
    done
    value="$out"
  fi
  printf '%s' "$value"
}

# jsonl_compose_record <key=value> ...
#   Compose one canonical JSONL record line. The first six fields are
#   framework-mandatory and emitted in canonical order; all subsequent
#   fields become author-declared extras in the order they appear.
#   The function reads named args via key=value pairs:
#     transformation_key=<key>      (required, MUST appear first)
#     schema_version=<int>          (required, MUST appear second)
#     outcome=<accepted|edited|skipped> (required)
#     proposed_value=<value>        (required)
#     user_value=<value>            (required for outcome=edited; omitted otherwise)
#     timestamp=<iso8601>           (required; caller supplies)
#   Any further pair becomes an extras key=value.
#
#   The function emits one valid JSON line to stdout (no trailing newline).
jsonl_compose_record() {
  local transformation_key="" schema_version="" outcome=""
  local proposed_value="" user_value="" timestamp=""
  local -a extras_keys=()
  local -a extras_values=()
  local has_user_value=0

  local pair key value
  for pair in "$@"; do
    case "$pair" in
      *=*) key="${pair%%=*}"; value="${pair#*=}" ;;
      *)
        echo "jsonl_compose_record: malformed pair '$pair'" >&2
        return 1
        ;;
    esac
    case "$key" in
      transformation_key) transformation_key="$value" ;;
      schema_version)     schema_version="$value" ;;
      outcome)            outcome="$value" ;;
      proposed_value)     proposed_value="$value" ;;
      user_value)         user_value="$value"; has_user_value=1 ;;
      timestamp)          timestamp="$value" ;;
      *)
        # Reject framework-mandatory names appearing in the extras slot
        # (collision with internal naming would corrupt round-trips).
        case "$key" in
          transformation_key|schema_version|outcome|proposed_value|user_value|timestamp)
            echo "jsonl_compose_record: reserved key '$key' in extras position" >&2
            return 1
            ;;
        esac
        # Enforce extras-key format (matches harness_extras_set check).
        if [[ ! "$key" =~ ^[a-z][a-z0-9_]*$ ]]; then
          echo "jsonl_compose_record: invalid extras key '$key'" >&2
          return 1
        fi
        extras_keys+=("$key")
        extras_values+=("$value")
        ;;
    esac
  done

  if [ -z "$transformation_key" ] || [ -z "$schema_version" ] \
     || [ -z "$outcome" ] || [ -z "$timestamp" ]; then
    echo "jsonl_compose_record: missing required field(s)" >&2
    return 1
  fi

  case "$outcome" in
    accepted|edited|skipped) ;;
    *)
      echo "jsonl_compose_record: invalid outcome '$outcome'" >&2
      return 1
      ;;
  esac

  # Emit canonical order. transformation_key MUST be first (so the
  # remover's anchored prefix match is well-defined). schema_version
  # MUST be second (so the resume parser can fail-fast on unknown
  # versions without parsing the rest of the line).
  printf '{"transformation_key":"%s","schema_version":%s,"outcome":"%s","proposed_value":"%s"' \
    "$(jsonl_json_escape "$transformation_key")" \
    "$schema_version" \
    "$outcome" \
    "$(jsonl_json_escape "$proposed_value")"
  if [ "$has_user_value" -eq 1 ]; then
    printf ',"user_value":"%s"' "$(jsonl_json_escape "$user_value")"
  fi
  printf ',"timestamp":"%s"' "$(jsonl_json_escape "$timestamp")"
  local i
  for ((i = 0; i < ${#extras_keys[@]}; i++)); do
    printf ',"%s":"%s"' \
      "${extras_keys[$i]}" \
      "$(jsonl_json_escape "${extras_values[$i]}")"
  done
  printf '}'
}
