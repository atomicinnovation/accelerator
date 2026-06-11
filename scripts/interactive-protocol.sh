#!/usr/bin/env bash
# Wire-protocol helpers shared by the interactive migration harness
# (migration-side, scripts/interactive-harness.sh) and the runner-side
# library (skills/config/migrate/scripts/interactive-lib.sh).
#
# Single source of truth: both sides MUST use the same escape/unescape
# functions, otherwise round-tripping breaks at the wire boundary.
#
# ── Protocol state machine ─────────────────────────────────────────────────
#
# Frames are line-delimited, TAB-separated. JSON does NOT appear on the
# wire; the session log is the only JSON surface (on disk).
#
# Runner → migration (migration's stdin):
#   INIT          resume_state_path  decisions_path     handshake
#   DECIDE        outcome  value                        user's choice
#   APPLY         key                                   record durable; mutate
#   DRIFT_CLEARED key                                   stale record removed
#   ABORT                                               cancel
#
# Migration → runner (migration's stdout):
#   READY              session_log_path
#   MECHANICAL_APPLIED key
#   RESUMED_APPLIED    key
#   RESUMED_SKIPPED    key
#   PROMPT             key path anchor proposed predicate_value extras_tsv display_b64
#   VALIDATE_ERR       message
#   RECORDED           key outcome proposed user_value extras_tsv
#   APPLIED_CONFIRM    key
#   DRIFT              key
#   DONE
#   FAIL               message
#
# Legal transitions per transformation:
#   PROMPT → DECIDE → optional (VALIDATE_ERR loop) → RECORDED → APPLY
#          → APPLIED_CONFIRM
#   On RESUMED_APPLIED / RESUMED_SKIPPED / MECHANICAL_APPLIED no DECIDE
#   round-trip happens.
#   DRIFT → DRIFT_CLEARED → PROMPT (fresh predicate evaluation + prompt).
#   FAIL aborts unconditionally.
#
# Field-escaping rule (applied in order):
#   1. backslash → \\
#   2. TAB       → \t
#   3. newline   → \n
# Unescape is a single-pass state machine reading \\ first.

# escape_field <value>
#   Encode <value> for TSV transmission. Emits the escaped bytes on
#   stdout (no trailing newline).
escape_field() {
  local v="$1"
  v=${v//\\/\\\\}
  v=${v//$'\t'/\\t}
  v=${v//$'\n'/\\n}
  printf '%s' "$v"
}

# unescape_field <value>
#   Decode <value> from TSV transmission. Single-pass: every \\ pair
#   becomes \, every \t becomes TAB, every \n becomes LF. A lone
#   backslash (no following \, t, or n) is preserved verbatim.
unescape_field() {
  local v="$1" out="" i ch next
  for ((i = 0; i < ${#v}; i++)); do
    ch="${v:$i:1}"
    # shellcheck disable=SC1003 # deliberate literal-backslash test for TSV unescaping, not a single-quote-escape mistake
    if [ "$ch" = '\' ] && [ $((i + 1)) -lt ${#v} ]; then
      next="${v:$((i + 1)):1}"
      # shellcheck disable=SC1003 # literal backslash branches for TSV decoding (\\ pair and lone \), not single-quote-escape mistakes
      case "$next" in
        '\\')
          out+='\\'
          i=$((i + 1))
          ;;
        '\')
          out+='\'
          i=$((i + 1))
          ;;
        t)
          out+=$'\t'
          i=$((i + 1))
          ;;
        n)
          out+=$'\n'
          i=$((i + 1))
          ;;
        *) out+="$ch" ;;
      esac
    else
      out+="$ch"
    fi
  done
  printf '%s' "$out"
}

# emit_frame <type> [field...]
#   Emit a TAB-separated frame line to stdout. Each non-type field is
#   escape_field-encoded. Caller redirects to the appropriate fd (the
#   runner writes to fd 7, the migration's stdin FIFO; the harness
#   writes to its own stdout, which the runner reads via the m2r FIFO).
#
#   Optional protocol-log capture: if MIGRATION_PROTOCOL_LOG_RUNNER /
#   MIGRATION_PROTOCOL_LOG_MIGRATION env var is set on the respective
#   side, the frame is also appended (test-only instrumentation).
emit_frame() {
  local type="$1"
  shift
  local line="$type"
  local f
  for f in "$@"; do
    line+=$'\t'$(escape_field "$f")
  done
  printf '%s\n' "$line"
  # Test-only instrumentation: protocol log capture. The runner side
  # exports MIGRATION_PROTOCOL_LOG_RUNNER; the migration side reads
  # MIGRATION_PROTOCOL_LOG_MIGRATION (set on the migration's environment
  # by the runner when it forks the child process).
  if [ -n "${INTERACTIVE_PROTOCOL_SIDE_LOG:-}" ]; then
    printf '%s\n' "$line" >>"$INTERACTIVE_PROTOCOL_SIDE_LOG"
  fi
}

# read_frame <fd>
#   Read one frame line from <fd> (default stdin). Sets:
#     FRAME_TYPE       — frame name
#     FRAME_FIELDS_RAW — raw escaped fields, TAB-joined (no leading TAB)
#     FRAME_FIELDS     — array of unescaped fields
#   Returns non-zero on EOF.
read_frame() {
  local fd="${1:-0}"
  local line
  if ! IFS= read -r -u "$fd" line; then
    FRAME_TYPE=""
    FRAME_FIELDS=()
    FRAME_FIELDS_RAW=""
    return 1
  fi
  if [ -n "${INTERACTIVE_PROTOCOL_SIDE_LOG:-}" ]; then
    printf '%s\n' "$line" >>"$INTERACTIVE_PROTOCOL_SIDE_LOG"
  fi
  # Split on TAB without touching anything else.
  local IFS_save="$IFS"
  IFS=$'\t'
  # shellcheck disable=SC2206
  local -a raw=($line)
  IFS="$IFS_save"
  FRAME_TYPE="${raw[0]:-}"
  FRAME_FIELDS=()
  local i
  for ((i = 1; i < ${#raw[@]}; i++)); do
    FRAME_FIELDS+=("$(unescape_field "${raw[$i]}")")
  done
  FRAME_FIELDS_RAW="${line#"$FRAME_TYPE"}"
  FRAME_FIELDS_RAW="${FRAME_FIELDS_RAW#$'\t'}"
}
