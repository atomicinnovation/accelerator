#!/usr/bin/env bash
set -euo pipefail

# work-item-push-decide.sh — the deterministic decision seam for
# /create-work-item's push state machine. It maps (dispatcher exit code, attempt
# number, post-dispatcher write result) to the next action, so the
# safety-critical retry/fallback logic is unit-tested here rather than left to
# the model. The SKILL invokes this and renders the decision.
#
# Usage:
#   work-item-push-decide.sh --code <dispatcher-exit-code> --attempt <n> [--write-failed]
#
#   --code N         the exit code returned by work-item-create-remote.sh
#                    (0 success; 70 retryable; 71 terminal; 72 not-available;
#                    73 unrecognised).
#   --attempt N      which push attempt produced --code (1 = first, 2 = the one
#                    retry). Retry is offered ONLY after the first attempt.
#   --write-failed   set when the dispatcher returned 0 but the single local
#                    Write then failed — a LOCAL failure no dispatcher code can
#                    express. Maps to loud-terminal: the remote issue exists and
#                    the identifier is known, but nothing is on disk.
#
# Prints exactly one action keyword on stdout:
#   write-once     substitute the returned identifier into external_id and Write once
#   retry          offer one retry (re-enters this decision with --attempt 2)
#   local-save     save locally without external_id (retry exhausted / not-available
#                  / unrecognised) — informational message rendered by the caller
#   loud-terminal  save locally without external_id AND print loud non-idempotent
#                  guidance — a remote issue may already exist; do NOT re-run blindly
#
# Retry is NEVER offered for a terminal-post-create outcome: the remote create is
# non-idempotent, so retrying a post-send failure would risk a DUPLICATE issue.

# Exit-code taxonomy (E_DISPATCH_*) is owned by one sourced definition shared by
# every bridge and the decision scripts — see work-item-bridge-codes.sh.
_WPD_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=skills/work/scripts/work-item-bridge-codes.sh
source "$_WPD_DIR/work-item-bridge-codes.sh"

_wpd_usage() {
  cat <<'USAGE' >&2
Usage: work-item-push-decide.sh --code N --attempt N [--write-failed]
USAGE
}

_wpd_main() {
  local code="" attempt="" write_failed=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --code)
        code="$2"
        shift 2
        ;;
      --attempt)
        attempt="$2"
        shift 2
        ;;
      --write-failed)
        write_failed=1
        shift
        ;;
      --help | -h)
        _wpd_usage
        exit 0
        ;;
      *)
        _wpd_usage
        return 2
        ;;
    esac
  done

  if ! [[ "$code" =~ ^[0-9]+$ ]] || ! [[ "$attempt" =~ ^[0-9]+$ ]]; then
    printf 'work-item-push-decide.sh: --code and --attempt must be integers\n' >&2
    return 2
  fi

  if [[ "$code" == "0" ]]; then
    # Success path: the only failure left is the local Write itself.
    if ((write_failed)); then
      printf 'loud-terminal\n'
    else
      printf 'write-once\n'
    fi
    return 0
  fi

  case "$code" in
    "$E_DISPATCH_RETRYABLE")
      # Safe to retry — but only once. The retry re-enters with --attempt 2.
      if ((attempt <= 1)); then
        printf 'retry\n'
      else
        printf 'local-save\n'
      fi
      ;;
    "$E_DISPATCH_TERMINAL")
      printf 'loud-terminal\n'
      ;;
    "$E_DISPATCH_NOT_AVAILABLE" | "$E_DISPATCH_UNRECOGNISED")
      printf 'local-save\n'
      ;;
    *)
      # Unknown dispatcher code → conservative: a remote issue may exist.
      printf 'loud-terminal\n'
      ;;
  esac
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  _wpd_main "$@"
fi
