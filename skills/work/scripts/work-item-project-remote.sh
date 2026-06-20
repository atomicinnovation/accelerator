#!/usr/bin/env bash
set -euo pipefail

# work-item-project-remote.sh — the per-tracker SEAM that projects a remote
# `show` payload into the comparable local shape and (per tracker) canonicalises
# it, so the remote side of change-detection is judged on the same rules as the
# local side. It is the single owner of the projection recipe, used identically
# wherever a remote_hash / remote_updated_at is computed (the sync apply step and
# the list/sync read paths), so the persisted baseline cannot drift between the
# path that WRITES it and the path that later READS it.
#
# Usage (reads the raw show JSON on stdin):
#   work-item-project-remote.sh --integration <sys> updated   # → remote updated
#   work-item-project-remote.sh --integration <sys> body      # → projected body
#
# updated → the tracker's remote `updated` field, raw (jira fields.updated,
#           linear updatedAt). Compared lexically against a baseline written from
#           the same tracker, so the raw string is correct.
# body    → the projected + canonicalised content to pipe into
#           `work-item-normalise.sh --stdin`. jira: summary line + ADF
#           description through `jq -S` (key-sorted, so ordering/whitespace
#           cannot flip equality). linear: title line + Markdown description
#           verbatim (Markdown-native, NO jq -S). The body shape (a title line
#           then the description) is fixed so a remote_hash is stable across runs.

_wipr_usage() {
  cat <<'USAGE' >&2
Usage (raw show JSON on stdin):
  work-item-project-remote.sh --integration <sys> updated
  work-item-project-remote.sh --integration <sys> body
  <sys> ∈ {jira, linear}
USAGE
}

_wipr_main() {
  local integration="" op=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --integration)
        integration="$2"
        shift 2
        ;;
      updated | body)
        op="$1"
        shift
        ;;
      --help | -h)
        _wipr_usage
        exit 0
        ;;
      *)
        _wipr_usage
        return 2
        ;;
    esac
  done
  [ -n "$op" ] || {
    _wipr_usage
    return 2
  }

  local show
  show=$(cat)

  case "$integration" in
    jira)
      case "$op" in
        updated) printf '%s' "$show" | jq -r '.fields.updated // ""' ;;
        body)
          local summary desc
          summary=$(printf '%s' "$show" | jq -r '.fields.summary // ""')
          desc=$(printf '%s' "$show" | jq -cS '.fields.description // null')
          printf '%s\n%s\n' "$summary" "$desc"
          ;;
      esac
      ;;
    linear)
      case "$op" in
        updated) printf '%s' "$show" | jq -r '.data.issue.updatedAt // ""' ;;
        body)
          local title desc
          title=$(printf '%s' "$show" | jq -r '.data.issue.title // ""')
          desc=$(printf '%s' "$show" | jq -r '.data.issue.description // ""')
          printf '%s\n%s\n' "$title" "$desc"
          ;;
      esac
      ;;
    *)
      printf 'work-item-project-remote.sh: unsupported integration: %q\n' \
        "$integration" >&2
      return 2
      ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _wipr_main "$@"
fi
