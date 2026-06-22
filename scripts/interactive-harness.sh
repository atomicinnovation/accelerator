#!/usr/bin/env bash
# Migration-side interactive harness. Source from a hook-declaring
# migration after sourcing atomic-common.sh:
#
#   source "$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh"
#   migration_emit_transformations() { ... }
#   migration_evaluate_predicate()    { ... }
#   migration_validate_edit()         { ... }
#   migration_apply_decision()        { ... }
#   # Optional: migration_session_log_path, migration_verify_applied
#   harness_run
#
# Author-facing helpers (defined below):
#   harness_emit_transformation key= path= anchor= proposed= predicate_value= display=
#   harness_extras_set <key> <value>
#   harness_extras_clear
#   harness_field <field_name>
#   harness_reject <message>

# bash 3.2 compatible: no coproc, no associative arrays, no mapfile.
# Concurrency uses two named FIFOs (one per direction) wired up by the
# runner; "associative" state is stored as two parallel indexed arrays
# searched linearly. The interactive corpus is bounded (the projected
# first consumer is ~140 transformations), so O(N) lookup is fine.

# Source shared protocol helpers. The migration is forked by the runner
# with CLAUDE_PLUGIN_ROOT exported, so the absolute path resolves there.
# shellcheck source=./interactive-protocol.sh
source "${CLAUDE_PLUGIN_ROOT:?CLAUDE_PLUGIN_ROOT not set in migration env}/scripts/interactive-protocol.sh"

# ── State carried across helper calls ────────────────────────────────────
# Accumulated extras key/value pairs for the next harness_emit_transformation
# call. Auto-cleared after each emission (per the spec — set-inside-loop is
# the intended pattern). Two parallel indexed arrays (bash 3.2 compatible).
_HARNESS_EXTRAS_KEYS=()
_HARNESS_EXTRAS_VALS=()

# Current TSV transformation line being evaluated by predicate / validator
# (used by harness_field for stdin-style extraction).
_HARNESS_CURRENT_TSV=""

# Resume state, populated by handshake. Indexed by parallel arrays keyed
# on transformation_key — linear search via _harness_resume_lookup.
RESUME_KEYS=()
RESUME_OUTCOMES=()
RESUME_PROPOSEDS=()
RESUME_USERS=()

