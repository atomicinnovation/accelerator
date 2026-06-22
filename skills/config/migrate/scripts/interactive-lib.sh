#!/usr/bin/env bash
# Runner-side interactive migration library. Sourced unconditionally by
# run-migrations.sh — bash 3.2 compatible (no coproc, no associative
# arrays, no `declare -A`). Concurrency uses two named FIFOs to wire
# bidirectional pipes between runner and migration.
#
# Exposes:
#   is_interactive_migration <path>
#   run_interactive_migration <path> <id>
#
# Depends on:
#   $PROJECT_ROOT, $PLUGIN_ROOT, $STATE_FILE (set by run-migrations.sh)
#   atomic_jsonl_append, atomic_jsonl_remove_by_key (atomic-common.sh)
#   jsonl_compose_record (jsonl-common.sh)
#   escape_field, unescape_field, emit_frame, read_frame
#                                              (interactive-protocol.sh)

# shellcheck source=../../../../scripts/interactive-protocol.sh
# shellcheck disable=SC2154 # PLUGIN_ROOT provided by the interactive-migration harness environment
source "$PLUGIN_ROOT/scripts/interactive-protocol.sh"

is_interactive_migration() {
  local path="$1"
  [ -f "$path" ] || return 1
  # Read the header into a variable first rather than piping head into
  # grep: under `set -o pipefail`, `grep -q` exiting on first match can
  # close the pipe before `head` finishes writing, leaving head with
  # SIGPIPE (141) as the pipeline status — which would misclassify an
  # interactive migration as mechanical. A here-string has no upstream
  # writer to kill.
  local header
  header=$(head -5 "$path")
  grep -qE '^# INTERACTIVE:[[:space:]]*yes$' <<<"$header"
}

# Build the resume-state file the harness will load on INIT. For each
# durable session-log record, emit one TSV line:
#   RESUMED<TAB>key<TAB>outcome<TAB>proposed<TAB>user_value
# All values are escape_field-encoded.
build_resume_state_file() {
  local session_log="$1" out="$2"
  : >"$out"
  [ -z "$session_log" ] || [ ! -f "$session_log" ] || [ ! -s "$session_log" ] &&
    return 0
  # shellcheck disable=SC2016 # single-quoted awk program; $ is awk's own syntax, intentionally not shell-expanded
  local awk_script='
function js_unescape(s,   r, i, c, n) {
  r = ""
  i = 1
  while (i <= length(s)) {
    c = substr(s, i, 1)
    if (c == "\\" && i < length(s)) {
      n = substr(s, i + 1, 1)
      if      (n == "\\") { r = r "\\";    i += 2 }
      else if (n == "\"") { r = r "\"";    i += 2 }
      else if (n == "n")  { r = r "\n";    i += 2 }
      else if (n == "r")  { r = r "\r";    i += 2 }
      else if (n == "t")  { r = r "\t";    i += 2 }
      else if (n == "b")  { r = r "\b";    i += 2 }
      else if (n == "f")  { r = r "\f";    i += 2 }
      else if (n == "/")  { r = r "/";     i += 2 }
      else if (n == "u") {
        r = r sprintf("%c", strtonum("0x" substr(s, i + 2, 4)))
        i += 6
      }
      else                { r = r c;       i += 1 }
    } else { r = r c; i += 1 }
  }
  return r
}
function tsv_escape(s,   r, i, c) {
  r = ""
  for (i = 1; i <= length(s); i++) {
    c = substr(s, i, 1)
    if      (c == "\\") r = r "\\\\"
    else if (c == "\t") r = r "\\t"
    else if (c == "\n") r = r "\\n"
    else                r = r c
  }
  return r
}
function extract_field(line, key,   pat, p, val_start, depth, ch, i, esc) {
  pat = "\"" key "\":\""
  p = index(line, pat)
  if (p == 0) return ""
  val_start = p + length(pat)
  # Walk forward, honouring \" escapes inside the string value.
  esc = 0
  for (i = val_start; i <= length(line); i++) {
    ch = substr(line, i, 1)
    if (esc) { esc = 0; continue }
    if (ch == "\\") { esc = 1; continue }
    if (ch == "\"") {
      return js_unescape(substr(line, val_start, i - val_start))
    }
  }
  return ""
}
{
  # Validate schema_version (second canonical field).
  if (!match($0, /"schema_version":[ ]*[0-9]+/)) {
    print "[resume] line " NR ": missing schema_version; skipping" > "/dev/stderr"
    next
  }
  sv_str = substr($0, RSTART + length("\"schema_version\":"), RLENGTH - length("\"schema_version\":"))
  gsub(/[ ]/, "", sv_str)
  sv = sv_str + 0
  if (sv != 1) {
    print "[resume] unknown schema_version " sv " — supported: {1}." > "/dev/stderr"
    print "[resume] To discard the session and re-prompt, run:" > "/dev/stderr"
    print "[resume]   rm " ENVIRON["RESUME_SESSION_LOG"] > "/dev/stderr"
    exit 2
  }
  key      = extract_field($0, "transformation_key")
  outcome  = extract_field($0, "outcome")
  proposed = extract_field($0, "proposed_value")
  user_v   = extract_field($0, "user_value")
  if (outcome != "accepted" && outcome != "edited" && outcome != "skipped") {
    print "[resume] line " NR ": unknown outcome " outcome > "/dev/stderr"
    exit 2
  }
  printf "RESUMED\t%s\t%s\t%s\t%s\n",
    tsv_escape(key), outcome, tsv_escape(proposed), tsv_escape(user_v)
}'
  RESUME_SESSION_LOG="$session_log" awk "$awk_script" "$session_log" >"$out"
}

