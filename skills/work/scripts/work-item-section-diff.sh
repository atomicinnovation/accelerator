#!/usr/bin/env bash
set -euo pipefail

# work-item-section-diff.sh — split two work-item-shaped files into named sections
# and emit a per-section textual diff, so large items stay reviewable during
# conflict resolution. Used by /sync-work-items' bidirectional conflict prompt.
#
# Usage:
#   work-item-section-diff.sh <local-file> <remote-file>
#
# Sections: `frontmatter` (the --- block), `(preamble)` (body before the first
# `## ` heading), then one section per `## ` heading (Summary, Context,
# Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions,
# Technical Notes, Drafting Notes, References, …). Headings are discovered from
# BOTH sides and unioned in first-appearance order.
#
# Direction is FIXED and documented: local is the baseline `-` side, remote is the
# change `+` side (matching the default-accept side of the prompt). Only the
# POSIX-portable `diff -u` surface is used (no GNU-only long flags, no --color).
# Byte-equality of a section is decided by the normaliser + hash — NOT by diff's
# exit status (whose 1=differences semantics is a portability trap) — so
# whitespace-only / restamp-only differences are normalised away and the section
# is omitted.

_WISD_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_WISD_REPO_SCRIPTS="$(cd "$_WISD_DIR/../../.." && pwd)/scripts"
# shellcheck source=scripts/config-common.sh
source "$_WISD_REPO_SCRIPTS/config-common.sh"
# shellcheck source=scripts/hash-common.sh
source "$_WISD_REPO_SCRIPTS/hash-common.sh"
_WISD_NORMALISE="$_WISD_DIR/work-item-normalise.sh"

_wisd_usage() {
  cat <<'USAGE' >&2
Usage: work-item-section-diff.sh <local-file> <remote-file>
USAGE
}

# Content of one section of a file.
_wisd_section() {
  local file="$1" name="$2"
  case "$name" in
    frontmatter)
      config_extract_frontmatter "$file"
      ;;
    "(preamble)")
      config_extract_body "$file" | awk '/^## / { exit } { print }'
      ;;
    *)
      config_extract_body "$file" | awk -v h="## $name" '
        $0 == h { grab = 1; next }
        grab && /^## / { grab = 0 }
        grab { print }
      '
      ;;
  esac
}

# Normalised-content hash of a string (for the byte-equality decision).
_wisd_norm_hash() {
  printf '%s' "$1" | bash "$_WISD_NORMALISE" --stdin | hash_sha256_stdin
}

# Ordered, de-duplicated `## ` heading list across both files.
_wisd_heading_union() {
  local a="$1" b="$2"
  {
    grep '^## ' "$a" 2>/dev/null || true
    grep '^## ' "$b" 2>/dev/null || true
  } | sed 's/^## //' | awk '!seen[$0]++'
}

_wisd_main() {
  local local_file="${1-}" remote_file="${2-}"
  if [ -z "$local_file" ] || [ -z "$remote_file" ]; then
    _wisd_usage
    return 2
  fi
  if [ ! -f "$local_file" ] || [ ! -f "$remote_file" ]; then
    echo "work-item-section-diff.sh: both arguments must be files" >&2
    return 2
  fi

  local tmp
  tmp=$(mktemp -d)
  # shellcheck disable=SC2064  # expand tmp now so the trap removes this dir
  trap "rm -rf '$tmp'" EXIT

  local -a sections=(frontmatter "(preamble)")
  local h
  while IFS= read -r h; do
    [ -n "$h" ] && sections+=("$h")
  done < <(_wisd_heading_union "$local_file" "$remote_file")

  local any=0 name lc rc
  for name in "${sections[@]}"; do
    lc=$(_wisd_section "$local_file" "$name")
    rc=$(_wisd_section "$remote_file" "$name")
    # Byte-equal after normalisation → omit.
    if [ "$(_wisd_norm_hash "$lc")" = "$(_wisd_norm_hash "$rc")" ]; then
      continue
    fi
    any=1
    printf '=== %s (- LOCAL / + REMOTE) ===\n' "$name"
    printf '%s\n' "$lc" >"$tmp/LOCAL"
    printf '%s\n' "$rc" >"$tmp/REMOTE"
    # diff -u exits 1 when files differ; that is expected here, not an error.
    (cd "$tmp" && diff -u LOCAL REMOTE) || true
    printf '\n'
  done

  if [ "$any" -eq 0 ]; then
    printf '(no differing sections after normalisation)\n'
  fi
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _wisd_main "$@"
fi
