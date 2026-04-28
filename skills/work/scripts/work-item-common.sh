#!/usr/bin/env bash
# Shared work-item helpers. Source this from work-item-* scripts:
#
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/work-item-common.sh"
#
# Centralises pattern parsing/compilation, ID canonicalisation, legacy
# detection, and frontmatter predicates so the same logic is not
# re-implemented across the resolver, list-work-items, update-work-item,
# allocator, and migration scripts.
#
# Calling convention: result on stdout, errors on stderr with stable
# E_* prefix, exit code 0 on success / non-zero on error. Boolean
# predicates use exit code only (no stdout).
#
# Stable error-code prefixes (testable contract):
#   E_PATTERN_NO_NUMBER_TOKEN    rule 1
#   E_PATTERN_HOSTILE_CHAR       rule 2
#   E_PATTERN_ADJACENT_TOKENS    rule 3
#   E_PATTERN_BAD_FORMAT_SPEC    rule 4 / unknown token / unmatched brace
#   E_PATTERN_BAD_PROJECT_VALUE  rule 5
#   E_PATTERN_OVERFLOW           allocator overflow
#   E_PATTERN_MISSING_PROJECT    pattern needs {project} but no value
#   E_PATTERN_PROJECT_UNUSED     --project given but pattern lacks {project}

# -- internal helpers --------------------------------------------------

# Per-char ERE escape for literal text emitted into the scan regex.
# Defensively escapes regex metachars so a future widening of the
# project-value rule is safe without code changes.
_wip_regex_escape_char() {
  case "$1" in
    '.'|'^'|'$'|'*'|'+'|'?'|'('|')'|'['|']'|'{'|'}'|'|'|'\')
      printf '\\%s' "$1"
      ;;
    *) printf '%s' "$1" ;;
  esac
}

