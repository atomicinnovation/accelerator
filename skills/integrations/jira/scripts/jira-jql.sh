#!/usr/bin/env bash
# jira-jql.sh — pure-bash JQL builder (sourceable library, no set -euo pipefail)
#
# Exit codes:
#   0  success
#  30  E_JQL_NO_PROJECT   — compose called without --project or --all-projects
#  31  E_JQL_UNSAFE_VALUE — value contains a control character (override with --unsafe)
#  32  E_JQL_BAD_FLAG     — unrecognised flag
#  33  E_JQL_EMPTY_VALUE  — empty string passed where a value was expected
#
# Public functions:
#   jql_quote_value [--unsafe] <value>
#   jql_filter <field> <value>
#   jql_in     <field> <value>...
#   jql_not_in <field> <value>...
#   jql_split_neg <value>...           sets JQL_POSITIVES and JQL_NEGATIVES arrays
#   jql_compose [flags...]             see function for full flag list

# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

# _jql_find_control_char <value>
# Prints "0xHH (NAME)" for the first control char found; exits 0 if found, 1 if not.
_jql_ctrl_names=(NUL SOH STX ETX EOT ENQ ACK BEL BS HT LF VT FF CR SO SI
                  DLE DC1 DC2 DC3 DC4 NAK SYN ETB CAN EM SUB ESC FS GS RS US)
