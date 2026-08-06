#!/usr/bin/env bash
# Atomic file-write helpers shared by the migration runner and
# migration scripts. Source this file from a shell script:
#
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/atomic-common.sh"
#
# All helpers create the temp file in the same directory as the
# target so the rename is atomic on the same filesystem (mv across
# filesystems is not atomic on POSIX). Failures clean up the temp
# file via an EXIT trap installed only for the duration of the call.

# atomic_write <target_path>
#   Reads stdin and writes it atomically to <target_path>. The
#   destination directory is created if it does not exist.
atomic_write() {
  local target="$1"
  if [ -z "$target" ]; then
    echo "atomic_write: missing target path" >&2
    return 1
  fi
  local dir
  dir="$(dirname "$target")"
  mkdir -p "$dir"
  local tmp
  tmp="$(mktemp "$dir/.atomic-write.XXXXXX")"
  # Trap inside a function: run the cleanup only if something interrupts before mv
  trap 'rm -f "'"$tmp"'"' EXIT
  cat >"$tmp"
  mv "$tmp" "$target"
  trap - EXIT
}

# atomic_append_unique <target_path> <line>
#   Atomically adds <line> to <target_path> if not already present.
#   Idempotent. The line is appended to the existing content, with a
#   trailing newline.
atomic_append_unique() {
  local target="$1"
  local line="$2"
  if [ -z "$target" ] || [ -z "$line" ]; then
    echo "atomic_append_unique: missing target or line" >&2
    return 1
  fi

  if [ -f "$target" ] && grep -Fxq -- "$line" "$target"; then
    return 0
  fi

  local existing=""
  if [ -f "$target" ]; then
    existing=$(cat "$target")
  fi

  {
    if [ -n "$existing" ]; then
      printf '%s\n' "$existing"
    fi
    printf '%s\n' "$line"
  } | atomic_write "$target"
}

# atomic_remove_line <target_path> <line>
#   Atomically removes every line equal to <line> from <target_path>.
#   Absence is a no-op. Empty target file results in no output.
atomic_remove_line() {
  local target="$1"
  local line="$2"
  if [ -z "$target" ] || [ -z "$line" ]; then
    echo "atomic_remove_line: missing target or line" >&2
    return 1
  fi
  if [ ! -f "$target" ]; then
    return 0
  fi
  if ! grep -Fxq -- "$line" "$target"; then
    return 0
  fi
  grep -Fxv -- "$line" "$target" | atomic_write "$target" || true
}

# ── JSONL helpers ────────────────────────────────────────────────────────────
# The JSONL helpers below need a portable mutex. The plan called for
# flock(1), but flock is not in POSIX and is absent from stock macOS.
# Rather than maintain a two-implementation hazard, we use mkdir(2) as
# the lock primitive: POSIX guarantees mkdir is atomic on a single
# filesystem, so a successful mkdir on the sidecar lockdir is exclusive
# acquisition. Held for the duration of a single read-modify-write
# critical section (a few ms on local filesystems).
#
# Source the shared JSONL composition helpers (single source of truth
# shared with the runner's session-log composition).
_ATOMIC_COMMON_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./jsonl-common.sh
source "$_ATOMIC_COMMON_DIR/jsonl-common.sh"

# _atomic_lock_acquire <lockdir>
#   Spin until mkdir succeeds. Each subshell re-seeds bash's RANDOM
#   from PID + EPOCHREALTIME (so concurrent appenders don't share the
#   inherited RANDOM state) and then uses jittered back-off bounded by
#   a 300 s ceiling — generous enough to absorb CI scheduler starvation
#   when many parallel tasks compete for cores. On acquisition it records
#   the holder PID so a later waiter can reclaim the lock if the holder
#   dies mid-critical-section instead of spinning to the timeout.
_atomic_lock_acquire() {
  local lockdir="$1"
  # Fail fast if the parent dir isn't writable. Without this check the
  # spin-loop would burn the full timeout on an error mkdir can never
  # recover from (e.g., chmod 555 on the parent).
  local parent_dir
  parent_dir=$(dirname "$lockdir")
  if [ ! -w "$parent_dir" ]; then
    echo "atomic_jsonl: cannot create lock under $parent_dir (not writable)" >&2
    return 1
  fi
  local waited_ms=0
  local base_ms=4
  # Re-seed RANDOM per call: concurrent subshells inherit the parent's
  # RANDOM state and would otherwise sleep in lockstep, defeating the
  # jitter. PID + nanosecond clock gives unique seeds across forks.
  RANDOM=$((($$ * 31 + ${RANDOM:-0}) ^ \
    $(date +%N 2>/dev/null | sed 's/^0*//' || echo 0)))
  while ! mkdir "$lockdir" 2>/dev/null; do
    # A lockdir whose recorded owner has died (e.g. OOM-killed mid
    # critical section, common under parallel CI load) would otherwise
    # stall every waiter for the full timeout. Reclaim it once the owner
    # is confirmed gone.
    _atomic_lock_reclaim_if_stale "$lockdir"
    if [ "$waited_ms" -gt 300000 ]; then
      echo "atomic_jsonl: lock acquisition timed out on $lockdir" >&2
      return 1
    fi
    local jitter_ms=$(((RANDOM % base_ms) + 1))
    sleep "0.$(printf '%03d' "$jitter_ms")"
    waited_ms=$((waited_ms + jitter_ms))
    [ "$base_ms" -lt 200 ] && base_ms=$((base_ms * 2))
  done
  # Record ownership so a waiter can detect us dying and reclaim the lock.
  # $BASHPID is the critical-section subshell's own PID (bash 4+); on bash
  # 3.2 it is unset, so we skip ownership and fall back to spin-only.
  #
  # The sentinel's NAME carries a nonce (`owner.<nonce>`), not just its
  # contents. That is what makes reclaim safe — see
  # _atomic_lock_reclaim_if_stale — and it is the same on-disk format the Rust
  # implementation in cli/corpus-adapters writes, so the two can reclaim after
  # each other on a shared `<target>.lockdir`.
  [ -n "${BASHPID:-}" ] &&
    printf '%s\n' "$BASHPID" >"$lockdir/owner.$(_atomic_lock_nonce)" \
    2>/dev/null || true
}

