#!/usr/bin/env bash
set -euo pipefail

# work-item-normalise.sh — emit the canonical NORMALISED form of a work item's
# content, for the sync change-detection contract's two-way equality check.
#
# It does exactly ONE job (emit normalised content) with two input modes — the
# earlier two-flag `--hash`/`--hash-stdin` design is removed; hashing is composed
# downstream (`work-item-normalise.sh <file> | hash_sha256_stdin`).
#
# Usage:
#   work-item-normalise.sh <file>     # normalise a local work-item file
#   work-item-normalise.sh --stdin    # normalise content on stdin (remote body)
#
# Normalisation (the fixed minimum from the plan's Decisions Locked #3):
#   - drop the ignored top-level frontmatter keys (IGNORE_KEYS) — provenance /
#     remote-managed fields that are not authored content, so a bare re-save that
#     only restamps last_updated/revision is NOT a change;
#   - trim leading/trailing whitespace per line;
#   - strip trailing blank lines.
#
# The whole pass runs under LANG=C/LC_ALL=C so BSD-vs-GNU awk/sed locale handling
# cannot change the normalised bytes across machines — load-bearing for the
# committed, cross-machine baseline.
#
# --stdin mode normalises pre-projected remote content: the CALLER is responsible
# for projecting a remote payload into the comparable local shape and (per
# tracker) canonicalising it (Jira ADF via `jq -S`; Linear Markdown as-is) BEFORE
# piping it here. This mode therefore only trims; it does not split frontmatter.

export LANG=C LC_ALL=C

_WIN_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_WIN_REPO_SCRIPTS="$(cd "$_WIN_DIR/../../.." && pwd)/scripts"
# shellcheck source=scripts/config-common.sh
source "$_WIN_REPO_SCRIPTS/config-common.sh"

IGNORE_KEYS="last_updated last_updated_by id external_id updated_at revision"

_win_usage() {
  cat <<'USAGE' >&2
Usage:
  work-item-normalise.sh <file>
  work-item-normalise.sh --stdin
USAGE
}

# Trim each line (leading/trailing whitespace) and drop trailing blank lines.
_win_trim() {
  awk '
    { gsub(/^[[:space:]]+/, ""); gsub(/[[:space:]]+$/, ""); l[++n] = $0 }
    END {
      while (n > 0 && l[n] == "") n--
      for (i = 1; i <= n; i++) print l[i]
    }
  '
}

# Drop ignored top-level frontmatter keys, then trim as above. A top-level key is
# `name:` at column 0; indented (nested) keys never match, so only top-level
# ignored keys are dropped.
_win_filter_frontmatter() {
  awk -v keys="$IGNORE_KEYS" '
    BEGIN { m = split(keys, a, " "); for (i = 1; i <= m; i++) ign[a[i]] = 1 }
    {
      line = $0
      if (match(line, /^[A-Za-z_][A-Za-z0-9_]*:/)) {
        k = substr(line, 1, RLENGTH - 1)
        if (k in ign) next
      }
      gsub(/^[[:space:]]+/, "", line)
      gsub(/[[:space:]]+$/, "", line)
      l[++n] = line
    }
    END {
      while (n > 0 && l[n] == "") n--
      for (i = 1; i <= n; i++) print l[i]
    }
  '
}

_win_main() {
  case "${1-}" in
    --help | -h)
      _win_usage
      exit 0
      ;;
    --stdin)
      [ $# -eq 1 ] || {
        _win_usage
        exit 1
      }
      _win_trim
      ;;
    -*)
      _win_usage
      exit 1
      ;;
    "")
      _win_usage
      exit 1
      ;;
    *)
      [ $# -eq 1 ] || {
        _win_usage
        exit 1
      }
      local file="$1"
      if [ ! -f "$file" ]; then
        printf 'work-item-normalise.sh: no such file: %s\n' "$file" >&2
        exit 1
      fi
      local fm body
      fm=$(config_extract_frontmatter "$file" | _win_filter_frontmatter)
      body=$(config_extract_body "$file" | _win_trim)
      printf '%s\n%s\n' "$fm" "$body"
      ;;
  esac
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  _win_main "$@"
fi
