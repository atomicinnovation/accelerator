#!/usr/bin/env bash
set -euo pipefail

# work-item-sync-apply.sh — the thin, fault-injectable apply helper that performs
# ONE item's per-item commit sequence in one auditable place, so the resumability
# contract (Decision #4: side-effect FIRST, then update that id's baseline entry
# LAST; global timestamp advanced only on clean completion) is CI-tested rather
# than left to SKILL prose.
#
# Sub-actions (a thin dispatcher delegating to one short function each):
#   push   --integration <s> --external-id <k> --id <id> --file <localfile>
#          --title <t> --body-file <pushbody>
#       Push the local-ahead item: update bridge → (fault hook) → post-push show
#       to capture the AUTHORITATIVE remote_updated_at + remote_hash (always from
#       show fidelity, never from the pushed bytes) → set the baseline entry.
#       A non-zero update outcome returns that dispatch code and leaves the
#       baseline entry UNSET (the next run re-classifies; a 71/terminal is never
#       auto-retried by the caller).
#
#   pull   --id <id> --file <localfile> --new-content-file <reconstructed>
#          --remote-updated <iso> --remote-body-file <projected-canonical>
#       Overwrite the local file from the remote: atomic_write → (fault hook) →
#       set the baseline entry. local_hash is hashed from the POST-overwrite file
#       and remote_hash from the projection actually written — never the pre-pull
#       content (which would self-corrupt the baseline into a phantom
#       locally-modified/conflict on the next run).
#
#   finalise --timestamp <epoch>
#       Advance the global pre-filter timestamp (run-START epoch, captured by the
#       SKILL before any item is read). A sibling sub-action, never folded into
#       the per-item path; the SKILL calls it once on clean completion and NEVER
#       under --preview (a preview that advanced the timestamp would poison the
#       next real run's pre-filter).
#
# Test seam (gated on ACCELERATOR_TEST_MODE=1): WORK_SYNC_FAIL_AFTER=side-effect
# aborts (exit 99) BETWEEN the side-effect and the baseline set, so a test can
# assert re-run idempotency for a non-VCS-recoverable remote write.

_WISA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_WISA_REPO_SCRIPTS="$(cd "$_WISA_DIR/../../.." && pwd)/scripts"
# shellcheck source=scripts/atomic-common.sh
source "$_WISA_REPO_SCRIPTS/atomic-common.sh"
# shellcheck source=scripts/hash-common.sh
source "$_WISA_REPO_SCRIPTS/hash-common.sh"

_WISA_UPDATE="$_WISA_DIR/work-item-update-remote.sh"
_WISA_FETCH="$_WISA_DIR/work-item-fetch-remote.sh"
_WISA_NORMALISE="$_WISA_DIR/work-item-normalise.sh"
_WISA_PROJECT="$_WISA_DIR/work-item-project-remote.sh"
_WISA_BASELINE="$_WISA_DIR/work-item-sync-baseline.sh"

_wisa_usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-sync-apply.sh push --integration <s> --external-id <k> --id <id>
    --file <localfile> --title <t> --body-file <pushbody>
  work-item-sync-apply.sh pull --id <id> --file <localfile>
    --new-content-file <reconstructed> --remote-updated <iso>
    --remote-body-file <projected-canonical>
  work-item-sync-apply.sh finalise --timestamp <epoch>
USAGE
}

# The fault hook: abort between the side-effect and the baseline set.
_wisa_fault_check() {
  if [ "${ACCELERATOR_TEST_MODE:-}" = "1" ] &&
    [ "${WORK_SYNC_FAIL_AFTER:-}" = "side-effect" ]; then
    echo "WORK_SYNC_FAIL_AFTER: injected abort after side-effect, before baseline set" >&2
    exit 99
  fi
}

apply_push() {
  local integration="" external_id="" id="" file="" title="" body_file=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --integration)
        integration="$2"
        shift 2
        ;;
      --external-id)
        external_id="$2"
        shift 2
        ;;
      --id)
        id="$2"
        shift 2
        ;;
      --file)
        file="$2"
        shift 2
        ;;
      --title)
        title="$2"
        shift 2
        ;;
      --body-file)
        body_file="$2"
        shift 2
        ;;
      *)
        _wisa_usage
        return 2
        ;;
    esac
  done

  # --- side-effect: push to the remote (NOT the create bridge) --------------
  local rc=0
  bash "$_WISA_UPDATE" --integration "$integration" update \
    --external-id "$external_id" --title "$title" --body-file "$body_file" || rc=$?
  if [ "$rc" -ne 0 ]; then
    # Leave the baseline entry UNSET so the next run re-classifies. The caller
    # interprets the dispatch code (71/terminal is never auto-retried).
    return "$rc"
  fi

  _wisa_fault_check

  # --- post-push show → authoritative remote baseline ----------------------
  local show remote_updated="" remote_hash="" local_hash
  if show=$(bash "$_WISA_FETCH" --integration "$integration" show \
    --external-id "$external_id" 2>/dev/null); then
    remote_updated=$(printf '%s' "$show" |
      bash "$_WISA_PROJECT" --integration "$integration" updated)
    remote_hash=$(printf '%s' "$show" |
      bash "$_WISA_PROJECT" --integration "$integration" body |
      bash "$_WISA_NORMALISE" --stdin | hash_sha256_stdin)
  fi
  local_hash=$(bash "$_WISA_NORMALISE" "$file" | hash_sha256_stdin)

  # --- baseline set (LAST) -------------------------------------------------
  bash "$_WISA_BASELINE" set "$id" "$remote_updated" "$remote_hash" "$local_hash"
}

apply_pull() {
  local id="" file="" new_content_file="" remote_updated="" remote_body_file=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --id)
        id="$2"
        shift 2
        ;;
      --file)
        file="$2"
        shift 2
        ;;
      --new-content-file)
        new_content_file="$2"
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
      *)
        _wisa_usage
        return 2
        ;;
    esac
  done

  # --- side-effect: overwrite the local file (atomic, never truncate-rewrite)
  atomic_write "$file" <"$new_content_file"

  _wisa_fault_check

  # --- baseline set (LAST) from the POST-overwrite state -------------------
  local local_hash remote_hash
  local_hash=$(bash "$_WISA_NORMALISE" "$file" | hash_sha256_stdin)
  remote_hash=$(bash "$_WISA_NORMALISE" --stdin <"$remote_body_file" | hash_sha256_stdin)
  bash "$_WISA_BASELINE" set "$id" "$remote_updated" "$remote_hash" "$local_hash"
}

apply_finalise() {
  local timestamp=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --timestamp)
        timestamp="$2"
        shift 2
        ;;
      *)
        _wisa_usage
        return 2
        ;;
    esac
  done
  bash "$_WISA_BASELINE" set-timestamp "$timestamp"
}

_wisa_main() {
  local sub="${1-}"
  case "$sub" in
    --help | -h)
      _wisa_usage
      exit 0
      ;;
    push)
      shift
      apply_push "$@"
      ;;
    pull)
      shift
      apply_pull "$@"
      ;;
    finalise)
      shift
      apply_finalise "$@"
      ;;
    *)
      _wisa_usage
      return 2
      ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _wisa_main "$@"
fi