_jql_find_control_char() {
  local val="$1"
  local i code char
  for (( i = 0; i < ${#val}; i++ )); do
    char="${val:$i:1}"
    # bash printf trick: 'X prints the numeric value of X
    code=$(printf '%d' "'$char" 2>/dev/null) || continue
    if (( code >= 0 && code <= 31 )); then
      printf '0x%02X (%s)\n' "$code" "${_jql_ctrl_names[$code]}"
      return 0
    elif (( code == 127 )); then
      printf '0x7F (DEL)\n'
      return 0
    fi
  done
  return 1
}

# ---------------------------------------------------------------------------
# jql_quote_value [--unsafe] <value>
# ---------------------------------------------------------------------------
jql_quote_value() {
  local unsafe=0
  if [[ "${1-}" == "--unsafe" ]]; then
    unsafe=1
    shift
  fi

  if [[ $# -eq 0 ]] || [[ -z "${1+set}" ]]; then
    echo "E_JQL_EMPTY_VALUE: empty value supplied; use --empty <field> for IS EMPTY clauses" >&2
    return 33
  fi

  local val="$1"

  if [[ -z "$val" ]]; then
    echo "E_JQL_EMPTY_VALUE: empty value supplied; use --empty <field> for IS EMPTY clauses" >&2
    return 33
  fi

  if [[ "$unsafe" -eq 0 ]]; then
    local ctrl_info
    ctrl_info=$(_jql_find_control_char "$val") || ctrl_info=""
    if [[ -n "$ctrl_info" ]]; then
      echo "E_JQL_UNSAFE_VALUE: control character $ctrl_info in value is not safely quotable; pass --unsafe to override" >&2
      return 31
    fi
  fi

  # Escape single quotes by doubling them, then wrap in single quotes.
  # Use a variable for the single-quote character to avoid backslash ambiguity
  # inside double-quoted ${//} expansions.
  local sq="'"
  local escaped="${val//$sq/$sq$sq}"
  printf "'%s'" "$escaped"
}

# ---------------------------------------------------------------------------
# jql_filter <field> <value>
# ---------------------------------------------------------------------------
jql_filter() {
  local field="$1" val="$2"
  local quoted
  quoted=$(jql_quote_value "$val") || return $?
  printf '%s = %s' "$field" "$quoted"
}

# ---------------------------------------------------------------------------
# jql_in <field> <value>...
# ---------------------------------------------------------------------------
jql_in() {
  local field="$1"
  shift
  local parts=()
  local v quoted
  for v in "$@"; do
    quoted=$(jql_quote_value "$v") || return $?
    parts+=("$quoted")
  done
  local list
  list=$(IFS=','; printf '%s' "${parts[*]}")
  printf '%s IN (%s)' "$field" "$list"
}

# ---------------------------------------------------------------------------
# jql_not_in <field> <value>...
# ---------------------------------------------------------------------------
jql_not_in() {
  local field="$1"
  shift
  local parts=()
  local v quoted
  for v in "$@"; do
    quoted=$(jql_quote_value "$v") || return $?
    parts+=("$quoted")
  done
  local list
  list=$(IFS=','; printf '%s' "${parts[*]}")
  printf '%s NOT IN (%s)' "$field" "$list"
}

# ---------------------------------------------------------------------------
# jql_split_neg <value>...
# Sets module-level JQL_POSITIVES and JQL_NEGATIVES arrays.
# Values prefixed with ~ go to JQL_NEGATIVES (prefix stripped).
# ---------------------------------------------------------------------------
jql_split_neg() {
  JQL_POSITIVES=()
  JQL_NEGATIVES=()
  local v
  for v in "$@"; do
    if [[ "$v" == ~* ]]; then
      JQL_NEGATIVES+=("${v:1}")
    else
      JQL_POSITIVES+=("$v")
    fi
  done
}

# ---------------------------------------------------------------------------
# jql_compose [flags...]
#
# Flags:
#   --project <key>      add project = 'KEY' clause
#   --all-projects       omit project clause entirely
#   --status <value>     accumulate status values (~ prefix = negation)
#   --label <value>      accumulate label values (~ prefix = negation)
#   --assignee <value>   accumulate assignee values (~ prefix = negation)
#   --empty <field>      add <field> IS EMPTY clause
#   --not-empty <field>  add <field> IS NOT EMPTY clause
#   --jql <raw>          append raw JQL verbatim (with AND); emits stderr warning
# ---------------------------------------------------------------------------
jql_compose() {
  local project="" all_projects=0 raw_jql=""
  local -a status_vals=() label_vals=() assignee_vals=()
  local -a empty_fields=() not_empty_fields=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --project)      project="$2";        shift 2 ;;
      --all-projects) all_projects=1;      shift ;;
      --status)       status_vals+=("$2"); shift 2 ;;
      --label)        label_vals+=("$2");  shift 2 ;;
      --assignee)     assignee_vals+=("$2"); shift 2 ;;
      --empty)        empty_fields+=("$2"); shift 2 ;;
      --not-empty)    not_empty_fields+=("$2"); shift 2 ;;
      --jql)          raw_jql="$2";        shift 2 ;;
      *)
        echo "E_JQL_BAD_FLAG: unrecognised flag: $1" >&2
        return 32
        ;;
    esac
  done

  if [[ "$all_projects" -eq 0 && -z "$project" ]]; then
    echo "E_JQL_NO_PROJECT: specify --project <key> or --all-projects" >&2
    return 30
  fi

  local -a clauses=()

  # Project clause
  if [[ -n "$project" ]]; then
    local pq
    pq=$(jql_quote_value "$project") || return $?
    clauses+=("project = $pq")
  fi

  # IS EMPTY / IS NOT EMPTY
  local f
  for f in "${empty_fields[@]+"${empty_fields[@]}"}"; do
    clauses+=("$f IS EMPTY")
  done
  for f in "${not_empty_fields[@]+"${not_empty_fields[@]}"}"; do
    clauses+=("$f IS NOT EMPTY")
  done

  # Multi-value fields: status, labels, assignee
  _jql_compose_field clauses status  status_vals  || return $?
  _jql_compose_field clauses labels  label_vals   || return $?
  _jql_compose_field clauses assignee assignee_vals || return $?

  # Raw JQL append
  if [[ -n "$raw_jql" ]]; then
    echo "Warning: raw JQL passed through without validation" >&2
    clauses+=("$raw_jql")
  fi

  local result="" c
  for c in "${clauses[@]+"${clauses[@]}"}"; do
    if [[ -z "$result" ]]; then
      result="$c"
    else
      result="$result AND $c"
    fi
  done
  printf '%s' "$result"
}

# _jql_compose_field <clauses_nameref> <jql_field> <values_nameref>
# Splits values into positives/negatives, appends IN/NOT IN clauses.
_jql_compose_field() {
  local -n _clauses="$1"
  local jql_field="$2"
  local -n _vals="$3"

  if [[ ${#_vals[@]} -eq 0 ]]; then
    return 0
  fi

  jql_split_neg "${_vals[@]}"

  local clause
  if [[ ${#JQL_POSITIVES[@]} -gt 0 ]]; then
    clause=$(jql_in "$jql_field" "${JQL_POSITIVES[@]}") || return $?
    _clauses+=("$clause")
  fi
  if [[ ${#JQL_NEGATIVES[@]} -gt 0 ]]; then
    clause=$(jql_not_in "$jql_field" "${JQL_NEGATIVES[@]}") || return $?
    _clauses+=("$clause")
  fi
}
