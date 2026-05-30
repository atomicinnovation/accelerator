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
  cat > "$tmp"
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
#   a 60 s ceiling.
_atomic_lock_acquire() {
  local lockdir="$1"
  local waited_ms=0
  local base_ms=4
  # Re-seed RANDOM per call: concurrent subshells inherit the parent's
  # RANDOM state and would otherwise sleep in lockstep, defeating the
  # jitter. PID + nanosecond clock gives unique seeds across forks.
  RANDOM=$(( ($$ * 31 + ${RANDOM:-0}) ^ \
    $(date +%N 2>/dev/null | sed 's/^0*//' || echo 0) ))
  while ! mkdir "$lockdir" 2>/dev/null; do
    if [ "$waited_ms" -gt 60000 ]; then
      echo "atomic_jsonl: lock acquisition timed out on $lockdir" >&2
      return 1
    fi
    local jitter_ms=$(( (RANDOM % base_ms) + 1 ))
    sleep "0.$(printf '%03d' "$jitter_ms")"
    waited_ms=$((waited_ms + jitter_ms))
    [ "$base_ms" -lt 200 ] && base_ms=$((base_ms * 2))
  done
}

_atomic_lock_release() {
  local lockdir="$1"
  rmdir "$lockdir" 2>/dev/null || true
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
    echo "atomic_jsonl_append: missing target or line" >&2; return 1
  fi
  case "$line" in *$'\n'*)
    echo "atomic_jsonl_append: line must not contain embedded newline" >&2
    return 1 ;;
  esac
  local dir
  dir=$(dirname "$target")
  if ! mkdir -p "$dir"; then
    echo "atomic_jsonl_append: cannot create directory $dir" >&2
    return 1
  fi
  local lockdir="${target}.lockdir"
  _atomic_lock_acquire "$lockdir" || return 1
  local rc=0
  {
    if [ -f "$target" ] && [ -s "$target" ]; then
      cat "$target"
    fi
    printf '%s\n' "$line"
  } | atomic_write "$target" || rc=$?
  _atomic_lock_release "$lockdir"
  return "$rc"
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
  _atomic_lock_acquire "$lockdir" || return 1
  local escaped_key prefix rc=0
  escaped_key=$(jsonl_json_escape "$key")
  prefix=$(printf '{"transformation_key":"%s",' "$escaped_key")
  # Pass the prefix via ENVIRON (NOT -v): awk's -v processes backslash
  # escapes in the assigned value, which would re-interpret \" and \\
  # inside the JSON-escaped key and silently break the match. ENVIRON
  # values are passed through unmodified.
  JSONL_REMOVE_PREFIX="$prefix" \
    awk 'BEGIN{p=ENVIRON["JSONL_REMOVE_PREFIX"]} index($0,p)!=1{print}' "$target" \
    | atomic_write "$target" || rc=$?
  _atomic_lock_release "$lockdir"
  return "$rc"
}