# Internal pattern compiler. Mode = "scan" or "format".
# Echoes the compiled string on stdout; non-zero on validation failure.
_wip_compile() {
  local pattern="$1"
  local project_value="${2-}"
  local mode="$3"

  if [ -z "$pattern" ]; then
    echo "E_PATTERN_BAD_FORMAT_SPEC: pattern is empty" >&2
    return 1
  fi

  local out=""
  local saw_number=0
  local last_was_dynamic=0
  local i=0
  local len="${#pattern}"
  local ch next

  while [ "$i" -lt "$len" ]; do
    ch="${pattern:$i:1}"
    next=""
    if [ "$((i + 1))" -lt "$len" ]; then
      next="${pattern:$((i+1)):1}"
    fi

    # Escape sequences {{ and }}
    if [ "$ch" = "{" ] && [ "$next" = "{" ]; then
      if [ "$mode" = "scan" ]; then
        out+='\{'
      else
        out+='{'
      fi
      i=$((i + 2))
      last_was_dynamic=0
      continue
    fi
    if [ "$ch" = "}" ] && [ "$next" = "}" ]; then
      if [ "$mode" = "scan" ]; then
        out+='\}'
      else
        out+='}'
      fi
      i=$((i + 2))
      last_was_dynamic=0
      continue
    fi

    if [ "$ch" = "}" ]; then
      echo "E_PATTERN_BAD_FORMAT_SPEC: unmatched '}' at offset $i in pattern '$pattern'" >&2
      return 1
    fi

    if [ "$ch" = "{" ]; then
      # Find closing '}' (no nesting permitted)
      local j=$((i + 1))
      local close_idx=-1
      while [ "$j" -lt "$len" ]; do
        if [ "${pattern:$j:1}" = "}" ]; then
          close_idx="$j"
          break
        fi
        if [ "${pattern:$j:1}" = "{" ]; then
          echo "E_PATTERN_BAD_FORMAT_SPEC: nested '{' in token starting at offset $i" >&2
          return 1
        fi
        j=$((j + 1))
      done
      if [ "$close_idx" -eq -1 ]; then
        echo "E_PATTERN_BAD_FORMAT_SPEC: unclosed token starting at offset $i in pattern '$pattern'" >&2
        return 1
      fi

      local token="${pattern:$((i + 1)):$((close_idx - i - 1))}"
      local token_total=$((close_idx - i + 1))

      if [ "$last_was_dynamic" -eq 1 ]; then
        echo "E_PATTERN_ADJACENT_TOKENS: dynamic tokens must be separated by literal text (rule 3): '$pattern'" >&2
        return 1
      fi

      if [ "$token" = "project" ]; then
        if [ -z "$project_value" ]; then
          # When called from --validate (no project value), accept and
          # emit a placeholder; substitution happens at compile time.
          if [ "$mode" = "validate" ]; then
            :
          else
            echo "E_PATTERN_MISSING_PROJECT: pattern '$pattern' contains {project} but no value supplied" >&2
            return 1
          fi
        else
          # Validate project value against rule 5
          if ! [[ "$project_value" =~ ^[A-Za-z][A-Za-z0-9]*$ ]]; then
            echo "E_PATTERN_BAD_PROJECT_VALUE: project value '$project_value' must match [A-Za-z][A-Za-z0-9]* (rule 5)" >&2
            return 1
          fi
          if [ "$mode" = "scan" ]; then
            local k=0 plen=${#project_value}
            while [ "$k" -lt "$plen" ]; do
              out+="$(_wip_regex_escape_char "${project_value:$k:1}")"
              k=$((k + 1))
            done
          elif [ "$mode" = "format" ]; then
            # Escape % to %% for printf-safety
            local escaped="${project_value//%/%%}"
            out+="$escaped"
          fi
        fi
        last_was_dynamic=1
      elif [[ "$token" =~ ^number(:(.+))?$ ]]; then
        local spec="${BASH_REMATCH[2]:-04d}"
        # Rule 4: spec must be 0Nd form, N >= 1
        if ! [[ "$spec" =~ ^0[1-9][0-9]*d$ ]]; then
          echo "E_PATTERN_BAD_FORMAT_SPEC: {number} format spec '$spec' must match 0Nd (rule 4)" >&2
          return 1
        fi
        if [ "$mode" = "scan" ]; then
          out+='([0-9]+)'
        elif [ "$mode" = "format" ]; then
          out+="%$spec"
        fi
        saw_number=1
        last_was_dynamic=1
      else
        echo "E_PATTERN_BAD_FORMAT_SPEC: unknown token '{$token}' in pattern '$pattern'" >&2
        return 1
      fi

      i=$((i + token_total))
      continue
    fi

    # Literal char
    case "$ch" in
      '/'|'\\'|':'|'*'|'?'|'<'|'>'|'|'|'"')
        echo "E_PATTERN_HOSTILE_CHAR: literal '$ch' is forbidden in patterns (rule 2)" >&2
        return 1
        ;;
    esac

    if [ "$mode" = "scan" ]; then
      out+="$(_wip_regex_escape_char "$ch")"
    elif [ "$mode" = "format" ]; then
      if [ "$ch" = "%" ]; then
        out+="%%"
      else
        out+="$ch"
      fi
    fi
    i=$((i + 1))
    last_was_dynamic=0
  done

  if [ "$saw_number" -eq 0 ]; then
    echo "E_PATTERN_NO_NUMBER_TOKEN: pattern '$pattern' must contain at least one {number} token (rule 1)" >&2
    return 1
  fi

  if [ "$mode" = "scan" ]; then
    printf '^%s-\n' "$out"
  elif [ "$mode" = "format" ]; then
    printf '%s\n' "$out"
  fi
  return 0
}

# -- public API --------------------------------------------------------

# wip_validate_pattern <pattern>
# Exit 0 if valid, non-zero with E_PATTERN_* on stderr otherwise.
# No stdout output.
wip_validate_pattern() {
  local pattern="${1-}"
  if [ -z "$pattern" ]; then
    echo "E_PATTERN_BAD_FORMAT_SPEC: pattern is empty" >&2
    return 1
  fi
  _wip_compile "$pattern" "" "validate" >/dev/null
}

# wip_compile_scan <pattern> <project_value>
# Echoes the ERE scan regex on stdout (capture group 1 = number).
wip_compile_scan() {
  local pattern="${1-}"
  local project_value="${2-}"
  _wip_compile "$pattern" "$project_value" "scan"
}

# wip_compile_format <pattern> <project_value>
# Echoes the printf format string on stdout.
wip_compile_format() {
  local pattern="${1-}"
  local project_value="${2-}"
  _wip_compile "$pattern" "$project_value" "format"
}

