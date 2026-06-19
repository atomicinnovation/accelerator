#!/usr/bin/env bash
set -euo pipefail

# work-item-update-remote.sh — the WRITE/UPDATE counterpart to
# work-item-create-remote.sh. The create bridge only creates; this one replaces
# an already-synced item's whole content (summary/title + body) on the active
# tracker. It serves the dominant write in bidirectional sync (push of a
# local-ahead synced item) and the conflict-override push.
#
# Usage:
#   work-item-update-remote.sh --integration <sys> update \
#     --external-id <key> --title <t> --body-file <path> [--dry-run]
#
# update → replace the remote issue's summary/title and description/body in one
#   call, then exit 0. The bridge accepts a uniform --title + --body-file
#   interface and maps per tracker (Linear's update flow takes --description
#   inline, so the bridge reads the file and passes it inline — mirroring the
#   create bridge's input normalisation).
# --dry-run → forwards the tracker's --print-payload real dry-run; makes no write.
#
# Exit taxonomy (shared with the create/fetch bridges — work-item-bridge-codes.sh):
#   0 success
#   70 retryable — failure provably BEFORE the mutation (arg/auth/4xx-reject/
#      rate-limit) — safe to retry.
#   71 terminal — failure AT/AFTER the mutation. NEVER auto-retried due to
#      RESPONSE UNCERTAINTY: the request may have applied but the response was
#      lost, so the run must report rather than guess. (A whole-item update is
#      idempotent, unlike create, so the hazard is uncertainty, not double-apply.)
#   72 not-available — trello/github-issues update not built.
#   73 unrecognised <sys> — fail closed.

_WIUR_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=skills/work/scripts/work-item-bridge-codes.sh
source "$_WIUR_DIR/work-item-bridge-codes.sh"
_WIUR_INTEGRATIONS="$(cd "$_WIUR_DIR/../../integrations" && pwd)"

_wiur_usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-update-remote.sh --integration <sys> update \
    --external-id <key> --title <t> --body-file <path> [--dry-run]
  <sys> ∈ {linear, jira, trello, github-issues}
USAGE
}

# Map a jira update outcome (jira-update-flow 110-117 arg errors + propagated
# jira-request 11-23/34 transport codes) to the dispatcher taxonomy. Retryable =
# provably no mutation (arg/validation/auth/4xx-reject/rate-limit/unresolvable).
# Everything else — bad-response (16), 5xx (20), connect/DNS/timeout (21), and
# any unrecognised code — is conservatively terminal: the PUT may have applied.
_wiur_map_jira() {
  case "$1" in
    110 | 111 | 112 | 113 | 114 | 115 | 116 | 117) return "$E_DISPATCH_RETRYABLE" ;;
    11 | 12 | 13 | 14 | 15 | 17 | 19 | 22 | 34) return "$E_DISPATCH_RETRYABLE" ;;
    *) return "$E_DISPATCH_TERMINAL" ;;
  esac
}

# Map a linear update outcome to the dispatcher taxonomy. linear-update-flow
# propagates linear-graphql's code with no pre/post-send distinction of its own,
# so only the flow's own validation codes (110-114) and the transport codes that
# reject BEFORE applying (auth/creds/test-gate/rate-limit/complexity) are
# retryable. Everything else — crucially a 200-body GraphQL error (34) or a
# dropped/5xx response (16/20/21) where the mutation may have applied — is
# conservatively terminal.
_wiur_map_linear() {
  case "$1" in
    110 | 111 | 112 | 113 | 114) return "$E_DISPATCH_RETRYABLE" ;;
    11 | 18 | 22 | 23 | 25 | 27 | 29 | 35 | 36) return "$E_DISPATCH_RETRYABLE" ;;
    *) return "$E_DISPATCH_TERMINAL" ;;
  esac
}

_wiur_main() {
  local integration="" op="" external_id="" title="" body_file="" dry_run=0
  local title_set=0 body_set=0

  while [ $# -gt 0 ]; do
    case "$1" in
      --integration)
        integration="$2"
        shift 2
        ;;
      update)
        op="update"
        shift
        ;;
      --external-id)
        external_id="$2"
        shift 2
        ;;
      --title)
        title="$2"
        title_set=1
        shift 2
        ;;
      --body-file)
        body_file="$2"
        body_set=1
        shift 2
        ;;
      --dry-run)
        dry_run=1
        shift
        ;;
      --help | -h)
        _wiur_usage
        exit 0
        ;;
      *)
        _wiur_usage
        return "$E_DISPATCH_UNRECOGNISED"
        ;;
    esac
  done

  if [ "$op" != "update" ]; then
    _wiur_usage
    return "$E_DISPATCH_UNRECOGNISED"
  fi
  if [ -z "$external_id" ]; then
    printf 'work-item-update-remote.sh: --external-id is required\n' >&2
    return "$E_DISPATCH_RETRYABLE"
  fi
  if ((!title_set)) || [ -z "$title" ]; then
    printf 'work-item-update-remote.sh: --title is required\n' >&2
    return "$E_DISPATCH_RETRYABLE"
  fi
  if ((!body_set)); then
    printf 'work-item-update-remote.sh: --body-file is required\n' >&2
    return "$E_DISPATCH_RETRYABLE"
  fi
  if [ ! -f "$body_file" ]; then
    printf 'work-item-update-remote.sh: --body-file not found: %q\n' "$body_file" >&2
    return "$E_DISPATCH_RETRYABLE"
  fi

  local rc=0
  case "$integration" in
    jira)
      local -a args=("$external_id" --summary "$title" --body-file "$body_file" --quiet)
      ((dry_run)) && args+=(--print-payload)
      bash "$_WIUR_INTEGRATIONS/jira/scripts/jira-update-flow.sh" "${args[@]}" || rc=$?
      if [ "$rc" -ne 0 ]; then
        _wiur_map_jira "$rc"
        return $?
      fi
      ;;
    linear)
      local body
      body=$(cat "$body_file")
      local -a args=("$external_id" --title "$title" --description "$body" --quiet)
      ((dry_run)) && args+=(--print-payload)
      bash "$_WIUR_INTEGRATIONS/linear/scripts/linear-update-flow.sh" "${args[@]}" || rc=$?
      if [ "$rc" -ne 0 ]; then
        _wiur_map_linear "$rc"
        return $?
      fi
      ;;
    trello | github-issues)
      printf 'E_DISPATCH_NOT_AVAILABLE: update support for %s is not built yet (see work items 0049/0050)\n' \
        "$integration" >&2
      return "$E_DISPATCH_NOT_AVAILABLE"
      ;;
    *)
      printf 'E_DISPATCH_UNRECOGNISED: unknown or empty work.integration value: %q\n' \
        "$integration" >&2
      return "$E_DISPATCH_UNRECOGNISED"
      ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _wiur_main "$@"
fi