# _harness_resume_lookup <key>
#   Sets RESUME_FOUND=1/0; if 1, also sets RESUME_LOOKUP_OUTCOME,
#   RESUME_LOOKUP_PROPOSED, RESUME_LOOKUP_USER. O(N) over the resume set.
_harness_resume_lookup() {
  local needle="$1" i
  RESUME_FOUND=0
  RESUME_LOOKUP_OUTCOME=""
  RESUME_LOOKUP_PROPOSED=""
  RESUME_LOOKUP_USER=""
  for ((i = 0; i < ${#RESUME_KEYS[@]}; i++)); do
    if [ "${RESUME_KEYS[$i]}" = "$needle" ]; then
      RESUME_FOUND=1
      RESUME_LOOKUP_OUTCOME="${RESUME_OUTCOMES[$i]}"
      RESUME_LOOKUP_PROPOSED="${RESUME_PROPOSEDS[$i]}"
      RESUME_LOOKUP_USER="${RESUME_USERS[$i]}"
      return 0
    fi
  done
}

# Test-only side log: the migration's protocol log destination is
# announced by the runner via env var.
if [ -n "${MIGRATION_PROTOCOL_LOG_MIGRATION:-}" ]; then
  export INTERACTIVE_PROTOCOL_SIDE_LOG="$MIGRATION_PROTOCOL_LOG_MIGRATION"
fi

# ── Author-facing helpers ────────────────────────────────────────────────

# harness_extras_set <key> <value>
#   Accumulate one extras pair for the next harness_emit_transformation
#   call. Keys must match ^[a-z][a-z0-9_]*$ and must not collide with
#   framework-mandatory names.
harness_extras_set() {
  local k="$1" v="$2" i
  if [[ ! "$k" =~ ^[a-z][a-z0-9_]*$ ]]; then
    echo "harness_extras_set: invalid extras key '$k'" >&2
    return 1
  fi
  case "$k" in
    transformation_key | schema_version | outcome | proposed_value | user_value | timestamp)
      echo "harness_extras_set: reserved key '$k'" >&2
      return 1
      ;;
  esac
  # Repeated set on the same key overwrites the value in place.
  for ((i = 0; i < ${#_HARNESS_EXTRAS_KEYS[@]}; i++)); do
    if [ "${_HARNESS_EXTRAS_KEYS[$i]}" = "$k" ]; then
      _HARNESS_EXTRAS_VALS[$i]="$v"
      return 0
    fi
  done
  _HARNESS_EXTRAS_KEYS+=("$k")
  _HARNESS_EXTRAS_VALS+=("$v")
}

harness_extras_clear() {
  _HARNESS_EXTRAS_KEYS=()
  _HARNESS_EXTRAS_VALS=()
}

# harness_emit_transformation key=K path=P anchor=A proposed=V \
#                             predicate_value=PV display=$'multi\nline'
#   Emit one TSV transformation record to stdout. The wire format is:
#     TX<TAB>key<TAB>path<TAB>anchor<TAB>proposed<TAB>predicate_value<TAB>extras_tsv<TAB>display_b64
#   Extras come from the harness_extras_set accumulator (then cleared).
harness_emit_transformation() {
  local key="" path="" anchor="" proposed="" predicate_value="" display=""
  local pair k v
  for pair in "$@"; do
    case "$pair" in
      *=*)
        k="${pair%%=*}"
        v="${pair#*=}"
        ;;
      *)
        echo "harness_emit_transformation: malformed pair '$pair'" >&2
        return 1
        ;;
    esac
    case "$k" in
      key) key="$v" ;;
      path) path="$v" ;;
      anchor) anchor="$v" ;;
      proposed) proposed="$v" ;;
      predicate_value) predicate_value="$v" ;;
      display) display="$v" ;;
      *)
        echo "harness_emit_transformation: unknown arg '$k'" >&2
        return 1
        ;;
    esac
  done
  if [ -z "$key" ] || [ -z "$path" ]; then
    echo "harness_emit_transformation: key and path are required" >&2
    return 1
  fi

  # Build extras_tsv as key=value pairs separated by ASCII Unit Separator
  # (0x1F). Values are TSV-field-escaped exactly like every other field
  # later in the pipeline (the runner applies its own escaping when
  # composing the session-log JSON record).
  local extras_tsv="" i ek ev US=$'\x1F'
  for ((i = 0; i < ${#_HARNESS_EXTRAS_KEYS[@]}; i++)); do
    ek="${_HARNESS_EXTRAS_KEYS[$i]}"
    ev="${_HARNESS_EXTRAS_VALS[$i]}"
    if [ -n "$extras_tsv" ]; then extras_tsv+="$US"; fi
    extras_tsv+="$ek=$(escape_field "$ev")"
  done
  harness_extras_clear

  # Display is base64-encoded so multi-line / arbitrary-byte content
  # crosses the wire without TSV escape budget.
  local display_b64
  display_b64=$(printf '%s' "$display" | base64 | tr -d '\n')

  # The "TX" record sentinel is internal to the harness (predicate
  # receives a TSV line of this shape on stdin). The runner never sees
  # this record type — the harness translates each TX into MECHANICAL_APPLIED
  # or PROMPT as appropriate.
  printf 'TX\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$(escape_field "$key")" \
    "$(escape_field "$path")" \
    "$(escape_field "$anchor")" \
    "$(escape_field "$proposed")" \
    "$(escape_field "$predicate_value")" \
    "$extras_tsv" \
    "$display_b64"
}

# harness_field <field_name>
#   Extract a named field from the current TSV transformation line
#   (set by the framework before each call to migration_evaluate_predicate
#   or migration_validate_edit). Returns the unescaped value on stdout.
harness_field() {
  local fname="$1"
  local IFS_save="$IFS"
  IFS=$'\t'
  # shellcheck disable=SC2206
  local -a parts=($_HARNESS_CURRENT_TSV)
  IFS="$IFS_save"
  case "$fname" in
    key) unescape_field "${parts[1]:-}" ;;
    path) unescape_field "${parts[2]:-}" ;;
    anchor) unescape_field "${parts[3]:-}" ;;
    proposed) unescape_field "${parts[4]:-}" ;;
    predicate_value) unescape_field "${parts[5]:-}" ;;
    *)
      # Extract from the extras_tsv field by key.
      local extras_tsv="${parts[6]:-}"
      local US=$'\x1F'
      local IFS_save2="$IFS"
      IFS="$US"
      # shellcheck disable=SC2206
      local -a extras=($extras_tsv)
      IFS="$IFS_save2"
      local kv k v
      for kv in "${extras[@]}"; do
        k="${kv%%=*}"
        v="${kv#*=}"
        if [ "$k" = "$fname" ]; then
          unescape_field "$v"
          return 0
        fi
      done
      ;;
  esac
}

