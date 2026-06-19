#!/usr/bin/env bash
set -euo pipefail

# work-item-sync-classify.sh — the shared change-detection engine. Both
# /list-work-items and /sync-work-items call it, so the five-state classification
# is never duplicated. It NEVER fetches; the caller hands it a pre-fetched remote
# record (the caller owns bulk-vs-show orchestration). The engine is
# tracker-agnostic: the caller projects + canonicalises a remote body into the
# comparable local shape (Jira ADF via `jq -S`; Linear Markdown as-is) BEFORE
# passing it as --remote-body-file.
#
# Usage:
#   work-item-sync-classify.sh \
#     --file <path> \
#     --external-id <value> \
#     --baseline <entry-json|""> \      # the last-sync.json entry for this id
#     --timestamp <epoch> \             # global baseline timestamp (mtime gate)
#     --remote-status present|absent|indeterminate \
#     [--remote-updated <iso>] \        # remote `updated` (when present)
#     [--remote-body-file <path>]       # projected+canonicalised body (when present)
#
# Prints ONE keyword on stdout:
#   synced | unsynced | locally-modified | remotely-modified | conflict
#   | remote-absent | indeterminate
# The first five render via work-item-sync-label.sh. remote-absent and
# indeterminate are handled by the caller (list: presence-only; sync: skip).
#
# Contract:
#   no external_id                       → unsynced (presence-only, never-pushed)
#   tracked + remote indeterminate       → indeterminate (failed/timed-out read)
#   tracked + remote absent              → remote-absent (gone from a complete fetch)
#   tracked + remote present:
#     local side  — mtime epoch (dual stat) ≤ timestamp ⇒ unchanged (advisory
#                   short-circuit); else hash(normalise(file)) == local_hash.
#                   An absent local_hash baseline counts as CHANGED (first-sync).
#     remote side — remote.updated == baseline.remote_updated_at ⇒ unchanged
#                   (trusted short-circuit, no body); else
#                   hash(normalise(body)) == remote_hash. An absent remote_hash
#                   baseline counts as CHANGED (first-sync).
#     verdict     — neither→synced, local→locally-modified,
#                   remote→remotely-modified, both→conflict.
# First-sync-on-dirty completeness: an item with an external_id but no baseline
# entry is judged by the FULL contract (absent hashes count as changed), so a
# first-sync item that is both remote-ahead and locally dirty surfaces as a
# conflict rather than being masked as synced. (The /list path passes such items
# through presence-only itself, so it never sees this branch.)

_WISC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_WISC_NORMALISE="$_WISC_DIR/work-item-normalise.sh"
# shellcheck source=scripts/hash-common.sh
source "$(cd "$_WISC_DIR/../../.." && pwd)/scripts/hash-common.sh"

_wisc_usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-sync-classify.sh --file <path> --external-id <value>
    --baseline <entry-json> --timestamp <epoch>
    --remote-status present|absent|indeterminate
    [--remote-updated <iso>] [--remote-body-file <path>]
USAGE
}

# Pure-integer mtime, dual stat (BSD `-f %m` || GNU `-c %Y`). A missing or
# non-numeric result is coerced to a large sentinel that FORCES the hash path —
# never an empty string into the `-le` arithmetic (which would abort under set -e).
_wisc_mtime() {
  local f="$1" m
  # The `||` MUST stay outside the command substitution: GNU `stat -f %m` treats
  # %m as a (missing) file yet still prints a `File:` filesystem block for $f to
  # stdout AND exits non-zero. An inside-`$(... || ...)` capture would splice that
  # block in front of the GNU epoch; separate assignments let the fallback
  # overwrite the BSD attempt cleanly (matches linear/jira common.sh).
  m=$(stat -f %m "$f" 2>/dev/null) ||
    m=$(stat -c %Y "$f" 2>/dev/null) || m=""
  if [[ "$m" =~ ^[0-9]+$ ]]; then
    printf '%s' "$m"
  else
    printf '9999999999'
  fi
}

