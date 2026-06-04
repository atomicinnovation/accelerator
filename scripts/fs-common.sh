#!/usr/bin/env bash
# Filesystem relocation helpers shared by the migration scripts. Source
# this file from a shell script:
#
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/fs-common.sh"
#
# This module is deliberately separate from atomic-common.sh: merge_move is
# explicitly NON-ATOMIC (the opposite of that file's atomic-write contract),
# and migrations that only need to relocate directories should not have to
# pull in atomic-common.sh's transitive JSONL/lock machinery.

# merge_move <src> <dst>
#   Move <src> onto <dst>, merging directories recursively. When the
#   destination is absent it is a plain move. When both are directories each
#   entry of <src> is merged into <dst>; same-named leaf files are overwritten
#   by the source (source-wins). The now-empty source directory is removed.
#   Missing <src> is a no-op.
#   NON-ATOMIC: a per-entry mv/rm sequence, so a mid-merge failure leaves a
#   partially-merged tree — a re-run converges (idempotent), VCS is the recovery
#   net. Same filesystem assumed (cross-fs mv is itself non-atomic on POSIX).
#   bash 3.2 compatible (no globstar/nullglob deps).
merge_move() {
  local src="$1" dst="$2"
  [ -e "$src" ] || return 0
  # Cheap bug-guard: refuse a destination that could escalate the rm/mv below
  # into a wide delete — empty, root, a trailing-slash empty leaf (e.g. an empty
  # rel component yielding "$PROJECT_ROOT/"), or a '..' escape. The current
  # callers all pass "$PROJECT_ROOT/<fixed-rel>", so this only fires on a future
  # caller bug. (Recovery is VCS; this guards a code bug, not a no-VCS user.)
  case "$dst" in
    "" | "/" | */)
      echo "merge_move: refusing unsafe destination '$dst' for '$src'" >&2
      return 1
      ;;
    *"/../"* | */..)
      echo "merge_move: refusing path-escaping destination '$dst'" >&2
      return 1
      ;;
  esac

  if [ ! -e "$dst" ]; then
    mkdir -p "$(dirname "$dst")"
    mv "$src" "$dst"
    return 0
  fi

  # Type mismatch or leaf collision: source wins wholesale. The non-empty
  # guard above ensures rm never targets an unintended path; `--` stops a
  # leading-dash path being read as an option.
  if [ ! -d "$src" ] || [ ! -d "$dst" ]; then
    rm -rf -- "$dst"
    mkdir -p "$(dirname "$dst")"
    mv "$src" "$dst"
    return 0
  fi

  # Both directories — merge each entry of src into dst. The three globs match
  # regular files, then dotfiles excluding '.' and '..' ('.[!.]*' and '..?*'),
  # so no basename '.'/'..' filter is needed (the old `basename "$src/.."`
  # never equalled '..' anyway). '[ -e ]' guards each unmatched literal glob.
  local entry name
  for entry in "$src"/* "$src"/.[!.]* "$src"/..?*; do
    [ -e "$entry" ] || continue
    name="${entry##*/}"
    merge_move "$entry" "$dst/$name"
  done
  # Source should now be empty; a non-empty source signals a non-converging
  # merge — surface it rather than swallowing the rmdir failure with `|| true`.
  if ! rmdir "$src" 2>/dev/null; then
    echo "merge_move: source '$src' not empty after merge — left in place" >&2
  fi
}