# harness_reject <message>
#   Used by migration_validate_edit to reject a user's edit. Prints to
#   stderr in a uniform format and returns non-zero.
harness_reject() {
  echo "[interactive] $1" >&2
  return 1
}

# ── Internal: load resume state ──────────────────────────────────────────
_harness_load_resume_state() {
  local path="$1"
  [ -z "$path" ] || [ ! -f "$path" ] || [ ! -s "$path" ] && return 0
  local tag key outcome proposed user_value
  while IFS=$'\t' read -r tag key outcome proposed user_value; do
    [ "$tag" != "RESUMED" ] && continue
    RESUME_KEYS+=("$(unescape_field "$key")")
    RESUME_OUTCOMES+=("$outcome")
    RESUME_PROPOSEDS+=("$(unescape_field "$proposed")")
    if [ -n "$user_value" ]; then
      RESUME_USERS+=("$(unescape_field "$user_value")")
    else
      RESUME_USERS+=("")
    fi
  done <"$path"
}

# ── harness_run: drives the per-transformation loop ──────────────────────
harness_run() {
  # Handshake. Expect INIT first.
  read_frame || {
    echo "interactive-harness: failed to read INIT" >&2
    return 1
  }
  if [ "$FRAME_TYPE" != "INIT" ]; then
    emit_frame FAIL "expected INIT, got $FRAME_TYPE"
    return 1
  fi
  local resume_state_path="${FRAME_FIELDS[0]:-}"
  # FRAME_FIELDS[1] = decisions_path (runner-side concern, unused here).
  # The runner selects a dry mode via the MIGRATION_HARNESS_MODE env var on the
  # forked child ("1" = list enumeration, "2" = dry-apply validation;
  # absent/empty = normal run). It rides on the environment rather than a third
  # INIT field because an empty decisions_path would collapse under IFS-tab
  # word-splitting and shift a trailing positional field out of place.
  local mode="${MIGRATION_HARNESS_MODE:-}"
  # Decisions path is not used by the harness — it's a runner-side
  # concern. The harness pretends the input is interactive; the runner
  # multiplexes either user input or the decisions file.
  _harness_load_resume_state "$resume_state_path"

  local session_log_path
  if declare -F migration_session_log_path >/dev/null; then
    session_log_path="$(migration_session_log_path)"
  else
    session_log_path=".accelerator/state/migrations-${MIGRATION_ID:-unknown}-session.jsonl"
  fi
  emit_frame READY "$session_log_path"

  # Buffer all transformations from migration_emit_transformations into
  # an array so we can iterate without re-running the emitter. The
  # transformation emission order is the canonical iteration order
  # (story-promoted runner-level decision).
  local -a TX_LINES=()
  local emit_output
  emit_output=$(
    migration_emit_transformations
    printf X
  )
  emit_output=${emit_output%X}
  if [ -n "$emit_output" ]; then
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      TX_LINES+=("$line")
    done <<<"$emit_output"
  fi

  # Dry modes branch BEFORE the decide handshake: list mode enumerates the
  # decision-requiring transformations and exits; dry-apply runs the real
  # decide/validate loop with every side effect suppressed. Both pass the
  # buffered lines as args (TX_LINES is local to harness_run) and route each
  # TX via the shared _harness_classify_tx, so all three modes share one
  # parse/resume/predicate definition.
  if [ "$mode" = "1" ]; then
    _harness_emit_list "${TX_LINES[@]+"${TX_LINES[@]}"}"
    return $?
  elif [ "$mode" = "2" ]; then
    _harness_dry_apply "${TX_LINES[@]+"${TX_LINES[@]}"}"
    return $?
  fi

  local tx i key path anchor proposed predicate_value extras_tsv display_b64
  for tx in "${TX_LINES[@]+"${TX_LINES[@]}"}"; do
    _harness_classify_tx "$tx"
    case "$ROUTE" in
      malformed)
        emit_frame FAIL "harness_emit_transformation: malformed record (got '${_CLASSIFY_RAW}')"
        return 1
        ;;
      resumed)
        _harness_handle_resume "$key" "$path" "$anchor" "$proposed" \
          "$predicate_value" "$extras_tsv" "$display_b64" || return 1
        ;;
      prompt)
        _harness_run_prompt "$key" "$path" "$anchor" "$proposed" \
          "$predicate_value" "$extras_tsv" "$display_b64" || return 1
        ;;
      mechanical)
        # Mechanical route: apply without prompting; emit MECHANICAL_APPLIED.
        if ! migration_apply_decision "$key" "$path" "$anchor" accept "$proposed"; then
          emit_frame FAIL "migration_apply_decision failed for key $key (mechanical route)"
          return 1
        fi
        emit_frame MECHANICAL_APPLIED "$key"
        ;;
      fail)
        emit_frame FAIL "migration_evaluate_predicate returned $PREDICATE_RC for key $key (contract: 0 prompt, 1 mechanical)"
        return 1
        ;;
    esac
  done

  emit_frame DONE
  return 0
}

