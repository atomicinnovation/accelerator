#!/usr/bin/env bash
set -euo pipefail

# Outputs the next sequential ticket number in NNNN format.
# Scans the configured tickets directory for the highest existing NNNN number
# and increments by one. Outputs "0001" if no tickets exist.
#
# Usage: ticket-next-number.sh [--count N]
#   --count N  Output N sequential numbers, one per line (default: 1)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/vcs-common.sh"

COUNT=1
while [ $# -gt 0 ]; do
  case "$1" in
    --count)
      if [ $# -lt 2 ]; then
        echo "Error: --count requires a value" >&2; exit 1
      fi
      COUNT="$2"; shift 2 ;;
    *) echo "Usage: ticket-next-number.sh [--count N]" >&2; exit 1 ;;
  esac
done

if ! [[ "$COUNT" =~ ^[1-9][0-9]*$ ]]; then
  echo "Error: --count requires a positive integer, got '$COUNT'" >&2
  exit 1
fi

REPO_ROOT=$(find_repo_root) || REPO_ROOT="$PWD"

TICKETS_PATH=$("$PLUGIN_ROOT/scripts/config-read-path.sh" tickets meta/tickets)

if [[ "$TICKETS_PATH" == /* ]]; then
  TICKETS_DIR="$TICKETS_PATH"
else
  TICKETS_DIR="$REPO_ROOT/$TICKETS_PATH"
fi

HIGHEST=0
if [ ! -d "$TICKETS_DIR" ]; then
  echo "Warning: tickets directory '$TICKETS_DIR' does not exist — defaulting to next number 0001. Run /accelerator:init or create the directory to persist tickets." >&2
else
  for f in "$TICKETS_DIR"/[0-9][0-9][0-9][0-9]-*; do
    [ -e "$f" ] || continue
    BASE=$(basename "$f")
    NUM=$(echo "$BASE" | grep -oE '^[0-9]+')
    if [ -n "$NUM" ] && [ "$((10#$NUM))" -gt "$HIGHEST" ]; then
      HIGHEST=$((10#$NUM))
    fi
  done
fi

# Clamp to the 4-digit number space. The scanning glob above requires
# exactly 4 digits followed by '-', so numbers beyond 9999 would be
# invisible on subsequent runs and cause collisions.
if [ "$((HIGHEST + COUNT))" -gt 9999 ]; then
  echo "Error: ticket number space exhausted (9999 reached); archive completed tickets to free numbers below 9999, or file an enhancement ticket requesting a 5-digit pattern" >&2
  # Still emit any numbers that fit before the boundary
  for ((i = 1; i <= COUNT; i++)); do
    NEXT=$((HIGHEST + i))
    [ "$NEXT" -gt 9999 ] && break
    printf "%04d\n" "$NEXT"
  done
  exit 1
fi

for ((i = 1; i <= COUNT; i++)); do
  printf "%04d\n" "$((HIGHEST + i))"
done