# _atomic_lock_nonce
#   A per-holder token for the sentinel filename. Two RANDOM draws plus the
#   subshell PID: RANDOM alone is 15 bits, which collides far too readily
#   across the many short-lived subshells a parallel run spawns.
_atomic_lock_nonce() {
  printf '%04x%04x%04x' "${BASHPID:-$$}" "$RANDOM" "$RANDOM"
}

_atomic_lock_release() {
  local lockdir="$1"
  # rm -rf, not rmdir: the lockdir also holds the owner sentinel file.
  rm -rf "$lockdir" 2>/dev/null || true
}

# _atomic_lock_reclaim_if_stale <lockdir>
#   Remove a lockdir whose recorded owner process is gone. Safe because a
#   dead process cannot be inside the critical section; PID reuse only
#   yields a false "still alive" reading, degrading to the spin-and-wait
#   path rather than ever breaking a live lock. A lockdir with no sentinel
#   yet (holder still mid-acquisition) is treated as live.
#
#   Reclaim is a read-then-act and the read can go stale: between reading a
#   dead owner and acting on it, another waiter can reclaim, mkdir and take
#   the lock. A bare `rm -rf` then deletes that new, LIVE holder's lockdir and
#   two callers run the critical section at once — a lost append, since the
#   guarded operation is a read-modify-write. So the removal is gated on
#   winning an `mv` of the sentinel: rename(2) is atomic, which makes the step
#   single-winner among waiters that read the same holder, while the nonce in
#   the name makes it a no-op against any OTHER holder. Re-nonced rather than
#   renamed to a fixed name so the same gate covers the crash-recovery case
#   below.
#
#   A holder that dies between the mv and the rm leaves `reclaiming.<nonce>`.
#   That state is picked up here too — otherwise the lockdir would be left
#   sentinel-less and every later waiter would read it as permanently held.
_atomic_lock_reclaim_if_stale() {
  local lockdir="$1" sentinel claimed
  sentinel=$(_atomic_lock_reclaimable "$lockdir") || return 0
  # The winner's OWN pid goes in the name, put there atomically by the mv.
  claimed="$lockdir/reclaiming.${BASHPID:-$$}.$(_atomic_lock_nonce)"
  mv "$lockdir/$sentinel" "$claimed" 2>/dev/null || return 0
  rm -rf "$lockdir" 2>/dev/null || true
}

# _atomic_lock_reclaimable <lockdir>
#   Echo the sentinel a reclaim may take, if any; non-zero when none may be.
_atomic_lock_reclaimable() {
  local lockdir="$1" sentinel owner pid
  sentinel=$(_atomic_lock_sentinel "$lockdir") || return 1
  case "$sentinel" in
    reclaiming.*)
      # A reclaim between its mv and its rm. Taking it over is only safe once
      # the RECLAIMER is dead — gating on the original holder instead would
      # let two waiters each win an mv of a different name and both proceed to
      # rm, the slower one landing on whatever fresh, live lockdir had since
      # replaced it. The reclaimer's pid is read from the NAME, published
      # atomically by the mv that created it.
      pid="${sentinel#reclaiming.}"
      pid="${pid%%.*}"
      [ -n "$pid" ] || return 1
      kill -0 "$pid" 2>/dev/null && return 1
      ;;
    *)
      owner=$(cat "$lockdir/$sentinel" 2>/dev/null) || return 1
      [ -n "$owner" ] || return 1
      kill -0 "$owner" 2>/dev/null && return 1
      ;;
  esac
  printf '%s\n' "$sentinel"
}