# _harness_classify_tx <tx>: parse one buffered TX line and decide its route.
# Sets the FULL set of per-TX globals (UNESCAPED, exactly as the main loop's
# unescape_field extraction): key/path/anchor/proposed AND predicate_value/
# extras_tsv/display_b64 (the live PROMPT build needs all seven), plus ROUTE in
# {malformed, resumed, mechanical, prompt, fail}. On the 'fail' route it also
# sets PREDICATE_RC; on 'malformed' it sets _CLASSIFY_RAW. The ONLY place TX
# parsing / resume / predicate routing lives — the main loop, list mode, and
# dry-apply all act on ROUTE and read these globals rather than re-splitting the
# TX. Callers that declare these names local (e.g. harness_run) receive the
# values via dynamic scope; otherwise they land as globals.
_harness_classify_tx() {
  local tx="$1"
  local IFS_save="$IFS"
  IFS=$'\t'
  # shellcheck disable=SC2206
  local -a fields=($tx)
  IFS="$IFS_save"
  if [ "${fields[0]:-}" != "TX" ]; then
    ROUTE=malformed
    _CLASSIFY_RAW="${fields[0]:-}"
    return 0
  fi
  key=$(unescape_field "${fields[1]:-}")
  path=$(unescape_field "${fields[2]:-}")
  anchor=$(unescape_field "${fields[3]:-}")
  proposed=$(unescape_field "${fields[4]:-}")
  predicate_value=$(unescape_field "${fields[5]:-}")
  extras_tsv="${fields[6]:-}"
  display_b64="${fields[7]:-}"
  _HARNESS_CURRENT_TSV="$tx"
  _harness_resume_lookup "$key"
  if [ "$RESUME_FOUND" -eq 1 ]; then
    ROUTE=resumed
    return 0
  fi
  # Evaluate predicate; if migration_evaluate_predicate is not declared,
  # default to "always prompt" (predicate=true).
  local predicate_rc=0
  if declare -F migration_evaluate_predicate >/dev/null; then
    # Feed the transformation via a here-string, NOT a pipe. A predicate that
    # returns without draining stdin (e.g. one that reads fields via
    # harness_field rather than stdin) would close the read end of a pipe
    # before `printf` finishes writing; under `set -o pipefail` the resulting
    # SIGPIPE (141) on printf becomes the pipeline's exit status and gets
    # misread as a contract violation. A here-string has no upstream writer to
    # receive SIGPIPE, so the predicate's own return code is preserved.
    migration_evaluate_predicate <<<"$tx" >/dev/null 2>&1 || predicate_rc=$?
  fi
  case "$predicate_rc" in
    0) ROUTE=prompt ;;
    1) ROUTE=mechanical ;;
    *)
      ROUTE=fail
      PREDICATE_RC="$predicate_rc"
      ;;
  esac
}

