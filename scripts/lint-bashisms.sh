#!/usr/bin/env bash
# lint-bashisms.sh — fail on a denylist of bash-4-only constructs so the
# migration framework's bash-3.2 floor (ADR-0016) is not silently regressed by
# a CI runner that ships bash 5.x.
#
# KNOWN-INCOMPLETE: this catches only the *enumerated* bash-4 constructs listed
# in the scanner below. It cannot prove bash-3.2 compatibility — a bash-4-only
# feature outside the list (e.g. the ${x@Q} transformation) would pass. The
# manual bash-3.2 replay remains the behavioural backstop; this lint is the
# regression gate for the known set. Heredoc bodies are a known minor
# false-positive surface (documented, not worked around).
#
# Usage: lint-bashisms.sh [FILE...]
#   With file arguments: scans exactly those files.
#   With none: scans tracked *.sh files, excluding fixtures, workspaces, and
#   test-helpers.sh.
# Per-line opt-out: append `# lint-bashisms: ignore` to a deliberate exception.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

files=()
if [ "$#" -gt 0 ]; then
  files=("$@")
else
  while IFS= read -r f; do
    case "$f" in
      */test-fixtures/* | workspaces/* | */test-helpers.sh) continue ;;
    esac
    [ -f "$REPO_ROOT/$f" ] && files+=("$REPO_ROOT/$f")
  done < <(cd "$REPO_ROOT" && git ls-files '*.sh')
  # bin/accelerator is extensionless, so the *.sh glob never matches it.
  [ -f "$REPO_ROOT/bin/accelerator" ] && files+=("$REPO_ROOT/bin/accelerator")
fi

[ "${#files[@]}" -eq 0 ] && exit 0

status=0
for f in "${files[@]}"; do
  # awk scans each line: skip opt-out lines, strip an unquoted trailing comment,
  # then test the remaining code against the bash-4 denylist. Exits 1 on any hit.
  if ! awk '
    /# lint-bashisms: ignore([[:space:]]|$)/ { next }
    {
      code = $0
      sub(/(^|[[:space:]])#.*$/, "", code)
      msg = ""
      if (code ~ /(declare|local|typeset)[[:space:]]+-A/)                 msg = "associative array (declare/local/typeset -A)"  # lint-bashisms: ignore
      else if (code ~ /(declare|local|typeset)[[:space:]]+-[[:alpha:]]*n/) msg = "nameref (declare/local/typeset -n)"            # lint-bashisms: ignore
      else if (code ~ /\$\{[^}]*:[-=+?][^}]*\\[{}]/)                      msg = "escaped brace in parameter-expansion default (bash 3.2 keeps the backslash)"  # lint-bashisms: ignore
      else if (code ~ /(^|[^[:alnum:]_])(mapfile|readarray)([^[:alnum:]_]|$)/) msg = "mapfile/readarray"                          # lint-bashisms: ignore
      else if (code ~ /\$\{[[:alnum:]_\[\]@*]+(\^|,)/)                    msg = "case-modification expansion (^^ ^ ,, ,)"        # lint-bashisms: ignore
      else if (code ~ /&>>/)                                             msg = "&>> append-both redirect"                       # lint-bashisms: ignore
      else if (code ~ /\|&/)                                             msg = "|& pipe-both"                                   # lint-bashisms: ignore
      else if (code ~ /\[-[[:digit:]]/)                                  msg = "negative array subscript"                       # lint-bashisms: ignore
      if (msg != "") {
        printf "%s:%d: bash-4 construct: %s\n", FILENAME, FNR, msg
        found = 1
      }
    }
    END { exit (found ? 1 : 0) }
  ' "$f"; then
    status=1
  fi
done

exit "$status"