# wip_pattern_max_number <pattern>
# Echoes 10^N - 1 for the configured {number} width N (default 4).
wip_pattern_max_number() {
  local pattern="${1-}"
  if [ -z "$pattern" ]; then
    echo "E_PATTERN_BAD_FORMAT_SPEC: pattern is empty" >&2
    return 1
  fi
  # Find the {number[:0Nd]} token width
  local width=4
  if [[ "$pattern" =~ \{number:0([1-9][0-9]*)d\} ]]; then
    width="${BASH_REMATCH[1]}"
  elif [[ "$pattern" =~ \{number\} ]]; then
    width=4
  else
    echo "E_PATTERN_NO_NUMBER_TOKEN: pattern '$pattern' has no {number} token" >&2
    return 1
  fi
  # 10^width - 1
  local cap=1
  local i=0
  while [ "$i" -lt "$width" ]; do
    cap=$((cap * 10))
    i=$((i + 1))
  done
  printf '%d\n' "$((cap - 1))"
}

# wip_is_legacy_id <id>
# Predicate: exit 0 iff <id> is a legacy bare-number ID — matches
# ^[0-9]+$ with <= 4 digits and at least one non-zero digit.
# No stdout.
wip_is_legacy_id() {
  local id="${1-}"
  [[ "$id" =~ ^[0-9]{1,4}$ ]] || return 1
  [[ "$id" =~ [1-9] ]] || return 1
  return 0
}

# wip_pad_legacy_number <input>
# Echoes <input> zero-padded to 4 digits.
wip_pad_legacy_number() {
  local input="${1-}"
  if ! [[ "$input" =~ ^[0-9]+$ ]]; then
    echo "E_PATTERN_BAD_FORMAT_SPEC: '$input' is not a positive integer" >&2
    return 1
  fi
  printf '%04d\n' "$((10#$input))"
}

# wip_parse_full_id <id> <pattern>
# Echoes <project>\t<number> on stdout (tab-separated). Exits non-zero
# if the ID does not parse against the pattern.
wip_parse_full_id() {
  local id="${1-}"
  local pattern="${2-}"
  # Build a lenient scan regex that captures both project (if any) and number.
  # We don't anchor the trailing '-' since this is parsing an ID, not a filename.
  local has_project=0
  if [[ "$pattern" == *"{project}"* ]]; then
    has_project=1
  fi

  if [ "$has_project" -eq 1 ]; then
    # Replace {project} with ([A-Za-z][A-Za-z0-9]*) and {number...} with ([0-9]+)
    local pat_re="$pattern"
    # Strip format spec, keeping the token only
    pat_re="${pat_re//\{number:[0-9][0-9]*d\}/$'\x01'NUM$'\x01'}"
    pat_re="${pat_re//\{number\}/$'\x01'NUM$'\x01'}"
    pat_re="${pat_re//\{project\}/$'\x01'PROJ$'\x01'}"
    # Escape literals
    local result=""
    local i=0
    local len="${#pat_re}"
    while [ "$i" -lt "$len" ]; do
      local ch="${pat_re:$i:1}"
      if [ "$ch" = $'\x01' ]; then
        # Token marker: read until next \x01
        local j=$((i + 1))
        while [ "${pat_re:$j:1}" != $'\x01' ]; do
          j=$((j + 1))
        done
        local marker="${pat_re:$((i+1)):$((j - i - 1))}"
        case "$marker" in
          NUM)  result+='([0-9]+)' ;;
          PROJ) result+='([A-Za-z][A-Za-z0-9]*)' ;;
        esac
        i=$((j + 1))
      else
        result+="$(_wip_regex_escape_char "$ch")"
        i=$((i + 1))
      fi
    done
    if [[ "$id" =~ ^${result}$ ]]; then
      printf '%s\t%s\n' "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}"
      return 0
    fi
    return 1
  else
    # Pattern has no {project}, just match the number.
    local fmt
    fmt=$(wip_compile_format "$pattern" "") || return 1
    if [[ "$id" =~ ^[0-9]+$ ]]; then
      printf '\t%s\n' "$id"
      return 0
    fi
    return 1
  fi
}