# _harness_emit_list <tx...>: dry enumeration of decision-requiring
# transformations. Takes the buffered lines as ARGS (TX_LINES is local to
# harness_run), routes each via the shared _harness_classify_tx, and emits
# LIST_ENTRY only for the 'prompt' route. Mutates nothing — resumed keys are
# already decided (no line consumed) and mechanical rows would mutate, so both
# are excluded. emit_frame re-escapes on the wire and the runner unescapes once
# on receipt, so the round-trip is byte-identical to the PROMPT path.
_harness_emit_list() {
  local tx
  for tx in "$@"; do
    _harness_classify_tx "$tx"
    case "$ROUTE" in
      malformed)
        emit_frame FAIL "harness_emit_transformation: malformed record (got '${_CLASSIFY_RAW}')"
        return 1
        ;;
      resumed) : ;;    # already decided -> no line consumed
      mechanical) : ;; # mutates + consumes no line, excluded
      prompt) emit_frame LIST_ENTRY "$key" "$path" "$anchor" "$proposed" ;;
      fail)
        emit_frame FAIL "predicate returned $PREDICATE_RC for key $key"
        return 1
        ;;
    esac
  done
  emit_frame LIST_DONE
}

# _harness_dry_apply <tx...>: dry-apply validation pass. Routes each buffered TX
# via the shared _harness_classify_tx and, for any transformation the live run
# would PROMPT — a fresh prompt route, OR a resumed key that has drifted / fails
# verification — runs the SAME decide handshake the live loop uses (emit PROMPT,
# receive DECIDE, run migration_validate_edit on an edit) but suppresses every
# side effect: no migration_apply_decision, no session record, no
# RECORDED/APPLY/APPLIED_CONFIRM, and no DRIFT round-trip (which would mutate the
# session log). A cleanly-resumed key and a mechanical row consume NO decision,
# exactly as the live run, so consumption matches by construction. On a
# validation failure it emits DRY_REJECT (a hard stop — not VALIDATE_ERR, so
# dry-apply fails fast rather than re-prompting). Emits DRY_DONE at the end.
_harness_dry_apply() {
  local tx
  for tx in "$@"; do
    _harness_classify_tx "$tx"
    case "$ROUTE" in
      malformed)
        emit_frame FAIL "harness_emit_transformation: malformed record (got '${_CLASSIFY_RAW}')"
        return 1
        ;;
      resumed)
        # Replicate the live resume drift check WITHOUT mutation: a recorded key
        # whose proposed_value still matches (and whose mutation verifies, if a
        # verifier is declared) consumes NO decision; otherwise the live run
        # re-prompts it, so dry-apply must too — to consume the same decision.
        if _harness_dry_resume_clean "$key" "$path" "$anchor" "$proposed"; then
          : # cleanly resumed -> no decision consumed (same as live)
        else
          _harness_dry_prompt "$key" "$path" "$anchor" "$proposed" \
            "$predicate_value" "$extras_tsv" "$display_b64" || return 1
        fi
        ;;
      mechanical) : ;; # mutates + consumes no decision; suppressed in dry mode
      prompt)
        _harness_dry_prompt "$key" "$path" "$anchor" "$proposed" \
          "$predicate_value" "$extras_tsv" "$display_b64" || return 1
        ;;
      fail)
        emit_frame FAIL "migration_evaluate_predicate returned $PREDICATE_RC for key $key (contract: 0 prompt, 1 mechanical)"
        return 1
        ;;
    esac
  done
  emit_frame DRY_DONE
}

# _harness_dry_resume_clean <key> <path> <anchor> <proposed>
#   Return 0 iff the recorded resume decision is still valid (no drift, and the
#   mutation verifies when a verifier is declared) — i.e. the live run would
#   replay it WITHOUT consuming a decision. Returns non-zero on drift or
#   verify-fail, the cases where the live run re-prompts. RESUME_LOOKUP_* were
#   set by _harness_classify_tx -> _harness_resume_lookup. Mutates nothing.
_harness_dry_resume_clean() {
  local key="$1" path="$2" anchor="$3" proposed="$4"
  [ "$RESUME_LOOKUP_PROPOSED" = "$proposed" ] || return 1 # source drift
  if [ "$RESUME_LOOKUP_OUTCOME" = "accepted" ] ||
    [ "$RESUME_LOOKUP_OUTCOME" = "edited" ]; then
    if declare -F migration_verify_applied >/dev/null; then
      migration_verify_applied "$key" "$path" "$anchor" \
        "$RESUME_LOOKUP_OUTCOME" "$RESUME_LOOKUP_PROPOSED" "$RESUME_LOOKUP_USER" ||
        return 1
    fi
  fi
  return 0
}