# Parse the extras_tsv field (US-separated key=value pairs, values
# TSV-escaped) into the named arrays.
_parse_extras_tsv() {
  local extras_tsv="$1"
  EXTRAS_KEYS=()
  EXTRAS_VALUES=()
  [ -z "$extras_tsv" ] && return 0
  local US=$'\x1F'
  local IFS_save="$IFS"
  IFS="$US"
  # shellcheck disable=SC2206
  local -a pairs=($extras_tsv)
  IFS="$IFS_save"
  local kv k v
  for kv in "${pairs[@]}"; do
    k="${kv%%=*}"
    v="${kv#*=}"
    EXTRAS_KEYS+=("$k")
    EXTRAS_VALUES+=("$(unescape_field "$v")")
  done
}

# write_session_record <key> <outcome> <proposed> <user_value> <extras_tsv>
# <session_log_path>
#   Compose the canonical JSONL record and append it atomically.
write_session_record() {
  local key="$1" outcome="$2" proposed="$3" user_value="$4"
  local extras_tsv="$5" session_log="$6"
  EXTRAS_KEYS=()
  EXTRAS_VALUES=()
  _parse_extras_tsv "$extras_tsv"
  # Defensive re-validation of extras keys.
  local i k
  for ((i = 0; i < ${#EXTRAS_KEYS[@]}; i++)); do
    k="${EXTRAS_KEYS[$i]}"
    case "$k" in
      transformation_key | schema_version | outcome | proposed_value | user_value | timestamp)
        echo "[interactive] runner rejected reserved extras key '$k'" >&2
        return 1
        ;;
    esac
    if [[ ! "$k" =~ ^[a-z][a-z0-9_]*$ ]]; then
      echo "[interactive] runner rejected invalid extras key '$k'" >&2
      return 1
    fi
  done
  local timestamp
  timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  local -a compose_args=(
    transformation_key="$key" schema_version=1 outcome="$outcome"
    proposed_value="$proposed" timestamp="$timestamp"
  )
  if [ "$outcome" = "edited" ]; then
    compose_args+=("user_value=$user_value")
  fi
  for ((i = 0; i < ${#EXTRAS_KEYS[@]}; i++)); do
    compose_args+=("${EXTRAS_KEYS[$i]}=${EXTRAS_VALUES[$i]}")
  done
  local record
  record=$(jsonl_compose_record "${compose_args[@]}") || return 1
  atomic_jsonl_append "$session_log" "$record"
}

# render_prompt: render the three-element display block + the inline
# help line (frequency-aware: full on first prompt + every prompt after
# VALIDATE_ERR, compact thereafter).
#
# Globals consumed:
#   PROMPT_INDEX, PROMPT_TOTAL_GUESS — running counters
#   LAST_PROMPT_HAD_VALIDATE_ERR    — set by handle_validate_err
#   SESSION_LOG_BANNER_EMITTED      — guard for one-time banner
#   SESSION_LOG                     — path to the JSONL session log
#   USER_OUT_FD                     — fd 1 (TTY) or fd 2 (piped) — runner picks
render_prompt() {
  local key="$1" path="$2" anchor="$3" proposed="$4"
  local predicate_value="$5" extras_tsv="$6" display_b64="$7"
  PROMPT_INDEX=$((PROMPT_INDEX + 1))
  if [ "${SESSION_LOG_BANNER_EMITTED:-0}" -eq 0 ]; then
    printf 'Session log: %s  (resume from this file by re-running /accelerator:migrate)\n\n' \
      "$SESSION_LOG" >&"$USER_OUT_FD"
    SESSION_LOG_BANNER_EMITTED=1
  fi
  {
    printf '── Transformation %d ────────────────────────\n' "$PROMPT_INDEX"
    printf 'Proposed:  %s\n' "$proposed"
    printf 'Source:    %s:%s\n' "$path" "$anchor"
    printf 'Predicate: %s\n' "$predicate_value"
    if [ -n "$display_b64" ]; then
      local decoded
      decoded=$(printf '%s' "$display_b64" | base64 -d 2>/dev/null || true)
      if [ -n "$decoded" ]; then
        printf '\n%s\n' "$decoded"
      fi
    fi
    printf '\n'
    # Inline help frequency: full on first prompt + every prompt after a
    # VALIDATE_ERR, compact otherwise. The PROMPT loop body resets the
    # validate-err flag after rendering.
    if [ "$PROMPT_INDEX" -eq 1 ] ||
      [ "${LAST_PROMPT_HAD_VALIDATE_ERR:-0}" -eq 1 ]; then
      printf '[accept | edit <new-value> | skip] > '
    else
      printf '> '
    fi
  } >&"$USER_OUT_FD"
  LAST_PROMPT_HAD_VALIDATE_ERR=0
}

# read_decision: read one line of input from the decisions file (if set)
# or /dev/tty (interactive). Sets DECIDE_OUTCOME, DECIDE_VALUE.
# Returns: 0 = decision read; 1 = read error / decisions file exhausted /
# TTY EOF; 2 = no input channel available (caller emits the structured stall).
read_decision() {
  local line
  if [ -n "${ACCELERATOR_MIGRATE_DECISIONS_FILE:-}" ]; then
    if ! IFS= read -r line <&"$DECISIONS_FD"; then
      echo "[interactive] decisions file exhausted" >&2
      return 1
    fi
    DECISIONS_LINE_NUM=$((DECISIONS_LINE_NUM + 1))
    # Normalise CRLF.
    line="${line%$'\r'}"
    # Skip empty lines.
    while [ -z "$line" ]; do
      if ! IFS= read -r line <&"$DECISIONS_FD"; then
        echo "[interactive] decisions file exhausted" >&2
        return 1
      fi
      DECISIONS_LINE_NUM=$((DECISIONS_LINE_NUM + 1))
      line="${line%$'\r'}"
    done
    echo "[decisions] consumed line $DECISIONS_LINE_NUM: ${line%% *}" >&2
  else
    if [ -t 0 ]; then
      IFS= read -r line </dev/tty || return 1
    else
      # No decisions file and stdin is not a TTY. Treat this as the no-input
      # case ONLY when the read fails AND nothing was read (EOF, no channel);
      # signal it distinctly (2) so callers emit the structured stall rather
      # than the opaque abort. A populated-but-unterminated final line (read
      # fails but `line` is non-empty) still carries a usable decision, so fall
      # through and parse it instead of discarding it.
      if ! IFS= read -r line && [ -z "$line" ]; then
        return 2
      fi
    fi
  fi
  # Parse outcome and value.
  case "$line" in
    accept | accept' '*)
      DECIDE_OUTCOME=accept
      DECIDE_VALUE=""
      ;;
    skip | skip' '*)
      DECIDE_OUTCOME=skip
      DECIDE_VALUE=""
      ;;
    edit' '*)
      DECIDE_OUTCOME=edit
      DECIDE_VALUE="${line#edit }"
      ;;
    edit)
      DECIDE_OUTCOME=edit
      DECIDE_VALUE=""
      ;;
    *)
      DECIDE_OUTCOME="$line"
      DECIDE_VALUE=""
      ;;
  esac
}