# wip_canonicalise_id <input> <pattern> <project_value>
# Echoes the canonical full-ID string on stdout.
# - If input is already a full ID matching the pattern, normalise (no change).
# - If input is a legacy bare number and the pattern lacks {project},
#   zero-pad to width.
# - If input is a bare number and the pattern has {project}, prepend
#   the supplied project value.
wip_canonicalise_id() {
  local input="${1-}"
  local pattern="${2-}"
  local project_value="${3-}"

  # Strip surrounding quotes
  if [[ "$input" =~ ^\"(.*)\"$ ]]; then
    input="${BASH_REMATCH[1]}"
  elif [[ "$input" =~ ^\'(.*)\'$ ]]; then
    input="${BASH_REMATCH[1]}"
  fi

  if [ -z "$input" ]; then
    echo "E_PATTERN_BAD_FORMAT_SPEC: empty input" >&2
    return 1
  fi

  local has_project=0
  if [[ "$pattern" == *"{project}"* ]]; then
    has_project=1
  fi

  # Determine width
  local width=4
  if [[ "$pattern" =~ \{number:0([1-9][0-9]*)d\} ]]; then
    width="${BASH_REMATCH[1]}"
  fi

  # Try parsing as a full ID against pattern
  local parsed
  if parsed=$(wip_parse_full_id "$input" "$pattern" 2>/dev/null); then
    # Re-emit zero-padded for consistent canonical form
    local proj num
    proj="${parsed%	*}"
    num="${parsed#*	}"
    local fmt
    fmt=$(wip_compile_format "$pattern" "$proj") 2>/dev/null || {
      # Pattern with no {project} — fmt requires no project value
      fmt=$(wip_compile_format "$pattern" "")
    }
    # shellcheck disable=SC2059
    printf "$fmt\n" "$((10#$num))"
    return 0
  fi

  # Bare-number input
  if [[ "$input" =~ ^[0-9]+$ ]]; then
    local num=$((10#$input))
    if [ "$has_project" -eq 1 ]; then
      if [ -z "$project_value" ]; then
        echo "E_PATTERN_MISSING_PROJECT: bare number '$input' under pattern '$pattern' requires a project value" >&2
        return 1
      fi
      local fmt
      fmt=$(wip_compile_format "$pattern" "$project_value") || return 1
      # shellcheck disable=SC2059
      printf "$fmt\n" "$num"
      return 0
    else
      local fmt
      fmt=$(wip_compile_format "$pattern" "") || return 1
      # shellcheck disable=SC2059
      printf "$fmt\n" "$num"
      return 0
    fi
  fi

  echo "E_PATTERN_BAD_FORMAT_SPEC: input '$input' is not a recognised ID shape" >&2
  return 1
}

# wip_extract_id_from_filename <name> <pattern> <project_value>
# Echoes the extracted full ID on stdout via the compiled scan regex.
# Exits non-zero if the filename does not match.
wip_extract_id_from_filename() {
  local name="${1-}"
  local pattern="${2-}"
  local project_value="${3-}"
  local scan
  scan=$(wip_compile_scan "$pattern" "$project_value") || return 1
  if [[ "$name" =~ $scan ]]; then
    # Reconstruct the full-ID portion: prefix before the captured number
    # plus the captured number. This is just the matched scan regex
    # contents minus the trailing '-'.
    local matched="${BASH_REMATCH[0]}"
    # Strip trailing '-'
    printf '%s\n' "${matched%-}"
    return 0
  fi
  return 1
}

# wip_is_work_item_file <path>
# Predicate: exit 0 iff the file has YAML frontmatter and a
# work_item_id field with a non-empty string value. No stdout.
wip_is_work_item_file() {
  local path="${1-}"
  [ -f "$path" ] || return 1
  awk '
    NR == 1 && /^---[[:space:]]*$/ { in_fm = 1; next }
    in_fm && /^---[[:space:]]*$/ { exit }
    in_fm {
      if ($0 ~ /^[[:space:]]*work_item_id[[:space:]]*:[[:space:]]*/) {
        val = $0
        sub(/^[[:space:]]*work_item_id[[:space:]]*:[[:space:]]*/, "", val)
        sub(/[[:space:]]+$/, "", val)
        # Strip surrounding quotes
        if (val ~ /^".*"$/ || val ~ /^'\''.*'\''$/) {
          val = substr(val, 2, length(val) - 2)
        }
        if (length(val) > 0) {
          found = 1
          exit
        }
      }
    }
    END { exit (found ? 0 : 1) }
  ' "$path"
}