# _harness_dry_prompt: the dry-mode counterpart of _harness_run_prompt. Emits
# PROMPT, reads DECIDE, validates an edit via migration_validate_edit (pure, no
# mutation), and emits DRY_OK — but never RECORDED/APPLY, so no artefact or
# session-log mutation occurs. On a rejected edit it mirrors the live run: emit
# VALIDATE_ERR and loop, so a decisions file that recovers via a subsequent line
# validates exactly as it applies (consuming the same lines). A terminal bad
# edit (no recovery line) surfaces on the runner side as the decisions file
# exhausting during re-prompt. _HARNESS_CURRENT_TSV was set to the raw TX line
# by _harness_classify_tx, so harness_field works inside the validator without a
# rebuild.
_harness_dry_prompt() {
  local key="$1" path="$2" anchor="$3" proposed="$4"
  local predicate_value="$5" extras_tsv="$6" display_b64="$7"
  emit_frame PROMPT "$key" "$path" "$anchor" "$proposed" \
    "$predicate_value" "$extras_tsv" "$display_b64"
  while true; do
    read_frame || {
      emit_frame FAIL "EOF awaiting DECIDE for $key"
      return 1
    }
    if [ "$FRAME_TYPE" != "DECIDE" ]; then
      emit_frame FAIL "expected DECIDE for $key, got $FRAME_TYPE"
      return 1
    fi
    local outcome="${FRAME_FIELDS[0]:-}"
    local value="${FRAME_FIELDS[1]:-}"
    case "$outcome" in
      accept | skip)
        emit_frame DRY_OK "$key"
        return 0
        ;;
      edit)
        if declare -F migration_validate_edit >/dev/null; then
          local err
          err=$(migration_validate_edit "$key" "$path" "$anchor" "$proposed" "$value" 2>&1) || {
            # Mirror the live re-prompt: surface the rejection and await another
            # DECIDE (the runner reads the next decision line, or fails closed if
            # the file is exhausted).
            emit_frame VALIDATE_ERR "$err"
            continue
          }
        fi
        emit_frame DRY_OK "$key"
        return 0
        ;;
      *)
        emit_frame DRY_REJECT "$key" "unknown decision outcome '$outcome'"
        return 1
        ;;
    esac
  done
}

# _harness_run_prompt: emit PROMPT, read DECIDE, write-ahead-log loop.
# Phase 3 stub: defer the full implementation to Phase 4/5; for now this
# is enough for handshake / FAIL paths.
_harness_run_prompt() {
  local key="$1" path="$2" anchor="$3" proposed="$4"
  local predicate_value="$5" extras_tsv="$6" display_b64="$7"
  emit_frame PROMPT "$key" "$path" "$anchor" "$proposed" \
    "$predicate_value" "$extras_tsv" "$display_b64"
  while true; do
    read_frame || {
      emit_frame FAIL "EOF awaiting DECIDE for $key"
      return 1
    }
    if [ "$FRAME_TYPE" != "DECIDE" ]; then
      emit_frame FAIL "expected DECIDE for $key, got $FRAME_TYPE"
      return 1
    fi
    local outcome="${FRAME_FIELDS[0]:-}"
    local value="${FRAME_FIELDS[1]:-}"
    case "$outcome" in
      accept)
        emit_frame RECORDED "$key" accepted "$proposed" "" "$extras_tsv"
        read_frame || {
          emit_frame FAIL "EOF awaiting APPLY for $key"
          return 1
        }
        if [ "$FRAME_TYPE" != "APPLY" ]; then
          emit_frame FAIL "expected APPLY for $key, got $FRAME_TYPE"
          return 1
        fi
        if ! migration_apply_decision "$key" "$path" "$anchor" accept "$proposed"; then
          emit_frame FAIL "migration_apply_decision failed for $key (accept)"
          return 1
        fi
        emit_frame APPLIED_CONFIRM "$key"
        return 0
        ;;
      skip)
        emit_frame RECORDED "$key" skipped "$proposed" "" "$extras_tsv"
        read_frame || {
          emit_frame FAIL "EOF awaiting APPLY for $key"
          return 1
        }
        if [ "$FRAME_TYPE" != "APPLY" ]; then
          emit_frame FAIL "expected APPLY for $key, got $FRAME_TYPE"
          return 1
        fi
        # skip: no artefact mutation.
        emit_frame APPLIED_CONFIRM "$key"
        return 0
        ;;
      edit)
        local err
        if declare -F migration_validate_edit >/dev/null; then
          _HARNESS_CURRENT_TSV="TX	$(escape_field "$key")	$(escape_field "$path")	$(escape_field "$anchor")	$(escape_field "$proposed")	$(escape_field "$predicate_value")	$extras_tsv	$display_b64"
          err=$(migration_validate_edit "$key" "$path" "$anchor" "$proposed" "$value" 2>&1) || {
            emit_frame VALIDATE_ERR "$err"
            continue
          }
        fi
        emit_frame RECORDED "$key" edited "$proposed" "$value" "$extras_tsv"
        read_frame || {
          emit_frame FAIL "EOF awaiting APPLY for $key"
          return 1
        }
        if [ "$FRAME_TYPE" != "APPLY" ]; then
          emit_frame FAIL "expected APPLY for $key, got $FRAME_TYPE"
          return 1
        fi
        if ! migration_apply_decision "$key" "$path" "$anchor" edit "$value"; then
          emit_frame FAIL "migration_apply_decision failed for $key (edit)"
          return 1
        fi
        emit_frame APPLIED_CONFIRM "$key"
        return 0
        ;;
      *)
        emit_frame FAIL "unknown decision outcome '$outcome' for $key"
        return 1
        ;;
    esac
  done
}

