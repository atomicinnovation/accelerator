#!/usr/bin/env bash
set -euo pipefail

# Outputs the next sequential ADR number in NNNN format.
# Scans the configured decisions directory for the highest existing ADR-NNNN number
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

# Read configured decisions path, defaulting to meta/decisions
DECISIONS_PATH=$("$PLUGIN_ROOT/scripts/config-read-path.sh" decisions meta/decisions)

# Resolve: absolute paths used as-is, relative paths resolved against repo root
if [[ "$DECISIONS_PATH" == /* ]]; then
  DECISIONS_DIR="$DECISIONS_PATH"
else
  DECISIONS_DIR="$REPO_ROOT/$DECISIONS_PATH"
fi

# If directory doesn't exist, output sequential numbers starting from 0001
if [ ! -d "$DECISIONS_DIR" ]; then
  echo "Warning: decisions directory '$DECISIONS_DIR' does not exist — defaulting to next number 0001" >&2
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
