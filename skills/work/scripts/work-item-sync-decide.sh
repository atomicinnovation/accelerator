#!/usr/bin/env bash
set -euo pipefail

# work-item-sync-decide.sh — the deterministic (mode × state) decision table for
# /sync-work-items, mirroring how work-item-push-decide.sh isolates the push
# state machine. It moves the safety-critical orchestration out of model-executed
# SKILL prose into one CI-testable place, including the forbidden-write cells for
# directional modes.
#
# Subcommands:
#   mode [--push-only] [--pull-only]
#       Resolve the mode flags to one keyword: bidirectional (default) /
#       push-only / pull-only. Supplying BOTH directional flags is an error
#       (exit 2) — they are mutually exclusive.
#
#   decide --mode <m> --state <s> [--dirty 0|1]
#       Map (mode, classified-state, local-dirty?) to ONE action keyword:
#         push          push the local-ahead item to the remote
#         pull          overwrite the local file from the remote
#         skip-conflict report a conflict and leave both sides unchanged
#         skip-dirty    skip a pull whose local file has uncommitted changes
#         prompt        hand to the interactive conflict/dirty resolver
#                       (bidirectional only; Phase 7)
#         noop          nothing to do / forbidden-write cell / unknown remote
#       <state> ∈ synced | locally-modified | remotely-modified | conflict
#                | remote-absent | indeterminate | unsynced
#       --dirty only affects the remotely-modified row (a pull overwrites the
#       local file; a dirty file must never be silently overwritten).
#
#   resolve-conflict-token <raw>
#       Map a typed conflict-prompt token to ONE action (Phase 7). After trimming
#       and case-folding: remote→accept-remote, local→push-local; EMPTY or any
#       UNRECOGNISED token→skip (never a destructive write). The SKILL owns the
#       re-prompt-once wording; the safe default lives here so the
#       destructive-choice interpretation is unit-tested.
#
# Forbidden-write cells (asserted in tests): a conflict or remote-ahead item
# under --push-only never pulls; a local-ahead item under --pull-only never
# pushes; an indeterminate or remote-absent item never writes either side.

_wisd_usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-sync-decide.sh mode [--push-only] [--pull-only]
  work-item-sync-decide.sh decide --mode <m> --state <s> [--dirty 0|1]
  work-item-sync-decide.sh resolve-conflict-token <raw>
USAGE
}

_wisd_mode() {
  local push_only=0 pull_only=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --push-only)
        push_only=1
        shift
        ;;
      --pull-only)
        pull_only=1
        shift
        ;;
      *)
        _wisd_usage
        return 2
        ;;
    esac
  done
  if [ "$push_only" -eq 1 ] && [ "$pull_only" -eq 1 ]; then
    echo "E_MODE_CONFLICT: --push-only and --pull-only are mutually exclusive" >&2
    return 2
  fi
  if [ "$push_only" -eq 1 ]; then
    printf 'push-only\n'
  elif [ "$pull_only" -eq 1 ]; then
    printf 'pull-only\n'
  else
    printf 'bidirectional\n'
  fi
}

_wisd_decide() {
  local mode="" state="" dirty=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --mode)
        mode="$2"
        shift 2
        ;;
      --state)
        state="$2"
        shift 2
        ;;
      --dirty)
        dirty="$2"
        shift 2
        ;;
      *)
        _wisd_usage
        return 2
        ;;
    esac
  done
  case "$mode" in
    bidirectional | push-only | pull-only) ;;
    *)
      echo "E_BAD_MODE: --mode must be bidirectional/push-only/pull-only" >&2
      return 2
      ;;
  esac

  case "$state" in
    synced | remote-absent | indeterminate | unsynced)
      # Nothing to reconcile, or unknown/absent remote → never write either side.
      printf 'noop\n'
      ;;
    locally-modified)
      # Local ahead → push, unless the mode forbids a remote write.
      case "$mode" in
        bidirectional | push-only) printf 'push\n' ;;
        pull-only) printf 'noop\n' ;; # forbidden-write cell
      esac
      ;;
    remotely-modified)
      # Remote ahead → pull (overwrites local), unless the mode forbids a local
      # write. A dirty local file must never be silently overwritten.
      case "$mode" in
        push-only) printf 'noop\n' ;; # forbidden-write cell
        bidirectional)
          if [ "$dirty" = "1" ]; then printf 'prompt\n'; else printf 'pull\n'; fi
          ;;
        pull-only)
          if [ "$dirty" = "1" ]; then printf 'skip-dirty\n'; else printf 'pull\n'; fi
          ;;
      esac
      ;;
    conflict)
      # Both ahead. Bidirectional resolves interactively (Phase 7); directional
      # modes report and skip (a resolution would need a write the mode forbids).
      case "$mode" in
        bidirectional) printf 'prompt\n' ;;
        push-only | pull-only) printf 'skip-conflict\n' ;;
      esac
      ;;
    *)
      echo "E_BAD_STATE: unknown state: $state" >&2
      return 2
      ;;
  esac
}

# Lowercase + trim a token (bash 3.2 floor: no ${var,,}).
_wisd_norm_token() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]' |
    sed 's/^[[:space:]]*//; s/[[:space:]]*$//'
}

_wisd_resolve_conflict_token() {
  local raw="${1-}" tok
  tok=$(_wisd_norm_token "$raw")
  case "$tok" in
    remote) printf 'accept-remote\n' ;;
    local) printf 'push-local\n' ;;
    *) printf 'skip\n' ;; # skip, empty, or anything unrecognised → safe skip
  esac
}

_wisd_main() {
  local cmd="${1-}"
  case "$cmd" in
    --help | -h)
      _wisd_usage
      exit 0
      ;;
    mode)
      shift
      _wisd_mode "$@"
      ;;
    decide)
      shift
      _wisd_decide "$@"
      ;;
    resolve-conflict-token)
      shift
      [ $# -ge 1 ] || {
        _wisd_usage
        return 2
      }
      _wisd_resolve_conflict_token "${1-}"
      ;;
    *)
      _wisd_usage
      return 2
      ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _wisd_main "$@"
fi