# _harness_handle_resume: emit RESUMED_APPLIED / RESUMED_SKIPPED / DRIFT
# based on the resume-state map. Full implementation in Phase 6; the
# Phase 3 stub forwards to PROMPT for any resumed key (no resume yet).
_harness_handle_resume() {
  local key="$1" path="$2" anchor="$3" proposed="$4"
  local predicate_value="$5" extras_tsv="$6" display_b64="$7"
  # Caller has already invoked _harness_resume_lookup "$key"; pull the
  # results out of the side-channel globals so a nested call can't
  # clobber them mid-flow.
  local r_outcome="$RESUME_LOOKUP_OUTCOME"
  local r_proposed="$RESUME_LOOKUP_PROPOSED"
  local r_user="$RESUME_LOOKUP_USER"
  if [ "$r_proposed" = "$proposed" ]; then
    local verify_failed=0
    if [ "$r_outcome" = "accepted" ] || [ "$r_outcome" = "edited" ]; then
      if declare -F migration_verify_applied >/dev/null; then
        migration_verify_applied "$key" "$path" "$anchor" "$r_outcome" "$r_proposed" "$r_user" ||
          verify_failed=1
      fi
    fi
    if [ "$verify_failed" -eq 1 ]; then
      emit_frame DRIFT "$key"
      read_frame || {
        emit_frame FAIL "EOF awaiting DRIFT_CLEARED for $key"
        return 1
      }
      if [ "$FRAME_TYPE" != "DRIFT_CLEARED" ]; then
        emit_frame FAIL "expected DRIFT_CLEARED for $key, got $FRAME_TYPE"
        return 1
      fi
      # Fall through to fresh predicate evaluation + prompt.
      _harness_run_prompt "$key" "$path" "$anchor" "$proposed" \
        "$predicate_value" "$extras_tsv" "$display_b64"
      return $?
    fi
    case "$r_outcome" in
      accepted | edited) emit_frame RESUMED_APPLIED "$key" ;;
      skipped) emit_frame RESUMED_SKIPPED "$key" ;;
      *)
        emit_frame FAIL "unknown resume outcome '$r_outcome' for key $key"
        return 1
        ;;
    esac
    return 0
  fi
  # Drift: live proposed_value differs from recorded.
  emit_frame DRIFT "$key"
  read_frame || {
    emit_frame FAIL "EOF awaiting DRIFT_CLEARED for $key"
    return 1
  }
  if [ "$FRAME_TYPE" != "DRIFT_CLEARED" ]; then
    emit_frame FAIL "expected DRIFT_CLEARED for $key, got $FRAME_TYPE"
    return 1
  fi
  _harness_run_prompt "$key" "$path" "$anchor" "$proposed" \
    "$predicate_value" "$extras_tsv" "$display_b64"
}