_wisc_main() {
  local file="" external_id="" baseline="" timestamp="0"
  local remote_status="" remote_updated="" remote_body_file=""

  while [ $# -gt 0 ]; do
    case "$1" in
      --file)
        file="$2"
        shift 2
        ;;
      --external-id)
        external_id="$2"
        shift 2
        ;;
      --baseline)
        baseline="$2"
        shift 2
        ;;
      --timestamp)
        timestamp="$2"
        shift 2
        ;;
      --remote-status)
        remote_status="$2"
        shift 2
        ;;
      --remote-updated)
        remote_updated="$2"
        shift 2
        ;;
      --remote-body-file)
        remote_body_file="$2"
        shift 2
        ;;
      --help | -h)
        _wisc_usage
        exit 0
        ;;
      *)
        _wisc_usage
        return 1
        ;;
    esac
  done

  # Never-pushed → presence-only.
  if [ -z "$external_id" ]; then
    printf 'unsynced\n'
    return 0
  fi

  case "$remote_status" in
    indeterminate)
      printf 'indeterminate\n'
      return 0
      ;;
    absent)
      printf 'remote-absent\n'
      return 0
      ;;
    present) ;;
    *)
      printf 'work-item-sync-classify.sh: --remote-status must be present/absent/indeterminate\n' >&2
      return 1
      ;;
  esac

  [ -n "$file" ] || {
    _wisc_usage
    return 1
  }
  if ! [[ "$timestamp" =~ ^[0-9]+$ ]]; then timestamp=0; fi

  local base_local_hash base_remote_hash base_remote_updated
  base_local_hash=$(printf '%s' "$baseline" | jq -r '.local_hash // empty' 2>/dev/null || true)
  base_remote_hash=$(printf '%s' "$baseline" | jq -r '.remote_hash // empty' 2>/dev/null || true)
  base_remote_updated=$(printf '%s' "$baseline" | jq -r '.remote_updated_at // empty' 2>/dev/null || true)

  # --- local side -----------------------------------------------------------
  local local_changed=1
  if [ -n "$base_local_hash" ]; then
    local mtime
    mtime=$(_wisc_mtime "$file")
    if [ "$mtime" -le "$timestamp" ]; then
      local_changed=0 # advisory pre-filter short-circuit to unchanged
    else
      local cur_hash
      cur_hash=$(bash "$_WISC_NORMALISE" "$file" | hash_sha256_stdin)
      if [ "$cur_hash" = "$base_local_hash" ]; then local_changed=0; fi
    fi
  fi

  # --- remote side ----------------------------------------------------------
  local remote_changed=1
  if [ -n "$base_remote_hash" ]; then
    if [ -n "$base_remote_updated" ] && [ "$remote_updated" = "$base_remote_updated" ]; then
      remote_changed=0 # trusted updated-equality short-circuit (no body)
    elif [ -n "$remote_body_file" ] && [ -f "$remote_body_file" ]; then
      local rem_hash
      rem_hash=$(bash "$_WISC_NORMALISE" --stdin <"$remote_body_file" | hash_sha256_stdin)
      if [ "$rem_hash" = "$base_remote_hash" ]; then remote_changed=0; fi
    fi
    # else: updated differs and no body provided → conservatively changed.
  fi

  # --- verdict --------------------------------------------------------------
  if [ "$local_changed" -eq 0 ] && [ "$remote_changed" -eq 0 ]; then
    printf 'synced\n'
  elif [ "$local_changed" -eq 1 ] && [ "$remote_changed" -eq 0 ]; then
    printf 'locally-modified\n'
  elif [ "$local_changed" -eq 0 ] && [ "$remote_changed" -eq 1 ]; then
    printf 'remotely-modified\n'
  else
    printf 'conflict\n'
  fi
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _wisc_main "$@"
fi
