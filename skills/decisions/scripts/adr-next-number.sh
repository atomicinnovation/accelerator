#!/usr/bin/env bash
set -euo pipefail

# Outputs the next sequential ADR number in NNNN format.
# Scans meta/decisions/ for the highest existing ADR-NNNN number
# and increments by one. Outputs "0001" if no ADRs exist.
#
# Usage: adr-next-number.sh [--count N]
#   --count N  Output N sequential numbers, one per line (default: 1)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Source VCS common for repo root detection
source "$PLUGIN_ROOT/scripts/vcs-common.sh"

COUNT=1
while [ $# -gt 0 ]; do
  case "$1" in
    --count) COUNT="$2"; shift 2 ;;
    *) echo "Usage: adr-next-number.sh [--count N]" >&2; exit 1 ;;
  esac
done

# Validate --count is a positive integer
if ! [[ "$COUNT" =~ ^[1-9][0-9]*$ ]]; then
  echo "Error: --count requires a positive integer, got '$COUNT'" >&2
  exit 1
fi

REPO_ROOT=$(find_repo_root) || REPO_ROOT="$PWD"
DECISIONS_DIR="$REPO_ROOT/meta/decisions"

if [ ! -d "$DECISIONS_DIR" ]; then
  for ((i = 1; i <= COUNT; i++)); do
    printf "%04d\n" "$i"
  done
  exit 0
fi

# Find highest ADR number using glob (avoids fragile ls parsing)
HIGHEST=0
for f in "$DECISIONS_DIR"/ADR-[0-9][0-9][0-9][0-9]*; do
  [ -e "$f" ] || continue
  BASE=$(basename "$f")
  NUM=$(echo "$BASE" | sed 's/^ADR-//' | grep -oE '^[0-9]+')
  if [ -n "$NUM" ] && [ "$((10#$NUM))" -gt "$HIGHEST" ]; then
    HIGHEST=$((10#$NUM))
  fi
done

for ((i = 1; i <= COUNT; i++)); do
  printf "%04d\n" "$((HIGHEST + i))"
done