# _atomic_lock_sentinel <lockdir>
#   Echo the lockdir's single `owner.<nonce>` sentinel, or its
#   `reclaiming.<nonce>` if a reclaim died mid-flight. Non-zero when there is
#   no sentinel at all (a holder mid-acquisition), or when there is more than
#   one of a kind — which no correct writer produces, so it is read as "do not
#   touch" rather than guessed at.
_atomic_lock_sentinel() {
  local lockdir="$1" prefix name found count
  for prefix in owner reclaiming; do
    found=""
    count=0
    for name in "$lockdir/$prefix".*; do
      [ -f "$name" ] || continue
      found="${name##*/}"
      count=$((count + 1))
    done
    if [ "$count" -eq 1 ]; then
      printf '%s\n' "$found"
      return 0
    fi
    [ "$count" -gt 1 ] && return 1
  done
  # A nonce-less `owner` is the pre-nonce on-disk format. Nothing writes it
  # any more, so it can only be an orphan left by a holder that died before
  # the upgrade — and reclaiming it is still single-winner, because the `mv`
  # gate applies to it exactly as to a nonced sentinel. Without this the
  # orphan would be unreadable and wedge that file's lock for the full
  # 300 s ceiling on every append.
  [ -f "$lockdir/owner" ] && printf 'owner\n' && return 0
  return 1
}

# atomic_jsonl_append <target_path> <json_line>
#   Append one JSONL record atomically at the record level.
#
#   The implementation reads the existing file (if any), concatenates
#   the new line, and rewrites via atomic_write (same-directory temp +
#   rename — POSIX rename(2) is atomic). Concurrent callers serialise
#   on a sidecar lockdir so two simultaneous appends never lose data.
#   A crash leaves either the prior content or the post-append content
#   fully visible on disk; no partial-line state is ever observable.
#
#   The caller is responsible for ensuring <json_line> is a single
#   well-formed JSON line. The helper does not validate JSON.
atomic_jsonl_append() {
  local target="$1" line="$2"
  if [ -z "$target" ] || [ -z "$line" ]; then
    echo "atomic_jsonl_append: missing target or line" >&2
    return 1
  fi
  case "$line" in *$'\n'*)
    echo "atomic_jsonl_append: line must not contain embedded newline" >&2
    return 1
    ;;
  esac
  local dir
  dir=$(dirname "$target")
  if ! mkdir -p "$dir"; then
    echo "atomic_jsonl_append: cannot create directory $dir" >&2
    return 1
  fi
  local lockdir="${target}.lockdir"
  # Run the critical section inside a subshell that owns its EXIT trap.
  # If the subshell is killed (e.g., OOM, parent signal) between acquire
  # and the explicit release, the trap still releases the lock — and
  # because the subshell scope owns the trap, the caller's own EXIT
  # trap is left untouched.
  (
    _atomic_lock_acquire "$lockdir" || exit 1
    trap '_atomic_lock_release "'"$lockdir"'"' EXIT
    {
      if [ -f "$target" ] && [ -s "$target" ]; then
        cat "$target"
      fi
      printf '%s\n' "$line"
    } | atomic_write "$target"
  )
}

# atomic_jsonl_remove_by_key <target_path> <transformation_key>
#   Rewrite <target_path> atomically, dropping every JSONL record
#   whose canonical first field "transformation_key" equals
#   <transformation_key>. Absence or empty file is a no-op.
#
#   Match is line-anchored against the canonical writer output: every
#   record begins exactly with the literal bytes
#     {"transformation_key":"<JSON-escaped-key>",
#   The writer (jsonl_compose_record) is responsible for enforcing this
#   ordering; this helper assumes it.
atomic_jsonl_remove_by_key() {
  local target="$1" key="$2"
  if [ -z "$target" ] || [ -z "$key" ]; then
    echo "atomic_jsonl_remove_by_key: missing target or key" >&2
    return 1
  fi
  if [ ! -f "$target" ] || [ ! -s "$target" ]; then
    return 0
  fi
  local lockdir="${target}.lockdir"
  # Critical section in a subshell (see atomic_jsonl_append for rationale).
  (
    _atomic_lock_acquire "$lockdir" || exit 1
    trap '_atomic_lock_release "'"$lockdir"'"' EXIT
    local escaped_key prefix
    escaped_key=$(jsonl_json_escape "$key")
    prefix=$(printf '{"transformation_key":"%s",' "$escaped_key")
    # Pass the prefix via ENVIRON (NOT -v): awk's -v processes backslash
    # escapes in the assigned value, which would re-interpret \" and \\
    # inside the JSON-escaped key and silently break the match. ENVIRON
    # values are passed through unmodified.
    JSONL_REMOVE_PREFIX="$prefix" \
      awk 'BEGIN{p=ENVIRON["JSONL_REMOVE_PREFIX"]} index($0,p)!=1{print}' "$target" |
      atomic_write "$target"
  )
}