# emit_no_input_stall <id> <key>
# Structured, parseable stall printed to stderr when no decision input channel
# exists. Replaces the opaque "failed to obtain {decision,re-decision}" abort.
# Diagnostic lines carry the [$id] log prefix; the resume command lines are
# emitted flush-left so they can be copied and run verbatim. Caller still
# performs `exec 7>&-; return 1`.
emit_no_input_stall() {
  local id="$1" key="$2"
  # PROJECT_ROOT and RUNNER_SCRIPT_DIR are both hard preconditions of the
  # interactive layer (set by run-migrations.sh before sourcing); the :-
  # default on the driver path is belt-and-braces, not a real fallback.
  local state_dir="$PROJECT_ROOT/.accelerator/state"
  local decisions_path="$state_dir/migrations-${id}-decisions.txt"
  local driver="${RUNNER_SCRIPT_DIR:-.}/run-migrations.sh"
  {
    echo "[$id] MIGRATION STALLED: no decision input available"
    echo "[$id]   pending decision: $key"
    echo "[$id]   No decisions file, terminal, or piped input was available to"
    echo "[$id]   answer this prompt, so the migration cannot proceed."
    echo "[$id]"
    echo "[$id]   This migration may have already partially modified the"
    echo "[$id]   working tree. Re-running /accelerator:migrate resumes this"
    echo "[$id]   partial run when the base revision is unchanged (decided"
    echo "[$id]   transformations are replayed, not re-applied)."
    echo "[$id]"
    echo "[$id]   To resume: each run answers the current prompt only (you"
    echo "[$id]   may be stalled again for the next undecided transformation):"
    echo "[$id]     1. write the decision (accept | skip | edit <value>),"
    echo "[$id]        one per line, to: $decisions_path"
    echo "[$id]        (create this file yourself; do not overwrite existing"
    echo "[$id]        migrations-${id}-* state files)"
    echo "[$id]     2. then run (copy-pasteable):"
    echo ""
    echo "bash $driver --decisions-file $decisions_path"
    echo ""
    echo "[$id]   equivalent env-var form:"
    echo ""
    echo "ACCELERATOR_MIGRATE_DECISIONS_FILE=$decisions_path bash $driver"
  } >&2
}

