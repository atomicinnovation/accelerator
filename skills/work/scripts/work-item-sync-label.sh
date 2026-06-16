#!/usr/bin/env bash
set -euo pipefail

# work-item-sync-label.sh — the single source of truth for the work-item sync
# status → {glyph, label} vocabulary that /list-work-items renders.
#
# Classification is PRESENCE-BASED: an item is *synced* iff it carries a
# non-empty external_id (the remote tracker's identifier), *unsynced* otherwise.
# "Non-empty" means length > 0 after stripping surrounding quotes and
# whitespace, so external_id: "" / quote-only / whitespace-only reads as
# unsynced. This is the same normalisation the Linear and Jira already-synced
# guards apply; keep the three in step.
#
# The label is MARKDOWN-NATIVE — a Unicode glyph plus distinct text, never ANSI
# escapes: /list-work-items emits a markdown table into the conversation, not to
# a TTY, so escape codes would surface as literal text. The two states differ in
# BOTH glyph and text so the signal survives monochrome / glyph-blind rendering.
#
# Usage:
#   work-item-sync-label.sh <external-id-value>   # classify + render label
#   work-item-sync-label.sh --classify <value>    # → status keyword only
#   work-item-sync-label.sh --label <status>      # → "<glyph> <text>" only
#
# Story 0051 extends the classifier and the label table with the
# baseline-dependent states (locally-modified, remotely-modified, conflict)
# without changing the /list-work-items rendering call site — add a case arm to
# sync_status_label (and the classifier) and the new state renders.

usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-sync-label.sh <external-id-value>
  work-item-sync-label.sh --classify <external-id-value>
  work-item-sync-label.sh --label <status>
USAGE
}

# Strip surrounding quotes and whitespace from a raw frontmatter scalar, then
# report whether anything remains. Mirrors the Linear guard's trimming so the
# classifier and the integration guards agree on "non-empty external_id".
sync_classify() {
  local raw="${1-}"
  local trimmed
  trimmed=$(printf '%s' "$raw" | sed "s/^[[:space:]\"']*//; s/[[:space:]\"']*\$//")
  if [ -n "$trimmed" ]; then
    printf 'synced'
  else
    printf 'unsynced'
  fi
}

# status keyword → "<glyph> <text>". The extensible seam: 0051 adds case arms.
sync_status_label() {
  local status="$1"
  case "$status" in
    synced) printf '🟢 synced' ;;
    unsynced) printf '⚪ unsynced' ;;
    *)
      printf 'work-item-sync-label.sh: unknown sync status: %s\n' "$status" >&2
      return 1
      ;;
  esac
}

main() {
  case "${1-}" in
    --classify)
      [ $# -eq 2 ] || {
        usage
        exit 1
      }
      sync_classify "$2"
      ;;
    --label)
      [ $# -eq 2 ] || {
        usage
        exit 1
      }
      sync_status_label "$2"
      ;;
    --help | -h)
      usage
      exit 0
      ;;
    -*)
      usage
      exit 1
      ;;
    *)
      [ $# -eq 1 ] || {
        usage
        exit 1
      }
      sync_status_label "$(sync_classify "$1")"
      ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  main "$@"
fi
