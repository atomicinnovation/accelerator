#!/usr/bin/env bash
set -euo pipefail

# work-item-sync-baseline.sh — read/write the last-sync.json content-parity
# baseline: a per-item map keyed by the stable local `id`, stored in the active
# integration's state subdirectory. Pure helper; all mutations go through
# atomic_write (same-dir temp + mv) so a mid-write crash never leaves a truncated
# baseline — the property the resumability design rests on.
#
# Schema (committed; keyed by local id):
#   { "timestamp": <epoch-seconds int>,   # run-START of the last clean sync;
#                                          # epoch (not ISO) so the local mtime
#                                          # pre-filter is a pure integer compare.
#     "items": { "<id>": {
#         "remote_updated_at": "<ISO8601>",  # remote-side pre-filter
#         "remote_hash": "<sha256 of normalised remote content at last sync>",
#         "local_hash":  "<sha256 of normalised local content at last sync>" } } }
#
# remote_hash makes the remote side authoritative and symmetric with local_hash
# (the engine decides "remote changed since baseline?" without storing the body).
#
# Usage:
#   work-item-sync-baseline.sh path
#   work-item-sync-baseline.sh get <id>
#   work-item-sync-baseline.sh set <id> <remote_updated_at> <remote_hash> <local_hash>
#   work-item-sync-baseline.sh set-timestamp <epoch-secs>
#   work-item-sync-baseline.sh remove <id>
#
# A missing OR present-but-unparseable (e.g. VCS conflict-markered) baseline is a
# valid EMPTY baseline: `get` prints nothing and exits 0; mutating commands start
# from an empty document. This hard contract lets a botched merge degrade to
# presence-only + a full re-hash on the next sync rather than crashing.

_WISB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_WISB_REPO_SCRIPTS="$(cd "$_WISB_DIR/../../.." && pwd)/scripts"
# shellcheck source=scripts/vcs-common.sh
source "$_WISB_REPO_SCRIPTS/vcs-common.sh"
# shellcheck source=scripts/atomic-common.sh
source "$_WISB_REPO_SCRIPTS/atomic-common.sh"

_wisb_usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-sync-baseline.sh path
  work-item-sync-baseline.sh get <id>
  work-item-sync-baseline.sh set <id> <remote_updated_at> <remote_hash> <local_hash>
  work-item-sync-baseline.sh set-timestamp <epoch-secs>
  work-item-sync-baseline.sh remove <id>
USAGE
}

# Resolve <integrations-path>/<work.integration>/last-sync.json. The <system>
# segment is assembled here (no shared helper appends it).
_wisb_path() {
  local root
  root=$(find_repo_root) || {
    echo "E_NO_REPO: cannot locate repository root" >&2
    return 1
  }
  local integration
  integration=$(cd "$root" && "${ACCELERATOR_BIN:-${_WISB_REPO_SCRIPTS%/scripts}/bin/accelerator}" config work integration)
  if [ -z "$integration" ]; then
    echo "E_NO_INTEGRATION: work.integration is not configured" >&2
    return 1
  fi
  local ipath
  ipath=$(cd "$root" && "${ACCELERATOR_BIN:-${_WISB_REPO_SCRIPTS%/scripts}/bin/accelerator}" config path integrations)
  local base
  if [ "${ipath#/}" != "$ipath" ]; then
    base="$ipath"
  else
    base="$root/$ipath"
  fi
  printf '%s/%s/last-sync.json\n' "$base" "$integration"
}

# Echo the current baseline document, or an empty one if the file is missing or
# unparseable (the hard degrade-to-empty contract).
_wisb_read_or_empty() {
  local f="$1"
  if [ -f "$f" ] && jq -e . "$f" >/dev/null 2>&1; then
    cat "$f"
  else
    printf '{"timestamp":0,"items":{}}'
  fi
}

_wisb_main() {
  local cmd="${1-}"
  case "$cmd" in
    --help | -h)
      _wisb_usage
      exit 0
      ;;
    path)
      _wisb_path
      ;;
    get)
      [ $# -eq 2 ] || {
        _wisb_usage
        exit 1
      }
      local id="$2" f
      f=$(_wisb_path) || return 1
      if [ -f "$f" ] && jq -e . "$f" >/dev/null 2>&1; then
        jq -c --arg id "$id" '.items[$id] // empty' "$f"
      fi
      ;;
    set)
      [ $# -eq 5 ] || {
        _wisb_usage
        exit 1
      }
      local id="$2" ru="$3" rh="$4" lh="$5" f cur
      f=$(_wisb_path) || return 1
      cur=$(_wisb_read_or_empty "$f")
      printf '%s' "$cur" | jq -c \
        --arg id "$id" --arg ru "$ru" --arg rh "$rh" --arg lh "$lh" \
        '.items[$id] = {remote_updated_at: $ru, remote_hash: $rh, local_hash: $lh}' |
        atomic_write "$f"
      ;;
    set-timestamp)
      [ $# -eq 2 ] || {
        _wisb_usage
        exit 1
      }
      local epoch="$2" f cur
      if ! [[ "$epoch" =~ ^[0-9]+$ ]]; then
        echo "E_BAD_TIMESTAMP: set-timestamp expects epoch seconds; got '$epoch'" >&2
        return 1
      fi
      f=$(_wisb_path) || return 1
      cur=$(_wisb_read_or_empty "$f")
      printf '%s' "$cur" | jq -c --argjson ts "$epoch" '.timestamp = $ts' |
        atomic_write "$f"
      ;;
    remove)
      [ $# -eq 2 ] || {
        _wisb_usage
        exit 1
      }
      local id="$2" f cur
      f=$(_wisb_path) || return 1
      cur=$(_wisb_read_or_empty "$f")
      printf '%s' "$cur" | jq -c --arg id "$id" 'del(.items[$id])' |
        atomic_write "$f"
      ;;
    *)
      _wisb_usage
      exit 1
      ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _wisb_main "$@"
fi