# read_decision_or_stall <id> <key> <verb>
# Reads the next decision. On no-input (status 2) emits the structured stall; on
# any other failure emits the legacy "failed to obtain <verb> for <key>" abort.
# Returns read_decision's status (0 on success); the caller tears down on nonzero.
read_decision_or_stall() {
  local id="$1" key="$2" verb="$3" rc=0
  read_decision || rc=$?
  if [ "$rc" -ne 0 ]; then
    if [ "$rc" -eq 2 ]; then
      emit_no_input_stall "$id" "$key"
    else
      echo "[$id] failed to obtain $verb for $key" >&2
    fi
  fi
  return "$rc"
}

# run_interactive_migration <path> <id>
run_interactive_migration() {
  local f="$1" id="$2"
  local resume_state_path stderr_file
  # shellcheck disable=SC2154 # PROJECT_ROOT provided by the interactive-migration harness environment
  resume_state_path="$PROJECT_ROOT/.accelerator/state/migrations-${id}-resume-state.tmp"
  stderr_file="$PROJECT_ROOT/.accelerator/state/migrations-${id}-stderr.log"
  mkdir -p "$(dirname "$resume_state_path")"
  : >"$stderr_file"

  # Default session-log path (the migration may override via
  # migration_session_log_path; the actual path is announced on READY).
  local default_session_log="$PROJECT_ROOT/.accelerator/state/migrations-${id}-session.jsonl"
  build_resume_state_file "$default_session_log" "$resume_state_path" || {
    echo "[$id] failed to build resume state" >&2
    return 1
  }

  # Open decisions-file fd if scripted (test-only). bash 3.2 has no
  # `{var}<` allocator, so use a literal fd number (9).
  local decisions_fd_open=0
  DECISIONS_LINE_NUM=0
  if [ -n "${ACCELERATOR_MIGRATE_DECISIONS_FILE:-}" ]; then
    exec 9<"$ACCELERATOR_MIGRATE_DECISIONS_FILE"
    DECISIONS_FD=9
    decisions_fd_open=1
  fi

  # User-facing output fd: stdout if TTY, stderr otherwise.
  if [ -t 1 ]; then
    USER_OUT_FD=1
  else
    USER_OUT_FD=2
  fi
  PROMPT_INDEX=0
  SESSION_LOG_BANNER_EMITTED=0
  LAST_PROMPT_HAD_VALIDATE_ERR=0

  # Set up the per-side protocol log if requested by the test harness.
  # The migration's side log is announced via env var so the migration's
  # interactive-harness.sh picks it up on source.
  local runner_log_path="${MIGRATION_PROTOCOL_LOG_RUNNER:-}"
  if [ -n "$runner_log_path" ]; then
    export INTERACTIVE_PROTOCOL_SIDE_LOG="$runner_log_path"
  fi

  # Bidirectional pipes via two named FIFOs (bash 3.2 compatible — no
  # coproc). One FIFO carries runner→migration frames, the other
  # carries migration→runner frames. The migration sees them as stdin
  # and stdout respectively. FIFO paths are deterministic under
  # .accelerator/state/ so an orphaned crash doesn't leak into /tmp.
  local fifo_runner_to_mig="$PROJECT_ROOT/.accelerator/state/migrations-${id}-r2m.fifo"
  local fifo_mig_to_runner="$PROJECT_ROOT/.accelerator/state/migrations-${id}-m2r.fifo"
  rm -f "$fifo_runner_to_mig" "$fifo_mig_to_runner"
  mkfifo "$fifo_runner_to_mig" "$fifo_mig_to_runner"

  # Open the runner→migration FIFO read-write (fd 7) so our write
  # doesn't block waiting for the migration to open the read side and
  # vice versa. fd 7 also serves as the channel we close to deliver
  # EOF to the migration's stdin.
  exec 7<>"$fifo_runner_to_mig"

  # Fork the migration: its stdin reads from r2m, its stdout writes
  # to m2r. Stderr captured to a per-migration file.
  PROJECT_ROOT="$PROJECT_ROOT" CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    ACCELERATOR_MIGRATION_MODE=1 MIGRATION_ID="$id" \
    MIGRATION_PROTOCOL_LOG_MIGRATION="${MIGRATION_PROTOCOL_LOG_MIGRATION:-}" \
    bash "$f" <"$fifo_runner_to_mig" >"$fifo_mig_to_runner" 2>"$stderr_file" &
  local pid=$!

  # Open the read end of m2r AFTER forking. Read-only so reads EOF
  # cleanly when the migration's stdout closes (i.e. when it exits).
  # The open blocks until the migration opens its stdout-write side.
  exec 8<"$fifo_mig_to_runner"
  local mig_in=7 mig_out=8

  # Send INIT.
  printf 'INIT\t%s\t%s\n' \
    "$(escape_field "$resume_state_path")" \
    "$(escape_field "${ACCELERATOR_MIGRATE_DECISIONS_FILE:-}")" >&"$mig_in"
  if [ -n "$runner_log_path" ]; then
    printf 'INIT\t%s\t%s\n' \
      "$(escape_field "$resume_state_path")" \
      "$(escape_field "${ACCELERATOR_MIGRATE_DECISIONS_FILE:-}")" \
      >>"$runner_log_path"
  fi

  SESSION_LOG="$default_session_log"
  local saw_done=0 saw_no_op_pending=0 saw_ready=0
  local frame

  while IFS= read -r -u "$mig_out" frame; do
    # Soft-defer sentinel — preserved from the mechanical contract.
    if [ "$frame" = "MIGRATION_RESULT: no_op_pending" ]; then
      if [ "$saw_ready" -eq 1 ]; then
        echo "[$id] protocol error: MIGRATION_RESULT: no_op_pending after READY" >&2
        return 1
      fi
      saw_no_op_pending=1
      exec 7>&-
      while IFS= read -r -u "$mig_out" _drain; do :; done
      break
    fi

    if [ -n "$runner_log_path" ]; then
      printf '%s\n' "$frame" >>"$runner_log_path"
    fi

    # Parse out type + fields. read_frame helper from interactive-protocol.sh
    # works on a real fd; here the frame is already in a variable.
    local type rest
    type="${frame%%$'\t'*}"
    if [[ "$frame" == *$'\t'* ]]; then
      rest="${frame#*$'\t'}"
    else
      rest=""
    fi
    local IFS_save="$IFS"
    IFS=$'\t'
    # shellcheck disable=SC2206
    local -a raw_fields=($rest)
    IFS="$IFS_save"
    local -a fields=()
    local i
    for ((i = 0; i < ${#raw_fields[@]}; i++)); do
      fields+=("$(unescape_field "${raw_fields[$i]}")")
    done

    case "$type" in
      READY)
        saw_ready=1
        SESSION_LOG="${fields[0]:-$default_session_log}"
        # Resolve relative session-log paths against PROJECT_ROOT — the
        # migration runs with PROJECT_ROOT exported but its cwd may be
        # the original shell's cwd. Humans naturally write relative
        # paths in migration_session_log_path.
        case "$SESSION_LOG" in
          /*) ;;
          *) SESSION_LOG="$PROJECT_ROOT/$SESSION_LOG" ;;
        esac
        # The session log must conform to the canonical shape
        # .accelerator/state/migrations-<id>-session.jsonl so the runner's
        # pre-flight owned-check predicates (is_session_artifact / is_session_log)
        # are total over real session logs. A non-canonical declared path would
        # escape both and fall to the generic FORCE-only refusal — neither resume
        # nor steer. (The default path's resume state was already built above.)
        if [ "$SESSION_LOG" != "$default_session_log" ]; then
          echo "[$id] migration declared a non-canonical session-log path:" >&2
          echo "[$id]   $SESSION_LOG" >&2
          echo "[$id]   expected: $default_session_log" >&2
          return 1
        fi
        ;;
      MECHANICAL_APPLIED | RESUMED_APPLIED | RESUMED_SKIPPED | APPLIED_CONFIRM)
        :
        ;;
      PROMPT)
        local p_key="${fields[0]:-}" p_path="${fields[1]:-}"
        local p_anchor="${fields[2]:-}" p_proposed="${fields[3]:-}"
        local p_predicate="${fields[4]:-}" p_extras="${fields[5]:-}"
        local p_display="${fields[6]:-}"
        render_prompt "$p_key" "$p_path" "$p_anchor" "$p_proposed" \
          "$p_predicate" "$p_extras" "$p_display"
        if ! read_decision_or_stall "$id" "$p_key" decision; then
          exec 7>&-
          return 1
        fi
        # Cache for VALIDATE_ERR re-prompt.
        LAST_PROMPT_KEY="$p_key"
        LAST_PROMPT_PATH="$p_path"
        LAST_PROMPT_ANCHOR="$p_anchor"
        LAST_PROMPT_PROPOSED="$p_proposed"
        LAST_PROMPT_PREDICATE="$p_predicate"
        LAST_PROMPT_EXTRAS="$p_extras"
        LAST_PROMPT_DISPLAY="$p_display"
        printf 'DECIDE\t%s\t%s\n' \
          "$(escape_field "$DECIDE_OUTCOME")" \
          "$(escape_field "$DECIDE_VALUE")" >&"$mig_in"
        if [ -n "$runner_log_path" ]; then
          printf 'DECIDE\t%s\t%s\n' \
            "$(escape_field "$DECIDE_OUTCOME")" \
            "$(escape_field "$DECIDE_VALUE")" >>"$runner_log_path"
        fi
        ;;
      VALIDATE_ERR)
        # The validator's message already carries a `[interactive] `
        # prefix if it used harness_reject; do not double-prefix.
        printf '%s\n' "${fields[0]:-}" >&"$USER_OUT_FD"
        LAST_PROMPT_HAD_VALIDATE_ERR=1
        # Re-render the same transformation — do NOT increment
        # PROMPT_INDEX, otherwise the "Transformation N" counter
        # double-counts validation re-prompts.
        PROMPT_INDEX=$((PROMPT_INDEX - 1))
        render_prompt "$LAST_PROMPT_KEY" "$LAST_PROMPT_PATH" \
          "$LAST_PROMPT_ANCHOR" "$LAST_PROMPT_PROPOSED" \
          "$LAST_PROMPT_PREDICATE" "$LAST_PROMPT_EXTRAS" \
          "$LAST_PROMPT_DISPLAY"
        if ! read_decision_or_stall "$id" "$LAST_PROMPT_KEY" re-decision; then
          exec 7>&-
          return 1
        fi
        printf 'DECIDE\t%s\t%s\n' \
          "$(escape_field "$DECIDE_OUTCOME")" \
          "$(escape_field "$DECIDE_VALUE")" >&"$mig_in"
        if [ -n "$runner_log_path" ]; then
          printf 'DECIDE\t%s\t%s\n' \
            "$(escape_field "$DECIDE_OUTCOME")" \
            "$(escape_field "$DECIDE_VALUE")" >>"$runner_log_path"
        fi
        ;;
      RECORDED)
        local r_key="${fields[0]:-}" r_outcome="${fields[1]:-}"
        local r_proposed="${fields[2]:-}" r_user="${fields[3]:-}"
        local r_extras="${fields[4]:-}"
        if ! write_session_record "$r_key" "$r_outcome" "$r_proposed" \
          "$r_user" "$r_extras" "$SESSION_LOG"; then
          echo "[$id] failed to persist record for $r_key" >&2
          exec 7>&-
          return 1
        fi
        printf 'APPLY\t%s\n' "$(escape_field "$r_key")" >&"$mig_in"
        if [ -n "$runner_log_path" ]; then
          printf 'APPLY\t%s\n' "$(escape_field "$r_key")" >>"$runner_log_path"
        fi
        ;;
      DRIFT)
        local d_key="${fields[0]:-}"
        atomic_jsonl_remove_by_key "$SESSION_LOG" "$d_key" || {
          echo "[$id] failed to remove stale record for $d_key" >&2
          exec 7>&-
          return 1
        }
        printf 'DRIFT_CLEARED\t%s\n' "$(escape_field "$d_key")" >&"$mig_in"
        if [ -n "$runner_log_path" ]; then
          printf 'DRIFT_CLEARED\t%s\n' "$(escape_field "$d_key")" >>"$runner_log_path"
        fi
        ;;
      DONE)
        saw_done=1
        break
        ;;
      FAIL)
        echo "[$id] ${fields[0]:-}" >&2
        wait "$pid" 2>/dev/null || true
        rm -f "$resume_state_path"
        return 1
        ;;
    esac
  done

  # Close our FIFO endpoints and remove the FIFOs from disk.
  exec 7>&- 8<&-
  rm -f "$fifo_runner_to_mig" "$fifo_mig_to_runner"
  if [ "$decisions_fd_open" -eq 1 ]; then
    exec 9<&-
  fi

  # Watchdog: 30s after sending the last frame, escalate.
  (
    sleep 30
    if kill -0 "$pid" 2>/dev/null; then
      echo "[$id] migration did not exit within 30s; sending SIGTERM" >&2
      kill -TERM "$pid" 2>/dev/null || true
      sleep 1
      if kill -0 "$pid" 2>/dev/null; then
        echo "[$id] migration unresponsive to SIGTERM; escalating to SIGKILL" >&2
        kill -KILL "$pid" 2>/dev/null || true
      fi
    fi
  ) &
  local watchdog_pid=$!
  local wait_status=0
  wait "$pid" 2>/dev/null || wait_status=$?
  kill "$watchdog_pid" 2>/dev/null || true
  wait "$watchdog_pid" 2>/dev/null || true

  if [ "$wait_status" -ne 0 ] ||
    { [ "$saw_done" -ne 1 ] && [ "$saw_no_op_pending" -ne 1 ]; }; then
    if [ -s "$stderr_file" ]; then
      echo "[$id] migration exited unexpectedly. Last stderr lines:" >&2
      tail -n 20 "$stderr_file" | sed "s/^/[$id]   /" >&2
      echo "[$id] full stderr preserved at: $stderr_file" >&2
    else
      echo "[$id] migration exited without DONE and produced no stderr." >&2
    fi
    rm -f "$resume_state_path"
    return 1
  fi

  if [ "$saw_no_op_pending" -eq 1 ]; then
    rm -f "$resume_state_path" "$stderr_file"
    echo "[${id}] no-op (stays pending)" >&2
    return 0
  fi

  # shellcheck disable=SC2154 # STATE_FILE provided by the interactive-migration harness environment
  mkdir -p "$(dirname "$STATE_FILE")"
  atomic_append_unique "$STATE_FILE" "$id"
  rm -f "$resume_state_path" "$stderr_file"
  echo "[${id}] applied" >&2
  # Signal to caller that this migration was fully applied (not soft-deferred)
  # so the apply counter can be bumped.
  # shellcheck disable=SC2034 # read by the sourcing orchestrator after this returns
  INTERACTIVE_APPLIED=1
}
