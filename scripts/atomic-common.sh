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
