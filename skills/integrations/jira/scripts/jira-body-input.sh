#!/usr/bin/env bash
# jira-body-input.sh — Sourceable helper for resolving a body string from one
# of four sources in priority order:
#   1. --body "<inline>"       (highest priority)
#   2. --body-file <path>
#   3. piped stdin             (only when --allow-stdin is set)
#   4. $EDITOR tempfile        (only when --allow-editor is set)
#
# Usage when sourced:
#   source "$DIR/jira-body-input.sh"
#   body=$(jira_resolve_body \
#     --body "$body_arg" \
#     --body-file "$body_file_arg" \
#     [--allow-stdin] [--allow-editor]) || return $?
#
# Caller pattern in flow helpers (pass flags only when user supplied them):
#   local resolve_args=(--allow-stdin --allow-editor)
#   (( opt_body_set ))      && resolve_args+=(--body      "$opt_body")
#   (( opt_body_file_set )) && resolve_args+=(--body-file "$opt_body_file")
#   body=$(jira_resolve_body "${resolve_args[@]}") || return 105
#
# Internal numeric codes emitted on stderr (callers map non-zero to their own
# flow-specific codes; these internal values are documented in EXIT_CODES.md):
#   1  E_BODY_BAD_FLAG          Unrecognised flag or duplicate --body/--body-file
#   2  E_BODY_FILE_NOT_FOUND    --body-file path does not exist
#   3  E_BODY_STDIN_DISALLOWED  Piped stdin present but --allow-stdin not set
#   4  E_BODY_EDITOR_FAILED     Editor process exited non-zero
#   5  E_BODY_NONE_PROVIDED     No body source available and no fallback
#   6  E_BODY_EDITOR_INVALID    $EDITOR contains characters outside [A-Za-z0-9_./-]
#
# This function never calls exit and does not register traps — it is safe to
# source and call from a parent flow helper that manages its own exit policy.

jira_resolve_body() {
  local body="" body_file=""
  local body_set=0 body_file_set=0
  local allow_stdin=0 allow_editor=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --body)
        if (( body_set )); then
          printf 'E_BODY_BAD_FLAG: --body specified more than once\n' >&2
          return 1
        fi
        body="$2"; body_set=1; shift 2 ;;
      --body-file)
        if (( body_file_set )); then
          printf 'E_BODY_BAD_FLAG: --body-file specified more than once\n' >&2
          return 1
        fi
        body_file="$2"; body_file_set=1; shift 2 ;;
      --allow-stdin)  allow_stdin=1;  shift ;;
      --allow-editor) allow_editor=1; shift ;;
      *)
        printf 'E_BODY_BAD_FLAG: unrecognised flag: %s\n' "$1" >&2
        return 1 ;;
    esac
  done

  # Priority 1: inline --body (empty string is valid)
  if (( body_set )); then
    printf '%s' "$body"
    return 0
  fi

  # Priority 2: --body-file
  if (( body_file_set )); then
    if [[ ! -f "$body_file" ]]; then
      printf 'E_BODY_FILE_NOT_FOUND: %s\n' "$body_file" >&2
      return 2
    fi
    cat "$body_file"
    return 0
  fi

  # Priority 3: piped stdin (only when stdin is not a terminal)
  # Test seam (honoured only when ACCELERATOR_TEST_MODE=1): JIRA_BODY_STDIN_IS_TTY_TEST=1
  # forces the function to treat stdin as a terminal, enabling EDITOR tests in CI.
  local _stdin_is_piped=0
  if [[ ! -t 0 ]]; then
    _stdin_is_piped=1
  fi
  if [[ "${ACCELERATOR_TEST_MODE:-}" == "1" && "${JIRA_BODY_STDIN_IS_TTY_TEST:-}" == "1" ]]; then
    _stdin_is_piped=0
  fi

  if (( _stdin_is_piped )); then
    if (( allow_stdin )); then
      cat
      return 0
    fi
    printf 'E_BODY_STDIN_DISALLOWED: piped stdin present but --allow-stdin not set\n' >&2
    return 3
  fi

  # Priority 4: $EDITOR tempfile
  if (( allow_editor )); then
    local editor="${EDITOR:-vi}"
    # Reject EDITOR values containing shell metacharacters or whitespace.
    # Only POSIX-portable characters plus "/", ".", "_", "-" are accepted.
    # This guards against injection via a compromised or malformed EDITOR env var.
    if [[ ! "$editor" =~ ^[A-Za-z0-9_./-]+$ ]]; then
      printf 'E_BODY_EDITOR_INVALID: $EDITOR contains disallowed characters: %s. Only [A-Za-z0-9_./-] are accepted (no spaces or shell flags). Set EDITOR to a bare executable path, e.g. EDITOR=/usr/bin/vim.\n' \
        "$editor" >&2
      return 6
    fi
    local tmp editor_rc=0
    tmp=$(mktemp)
    "$editor" "$tmp" || editor_rc=$?
    if (( editor_rc != 0 )); then
      rm -f "$tmp"
      printf 'E_BODY_EDITOR_FAILED: editor exited %d\n' "$editor_rc" >&2
      return 4
    fi
    cat "$tmp"
    rm -f "$tmp"
    return 0
  fi

  printf 'E_BODY_NONE_PROVIDED: --body, --body-file, piped stdin (--allow-stdin), or --allow-editor required\n' >&2
  return 5
}
